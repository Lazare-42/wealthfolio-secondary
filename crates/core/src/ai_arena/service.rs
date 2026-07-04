use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::Value;

use crate::assets::{Asset, AssetServiceTrait, InstrumentType};
use crate::errors::ValidationError;
use crate::quotes::QuoteServiceTrait;
use crate::{Error, Result};

use super::model::*;
use super::traits::{AiArenaDecisionRunner, AiArenaRepositoryTrait, AiArenaServiceTrait};

pub struct AiArenaService {
    repository: Arc<dyn AiArenaRepositoryTrait>,
    quote_service: Arc<dyn QuoteServiceTrait>,
    asset_service: Arc<dyn AssetServiceTrait>,
    decision_runner: Option<Arc<dyn AiArenaDecisionRunner>>,
}

impl AiArenaService {
    pub fn new(
        repository: Arc<dyn AiArenaRepositoryTrait>,
        quote_service: Arc<dyn QuoteServiceTrait>,
        asset_service: Arc<dyn AssetServiceTrait>,
        decision_runner: Option<Arc<dyn AiArenaDecisionRunner>>,
    ) -> Self {
        Self {
            repository,
            quote_service,
            asset_service,
            decision_runner,
        }
    }

    fn validate_agent_input(input: &CreateArenaAgentRequest) -> Result<()> {
        require_non_empty(&input.name, "Agent name cannot be empty")?;
        require_non_empty(&input.provider_id, "Provider id cannot be empty")?;
        require_non_empty(&input.model_id, "Model id cannot be empty")?;
        Ok(())
    }

    fn validate_challenge_input(input: &CreateArenaChallengeRequest) -> Result<()> {
        require_non_empty(&input.name, "Challenge name cannot be empty")?;
        if input.market.trim() != "us-stock" {
            return invalid("AI Arena currently supports only us-stock challenges");
        }
        require_positive(input.initial_cash, "initialCash must be positive")?;
        require_pct(input.max_position_pct, "maxPositionPct")?;
        require_pct(input.max_drawdown_pct, "maxDrawdownPct")?;
        let start_at = parse_challenge_time(input.start_at.as_deref(), "startAt")?;
        let end_at = parse_challenge_time(input.end_at.as_deref(), "endAt")?;
        if let (Some(start_at), Some(end_at)) = (start_at, end_at) {
            if end_at <= start_at {
                return invalid("endAt must be after startAt");
            }
        }
        if let Some(scheduled_time) = input
            .scheduled_time_local
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            parse_hh_mm(scheduled_time, "scheduledTimeLocal")?;
        }
        Ok(())
    }

    async fn build_prompt(
        &self,
        agent: &ArenaAgent,
        challenge: &ArenaChallenge,
        portfolio: &ArenaPortfolio,
    ) -> String {
        let positions_json = serde_json::to_string_pretty(&portfolio.positions).unwrap_or_default();
        let trades: Vec<_> = portfolio.trades.iter().rev().take(20).cloned().collect();
        let trades_json = serde_json::to_string_pretty(&trades).unwrap_or_default();
        let universe = if challenge.universe.is_empty() {
            "Open US stocks and equity ETFs already resolvable by Wealthfolio market data."
                .to_string()
        } else {
            challenge.universe.join(", ")
        };

        format!(
            "You are competing in Wealthfolio AI Arena.\n\
Rules:\n\
- Paper trading only. Never request real orders.\n\
- Long-only buy/sell orders. No shorts, options, futures, crypto, forex, bonds, or bond ETFs.\n\
- Cash earns 0.\n\
- Maximum position size: {:.2}% of portfolio value.\n\
- Order notional must be positive USD.\n\
- Return strict JSON only, no markdown.\n\n\
Challenge:\n\
- name: {}\n\
- market: {}\n\
- scoring: {}\n\
- universe: {}\n\
- agent: {}\n\n\
Current portfolio:\n\
- cash: {:.2}\n\
- totalValue: {:.2}\n\
- returnPct: {:.4}\n\
- maxDrawdownPct: {:.4}\n\
- positions: {}\n\
- recentTrades: {}\n\n\
Return JSON shape:\n\
{{\"summary\":\"short rationale\",\"orders\":[{{\"symbol\":\"AAPL\",\"side\":\"buy\",\"notional\":1000,\"rationale\":\"why\"}}],\"theses\":[{{\"symbol\":\"AAPL\",\"rating\":\"bullish\",\"confidence\":0.7,\"horizon\":\"3m\",\"thesis\":\"...\",\"risks\":[\"...\"],\"catalysts\":[\"...\"]}}],\"memory\":\"optional\"}}\n\
Use at most 5 orders. It is valid to return an empty orders array.",
            challenge.max_position_pct,
            challenge.name,
            challenge.market,
            challenge.scoring_method.as_str(),
            universe,
            agent.name,
            portfolio.cash,
            portfolio.total_value,
            portfolio.return_pct,
            portfolio.max_drawdown_pct,
            positions_json,
            trades_json,
        )
    }

    async fn run_decision(
        &self,
        agent: &ArenaAgent,
        prompt: String,
    ) -> Result<(String, Value, ArenaAgentDecision)> {
        let runner = self.decision_runner.as_ref().ok_or_else(|| {
            Error::Validation(ValidationError::InvalidInput(
                "No AI Arena decision runner is configured".to_string(),
            ))
        })?;
        let raw = runner
            .run_decision(ArenaDecisionRequest {
                provider_id: agent.provider_id.clone(),
                model_id: agent.model_id.clone(),
                system_prompt: agent.persona.clone(),
                prompt,
            })
            .await?;
        let parsed = parse_model_json(&raw)?;
        let decision: ArenaAgentDecision = serde_json::from_value(parsed.clone()).map_err(|e| {
            Error::Validation(ValidationError::InvalidInput(format!(
                "Arena decision JSON did not match schema: {e}"
            )))
        })?;
        Ok((raw, parsed, decision))
    }

    async fn apply_decision(
        &self,
        run: &ArenaRun,
        challenge: &ArenaChallenge,
        participant: &ArenaParticipant,
        decision: ArenaAgentDecision,
    ) -> Result<usize> {
        let mut rejected = 0usize;
        for thesis in decision.theses {
            if thesis.thesis.trim().is_empty() {
                continue;
            }
            let input = CreateCompanyThesisRequest {
                symbol: thesis.symbol,
                agent_id: Some(run.agent_id.clone()),
                challenge_id: Some(run.challenge_id.clone()),
                run_id: Some(run.id.clone()),
                rating: thesis.rating,
                confidence: thesis.confidence,
                horizon: thesis.horizon,
                thesis: thesis.thesis,
                risks: thesis.risks,
                catalysts: thesis.catalysts,
            };
            self.repository
                .create_thesis(CompanyThesis::new(input))
                .await?;
        }

        for order in decision.orders.into_iter().take(5) {
            let trade = self
                .normalize_order(run, challenge, participant, order)
                .await?;
            if trade.status == ArenaTradeStatus::Rejected {
                rejected += 1;
            }
            self.repository.create_trade(trade).await?;
        }
        Ok(rejected)
    }

    async fn normalize_order(
        &self,
        run: &ArenaRun,
        challenge: &ArenaChallenge,
        participant: &ArenaParticipant,
        order: ArenaOrderDecision,
    ) -> Result<ArenaTrade> {
        let symbol = normalize_symbol(&order.symbol);
        let side = ArenaTradeSide::parse(&order.side);
        let raw_notional = order.notional.unwrap_or(0.0);
        // Decimal cannot represent NaN/inf, so clamp bad model output to zero
        // for the rejected-trade record.
        let requested_notional = Decimal::from_f64(raw_notional.max(0.0)).unwrap_or(Decimal::ZERO);
        let rejected = |reason: String, price: Decimal| {
            Ok(ArenaTrade::new(
                challenge.id.clone(),
                participant.id.clone(),
                Some(run.id.clone()),
                symbol.clone(),
                side.unwrap_or(ArenaTradeSide::Buy),
                Decimal::ZERO,
                price,
                requested_notional,
                ArenaTradeStatus::Rejected,
                order.rationale.clone(),
                Some(reason),
            ))
        };

        if symbol.is_empty() {
            return rejected("symbol is required".to_string(), Decimal::ZERO);
        }
        let Some(side) = side else {
            return rejected("side must be buy or sell".to_string(), Decimal::ZERO);
        };
        if !raw_notional.is_finite() || requested_notional <= Decimal::ZERO {
            return rejected(
                "notional must be a positive number".to_string(),
                Decimal::ZERO,
            );
        }
        if !challenge.universe.is_empty() && !challenge.universe.iter().any(|s| s == &symbol) {
            return rejected(
                format!("{symbol} is outside the challenge universe"),
                Decimal::ZERO,
            );
        }

        let (resolved_symbol, price) = match self.resolve_allowed_price(&symbol).await {
            Ok(value) => value,
            Err(err) => return rejected(err.to_string(), Decimal::ZERO),
        };
        if price <= Decimal::ZERO {
            return rejected(format!("No usable price for {resolved_symbol}"), price);
        }

        let portfolio = self.build_portfolio(participant).await?;
        match side {
            ArenaTradeSide::Buy => {
                if requested_notional > portfolio.cash {
                    return rejected("insufficient cash".to_string(), price);
                }
                let existing_value = portfolio
                    .positions
                    .iter()
                    .find(|p| p.symbol == resolved_symbol)
                    .map(|p| p.market_value)
                    .unwrap_or(Decimal::ZERO);
                let post_position_pct = ((existing_value + requested_notional)
                    / portfolio.total_value.max(dec!(0.0001)))
                    * dec!(100);
                if post_position_pct > challenge.max_position_pct {
                    return rejected(
                        format!(
                            "position would exceed {:.2}% max position limit",
                            challenge.max_position_pct
                        ),
                        price,
                    );
                }
                Ok(ArenaTrade::new(
                    challenge.id.clone(),
                    participant.id.clone(),
                    Some(run.id.clone()),
                    resolved_symbol,
                    side,
                    requested_notional / price,
                    price,
                    requested_notional,
                    ArenaTradeStatus::Executed,
                    order.rationale,
                    None,
                ))
            }
            ArenaTradeSide::Sell => {
                let Some(position) = portfolio
                    .positions
                    .iter()
                    .find(|p| p.symbol == resolved_symbol)
                    .cloned()
                else {
                    return rejected("cannot sell a symbol with no position".to_string(), price);
                };
                let holding_value = position.quantity * price;
                if requested_notional > holding_value {
                    return rejected(
                        "sell notional exceeds current position value".to_string(),
                        price,
                    );
                }
                let quantity = (requested_notional / price).min(position.quantity);
                Ok(ArenaTrade::new(
                    challenge.id.clone(),
                    participant.id.clone(),
                    Some(run.id.clone()),
                    resolved_symbol,
                    side,
                    quantity,
                    price,
                    quantity * price,
                    ArenaTradeStatus::Executed,
                    order.rationale,
                    None,
                ))
            }
        }
    }

    async fn resolve_allowed_price(&self, symbol: &str) -> Result<(String, Decimal)> {
        if let Some(asset) = self.find_local_asset(symbol)? {
            self.validate_asset_allowed(&asset)?;
            let quote = self
                .quote_service
                .get_latest_quote(&asset.id)
                .or_else(|_| {
                    self.quote_service
                        .get_latest_quote(asset.instrument_symbol.as_deref().unwrap_or(symbol))
                })?;
            return Ok((display_symbol_for_asset(&asset, symbol), quote.close));
        }

        let results = self
            .quote_service
            .search_symbol_with_currency(symbol, Some("USD"))
            .await?;
        let candidate = results
            .into_iter()
            .find(|result| {
                let candidate_symbol = result
                    .canonical_symbol
                    .as_deref()
                    .unwrap_or(result.symbol.as_str());
                candidate_symbol.eq_ignore_ascii_case(symbol)
                    || result.symbol.eq_ignore_ascii_case(symbol)
            })
            .ok_or_else(|| {
                Error::Validation(ValidationError::InvalidInput(format!(
                    "No supported market data result for {symbol}"
                )))
            })?;

        validate_external_quote_type(&candidate.quote_type, &candidate.short_name)?;
        let instrument_type = InstrumentType::from_external_str(&candidate.quote_type);
        let resolved = self
            .quote_service
            .resolve_symbol_quote(
                candidate
                    .canonical_symbol
                    .as_deref()
                    .unwrap_or(candidate.symbol.as_str()),
                candidate.exchange_mic.as_deref(),
                instrument_type.as_ref(),
                candidate.currency.as_deref().or(Some("USD")),
                candidate.provider_id.as_deref(),
            )
            .await?;
        let price = resolved.price.ok_or_else(|| {
            Error::Validation(ValidationError::InvalidInput(format!(
                "No usable provider price for {symbol}"
            )))
        })?;
        Ok((
            normalize_symbol(
                candidate
                    .canonical_symbol
                    .as_deref()
                    .unwrap_or(candidate.symbol.as_str()),
            ),
            price,
        ))
    }

    fn find_local_asset(&self, symbol: &str) -> Result<Option<Asset>> {
        let symbol = normalize_symbol(symbol);
        Ok(self.asset_service.get_assets()?.into_iter().find(|asset| {
            asset.id.eq_ignore_ascii_case(&symbol)
                || asset
                    .display_code
                    .as_deref()
                    .map(|value| value.eq_ignore_ascii_case(&symbol))
                    .unwrap_or(false)
                || asset
                    .instrument_symbol
                    .as_deref()
                    .map(|value| value.eq_ignore_ascii_case(&symbol))
                    .unwrap_or(false)
        }))
    }

    fn validate_asset_allowed(&self, asset: &Asset) -> Result<()> {
        if !asset.is_equity_like() || asset.is_option() || asset.is_bond() || asset.is_metal() {
            return invalid("AI Arena allows only stocks and equity ETFs");
        }
        let label = format!(
            "{} {}",
            asset.display_code.as_deref().unwrap_or(""),
            asset.name.as_deref().unwrap_or("")
        );
        if looks_like_bond_fund(&label) {
            return invalid("AI Arena rejects bond and fixed-income funds");
        }
        Ok(())
    }

    async fn build_portfolio(&self, participant: &ArenaParticipant) -> Result<ArenaPortfolio> {
        self.build_portfolio_with_marks(participant, true).await
    }

    async fn build_portfolio_with_marks(
        &self,
        participant: &ArenaParticipant,
        mark_open_positions: bool,
    ) -> Result<ArenaPortfolio> {
        let challenge = self.repository.get_challenge(&participant.challenge_id)?;
        let agent = self.repository.get_agent(&participant.agent_id)?;
        let trades = self
            .repository
            .list_trades_for_participant(&participant.id)?;
        let snapshots = self
            .repository
            .list_snapshots_for_participant(&participant.id)?;
        let executed: Vec<ArenaTrade> = trades
            .iter()
            .filter(|trade| trade.status == ArenaTradeStatus::Executed)
            .cloned()
            .collect();

        let mut cash = participant.starting_cash;
        let mut positions: HashMap<String, ReplayPosition> = HashMap::new();
        for trade in &executed {
            match trade.side {
                ArenaTradeSide::Buy => {
                    cash -= trade.notional;
                    let entry = positions.entry(trade.symbol.clone()).or_default();
                    let previous_cost = entry.quantity * entry.avg_entry_price;
                    entry.quantity += trade.quantity;
                    if entry.quantity > Decimal::ZERO {
                        entry.avg_entry_price = (previous_cost + trade.notional) / entry.quantity;
                    }
                    entry.last_price = trade.price;
                }
                ArenaTradeSide::Sell => {
                    cash += trade.notional;
                    if let Some(entry) = positions.get_mut(&trade.symbol) {
                        entry.quantity -= trade.quantity;
                        entry.last_price = trade.price;
                        if entry.quantity <= dec!(0.00000001) {
                            positions.remove(&trade.symbol);
                        }
                    }
                }
            }
        }

        let symbols: Vec<String> = positions.keys().cloned().collect();
        let latest_quotes = if mark_open_positions {
            self.quote_service
                .get_latest_quotes(&symbols)
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        let mut output_positions = Vec::new();
        for (symbol, position) in positions {
            let current_price = latest_quotes
                .get(&symbol)
                .map(|quote| quote.close)
                .unwrap_or_else(|| position.last_price.max(position.avg_entry_price));
            let market_value = position.quantity * current_price;
            let unrealized_pnl_pct = if position.avg_entry_price > Decimal::ZERO {
                ((current_price - position.avg_entry_price) / position.avg_entry_price) * dec!(100)
            } else {
                Decimal::ZERO
            };
            output_positions.push(ArenaPosition {
                symbol,
                quantity: position.quantity,
                avg_entry_price: position.avg_entry_price,
                current_price,
                market_value,
                unrealized_pnl_pct,
            });
        }
        output_positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        let positions_value: Decimal = output_positions.iter().map(|p| p.market_value).sum();
        let total_value = cash + positions_value;
        let return_pct = if participant.starting_cash > Decimal::ZERO {
            ((total_value - participant.starting_cash) / participant.starting_cash) * dec!(100)
        } else {
            Decimal::ZERO
        };
        let mut equity_curve: Vec<ArenaEquityPoint> = snapshots
            .iter()
            .map(|snapshot| ArenaEquityPoint {
                date: snapshot.snapshot_date.clone(),
                value: snapshot.total_value,
            })
            .collect();
        equity_curve.push(ArenaEquityPoint {
            date: Utc::now().date_naive().to_string(),
            value: total_value,
        });
        let max_drawdown_pct = max_drawdown_pct(&equity_curve);

        Ok(ArenaPortfolio {
            participant: participant.clone(),
            agent,
            challenge,
            cash,
            total_value,
            return_pct,
            max_drawdown_pct,
            trade_count: executed.len(),
            positions: output_positions,
            equity_curve,
            trades,
        })
    }

    async fn leaderboard_entries(
        &self,
        challenge: &ArenaChallenge,
        mark_open_positions: bool,
    ) -> Result<Vec<ArenaLeaderboardEntry>> {
        let mark_open_positions =
            mark_open_positions && challenge.status != ArenaChallengeStatus::Settled;
        if challenge.status == ArenaChallengeStatus::Settled {
            let results = self.repository.list_results(&challenge.id)?;
            if !results.is_empty() {
                let participants = self.repository.list_participants(&challenge.id)?;
                let agents = self.repository.list_agents()?;
                let participant_by_id: HashMap<String, ArenaParticipant> = participants
                    .into_iter()
                    .map(|p| (p.id.clone(), p))
                    .collect();
                let agent_by_id: HashMap<String, ArenaAgent> =
                    agents.into_iter().map(|a| (a.id.clone(), a)).collect();
                return Ok(results
                    .into_iter()
                    .filter_map(|result| {
                        let participant = participant_by_id.get(&result.participant_id)?;
                        let agent = agent_by_id.get(&participant.agent_id)?;
                        Some(ArenaLeaderboardEntry {
                            rank: result.rank,
                            participant_id: result.participant_id,
                            agent_id: agent.id.clone(),
                            agent_name: agent.name.clone(),
                            total_value: participant.starting_cash
                                * (Decimal::ONE + result.return_pct / dec!(100)),
                            cash: Decimal::ZERO,
                            return_pct: result.return_pct,
                            max_drawdown_pct: result.max_drawdown_pct,
                            risk_adjusted_score: result.risk_adjusted_score,
                            final_score: result.final_score,
                            trade_count: result.trade_count as usize,
                            disqualified_reason: result.disqualified_reason,
                        })
                    })
                    .collect());
            }
        }

        let participants = self.repository.list_participants(&challenge.id)?;
        let mut entries = Vec::new();
        for participant in participants {
            let portfolio = self
                .build_portfolio_with_marks(&participant, mark_open_positions)
                .await?;
            let disqualified_reason = if portfolio.max_drawdown_pct > challenge.max_drawdown_pct {
                Some(format!(
                    "max drawdown {:.2}% exceeded {:.2}%",
                    portfolio.max_drawdown_pct, challenge.max_drawdown_pct
                ))
            } else {
                None
            };
            let risk_adjusted_score = risk_adjusted_score(
                portfolio.return_pct,
                portfolio.max_drawdown_pct,
                challenge.max_drawdown_pct,
            );
            let final_score = match challenge.scoring_method {
                ArenaScoringMethod::ReturnOnly => portfolio.return_pct,
                ArenaScoringMethod::RiskAdjusted => risk_adjusted_score,
            };
            let final_score =
                rankable_final_score(portfolio.trade_count, &disqualified_reason, final_score);
            entries.push(ArenaLeaderboardEntry {
                rank: None,
                participant_id: participant.id.clone(),
                agent_id: portfolio.agent.id.clone(),
                agent_name: portfolio.agent.name.clone(),
                total_value: portfolio.total_value,
                cash: portfolio.cash,
                return_pct: portfolio.return_pct,
                max_drawdown_pct: portfolio.max_drawdown_pct,
                risk_adjusted_score,
                final_score,
                trade_count: portfolio.trade_count,
                disqualified_reason,
            });
        }

        let mut rankable: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.final_score.is_some())
            .map(|(idx, _)| idx)
            .collect();
        rankable.sort_by(|a, b| entries[*b].final_score.cmp(&entries[*a].final_score));
        for (rank, idx) in rankable.into_iter().enumerate() {
            entries[idx].rank = Some((rank + 1) as i32);
        }
        entries.sort_by(|a, b| match (a.rank, b.rank) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.final_score.cmp(&a.final_score),
        });
        Ok(entries)
    }
}

#[async_trait]
impl AiArenaServiceTrait for AiArenaService {
    async fn create_agent(&self, input: CreateArenaAgentRequest) -> Result<ArenaAgent> {
        Self::validate_agent_input(&input)?;
        self.repository.create_agent(ArenaAgent::new(input)).await
    }

    fn get_agent(&self, id: &str) -> Result<ArenaAgent> {
        self.repository.get_agent(id)
    }

    fn list_agents(&self) -> Result<Vec<ArenaAgent>> {
        self.repository.list_agents()
    }

    async fn create_challenge(&self, input: CreateArenaChallengeRequest) -> Result<ArenaChallenge> {
        Self::validate_challenge_input(&input)?;
        self.repository
            .create_challenge(ArenaChallenge::new(input))
            .await
    }

    fn list_challenges(&self) -> Result<Vec<ArenaChallenge>> {
        self.repository.list_challenges()
    }

    async fn join_challenge(&self, challenge_id: &str, agent_id: &str) -> Result<ArenaParticipant> {
        let challenge = self.repository.get_challenge(challenge_id)?;
        ensure_challenge_joinable(&challenge)?;
        if let Some(existing) = self.repository.get_participant(challenge_id, agent_id)? {
            return Ok(existing);
        }
        self.repository.get_agent(agent_id)?;
        self.repository
            .create_participant(ArenaParticipant::new(&challenge, agent_id))
            .await
    }

    fn list_participants(&self, challenge_id: &str) -> Result<Vec<ArenaParticipant>> {
        self.repository.list_participants(challenge_id)
    }

    async fn run_agent(&self, request: RunArenaAgentRequest) -> Result<ArenaRun> {
        let run_type = request.run_type.unwrap_or(ArenaRunType::Manual);
        let challenge = self.repository.get_challenge(&request.challenge_id)?;
        ensure_challenge_tradeable(&challenge)?;
        let agent = self.repository.get_agent(&request.agent_id)?;
        if !agent.enabled {
            return invalid("Agent is disabled");
        }
        let participant = self.join_challenge(&challenge.id, &agent.id).await?;
        let idempotency_key = if run_type == ArenaRunType::Scheduled {
            Some(format!(
                "{}:{}:{}:scheduled",
                challenge.id,
                agent.id,
                Utc::now().date_naive()
            ))
        } else {
            None
        };
        if let Some(key) = idempotency_key.as_deref() {
            if let Some(existing) = self.repository.get_run_by_idempotency_key(key)? {
                return Ok(existing);
            }
        }

        let portfolio = self.build_portfolio(&participant).await?;
        let prompt = self.build_prompt(&agent, &challenge, &portfolio).await;
        let mut run = self
            .repository
            .create_run(ArenaRun::new(
                challenge.id.clone(),
                agent.id.clone(),
                participant.id.clone(),
                run_type,
                idempotency_key,
                prompt.clone(),
            ))
            .await?;

        match self.run_decision(&agent, prompt).await {
            Ok((raw, parsed, decision)) => {
                // Any failure while applying the decision must still complete the
                // run row; otherwise it stays "running" forever and its idempotency
                // key blocks scheduled retries.
                let (status, error) = match self
                    .apply_decision(&run, &challenge, &participant, decision)
                    .await
                {
                    Ok(rejected) if rejected > 0 => (ArenaRunStatus::CompletedWithRejections, None),
                    Ok(_) => (ArenaRunStatus::Completed, None),
                    Err(err) => (ArenaRunStatus::Failed, Some(err.to_string())),
                };
                run = self
                    .repository
                    .update_run(ArenaRunUpdate {
                        id: run.id.clone(),
                        status,
                        raw_response: Some(raw),
                        parsed_json: Some(parsed),
                        error,
                        completed_at: Some(now_string()),
                    })
                    .await?;
                if run.status != ArenaRunStatus::Failed {
                    let portfolio = self.build_portfolio(&participant).await?;
                    let snapshot = ArenaSnapshot::from_portfolio(
                        &portfolio,
                        Utc::now().date_naive().to_string(),
                    );
                    let _ = self.repository.upsert_snapshot(snapshot).await?;
                }
                Ok(run)
            }
            Err(err) => {
                self.repository
                    .update_run(ArenaRunUpdate {
                        id: run.id.clone(),
                        status: ArenaRunStatus::Failed,
                        raw_response: None,
                        parsed_json: None,
                        error: Some(err.to_string()),
                        completed_at: Some(now_string()),
                    })
                    .await
            }
        }
    }

    async fn run_due_scheduled(&self) -> Result<Vec<ArenaRun>> {
        let mut runs = Vec::new();
        let local_now = Local::now().time();
        for challenge in self.repository.list_challenges()? {
            if challenge.status != ArenaChallengeStatus::Active || challenge.run_cadence != "daily"
            {
                continue;
            }
            if !challenge_is_tradeable_now(&challenge)? {
                continue;
            }
            if !scheduled_time_is_due(challenge.scheduled_time_local.as_deref(), local_now)? {
                continue;
            }
            for participant in self.repository.list_participants(&challenge.id)? {
                let agent = self.repository.get_agent(&participant.agent_id)?;
                if agent.enabled && agent.schedule_enabled {
                    runs.push(
                        self.run_agent(RunArenaAgentRequest {
                            challenge_id: challenge.id.clone(),
                            agent_id: agent.id,
                            run_type: Some(ArenaRunType::Scheduled),
                        })
                        .await?,
                    );
                }
            }
        }
        Ok(runs)
    }

    async fn settle_challenge(&self, challenge_id: &str) -> Result<ArenaLeaderboard> {
        let mut challenge = self.repository.get_challenge(challenge_id)?;
        let entries = self.leaderboard_entries(&challenge, false).await?;
        let settled_at = now_string();
        let results = entries
            .iter()
            .map(|entry| ArenaResult {
                id: uuid::Uuid::new_v4().to_string(),
                challenge_id: challenge.id.clone(),
                participant_id: entry.participant_id.clone(),
                return_pct: entry.return_pct,
                max_drawdown_pct: entry.max_drawdown_pct,
                risk_adjusted_score: entry.risk_adjusted_score,
                final_score: entry.final_score,
                rank: entry.rank,
                trade_count: entry.trade_count as i32,
                disqualified_reason: entry.disqualified_reason.clone(),
                metrics: serde_json::to_value(entry).unwrap_or(Value::Null),
                settled_at: settled_at.clone(),
            })
            .collect();
        self.repository
            .replace_results(&challenge.id, results)
            .await?;
        challenge.status = ArenaChallengeStatus::Settled;
        challenge.settled_at = Some(settled_at);
        challenge.updated_at = now_string();
        let challenge = self.repository.update_challenge(challenge).await?;
        Ok(ArenaLeaderboard { challenge, entries })
    }

    async fn get_leaderboard(&self, challenge_id: &str) -> Result<ArenaLeaderboard> {
        let challenge = self.repository.get_challenge(challenge_id)?;
        let entries = self.leaderboard_entries(&challenge, true).await?;
        Ok(ArenaLeaderboard { challenge, entries })
    }

    async fn get_portfolio(&self, participant_id: &str) -> Result<ArenaPortfolio> {
        let participant = self.repository.get_participant_by_id(participant_id)?;
        self.build_portfolio(&participant).await
    }

    fn list_runs(&self, challenge_id: &str) -> Result<Vec<ArenaRun>> {
        self.repository.list_runs(challenge_id)
    }

    fn list_trades(&self, challenge_id: &str) -> Result<Vec<ArenaTrade>> {
        self.repository.list_trades_for_challenge(challenge_id)
    }

    async fn create_thesis(&self, input: CreateCompanyThesisRequest) -> Result<CompanyThesis> {
        if input.symbol.trim().is_empty() {
            return invalid("Thesis symbol cannot be empty");
        }
        if input.thesis.trim().is_empty() {
            return invalid("Thesis cannot be empty");
        }
        self.repository
            .create_thesis(CompanyThesis::new(input))
            .await
    }

    fn list_theses(
        &self,
        symbol: Option<&str>,
        challenge_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CompanyThesis>> {
        self.repository.list_theses(symbol, challenge_id, limit)
    }
}

#[derive(Default)]
struct ReplayPosition {
    quantity: Decimal,
    avg_entry_price: Decimal,
    last_price: Decimal,
}

fn require_non_empty(value: &str, message: &str) -> Result<()> {
    if value.trim().is_empty() {
        return invalid(message);
    }
    Ok(())
}

fn require_positive(value: Decimal, field: &str) -> Result<()> {
    if value <= Decimal::ZERO {
        return invalid(field);
    }
    Ok(())
}

fn require_pct(value: Decimal, field: &str) -> Result<()> {
    if value <= Decimal::ZERO || value > dec!(100) {
        return invalid(&format!("{field} must be between 0 and 100"));
    }
    Ok(())
}

fn invalid<T>(message: &str) -> Result<T> {
    Err(Error::Validation(ValidationError::InvalidInput(
        message.to_string(),
    )))
}

fn parse_challenge_time(value: Option<&str>, field: &str) -> Result<Option<DateTime<Utc>>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(Some(parsed.with_timezone(&Utc)));
    }
    let date = value.get(0..10).ok_or_else(|| {
        Error::Validation(ValidationError::InvalidInput(format!(
            "{field} must be an RFC3339 timestamp or YYYY-MM-DD date"
        )))
    })?;
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
        Error::Validation(ValidationError::InvalidInput(format!(
            "{field} must be an RFC3339 timestamp or YYYY-MM-DD date"
        )))
    })?;
    let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
        Error::Validation(ValidationError::InvalidInput(format!(
            "{field} must be a valid date"
        )))
    })?;
    Ok(Some(DateTime::from_naive_utc_and_offset(midnight, Utc)))
}

fn parse_hh_mm(value: &str, field: &str) -> Result<NaiveTime> {
    let value = value.trim();
    let candidate = value.get(0..5).unwrap_or(value);
    NaiveTime::parse_from_str(candidate, "%H:%M").map_err(|_| {
        Error::Validation(ValidationError::InvalidInput(format!(
            "{field} must use HH:mm"
        )))
    })
}

fn scheduled_time_is_due(value: Option<&str>, local_now: NaiveTime) -> Result<bool> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(true);
    };
    Ok(local_now >= parse_hh_mm(value, "scheduledTimeLocal")?)
}

fn challenge_is_joinable_now(challenge: &ArenaChallenge) -> Result<bool> {
    if challenge.status != ArenaChallengeStatus::Active {
        return Ok(false);
    }
    if let Some(end_at) = parse_challenge_time(challenge.end_at.as_deref(), "endAt")? {
        if Utc::now() >= end_at {
            return Ok(false);
        }
    }
    Ok(true)
}

fn challenge_is_tradeable_now(challenge: &ArenaChallenge) -> Result<bool> {
    if !challenge_is_joinable_now(challenge)? {
        return Ok(false);
    }
    if let Some(start_at) = parse_challenge_time(challenge.start_at.as_deref(), "startAt")? {
        if Utc::now() < start_at {
            return Ok(false);
        }
    }
    Ok(true)
}

fn ensure_challenge_joinable(challenge: &ArenaChallenge) -> Result<()> {
    if !challenge_is_joinable_now(challenge)? {
        return invalid("Challenge is not joinable");
    }
    Ok(())
}

fn ensure_challenge_tradeable(challenge: &ArenaChallenge) -> Result<()> {
    if !challenge_is_tradeable_now(challenge)? {
        return invalid("Challenge is not active for trading");
    }
    Ok(())
}

fn rankable_final_score(
    trade_count: usize,
    disqualified_reason: &Option<String>,
    score: Decimal,
) -> Option<Decimal> {
    if trade_count > 0 && disqualified_reason.is_none() {
        Some(score)
    } else {
        None
    }
}

/// Parse a model response into JSON, tolerating markdown fences and prose
/// around the object. Shared with `wealthfolio-ai` (arena spec generation).
pub fn parse_model_json(raw: &str) -> Result<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Ok(value);
    }
    let trimmed = raw.trim();
    let without_fence = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim);
    if let Some(value) = without_fence {
        if let Ok(parsed) = serde_json::from_str::<Value>(value) {
            return Ok(parsed);
        }
    }
    let start = trimmed.find('{').ok_or_else(|| {
        Error::Validation(ValidationError::InvalidInput(
            "Arena decision did not contain a JSON object".to_string(),
        ))
    })?;
    let end = trimmed.rfind('}').ok_or_else(|| {
        Error::Validation(ValidationError::InvalidInput(
            "Arena decision did not contain a complete JSON object".to_string(),
        ))
    })?;
    serde_json::from_str::<Value>(&trimmed[start..=end]).map_err(|e| {
        Error::Validation(ValidationError::InvalidInput(format!(
            "Arena decision was not valid JSON: {e}"
        )))
    })
}

fn display_symbol_for_asset(asset: &Asset, fallback: &str) -> String {
    normalize_symbol(
        asset
            .instrument_symbol
            .as_deref()
            .or(asset.display_code.as_deref())
            .unwrap_or(fallback),
    )
}

fn validate_external_quote_type(quote_type: &str, name: &str) -> Result<()> {
    let upper = quote_type.to_ascii_uppercase();
    let allowed = matches!(upper.as_str(), "EQUITY" | "STOCK" | "ETF");
    if !allowed || looks_like_bond_fund(&format!("{quote_type} {name}")) {
        return invalid("AI Arena allows only stocks and equity ETFs");
    }
    Ok(())
}

fn looks_like_bond_fund(label: &str) -> bool {
    let upper = label.to_ascii_uppercase();
    [
        "BOND",
        "TREASURY",
        "T-BILL",
        "FIXED INCOME",
        "MONEY MARKET",
        "MUNICIPAL",
        "MUNI",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

fn risk_adjusted_score(
    return_pct: Decimal,
    max_drawdown_pct: Decimal,
    allowed_drawdown_pct: Decimal,
) -> Decimal {
    return_pct - (max_drawdown_pct - allowed_drawdown_pct).max(Decimal::ZERO)
}

fn max_drawdown_pct(points: &[ArenaEquityPoint]) -> Decimal {
    let mut peak = Decimal::ZERO;
    let mut max_drawdown = Decimal::ZERO;
    for point in points {
        if point.value > peak {
            peak = point.value;
        }
        if peak > Decimal::ZERO {
            let drawdown = ((peak - point.value) / peak) * dec!(100);
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
    }
    max_drawdown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_json() {
        let parsed = parse_model_json("```json\n{\"orders\":[]}\n```").unwrap();
        assert_eq!(parsed["orders"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn drawdown_tracks_peak_to_trough() {
        let points = vec![
            ArenaEquityPoint {
                date: "a".into(),
                value: dec!(100),
            },
            ArenaEquityPoint {
                date: "b".into(),
                value: dec!(120),
            },
            ArenaEquityPoint {
                date: "c".into(),
                value: dec!(90),
            },
        ];
        assert_eq!(max_drawdown_pct(&points), dec!(25));
    }

    #[test]
    fn risk_adjusted_penalizes_drawdown_above_limit() {
        assert_eq!(
            risk_adjusted_score(dec!(10), dec!(30), dec!(20)),
            Decimal::ZERO
        );
    }

    #[test]
    fn quote_type_validator_allows_only_stock_or_etf() {
        assert!(validate_external_quote_type("EQUITY", "Apple Inc.").is_ok());
        assert!(validate_external_quote_type("ETF", "SPDR S&P 500 ETF").is_ok());
        assert!(validate_external_quote_type("METAL", "Gold").is_err());
        assert!(validate_external_quote_type("COMMODITY", "Crude Oil").is_err());
        assert!(validate_external_quote_type("INDEX", "S&P 500").is_err());
        assert!(validate_external_quote_type("ETF", "Treasury Bond ETF").is_err());
    }

    #[test]
    fn challenge_time_validation_rejects_reversed_windows() {
        let mut request = CreateArenaChallengeRequest {
            name: "Window check".to_string(),
            description: None,
            market: "us-stock".to_string(),
            scoring_method: ArenaScoringMethod::RiskAdjusted,
            initial_cash: dec!(100_000),
            max_position_pct: dec!(50),
            max_drawdown_pct: dec!(25),
            run_cadence: "daily".to_string(),
            scheduled_time_local: Some("09:30".to_string()),
            universe: vec![],
            start_at: Some("2026-06-30T10:00:00Z".to_string()),
            end_at: Some("2026-06-30T09:59:00Z".to_string()),
        };
        assert!(AiArenaService::validate_challenge_input(&request).is_err());

        request.end_at = Some("2026-06-30T10:01:00Z".to_string());
        assert!(AiArenaService::validate_challenge_input(&request).is_ok());
    }

    #[test]
    fn scheduled_time_gate_uses_local_clock_minutes() {
        let before = NaiveTime::from_hms_opt(9, 29, 0).unwrap();
        let at_time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        assert!(!scheduled_time_is_due(Some("09:30"), before).unwrap());
        assert!(scheduled_time_is_due(Some("09:30"), at_time).unwrap());
        assert!(scheduled_time_is_due(None, before).unwrap());
    }

    #[test]
    fn final_score_is_null_for_unrankable_rows() {
        assert_eq!(rankable_final_score(0, &None, Decimal::ZERO), None);
        assert_eq!(
            rankable_final_score(1, &Some("max_drawdown_pct_exceeded".to_string()), dec!(10)),
            None
        );
        assert_eq!(
            rankable_final_score(1, &None, Decimal::ZERO),
            Some(Decimal::ZERO)
        );
    }
}
