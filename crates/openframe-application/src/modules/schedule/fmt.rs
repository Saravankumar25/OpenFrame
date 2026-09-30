//! Small deterministic formatting helpers: stable fingerprints, shoot dates,
//! durations and page counts.

use openframe_domain::{AppError, AppResult};
use time::{Date, Month, Weekday};

/// FNV-1a 64-bit hash as hex. Stable across builds and releases (unlike the
/// std hasher), so persisted fingerprints stay comparable after upgrades.
pub fn fnv_hex(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Validate a shoot date entered as `YYYY-MM-DD`.
pub fn parse_date(s: &str) -> AppResult<Date> {
    let bad = || {
        AppError::validation(
            "date",
            "Enter the shooting date as a calendar date (YYYY-MM-DD).",
        )
    };
    let parts: Vec<&str> = s.trim().split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(bad());
    }
    let y: i32 = parts[0].parse().map_err(|_| bad())?;
    let m: u8 = parts[1].parse().map_err(|_| bad())?;
    let d: u8 = parts[2].parse().map_err(|_| bad())?;
    let month = Month::try_from(m).map_err(|_| bad())?;
    Date::from_calendar_date(y, month, d).map_err(|_| bad())
}

pub fn clean_date(s: Option<String>) -> AppResult<Option<String>> {
    match s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(v) => {
            let d = parse_date(&v)?;
            Ok(Some(format!(
                "{:04}-{:02}-{:02}",
                d.year(),
                u8::from(d.month()),
                d.day()
            )))
        }
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn weekday_short(w: Weekday) -> &'static str {
    match w {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

fn weekday_long(w: Weekday) -> &'static str {
    match w {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

/// "Mon 14 Jun 2027".
pub fn date_short(s: &str) -> String {
    match parse_date(s) {
        Ok(d) => format!(
            "{} {} {} {}",
            weekday_short(d.weekday()),
            d.day(),
            MONTHS[usize::from(u8::from(d.month())) - 1],
            d.year()
        ),
        Err(_) => s.to_string(),
    }
}

/// "MONDAY 14 JUNE 2027" (call sheet header).
pub fn date_long_upper(s: &str) -> String {
    match parse_date(s) {
        Ok(d) => format!(
            "{} {} {} {}",
            weekday_long(d.weekday()),
            d.day(),
            MONTHS_LONG[usize::from(u8::from(d.month())) - 1],
            d.year()
        )
        .to_uppercase(),
        Err(_) => s.to_string(),
    }
}

/// "5h 30m", "45m", "0m".
pub fn minutes(m: i64) -> String {
    let h = m / 60;
    let r = m % 60;
    match (h, r) {
        (0, r) => format!("{r}m"),
        (h, 0) => format!("{h}h"),
        (h, r) => format!("{h}h {r:02}m"),
    }
}

/// "1 2/8", "3/8", "2".
pub fn pages(eighths: i64) -> String {
    let whole = eighths / 8;
    let rest = eighths % 8;
    match (whole, rest) {
        (0, r) => format!("{r}/8"),
        (w, 0) => format!("{w}"),
        (w, r) => format!("{w} {r}/8"),
    }
}

/// Validate an optional clock time such as `18:00` (24h).
pub fn clean_time(s: Option<String>, what: &str) -> AppResult<Option<String>> {
    match s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(v) => {
            let ok = v.len() <= 5
                && v.split_once(':').is_some_and(|(h, m)| {
                    h.parse::<u8>().is_ok_and(|h| h < 24)
                        && m.len() == 2
                        && m.parse::<u8>().is_ok_and(|m| m < 60)
                });
            if ok {
                let (h, m) = v.split_once(':').unwrap_or(("0", "0"));
                Ok(Some(format!("{:02}:{m}", h.parse::<u8>().unwrap_or(0))))
            } else {
                Err(AppError::invalid_input(format!(
                    "Enter the {what} as a 24-hour time such as 18:30."
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(date_short("2027-06-14"), "Mon 14 Jun 2027");
        assert_eq!(date_long_upper("2027-06-14"), "MONDAY 14 JUNE 2027");
        assert_eq!(minutes(330), "5h 30m");
        assert_eq!(minutes(600), "10h");
        assert_eq!(minutes(45), "45m");
        assert_eq!(pages(10), "1 2/8");
        assert_eq!(pages(3), "3/8");
        assert_eq!(pages(16), "2");
        assert!(parse_date("2027-02-30").is_err());
        assert_eq!(
            clean_date(Some(" 2027-06-14 ".into())).unwrap().as_deref(),
            Some("2027-06-14")
        );
        assert_eq!(
            clean_time(Some("8:05".into()), "call time")
                .unwrap()
                .as_deref(),
            Some("08:05")
        );
        assert!(clean_time(Some("25:00".into()), "call time").is_err());
        assert_eq!(fnv_hex("a"), fnv_hex("a"));
        assert_ne!(fnv_hex("a"), fnv_hex("b"));
    }
}
