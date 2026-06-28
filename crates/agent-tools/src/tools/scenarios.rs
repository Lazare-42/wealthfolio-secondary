//! Saved scenario tools.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use wealthfolio_core::portfolio::performance::performance_summary_scope_key;
use wealthfolio_core::portfolios::AccountScope;
use wealthfolio_core::scenarios::{NewPortfolioScenario, PortfolioScenario};

use crate::env::AgentEnvironment;
use crate::scope::AgentScope;
use crate::tool::{AgentTool, AgentToolAccess, AgentToolError, AgentToolResult};

use super::market_data::{
    calculate_portfolio_comparison_for_account_ids, calculate_symbol_performance_from_quotes,
    load_benchmark_quotes, parse_optional_date, resolve_symbol_input, validate_date_order,
    ComparePortfolioToSymbolsOutput, PriceBasis, DEFAULT_SYMBOL_SAMPLE_POINTS,
    MAX_SYMBOL_SAMPLE_POINTS,
};

const MAX_SAVED_SCENARIO_COMPARE_SYMBOLS: usize = 5;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePortfolioScenarioArgs {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub portfolio_id: Option<String>,
    #[serde(default)]
    pub account_ids: Vec<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default)]
    pub benchmark_symbols: Vec<String>,
    #[serde(default)]
    pub assumptions: Option<Value>,
}

pub struct CreatePortfolioScenario;

#[async_trait::async_trait]
impl AgentTool for CreatePortfolioScenario {
    fn name(&self) -> &'static str {
        "create_portfolio_scenario"
    }

    fn description(&self) -> &'static str {
        "Create a saved portfolio scenario definition with account scope, optional historical as-of date, benchmark symbols, and assumptions. This stores scenario metadata only; it does not modify transactions or holdings."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Scenario name." },
                "description": { "type": "string", "description": "Optional scenario description." },
                "accountId": { "type": "string", "description": "Optional single account id." },
                "portfolioId": { "type": "string", "description": "Optional saved portfolio id." },
                "accountIds": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional explicit account ids. Used only when accountId and portfolioId are omitted."
                },
                "asOfDate": {
                    "type": "string",
                    "description": "Optional historical portfolio date in YYYY-MM-DD format. Used as the comparison start date."
                },
                "benchmarkSymbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": 5,
                    "description": "Benchmark symbols or canonical Wealthfolio asset ids, e.g. SEC:SPY:ARCX. At most 5 so the saved scenario stays comparable."
                },
                "assumptions": {
                    "type": "object",
                    "description": "Optional structured assumptions such as cash flows, expected returns, or notes."
                }
            },
            "required": ["name"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[AgentScope::ScenariosWrite]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Write
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: CreatePortfolioScenarioArgs = serde_json::from_value(args)?;
        let account_scope = account_scope_from_args(
            args.account_id.as_deref(),
            args.portfolio_id.as_deref(),
            &args.account_ids,
        );
        let scenario = env
            .scenario_service()
            .create_scenario(NewPortfolioScenario {
                name: args.name,
                description: args.description,
                account_scope,
                as_of_date: args.as_of_date,
                benchmark_symbols: args.benchmark_symbols,
                assumptions: args
                    .assumptions
                    .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
            })
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;

        Ok(AgentToolResult {
            content: serde_json::to_value(scenario)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPortfolioScenarioArgs {
    pub id: String,
}

pub struct GetPortfolioScenario;

#[async_trait::async_trait]
impl AgentTool for GetPortfolioScenario {
    fn name(&self) -> &'static str {
        "get_portfolio_scenario"
    }

    fn description(&self) -> &'static str {
        "Get one saved portfolio scenario by id."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Scenario id." }
            },
            "required": ["id"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[AgentScope::ScenariosRead]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Read
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: GetPortfolioScenarioArgs = serde_json::from_value(args)?;
        let scenario = env
            .scenario_service()
            .get_scenario(&args.id)
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        Ok(AgentToolResult {
            content: serde_json::to_value(scenario)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPortfolioScenariosOutput {
    pub scenarios: Vec<PortfolioScenario>,
}

pub struct ListPortfolioScenarios;

#[async_trait::async_trait]
impl AgentTool for ListPortfolioScenarios {
    fn name(&self) -> &'static str {
        "list_portfolio_scenarios"
    }

    fn description(&self) -> &'static str {
        "List saved portfolio scenarios so the user can pick one for comparison."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[AgentScope::ScenariosRead]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Read
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        _args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let scenarios = env
            .scenario_service()
            .list_scenarios()
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        Ok(AgentToolResult {
            content: serde_json::to_value(ListPortfolioScenariosOutput { scenarios })?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePortfolioScenarioArgs {
    pub id: String,
}

pub struct DeletePortfolioScenario;

#[async_trait::async_trait]
impl AgentTool for DeletePortfolioScenario {
    fn name(&self) -> &'static str {
        "delete_portfolio_scenario"
    }

    fn description(&self) -> &'static str {
        "Delete a saved portfolio scenario by id. This deletes only scenario metadata."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Scenario id." }
            },
            "required": ["id"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[AgentScope::ScenariosWrite]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Write
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: DeletePortfolioScenarioArgs = serde_json::from_value(args)?;
        env.scenario_service()
            .delete_scenario(&args.id)
            .await
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        Ok(AgentToolResult {
            content: serde_json::json!({ "deleted": true, "id": args.id }),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSavedScenarioArgs {
    pub id: String,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub basis: Option<String>,
    #[serde(default)]
    pub sample_points: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareSavedScenarioOutput {
    pub scenario: PortfolioScenario,
    pub comparison: ComparePortfolioToSymbolsOutput,
}

pub struct CompareSavedScenario;

#[async_trait::async_trait]
impl AgentTool for CompareSavedScenario {
    fn name(&self) -> &'static str {
        "compare_saved_scenario"
    }

    fn description(&self) -> &'static str {
        "Compare a saved scenario's account scope against its benchmark symbols, using the scenario asOfDate as the start date when present."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Saved scenario id." },
                "endDate": {
                    "type": "string",
                    "description": "Optional comparison end date in YYYY-MM-DD format. Defaults to today."
                },
                "basis": {
                    "type": "string",
                    "enum": ["adjclose", "close"],
                    "description": "Benchmark price basis. Defaults to adjusted close."
                },
                "samplePoints": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 120,
                    "description": "Maximum sampled points per benchmark. Defaults to 36."
                }
            },
            "required": ["id"]
        })
    }

    fn required_scopes(&self) -> &'static [AgentScope] {
        &[AgentScope::ScenariosRead, AgentScope::PerformanceRead]
    }

    fn access_level(&self) -> AgentToolAccess {
        AgentToolAccess::Read
    }

    async fn call(
        &self,
        env: Arc<dyn AgentEnvironment>,
        args: serde_json::Value,
    ) -> Result<AgentToolResult, AgentToolError> {
        let args: CompareSavedScenarioArgs = serde_json::from_value(args)?;
        let scenario = env
            .scenario_service()
            .get_scenario(&args.id)
            .map_err(|e| AgentToolError::ExecutionFailed(e.to_string()))?;
        if scenario.benchmark_symbols.is_empty() {
            return Err(AgentToolError::InvalidInput(
                "Saved scenario has no benchmarkSymbols to compare.".to_string(),
            ));
        }
        if scenario.benchmark_symbols.len() > MAX_SAVED_SCENARIO_COMPARE_SYMBOLS {
            return Err(AgentToolError::InvalidInput(format!(
                "Saved scenario comparison supports at most {MAX_SAVED_SCENARIO_COMPARE_SYMBOLS} benchmark symbols"
            )));
        }

        let start_date = parse_optional_date(scenario.as_of_date.as_deref(), "asOfDate")?;
        let end_date = parse_optional_date(args.end_date.as_deref(), "endDate")?;
        validate_date_order(start_date, end_date)?;
        let basis = PriceBasis::parse(args.basis.as_deref())?;
        let sample_points = args
            .sample_points
            .unwrap_or(DEFAULT_SYMBOL_SAMPLE_POINTS)
            .min(MAX_SYMBOL_SAMPLE_POINTS);

        let account_id = account_id_for_comparison(&scenario.account_scope);
        let scope_id = scenario_scope_id(&scenario.account_scope, &scenario.resolved_account_ids);
        let portfolio = calculate_portfolio_comparison_for_account_ids(
            env.clone(),
            &scope_id,
            &scenario.resolved_account_ids,
            start_date,
            end_date,
        )
        .await?;

        let mut benchmarks = Vec::with_capacity(scenario.benchmark_symbols.len());
        let mut warnings = Vec::new();
        for symbol in &scenario.benchmark_symbols {
            let resolved = resolve_symbol_input(env.clone(), symbol, symbol.contains(':')).await?;
            let (quotes, fetch_warnings) = load_benchmark_quotes(
                env.clone(),
                &resolved.asset_id,
                resolved.currency.as_deref(),
                start_date,
                end_date,
            )
            .await;
            let mut symbol_warnings = resolved.warnings;
            symbol_warnings.extend(fetch_warnings);
            let output = calculate_symbol_performance_from_quotes(
                resolved.symbol,
                resolved.asset_id,
                basis,
                start_date,
                end_date,
                quotes,
                sample_points,
                symbol_warnings,
            );
            if output.data_quality.status != "ok" {
                warnings.push(format!(
                    "{} data quality is {}.",
                    output.symbol, output.data_quality.status
                ));
            }
            benchmarks.push(output);
        }

        let comparison = ComparePortfolioToSymbolsOutput {
            period: if start_date.is_some() {
                "CUSTOM".to_string()
            } else {
                "YTD".to_string()
            },
            account_id,
            comparison_basis: "Portfolio metrics come from the saved scenario account scope; benchmarks are price-based returns from local or provider-fetched quote history using the selected basis.".to_string(),
            portfolio,
            benchmarks,
            warnings,
        };

        Ok(AgentToolResult {
            content: serde_json::to_value(CompareSavedScenarioOutput {
                scenario,
                comparison,
            })?,
        })
    }
}

fn account_scope_from_args(
    account_id: Option<&str>,
    portfolio_id: Option<&str>,
    account_ids: &[String],
) -> AccountScope {
    if let Some(account_id) = account_id.map(str::trim).filter(|id| !id.is_empty()) {
        AccountScope::Account {
            account_id: account_id.to_string(),
        }
    } else if let Some(portfolio_id) = portfolio_id.map(str::trim).filter(|id| !id.is_empty()) {
        AccountScope::Portfolio {
            portfolio_id: portfolio_id.to_string(),
        }
    } else if account_ids.iter().any(|id| !id.trim().is_empty()) {
        AccountScope::Accounts {
            account_ids: account_ids
                .iter()
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect(),
        }
    } else {
        AccountScope::All
    }
}

fn account_id_for_comparison(scope: &AccountScope) -> Option<String> {
    match scope {
        AccountScope::Account { account_id } => Some(account_id.clone()),
        AccountScope::All | AccountScope::Portfolio { .. } | AccountScope::Accounts { .. } => None,
    }
}

fn scenario_scope_id(scope: &AccountScope, resolved_account_ids: &[String]) -> String {
    match scope {
        AccountScope::All => "all".to_string(),
        AccountScope::Account { account_id } => format!("account:{account_id}"),
        AccountScope::Portfolio { portfolio_id } => format!("portfolio:{portfolio_id}"),
        AccountScope::Accounts { .. } => performance_summary_scope_key(resolved_account_ids),
    }
}
