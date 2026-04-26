//! Tests for the reconciliation matching algorithm.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::reconciliation_service::{ParsedBankRow, ReconciliationService};
use super::MatchStatus;
use crate::activities::{Activity, ActivityStatus};
use chrono::{TimeZone, Utc};

fn make_activity(id: &str, date: &str, amount: Decimal, currency: &str) -> Activity {
    let dt = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
    Activity {
        id: id.to_string(),
        account_id: "acc1".to_string(),
        asset_id: None,
        activity_type: "DEPOSIT".to_string(),
        activity_type_override: None,
        source_type: None,
        subtype: None,
        status: ActivityStatus::Posted,
        activity_date: Utc.from_utc_datetime(&dt.and_hms_opt(0, 0, 0).unwrap()),
        settlement_date: None,
        quantity: None,
        unit_price: None,
        amount: Some(amount),
        fee: None,
        currency: currency.to_string(),
        fx_rate: None,
        notes: None,
        metadata: None,
        source_system: None,
        source_record_id: None,
        source_group_id: None,
        idempotency_key: None,
        import_run_id: None,
        is_user_modified: false,
        needs_review: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_bank_row(idx: u32, date: &str, amount: Decimal) -> ParsedBankRow {
    ParsedBankRow {
        row_index: idx,
        date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        amount,
        currency: "USD".to_string(),
        description: Some(format!("Row {}", idx)),
        raw_type: None,
    }
}

#[test]
fn test_exact_match() {
    let existing = vec![make_activity("a1", "2024-01-15", dec!(100.00), "USD")];
    let bank_rows = vec![make_bank_row(0, "2024-01-15", dec!(100.00))];

    let items = ReconciliationService::reconcile(&bank_rows, &existing, dec!(0.01));

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].match_status, MatchStatus::Matched);
    assert_eq!(items[0].confidence, 1.0);
    assert!(items[0].matched_activity.is_some());
}

#[test]
fn test_match_within_tolerance() {
    let existing = vec![make_activity("a1", "2024-01-15", dec!(100.01), "USD")];
    let bank_rows = vec![make_bank_row(0, "2024-01-15", dec!(100.00))];

    let items = ReconciliationService::reconcile(&bank_rows, &existing, dec!(0.02));
    assert_eq!(items[0].match_status, MatchStatus::Matched);
}

#[test]
fn test_conflict_same_date_different_amount() {
    let existing = vec![make_activity("a1", "2024-01-15", dec!(200.00), "USD")];
    let bank_rows = vec![make_bank_row(0, "2024-01-15", dec!(100.00))];

    let items = ReconciliationService::reconcile(&bank_rows, &existing, dec!(0.01));
    assert_eq!(items[0].match_status, MatchStatus::Conflict);
}

#[test]
fn test_unmatched_no_date_match() {
    let existing = vec![make_activity("a1", "2024-01-10", dec!(100.00), "USD")];
    let bank_rows = vec![make_bank_row(0, "2024-01-15", dec!(100.00))];

    let items = ReconciliationService::reconcile(&bank_rows, &existing, dec!(0.01));

    let unmatched: Vec<_> = items
        .iter()
        .filter(|i| i.match_status == MatchStatus::Unmatched)
        .collect();
    let missing: Vec<_> = items
        .iter()
        .filter(|i| i.match_status == MatchStatus::Missing)
        .collect();
    assert_eq!(unmatched.len(), 1);
    assert_eq!(missing.len(), 1);
}

#[test]
fn test_greedy_one_to_one_matching() {
    let existing = vec![
        make_activity("a1", "2024-01-15", dec!(100.00), "USD"),
        make_activity("a2", "2024-01-15", dec!(200.00), "USD"),
    ];
    let bank_rows = vec![
        make_bank_row(0, "2024-01-15", dec!(100.00)),
        make_bank_row(1, "2024-01-15", dec!(200.00)),
    ];

    let items = ReconciliationService::reconcile(&bank_rows, &existing, dec!(0.01));

    let matched: Vec<_> = items
        .iter()
        .filter(|i| i.match_status == MatchStatus::Matched)
        .collect();
    assert_eq!(matched.len(), 2);
}

#[test]
fn test_negative_amount_matching() {
    let existing = vec![make_activity("a1", "2024-01-15", dec!(-50.00), "USD")];
    let bank_rows = vec![make_bank_row(0, "2024-01-15", dec!(-50.00))];

    let items = ReconciliationService::reconcile(&bank_rows, &existing, dec!(0.01));
    assert_eq!(items[0].match_status, MatchStatus::Matched);
}

#[test]
fn test_empty_inputs() {
    let items = ReconciliationService::reconcile(&[], &[], dec!(0.01));
    assert!(items.is_empty());

    let bank_rows = vec![make_bank_row(0, "2024-01-15", dec!(100.00))];
    let items = ReconciliationService::reconcile(&bank_rows, &[], dec!(0.01));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].match_status, MatchStatus::Unmatched);
}
