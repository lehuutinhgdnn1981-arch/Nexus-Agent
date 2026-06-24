//! Natural language → cron / fire_at parser.
//!
//! Hỗ trợ các pattern:
//!   - "tomorrow 9am"
//!   - "in 2 hours"
//!   - "in 30 minutes"
//!   - "every day 9am"
//!   - "every monday 10am"
//!   - "every weekday 8:30am"
//!   - "every hour"

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Utc, Weekday};

use crate::error::{Result, SchedulerError};
use crate::scheduler::job::JobKind;

/// Parse natural language time → JobKind.
///
/// Trả về:
/// - `JobKind::OneTime` cho "in X", "tomorrow 9am"
/// - `JobKind::Recurring` cho "every ..."
pub fn parse_natural_language(input: &str) -> Result<JobKind> {
    let input = input.trim().to_lowercase();

    if input.is_empty() {
        return Err(SchedulerError::InvalidNaturalLanguage("empty input".into()).into());
    }

    // === Recurring patterns ===
    if input.starts_with("every") {
        return parse_recurring(&input);
    }

    // === One-time patterns ===
    if input.starts_with("in ") {
        return parse_relative(&input);
    }

    if input.starts_with("tomorrow") {
        return parse_tomorrow(&input);
    }

    // Today HH:MM
    if let Some(rest) = input.strip_prefix("today ") {
        let time = parse_time(rest)?;
        let now = Local::now();
        let mut dt = now.date_naive().and_time(time);
        let utc = Local
            .from_local_datetime(&dt)
            .single()
            .ok_or_else(|| SchedulerError::InvalidNaturalLanguage("ambiguous local time".into()))?
            .with_timezone(&Utc);
        if utc <= Utc::now() {
            // schedule for tomorrow same time
            dt += Duration::days(1);
        }
        let _ = dt;
        return Ok(JobKind::OneTime { fire_at: utc });
    }

    Err(SchedulerError::InvalidNaturalLanguage(format!("unrecognized: `{input}`")).into())
}

fn parse_recurring(input: &str) -> Result<JobKind> {
    // "every day 9am" → cron "0 9 * * *"
    // "every day 9:30" → "30 9 * * *"
    // "every weekday 8:30am" → "30 8 * * 1-5"
    // "every monday 10am" → "0 10 * * 1"
    // "every hour" → "0 * * * *"
    // "every 30 minutes" → "*/30 * * * *"
    // "every 2 hours" → "0 */2 * * *"

    if input == "every hour" {
        return Ok(JobKind::Recurring { cron: "0 * * * *".into() });
    }

    if let Some(rest) = input.strip_prefix("every ") {
        // Try "every N minutes" / "every N hours"
        if let Some(cron) = try_parse_every_n(rest) {
            return Ok(JobKind::Recurring { cron });
        }

        // Try weekday / day name
        let (day_filter, time_part) = extract_day_and_time(rest);
        let time = if time_part.is_empty() {
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        } else {
            parse_time(time_part)?
        };

        let cron = build_cron(time, day_filter);
        return Ok(JobKind::Recurring { cron });
    }

    Err(SchedulerError::InvalidNaturalLanguage(format!("not a recurring pattern: `{input}`")).into())
}

fn try_parse_every_n(rest: &str) -> Option<String> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    let n: u32 = parts[0].parse().ok()?;
    let unit = parts[1];
    match unit {
        "minutes" | "minute" => Some(format!("*/{n} * * * *")),
        "hours" | "hour" => Some(format!("0 */{n} * * *")),
        _ => None,
    }
}

/// Trích ra (day_filter, time_part) từ "every ..." remainder.
/// - "day 9am" → ("*", "9am")
/// - "weekday 8:30am" → ("1-5", "8:30am")
/// - "monday 10am" → ("1", "10am")
/// - "tuesday 10am" → ("2", "10am")
fn extract_day_and_time(rest: &str) -> (&'static str, &str) {
    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
    if parts.len() == 1 {
        return ("*", parts[0]);
    }
    let day_token = parts[0];
    let time_part = parts[1];
    let day_filter: &str = match day_token {
        "day" => "*",
        "weekday" => "1-5",
        "weekend" => "0,6",
        "monday" | "mon" => "1",
        "tuesday" | "tue" => "2",
        "wednesday" | "wed" => "3",
        "thursday" | "thu" => "4",
        "friday" | "fri" => "5",
        "saturday" | "sat" => "6",
        "sunday" | "sun" => "0",
        _ => "*",
    };
    (day_filter, time_part)
}

fn build_cron(time: NaiveTime, day_filter: &str) -> String {
    format!("{} {} * * {}", time.minute(), time.hour(), day_filter)
}

fn parse_relative(input: &str) -> Result<JobKind> {
    let rest = input.strip_prefix("in ").unwrap_or("");
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(SchedulerError::InvalidNaturalLanguage(format!("invalid relative: `{input}`")).into());
    }
    let n: i64 = parts[0]
        .parse()
        .map_err(|_| SchedulerError::InvalidNaturalLanguage(format!("invalid number: `{}`", parts[0])))?;
    let unit = parts[1];
    let dur = match unit.trim_end_matches('s') {
        "minute" => Duration::minutes(n),
        "hour" => Duration::hours(n),
        "day" => Duration::days(n),
        "week" => Duration::weeks(n),
        other => {
            return Err(SchedulerError::InvalidNaturalLanguage(format!("unknown unit: `{other}`")).into());
        }
    };
    Ok(JobKind::OneTime {
        fire_at: Utc::now() + dur,
    })
}

fn parse_tomorrow(input: &str) -> Result<JobKind> {
    let rest = input.strip_prefix("tomorrow").unwrap_or("").trim();
    let time = if rest.is_empty() {
        NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    } else {
        parse_time(rest)?
    };
    let now = Local::now();
    let tomorrow = now.date_naive() + Duration::days(1);
    let dt = tomorrow.and_time(time);
    let utc = Local
        .from_local_datetime(&dt)
        .single()
        .ok_or_else(|| SchedulerError::InvalidNaturalLanguage("ambiguous local time".into()))?
        .with_timezone(&Utc);
    Ok(JobKind::OneTime { fire_at: utc })
}

/// Parse "9am", "9:30am", "17:30", "17:00:00" → NaiveTime.
fn parse_time(s: &str) -> Result<NaiveTime> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    }

    // HH:MM(:SS)?
    if let Some(_) = s.find(':') {
        let naive = NaiveTime::parse_from_str(s, "%H:%M")
            .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
            .map_err(|e| SchedulerError::InvalidNaturalLanguage(format!("time parse: {e}")))?;
        return Ok(naive);
    }

    // H(am|pm) or HHam/HHpm
    let lower = s.to_lowercase();
    let (num_str, is_pm) = if let Some(rest) = lower.strip_suffix("am") {
        (rest, false)
    } else if let Some(rest) = lower.strip_suffix("pm") {
        (rest, true)
    } else {
        // Pure number = hour
        let h: u32 = lower
            .parse()
            .map_err(|_| SchedulerError::InvalidNaturalLanguage(format!("time parse: `{s}`")))?;
        return Ok(NaiveTime::from_hms_opt(h, 0, 0)
            .ok_or_else(|| SchedulerError::InvalidNaturalLanguage(format!("invalid hour: {h}")))?);
    };

    let mut h: u32 = num_str
        .trim()
        .parse()
        .map_err(|_| SchedulerError::InvalidNaturalLanguage(format!("hour parse: `{num_str}`")))?;
    if is_pm && h < 12 {
        h += 12;
    }
    if !is_pm && h == 12 {
        h = 0;
    }
    Ok(NaiveTime::from_hms_opt(h, 0, 0)
        .ok_or_else(|| SchedulerError::InvalidNaturalLanguage(format!("invalid hour: {h}")))?)
}

/// Convert Weekday → cron number (0=Sunday, 1=Monday, ...).
#[allow(dead_code)]
fn weekday_to_cron(d: Weekday) -> u32 {
    match d {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        Weekday::Sun => 0,
    }
}

#[allow(dead_code)]
fn now_weekday() -> Weekday {
    Utc::now().weekday()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_day_9am() {
        let k = parse_natural_language("every day 9am").unwrap();
        match k {
            JobKind::Recurring { cron } => assert_eq!(cron, "0 9 * * *"),
            _ => panic!("expected recurring"),
        }
    }

    #[test]
    fn every_weekday_830am() {
        let k = parse_natural_language("every weekday 8:30am").unwrap();
        match k {
            JobKind::Recurring { cron } => assert_eq!(cron, "30 8 * * 1-5"),
            _ => panic!("expected recurring"),
        }
    }

    #[test]
    fn every_monday_10am() {
        let k = parse_natural_language("every monday 10am").unwrap();
        match k {
            JobKind::Recurring { cron } => assert_eq!(cron, "0 10 * * 1"),
            _ => panic!("expected recurring"),
        }
    }

    #[test]
    fn every_30_minutes() {
        let k = parse_natural_language("every 30 minutes").unwrap();
        match k {
            JobKind::Recurring { cron } => assert_eq!(cron, "*/30 * * * *"),
            _ => panic!("expected recurring"),
        }
    }

    #[test]
    fn in_2_hours() {
        let k = parse_natural_language("in 2 hours").unwrap();
        match k {
            JobKind::OneTime { fire_at } => {
                let delta = fire_at - Utc::now();
                assert!(delta.num_minutes() >= 110 && delta.num_minutes() <= 125);
            }
            _ => panic!("expected one-time"),
        }
    }

    #[test]
    fn in_30_minutes() {
        let k = parse_natural_language("in 30 minutes").unwrap();
        match k {
            JobKind::OneTime { fire_at } => {
                let delta = fire_at - Utc::now();
                assert!(delta.num_minutes() >= 29 && delta.num_minutes() <= 31);
            }
            _ => panic!("expected one-time"),
        }
    }

    #[test]
    fn tomorrow_9am() {
        let k = parse_natural_language("tomorrow 9am").unwrap();
        match k {
            JobKind::OneTime { fire_at } => {
                let now = Utc::now();
                assert!(fire_at > now);
                let delta_hours = (fire_at - now).num_hours();
                assert!(delta_hours >= 8 && delta_hours <= 36);
            }
            _ => panic!("expected one-time"),
        }
    }

    #[test]
    fn every_hour() {
        let k = parse_natural_language("every hour").unwrap();
        match k {
            JobKind::Recurring { cron } => assert_eq!(cron, "0 * * * *"),
            _ => panic!("expected recurring"),
        }
    }
}
