use super::Backoff;
use std::time::Duration;

const INITIAL: Duration = Duration::from_secs(1);
const MAX: Duration = Duration::from_secs(30);
const STABLE: Duration = Duration::from_secs(300);

fn backoff() -> Backoff {
    Backoff::new(INITIAL, MAX, STABLE)
}

#[test]
fn first_delay_is_the_initial_delay() {
    assert_eq!(backoff().next_delay(), INITIAL);
}

#[test]
fn delay_doubles_on_each_consecutive_failure() {
    let mut b = backoff();
    let delays: Vec<_> = (0..5).map(|_| b.next_delay().as_secs()).collect();
    assert_eq!(delays, vec![1, 2, 4, 8, 16]);
}

#[test]
fn delay_is_capped_at_max() {
    let mut b = backoff();
    for _ in 0..20 {
        b.next_delay();
    }
    assert_eq!(b.next_delay(), MAX);
}

#[test]
fn stable_connection_resets_the_delay() {
    let mut b = backoff();
    for _ in 0..5 {
        b.next_delay();
    }

    b.record_uptime(STABLE);
    assert_eq!(b.next_delay(), INITIAL);
}

#[test]
fn brief_connection_does_not_reset_the_delay() {
    let mut b = backoff();
    assert_eq!(b.next_delay(), INITIAL);

    // Came up, died again well inside the stability window — keep escalating.
    b.record_uptime(STABLE - Duration::from_secs(1));
    assert_eq!(b.next_delay(), INITIAL * 2);
}

#[test]
fn uptime_beyond_the_window_also_resets() {
    let mut b = backoff();
    for _ in 0..5 {
        b.next_delay();
    }

    b.record_uptime(STABLE * 10);
    assert_eq!(b.next_delay(), INITIAL);
}
