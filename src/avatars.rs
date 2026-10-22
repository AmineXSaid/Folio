//! Your avatar: one of twelve faceless head-and-shoulders portraits
//! (`assets/avatars`, from forge-gpui), six women and six men. A new user
//! gets one at random on the welcome screen and can pick another there or
//! from the drawer.
//!
//! They're served as `avatars/<name>-<px>.svg`, sized to the pixels they're
//! drawn at, so each is rasterised once at that size and never scaled.

macro_rules! avatars {
    ($($name:literal),* $(,)?) => {
        const FILES: &[(&str, &str)] = &[$(($name, include_str!(concat!("../assets/avatars/", $name, ".svg"))),)*];
    };
}

avatars!("bow", "spacebuns", "flowerbun", "ponytail", "headband", "hijab", "quiff", "beanie", "cap", "beard", "curls", "phones");

pub const WOMEN: [&str; 6] = ["bow", "spacebuns", "flowerbun", "ponytail", "headband", "hijab"];
pub const MEN: [&str; 6] = ["quiff", "beanie", "cap", "beard", "curls", "phones"];

/// Every avatar, women first.
pub fn all() -> impl Iterator<Item = &'static str> {
    WOMEN.into_iter().chain(MEN)
}

/// One at random, from `seed`.
pub fn random(seed: u64) -> &'static str {
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    all().nth((x % 12) as usize).unwrap_or(WOMEN[0])
}

/// The asset path for `name` drawn `px` pixels square (twice that, for
/// sharp edges on high-density screens).
pub fn path(name: &str, px: u32) -> String {
    format!("avatars/{name}-{}.svg", px * 2)
}

/// The SVG for an asset path, sized to its pixels.
pub fn load(path: &str) -> Option<Vec<u8>> {
    let (name, px) = path.strip_prefix("avatars/")?.strip_suffix(".svg")?.rsplit_once('-')?;
    let px: u32 = px.parse().ok().filter(|p| (8..=1024).contains(p))?;
    let svg = FILES.iter().find(|(n, _)| *n == name)?.1;
    Some(svg.replacen(r#"width="64" height="64""#, &format!(r#"width="{px}" height="{px}""#), 1).into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_avatar_is_bundled_and_sizes() {
        assert_eq!(all().count(), FILES.len());
        for name in all() {
            let svg = String::from_utf8(load(&path(name, 48)).unwrap()).unwrap();
            assert!(svg.contains(r#"width="96" height="96""#), "{name}");
        }
        assert!(load("avatars/bow-0.svg").is_none());
        assert!(load("avatars/nobody-32.svg").is_none());
        let picked: std::collections::HashSet<&str> = (0..200).map(random).collect();
        assert_eq!(picked.len(), 12);
    }
}
