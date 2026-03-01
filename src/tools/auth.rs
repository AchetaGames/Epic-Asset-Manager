use chrono::{DateTime, Utc};

const MIN_TOKEN_VALIDITY_SECS: i64 = 600;

pub fn parse_token_time(rfc3339: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

pub fn is_token_usable(
    now: DateTime<Utc>,
    token_expiration: Option<DateTime<Utc>>,
    has_token: bool,
) -> bool {
    if let Some(exp) = token_expiration {
        let remaining = exp - now;
        if remaining.num_seconds() > MIN_TOKEN_VALIDITY_SECS && has_token {
            return true;
        }
    }
    false
}

pub fn can_relogin(
    now: DateTime<Utc>,
    access_token_expiration: Option<DateTime<Utc>>,
    has_access_token: bool,
    refresh_token_expiration: Option<DateTime<Utc>>,
    has_refresh_token: bool,
) -> bool {
    is_token_usable(now, access_token_expiration, has_access_token)
        || is_token_usable(now, refresh_token_expiration, has_refresh_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Duration, Timelike};

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    // --- parse_token_time ---

    #[test]
    fn parse_valid_rfc3339() {
        let dt = parse_token_time("2025-06-15T12:00:00Z");
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().year(), 2025);
    }

    #[test]
    fn parse_rfc3339_with_offset() {
        let dt = parse_token_time("2025-06-15T14:00:00+02:00");
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().hour(), 12);
    }

    #[test]
    fn parse_invalid_string() {
        assert!(parse_token_time("not-a-date").is_none());
    }

    #[test]
    fn parse_empty_string() {
        assert!(parse_token_time("").is_none());
    }

    // --- is_token_usable ---

    #[test]
    fn token_valid_and_exists() {
        let n = now();
        let exp = n + Duration::hours(1);
        assert!(is_token_usable(n, Some(exp), true));
    }

    #[test]
    fn token_valid_but_missing() {
        let n = now();
        let exp = n + Duration::hours(1);
        assert!(!is_token_usable(n, Some(exp), false));
    }

    #[test]
    fn token_expired() {
        let n = now();
        let exp = n - Duration::hours(1);
        assert!(!is_token_usable(n, Some(exp), true));
    }

    #[test]
    fn token_nearly_expired_under_threshold() {
        let n = now();
        let exp = n + Duration::seconds(300);
        assert!(!is_token_usable(n, Some(exp), true));
    }

    #[test]
    fn token_exactly_at_threshold() {
        let n = now();
        let exp = n + Duration::seconds(600);
        assert!(!is_token_usable(n, Some(exp), true));
    }

    #[test]
    fn token_just_above_threshold() {
        let n = now();
        let exp = n + Duration::seconds(601);
        assert!(is_token_usable(n, Some(exp), true));
    }

    #[test]
    fn no_expiration_date() {
        assert!(!is_token_usable(now(), None, true));
    }

    // --- can_relogin ---

    #[test]
    fn relogin_with_valid_access_token() {
        let n = now();
        let access_exp = n + Duration::hours(1);
        assert!(can_relogin(n, Some(access_exp), true, None, false));
    }

    #[test]
    fn relogin_with_valid_refresh_token_only() {
        let n = now();
        let refresh_exp = n + Duration::hours(24);
        assert!(can_relogin(n, None, false, Some(refresh_exp), true));
    }

    #[test]
    fn relogin_access_expired_refresh_valid() {
        let n = now();
        let access_exp = n - Duration::hours(1);
        let refresh_exp = n + Duration::hours(24);
        assert!(can_relogin(
            n,
            Some(access_exp),
            true,
            Some(refresh_exp),
            true
        ));
    }

    #[test]
    fn relogin_both_expired() {
        let n = now();
        let access_exp = n - Duration::hours(1);
        let refresh_exp = n - Duration::hours(1);
        assert!(!can_relogin(
            n,
            Some(access_exp),
            true,
            Some(refresh_exp),
            true
        ));
    }

    #[test]
    fn relogin_no_tokens_at_all() {
        assert!(!can_relogin(now(), None, false, None, false));
    }

    #[test]
    fn relogin_tokens_valid_but_not_present() {
        let n = now();
        let exp = n + Duration::hours(1);
        assert!(!can_relogin(n, Some(exp), false, Some(exp), false));
    }
}
