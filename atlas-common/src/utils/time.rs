use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current UNIX timestamp in seconds.
///
/// This represents the number of seconds since 1970-01-01 UTC.
/// Used throughout the blockchain to timestamp operations such as
/// KYC issuance, revocations, transaction tracking, etc.
///
/// # Panics
///
/// Panics if the system clock is set before the UNIX epoch.
pub fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before UNIX EPOCH")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_time_non_zero() {
        let timestamp = current_time();
        assert!(timestamp > 0, "Timestamp should be greater than zero");
    }

    #[test]
    fn test_current_time_monotonic() {
        let t1 = current_time();
        let t2 = current_time();
        // Pode ser igual ou t2 > t1, porque o clock pode avançar entre chamadas
        assert!(t2 >= t1, "Second timestamp should be greater than or equal to the first");
    }
}
