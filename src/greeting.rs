//! The line that greets you on the home screen. It changes: every few
//! minutes it moves on, and the time of day has its own lines.

/// The part of the day an hour (0..24, local) falls in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DayPart {
    Morning,
    Afternoon,
    Evening,
    Night,
}

pub fn day_part(hour: u32) -> DayPart {
    match hour {
        5..=11 => DayPart::Morning,
        12..=16 => DayPart::Afternoon,
        17..=21 => DayPart::Evening,
        _ => DayPart::Night,
    }
}

/// Lines for any time. `{}` is your name.
const ANYTIME: &[&str] = &[
    "Welcome back, {}",
    "Good to see you, {}",
    "Ready when you are, {}",
    "What's on your mind, {}?",
    "Your sheets missed you, {}",
    "Let's get it down, {}",
    "Pick up where you left off, {}",
];

fn of_day(part: DayPart) -> &'static [&'static str] {
    match part {
        DayPart::Morning => &["Good morning, {}", "Morning, {}. Fresh page?", "Coffee and a clean sheet, {}?"],
        DayPart::Afternoon => &["Good afternoon, {}", "Afternoon, {}. What's next?", "Keep it moving, {}"],
        DayPart::Evening => &["Good evening, {}", "Evening, {}. Wrap up the day?", "Winding down, {}?"],
        DayPart::Night => &["Burning the midnight oil, {}?", "Late thoughts, {}?", "Quiet hours, {}"],
    }
}

/// The words under the greeting, by the time of day.
pub fn sub(hour: u32) -> &'static str {
    match day_part(hour) {
        DayPart::Morning => "A good time to plan the day",
        DayPart::Afternoon => "Catch what's in your head before it slips",
        DayPart::Evening => "Note what's done, park what isn't",
        DayPart::Night => "Write it down and let it rest",
    }
}

/// A small deterministic mix, so a seed always picks the same line.
fn mix(x: u64) -> u64 {
    let mut x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// The greeting for `name` at `hour`, picked by `seed` from the time of
/// day's lines and the ones for any time.
pub fn pick(name: &str, hour: u32, seed: u64) -> String {
    let name = name.trim();
    if name.is_empty() {
        return "Welcome to Folio".into();
    }
    let day = of_day(day_part(hour));
    let all = day.len() + ANYTIME.len();
    let i = (mix(seed) % all as u64) as usize;
    let line = if i < day.len() { day[i] } else { ANYTIME[i - day.len()] };
    line.replace("{}", name)
}

/// The local hour now.
pub fn hour_now() -> u32 {
    use chrono::Timelike;
    chrono::Local::now().hour()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greetings_name_you_and_follow_the_day() {
        for seed in 0..40 {
            assert!(pick("Amine", 9, seed).contains("Amine"));
        }
        assert_eq!(pick("  ", 9, 1), "Welcome to Folio");
        assert_eq!(day_part(23), DayPart::Night);
        assert_eq!(day_part(14), DayPart::Afternoon);
        let lines: std::collections::HashSet<String> = (0..60).map(|s| pick("A", 9, s)).collect();
        assert!(lines.len() > 4, "it varies");
    }
}
