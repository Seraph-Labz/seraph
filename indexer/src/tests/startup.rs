use super::{
    DEFAULT_RECONCILE_INTERVAL_SECS, MIN_RECONCILE_INTERVAL_SECS, parse_reconcile_interval,
};
use std::time::Duration;

#[test]
fn unset_uses_the_default() {
    assert_eq!(
        parse_reconcile_interval(None),
        Duration::from_secs(DEFAULT_RECONCILE_INTERVAL_SECS)
    );
}

#[test]
fn a_valid_value_is_used_as_given() {
    assert_eq!(
        parse_reconcile_interval(Some("120")),
        Duration::from_secs(120)
    );
}

#[test]
fn garbage_falls_back_to_the_default() {
    assert_eq!(
        parse_reconcile_interval(Some("soon")),
        Duration::from_secs(DEFAULT_RECONCILE_INTERVAL_SECS)
    );
}

#[test]
fn zero_is_clamped_to_the_floor() {
    // tokio::time::interval panics on a zero period, so this must never be zero.
    assert_eq!(
        parse_reconcile_interval(Some("0")),
        Duration::from_secs(MIN_RECONCILE_INTERVAL_SECS)
    );
    assert!(!parse_reconcile_interval(Some("0")).is_zero());
}
