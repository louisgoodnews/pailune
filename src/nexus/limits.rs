//! Nexus API rate limits, read from the `X-RL-*` response headers.
//!
//! Nexus allows a daily and an hourly budget per account. Pylune records
//! the remaining counts after every API call and stops issuing calls when
//! the hourly budget is nearly spent, instead of running into HTTP 429.

use std::sync::atomic::{AtomicI64, Ordering};

/// Unknown until the first response.
const UNKNOWN: i64 = -1;
/// Calls kept in reserve for the user's own actions.
const RESERVE: i64 = 5;

static HOURLY: AtomicI64 = AtomicI64::new(UNKNOWN);
static DAILY: AtomicI64 = AtomicI64::new(UNKNOWN);

/// The latest known budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub hourly: Option<i64>,
    pub daily: Option<i64>,
}

fn known(value: i64) -> Option<i64> {
    (value != UNKNOWN).then_some(value)
}

/// Remembers the counts carried by a response.
pub fn record(headers: &ureq::http::HeaderMap) {
    let read = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<i64>().ok())
    };
    if let Some(n) = read("x-rl-hourly-remaining") {
        HOURLY.store(n, Ordering::Relaxed);
    }
    if let Some(n) = read("x-rl-daily-remaining") {
        DAILY.store(n, Ordering::Relaxed);
    }
}

/// The latest known budget.
pub fn current() -> Limits {
    Limits {
        hourly: known(HOURLY.load(Ordering::Relaxed)),
        daily: known(DAILY.load(Ordering::Relaxed)),
    }
}

/// Whether another call may be made (keeps a small reserve).
pub fn allow() -> Result<(), String> {
    match current() {
        Limits {
            hourly: Some(h),
            daily: Some(d),
        } if h <= RESERVE && d <= RESERVE => {
            Err("the Nexus API budget is used up (hourly and daily); try again later".into())
        }
        Limits {
            hourly: Some(h), ..
        } if h <= RESERVE => {
            Err("the hourly Nexus API budget is nearly used up; try again within the hour".into())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_the_budget_from_headers() {
        let mut headers = ureq::http::HeaderMap::new();
        headers.insert("x-rl-hourly-remaining", "120".parse().unwrap());
        headers.insert("x-rl-daily-remaining", "9000".parse().unwrap());
        record(&headers);
        assert_eq!(
            current(),
            Limits {
                hourly: Some(120),
                daily: Some(9000)
            }
        );
        assert!(allow().is_ok());
        headers.insert("x-rl-hourly-remaining", "3".parse().unwrap());
        record(&headers);
        assert!(allow().unwrap_err().contains("hourly"));
        headers.insert("x-rl-hourly-remaining", "500".parse().unwrap());
        record(&headers);
        assert!(allow().is_ok());
    }
}
