use std::time::Duration;

/// Formats a [`Duration`] as a compact string without padding.
///
/// | Range    | Format  | Example |
/// |----------|---------|---------|
/// | < 1s     | `{N}ms` | `42ms`  |
/// | < 60s    | `{N}s`  | `12s`   |
/// | < 1h     | `{N}m`  | `5m`    |
/// | < 24h    | `{N}h`  | `2h`    |
/// | ≥ 24h    | `{N}d`  | `3d`    |
pub fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1_000 {
        format!("{}ms", millis)
    } else {
        let total_secs = duration.as_secs();
        if total_secs < 60 {
            format!("{}s", total_secs)
        } else {
            let total_mins = total_secs / 60;
            if total_mins < 60 {
                format!("{}m", total_mins)
            } else {
                let total_hours = total_mins / 60;
                if total_hours < 24 {
                    format!("{}h", total_hours)
                } else {
                    format!("{}d", total_hours / 24)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_ms() {
        assert_eq!(format_duration(Duration::from_millis(0)), "0ms");
    }

    #[test]
    fn one_ms() {
        assert_eq!(format_duration(Duration::from_millis(1)), "1ms");
    }

    #[test]
    fn sub_ms_truncates_to_zero() {
        assert_eq!(format_duration(Duration::from_micros(999)), "0ms");
    }

    #[test]
    fn middle_ms() {
        assert_eq!(format_duration(Duration::from_millis(42)), "42ms");
    }

    #[test]
    fn max_ms() {
        assert_eq!(format_duration(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn almost_one_second() {
        assert_eq!(format_duration(Duration::from_micros(999_999)), "999ms");
    }

    #[test]
    fn one_second() {
        assert_eq!(format_duration(Duration::from_secs(1)), "1s");
    }

    #[test]
    fn middle_seconds() {
        assert_eq!(format_duration(Duration::from_secs(12)), "12s");
    }

    #[test]
    fn max_seconds() {
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn one_minute() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m");
    }

    #[test]
    fn middle_minutes() {
        // 150s = 2m30s → truncates to 2m
        assert_eq!(format_duration(Duration::from_secs(150)), "2m");
    }

    #[test]
    fn max_minutes() {
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m");
    }

    #[test]
    fn one_hour() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
    }

    #[test]
    fn middle_hours() {
        assert_eq!(format_duration(Duration::from_secs(7200 + 1800)), "2h");
    }

    #[test]
    fn max_hours() {
        assert_eq!(format_duration(Duration::from_secs(86399)), "23h");
    }

    #[test]
    fn one_day() {
        assert_eq!(format_duration(Duration::from_secs(86400)), "1d");
    }

    #[test]
    fn large_days() {
        assert_eq!(format_duration(Duration::from_secs(86400 * 100)), "100d");
    }
}
