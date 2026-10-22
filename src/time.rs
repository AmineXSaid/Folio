//! Time as Folio shows it: "5 min ago", "Tomorrow 09:00", and the
//! reminder presets. Unix seconds in, local time out.

use chrono::{Datelike, Duration, Local, NaiveTime, TimeZone, Weekday};

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn local(t: u64) -> chrono::DateTime<Local> {
    Local.timestamp_opt(t as i64, 0).single().unwrap_or_else(Local::now)
}

fn at(day: chrono::NaiveDate, h: u32, m: u32) -> u64 {
    let naive = day.and_time(NaiveTime::from_hms_opt(h, m, 0).unwrap_or_default());
    Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp().max(0) as u64).unwrap_or(0)
}

/// Today's date, for backup file names: 2026-10-21.
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// The local time as `HHMM`, for a backup's name.
pub fn hhmm() -> String {
    Local::now().format("%H%M").to_string()
}

/// How long ago `t` was: "just now", "12 min ago", "3 h ago", "Yesterday",
/// "4 days ago", "12 Oct".
pub fn ago(t: u64, now: u64) -> String {
    if t == 0 {
        return "a while ago".into();
    }
    let d = now.saturating_sub(t);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86_399 => format!("{} h ago", d / 3600),
        86_400..=172_799 => "Yesterday".into(),
        172_800..=604_799 => format!("{} days ago", d / 86_400),
        _ => local(t).format("%-d %b").to_string(),
    }
}

/// When a reminder is due: "Overdue · 09:00", "Today 18:00",
/// "Tomorrow 09:00", "Mon 09:00", "3 Nov 09:00".
pub fn due_label(due: u64, now: u64) -> String {
    let d = local(due);
    let n = local(now);
    let days = (d.date_naive() - n.date_naive()).num_days();
    let hm = d.format("%H:%M");
    if due <= now {
        if days == 0 {
            return format!("Overdue · {hm}");
        }
        return format!("Overdue · {}", d.format("%-d %b"));
    }
    match days {
        0 => format!("Today {hm}"),
        1 => format!("Tomorrow {hm}"),
        2..=6 => format!("{} {hm}", d.format("%a")),
        _ => format!("{} {hm}", d.format("%-d %b")),
    }
}

/// Whether `due` falls today or earlier.
pub fn due_by_today(due: u64, now: u64) -> bool {
    local(due).date_naive() <= local(now).date_naive()
}

/// Reminder presets: (label, unix seconds).
pub fn presets(now: u64) -> Vec<(&'static str, u64)> {
    let n = local(now);
    let today = n.date_naive();
    let tomorrow = today + Duration::days(1);
    let evening = if n.hour_of_day() < 17 { at(today, 18, 0) } else { at(tomorrow, 18, 0) };
    let to_monday = (7 - today.weekday().num_days_from_monday()) % 7;
    let monday = today + Duration::days(if to_monday == 0 { 7 } else { to_monday as i64 });
    vec![
        ("In 1 hour", now + 3600),
        (if n.hour_of_day() < 17 { "This evening, 18:00" } else { "Tomorrow evening, 18:00" }, evening),
        ("Tomorrow morning, 09:00", at(tomorrow, 9, 0)),
        (if today.weekday() == Weekday::Mon { "Next Monday, 09:00" } else { "Monday, 09:00" }, at(monday, 9, 0)),
    ]
}

trait Hour {
    fn hour_of_day(&self) -> u32;
}
impl Hour for chrono::DateTime<Local> {
    fn hour_of_day(&self) -> u32 {
        chrono::Timelike::hour(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ago_reads_naturally() {
        let now = 2_000_000_000;
        assert_eq!(ago(now - 5, now), "just now");
        assert_eq!(ago(now - 600, now), "10 min ago");
        assert_eq!(ago(now - 7200, now), "2 h ago");
        assert_eq!(ago(now - 90_000, now), "Yesterday");
        assert_eq!(ago(now - 4 * 86_400, now), "4 days ago");
        assert_eq!(ago(0, now), "a while ago");
    }

    #[test]
    fn presets_are_in_the_future_and_ordered() {
        let now = super::now();
        let p = presets(now);
        assert_eq!(p.len(), 4);
        assert!(p.iter().all(|(_, t)| *t > now));
        assert!(p[2].1 >= p[0].1);
        assert!(due_label(p[2].1, now).starts_with("Tomorrow 09:00"));
        assert!(due_label(now - 60, now).starts_with("Overdue"));
        assert!(due_by_today(now - 60, now));
        assert!(!due_by_today(now + 3 * 86_400, now));
    }
}
