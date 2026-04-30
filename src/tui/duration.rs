use std::time::Duration;

/// Formats a [`Duration`] into a fixed-width 7-character string for terminal
/// display.
///
/// | Range      | Example output  |
/// |------------|-----------------|
/// | < 1 ms     | `"  123µs"`     |
/// | 1 ms – <1s | `"  1.0ms"`     |
/// | 1s – <60s  | `"  1.00s"`     |
/// | ≥ 60s      | `"  1m 0s"`     |
pub fn format_duration(duration: Duration) -> String {
    let micros = duration.as_micros();
    if micros < 1_000 {
        // "  999µs" — {:>5} pads to 5 chars + "µs" = 7 visual chars
        format!("{:>5}µs", micros)
    } else if micros < 1_000_000 {
        // "999.9ms" — {:>3} pads ms integer to 3 chars + "." + tenths digit + "ms" = 7
        // chars
        let millis = micros / 1_000;
        let tenths = (micros % 1_000) / 100;
        format!("{:>3}.{}ms", millis, tenths)
    } else if micros < 60_000_000 {
        // " 59.99s" — {:>3} pads seconds to 3 chars + "." + 2-digit centiseconds + "s"
        // = 7 chars
        let secs = micros / 1_000_000;
        let centis = (micros % 1_000_000) / 10_000;
        format!("{:>3}.{:0>2}s", secs, centis)
    } else {
        // "  1m 0s" — {:>3} pads minutes to 3 chars + "m" + {:>2} pads seconds to 2
        // chars + "s" = 7 chars
        let total_secs = duration.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:>3}m{:>2}s", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_microseconds() {
        assert_eq!(format_duration(Duration::from_micros(0)), "    0µs");
    }

    #[test]
    fn one_microsecond() {
        assert_eq!(format_duration(Duration::from_micros(1)), "    1µs");
    }

    #[test]
    fn max_microseconds() {
        assert_eq!(format_duration(Duration::from_micros(999)), "  999µs");
    }

    #[test]
    fn one_millisecond() {
        assert_eq!(format_duration(Duration::from_micros(1_000)), "  1.0ms");
    }

    #[test]
    fn one_millisecond_and_a_half() {
        assert_eq!(format_duration(Duration::from_micros(1_500)), "  1.5ms");
    }

    #[test]
    fn middle_milliseconds() {
        assert_eq!(format_duration(Duration::from_micros(42_000)), " 42.0ms");
    }

    #[test]
    fn max_milliseconds() {
        // 999_999µs → 999ms, tenths = 9
        assert_eq!(format_duration(Duration::from_micros(999_999)), "999.9ms");
    }

    #[test]
    fn one_second() {
        assert_eq!(format_duration(Duration::from_secs(1)), "  1.00s");
    }

    #[test]
    fn one_second_and_a_half() {
        assert_eq!(format_duration(Duration::from_micros(1_500_000)), "  1.50s");
    }

    #[test]
    fn middle_seconds() {
        assert_eq!(format_duration(Duration::from_micros(12_340_000)), " 12.34s");
    }

    #[test]
    fn max_seconds() {
        // 59_999_999µs → 59s, centis = 99
        assert_eq!(format_duration(Duration::from_micros(59_999_999)), " 59.99s");
    }

    #[test]
    fn sixty_seconds() {
        assert_eq!(format_duration(Duration::from_secs(60)), "  1m 0s");
    }

    #[test]
    fn one_minute_thirty_seconds() {
        assert_eq!(format_duration(Duration::from_secs(90)), "  1m30s");
    }

    #[test]
    fn large_minutes() {
        assert_eq!(format_duration(Duration::from_secs(3661)), " 61m 1s");
    }

    /// Each output must be exactly 7 visual characters (µ is 1 column wide).
    #[test]
    fn all_outputs_are_7_chars() {
        let cases = [
            Duration::from_micros(0),
            Duration::from_micros(1),
            Duration::from_micros(999),
            Duration::from_micros(1_000),
            Duration::from_micros(1_500),
            Duration::from_micros(42_000),
            Duration::from_micros(999_999),
            Duration::from_secs(1),
            Duration::from_micros(12_340_000),
            Duration::from_micros(59_999_999),
            Duration::from_secs(60),
            Duration::from_secs(90),
            Duration::from_secs(3661),
        ];
        for d in cases {
            let s = format_duration(d);
            let width = unicode_width::UnicodeWidthStr::width(s.as_str());
            assert_eq!(width, 7, "expected 7 chars for {:?}, got {:?} (width {})", d, s, width);
        }
    }
}
