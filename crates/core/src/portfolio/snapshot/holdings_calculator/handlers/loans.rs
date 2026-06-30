//! Loan handlers (LOAN_ORIGINATION / LOAN_PAYMENT). `impl HoldingsCalculator`.
//!
//! A loan is tracked as a positive-quantity outstanding-balance position. The
//! loan asset's kind decides the direction:
//! - Liability asset: I'm the borrower (debt I owe). Origination brings cash in;
//!   payment takes cash out. The position subtracts from net worth.
//! - Non-liability asset: I'm the lender (credit I issue). Origination pays cash
//!   out (I disburse); payment brings cash in (I collect). The position adds to
//!   net worth.
//!
//! Neither origination nor payment affects net_contribution — lending/borrowing
//! is not capital entering or leaving the portfolio.
use super::super::economics::*;
use super::super::HoldingsCalculator;
use crate::activities::Activity;
use crate::errors::{CalculatorError, Result};
use crate::portfolio::snapshot::AccountStateSnapshot;
use log::warn;

impl HoldingsCalculator {
    /// Handle LOAN_ORIGINATION activity.
    /// Creates the outstanding-balance position (positive quantity) and books
    /// the cash effect according to the loan direction (see module docs).
    pub(crate) fn handle_loan_origination(
        &self,
        activity: &Activity,
        state: &mut AccountStateSnapshot,
        account_currency: &str,
        asset_cache: &mut AssetCache,
    ) -> Result<()> {
        let activity_currency = &activity.currency;
        let asset_id = activity.asset_id.as_deref().unwrap_or("");

        if asset_id.is_empty() {
            return Err(CalculatorError::InvalidActivity(format!(
                "LOAN_ORIGINATION activity {} requires an asset_id (loan asset)",
                activity.id
            )));
        }

        // Resolve direction from the asset kind before the mutable position borrow.
        // Default to liability (borrower) when the asset can't be resolved.
        self.ensure_asset_cached(asset_id, activity_currency, asset_cache);
        let is_liability = asset_cache
            .get(asset_id)
            .map(|info| info.is_liability)
            .unwrap_or(true);

        // Book-basis is derived from the activity independently of the position borrow.
        let book_basis =
            self.lot_book_basis_for_activity(activity, activity_currency, account_currency);

        // Create/get position for the loan asset (positive quantity = outstanding balance)
        let position = self.get_or_create_position_mut_cached(
            state,
            asset_id,
            activity_currency,
            activity.activity_date,
            asset_cache,
        )?;

        // Add lot: quantity = principal, unit_price = face value (typically 1.0),
        // fee = origination fees.
        let _cost_basis = position.add_lot_values(
            activity.id.clone(),
            activity.qty(),
            activity.price(),
            activity.fee_amt(),
            activity.activity_date,
            None,
            Some(activity.id.clone()),
            book_basis,
        )?;

        let principal = activity.qty() * activity.price();
        if is_liability {
            // Borrowing: receive net proceeds = principal - origination fees.
            add_cash(state, activity_currency, principal - activity.fee_amt());
        } else {
            // Lending out (credit I issue): disburse principal plus any fee I pay.
            add_cash(state, activity_currency, -(principal + activity.fee_amt()));
        }

        Ok(())
    }

    /// Handle LOAN_PAYMENT activity.
    /// Reduces the outstanding-balance position by the principal portion and
    /// books the cash effect according to the loan direction (see module docs).
    pub(crate) fn handle_loan_payment(
        &self,
        activity: &Activity,
        state: &mut AccountStateSnapshot,
        _account_currency: &str,
        asset_cache: &mut AssetCache,
    ) -> Result<()> {
        let activity_currency = &activity.currency;
        let asset_id = activity.asset_id.as_deref().unwrap_or("");

        if asset_id.is_empty() {
            return Err(CalculatorError::InvalidActivity(format!(
                "LOAN_PAYMENT activity {} requires an asset_id (loan asset)",
                activity.id
            )));
        }

        // Resolve direction from the asset kind. Default to liability (repaying debt).
        self.ensure_asset_cached(asset_id, activity_currency, asset_cache);
        let is_liability = asset_cache
            .get(asset_id)
            .map(|info| info.is_liability)
            .unwrap_or(true);

        // Total payment = (principal * unit_price) + interest (fee).
        let total_payment = (activity.qty() * activity.price()) + activity.fee_amt();
        if is_liability {
            // Repaying my debt: cash outflow.
            add_cash(state, activity_currency, -total_payment);
        } else {
            // Collecting on credit I issued: cash inflow.
            add_cash(state, activity_currency, total_payment);
        }

        // Reduce the outstanding position by the principal portion.
        if let Some(position) = state.positions.get_mut(asset_id) {
            let _reduction = position.reduce_lots_fifo(activity.qty())?;
        } else {
            warn!(
                "LOAN_PAYMENT activity {} references non-existent loan position {}. Cash effect applied only.",
                activity.id, asset_id
            );
        }

        Ok(())
    }
}
