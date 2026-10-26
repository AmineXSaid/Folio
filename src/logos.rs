//! Product logos for the sheet: the SVG Logos set (Gil Barbara, CC0), as
//! Iconify packs it, fetched by `scripts/logos.sh` into
//! `assets/logos/logos.json.z`. Read and unpacked on first use. Each logo
//! is kept as its SVG; for drawing it turned it's also traced (by usvg)
//! into filled polygons in its own colours. Nothing here knows about GPUI.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::model::{v2, Vec2};

/// One logo, by its place in the set; written to files by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LogoId(pub u16);

/// A logo a file names that this Folio doesn't have: drawn as a blank box.
pub const MISSING: LogoId = LogoId(u16::MAX);

struct Logo {
    name: &'static str,
    title: &'static str,
    keywords: &'static str,
    w: f32,
    h: f32,
    body: &'static str,
}

struct Table {
    logos: Vec<Logo>,
    by_name: HashMap<&'static str, u16>,
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        let packed = include_bytes!("../assets/logos/logos.json.z");
        let json = miniz_oxide::inflate::decompress_to_vec_zlib(packed).unwrap_or_default();
        let raw: HashMap<String, (f32, f32, String)> = serde_json::from_slice(&json).unwrap_or_default();
        let mut logos: Vec<Logo> = raw
            .into_iter()
            .map(|(name, (w, h, body))| {
                let title = title_of(&name);
                let keywords = format!("{} logo brand", name.replace('-', " "));
                Logo { name: leak(name), title: leak(title), keywords: leak(keywords), w, h, body: leak(body) }
            })
            .collect();
        logos.sort_by(|a, b| a.name.cmp(b.name));
        let by_name = logos.iter().enumerate().map(|(i, l)| (l.name, i as u16)).collect();
        Table { logos, by_name }
    })
}

/// Kept for the life of the app, as the table is.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// "microsoft-azure" as "Microsoft Azure"; "aws" as "AWS".
fn title_of(name: &str) -> String {
    const UPPER: [&str; 14] = ["aws", "ibm", "hp", "api", "css", "html", "php", "sql", "ui", "ux", "ai", "io", "js", "vs"];
    name.split('-')
        .filter(|w| *w != "icon")
        .map(|w| {
            if UPPER.contains(&w) {
                w.to_uppercase()
            } else {
                let mut c = w.chars();
                c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl LogoId {
    pub fn by_name(name: &str) -> Option<LogoId> {
        table().by_name.get(name).map(|&i| LogoId(i))
    }
    fn get(self) -> Option<&'static Logo> {
        table().logos.get(self.0 as usize)
    }
    pub fn name(self) -> &'static str {
        self.get().map_or("missing", |l| l.name)
    }
    pub fn title(self) -> &'static str {
        self.get().map_or("Missing logo", |l| l.title)
    }
    pub fn keywords(self) -> &'static str {
        self.get().map_or("", |l| l.keywords)
    }
    /// Width over height.
    pub fn aspect(self) -> f32 {
        self.get().map_or(1., |l| l.w / l.h.max(1.))
    }
    /// The logo as a whole SVG document.
    pub fn svg(self) -> String {
        match self.get() {
            Some(l) => format!(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 {} {}">{}</svg>"#, l.w, l.h, l.body),
            None => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"/>"#.to_string(),
        }
    }
    /// Its own size: the box its drawing is laid out in.
    pub fn size(self) -> (f32, f32) {
        self.get().map_or((1., 1.), |l| (l.w, l.h))
    }
}

/// Written by name, so a file means the same logo whatever order the set
/// comes in; a name this Folio lacks reads as `MISSING`.
impl Serialize for LogoId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for LogoId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Ok(LogoId::by_name(&name).unwrap_or(MISSING))
    }
}

/// Every logo, in name order.
pub fn all() -> impl Iterator<Item = LogoId> {
    (0..table().logos.len() as u16).map(LogoId)
}

/// The logos whose name starts with each word of `q` (or has a part that
/// does), names starting with it first; at most `limit`.
pub fn search(q: &str, limit: usize) -> Vec<LogoId> {
    let words: Vec<String> = q.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(bool, LogoId)> = all()
        .filter(|id| {
            let parts: Vec<&str> = id.name().split('-').collect();
            words.iter().all(|w| parts.iter().any(|p| p.starts_with(w.as_str())) || id.name().replace('-', "").starts_with(w.as_str()))
        })
        .map(|id| (!id.name().starts_with(words[0].as_str()), id))
        .collect();
    // The plain mark before its "-icon" and "-wordmark" kin.
    hits.sort_by_key(|(later, id)| (*later, id.name().len()));
    hits.into_iter().take(limit).map(|(_, id)| id).collect()
}

/// The ones the panel shows before any search: clouds, platforms and the
/// tools diagrams most often name.
pub const FEATURED: &[&str] = &[
    "aws", "microsoft-azure", "google-cloud", "ibm", "salesforce", "oracle", "cloudflare", "digital-ocean", "heroku", "vercel",
    "kubernetes", "docker-icon", "terraform-icon", "ansible", "jenkins", "github-icon", "gitlab", "bitbucket", "linux-tux", "ubuntu",
    "microsoft-windows", "apple", "android-icon", "postgresql", "mysql-icon", "mongodb-icon", "redis", "elasticsearch", "kafka-icon", "rabbitmq-icon",
    "nginx", "apache", "nodejs-icon", "python", "java", "go", "rust", "react", "vue", "angular-icon",
    "slack-icon", "microsoft-teams", "jira", "confluence", "figma", "notion-icon", "google-drive", "dropbox", "stripe", "paypal",
];

/// One filled region of a logo, in its own units: rings (filled by
/// `even_odd` or non-zero), and its colour as red, green, blue, alpha.
pub struct Region {
    pub rings: Vec<Vec<Vec2>>,
    pub even_odd: bool,
    pub rgba: [f32; 4],
}

/// The logo traced into filled regions, back to front. Traced once each.
pub fn regions(id: LogoId) -> Arc<Vec<Region>> {
    static TRACED: OnceLock<Mutex<HashMap<LogoId, Arc<Vec<Region>>>>> = OnceLock::new();
    let cache = TRACED.get_or_init(Default::default);
    if let Some(r) = cache.lock().ok().and_then(|c| c.get(&id).cloned()) {
        return r;
    }
    let traced = Arc::new(trace(&id.svg()));
    if let Ok(mut c) = cache.lock() {
        c.insert(id, traced.clone());
    }
    traced
}

fn trace(svg: &str) -> Vec<Region> {
    let Ok(tree) = usvg::Tree::from_str(svg, &usvg::Options::default()) else { return Vec::new() };
    let mut out = Vec::new();
    walk(tree.root(), 1., &mut out);
    out
}

fn walk(g: &usvg::Group, opacity: f32, out: &mut Vec<Region>) {
    let opacity = opacity * g.opacity().get();
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => walk(g, opacity, out),
            usvg::Node::Path(p) if p.is_visible() => {
                let ts = p.abs_transform();
                if let Some(f) = p.fill() {
                    if let (Some(rgba), Some(path)) = (colour(f.paint(), f.opacity().get() * opacity), p.data().clone().transform(ts)) {
                        out.push(Region { rings: rings(&path), even_odd: f.rule() == usvg::FillRule::EvenOdd, rgba });
                    }
                }
                if let Some(s) = p.stroke() {
                    let outline = p.data().stroke(&s.to_tiny_skia(), 1.).and_then(|o| o.transform(ts));
                    if let (Some(rgba), Some(path)) = (colour(s.paint(), s.opacity().get() * opacity), outline) {
                        out.push(Region { rings: rings(&path), even_odd: false, rgba });
                    }
                }
            }
            _ => {}
        }
    }
}

/// A paint's colour: a gradient goes by the middle of its stops.
fn colour(p: &usvg::Paint, opacity: f32) -> Option<[f32; 4]> {
    let stop = |s: &[usvg::Stop]| s.get(s.len() / 2).map(|s| (s.color(), s.opacity().get()));
    let (c, a) = match p {
        usvg::Paint::Color(c) => (*c, 1.),
        usvg::Paint::LinearGradient(g) => stop(g.stops())?,
        usvg::Paint::RadialGradient(g) => stop(g.stops())?,
        usvg::Paint::Pattern(_) => return None,
    };
    Some([f32::from(c.red) / 255., f32::from(c.green) / 255., f32::from(c.blue) / 255., a * opacity])
}

/// A path's runs as polygons, curves flattened.
fn rings(path: &tiny_skia::Path) -> Vec<Vec<Vec2>> {
    let mut out = Vec::new();
    let mut run: Vec<Vec2> = Vec::new();
    let p = |q: tiny_skia::Point| v2(q.x, q.y);
    for seg in path.segments() {
        match seg {
            tiny_skia::PathSegment::MoveTo(a) => {
                if run.len() > 2 {
                    out.push(std::mem::take(&mut run));
                }
                run.clear();
                run.push(p(a));
            }
            tiny_skia::PathSegment::LineTo(a) => run.push(p(a)),
            tiny_skia::PathSegment::QuadTo(c, e) => {
                let s = *run.last().unwrap_or(&p(c));
                let (c, e) = (p(c), p(e));
                for i in 1..=6 {
                    let t = i as f32 / 6.;
                    let u = 1. - t;
                    run.push(v2(u * u * s.x + 2. * u * t * c.x + t * t * e.x, u * u * s.y + 2. * u * t * c.y + t * t * e.y));
                }
            }
            tiny_skia::PathSegment::CubicTo(c1, c2, e) => {
                let s = *run.last().unwrap_or(&p(c1));
                let (c1, c2, e) = (p(c1), p(c2), p(e));
                for i in 1..=8 {
                    let t = i as f32 / 8.;
                    let u = 1. - t;
                    let (a, b, c, d) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
                    run.push(v2(a * s.x + b * c1.x + c * c2.x + d * e.x, a * s.y + b * c1.y + c * c2.y + d * e.y));
                }
            }
            tiny_skia::PathSegment::Close => {
                if run.len() > 2 {
                    out.push(std::mem::take(&mut run));
                }
                run.clear();
            }
        }
    }
    if run.len() > 2 {
        out.push(run);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_set_unpacks_and_finds_the_clouds() {
        assert!(all().count() > 2000);
        let aws = LogoId::by_name("aws").expect("aws");
        assert_eq!(aws.title(), "AWS");
        assert_eq!(LogoId::by_name("microsoft-azure").unwrap().title(), "Microsoft Azure");
        assert_eq!(search("aws", 5)[0], aws);
        assert!(search("azure", 10).iter().any(|l| l.name() == "microsoft-azure"));
        assert!(search("kuber", 3).iter().any(|l| l.name().starts_with("kubernetes")));
        for name in FEATURED {
            assert!(LogoId::by_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn logos_trace_into_coloured_regions_inside_their_box() {
        let aws = LogoId::by_name("aws").unwrap();
        let r = regions(aws);
        assert!(r.len() >= 2);
        let (w, h) = aws.size();
        for p in r.iter().flat_map(|r| r.rings.iter().flatten()) {
            assert!(p.x > -1. && p.x < w + 1. && p.y > -1. && p.y < h + 1., "{p:?}");
        }
        // AWS's smile is orange.
        assert!(r.iter().any(|g| g.rgba[0] > 0.9 && g.rgba[1] > 0.5 && g.rgba[2] < 0.2));
    }

    #[test]
    fn logos_are_written_by_name_and_unknown_ones_still_load() {
        let id = LogoId::by_name("docker-icon").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""docker-icon""#);
        assert_eq!(serde_json::from_str::<LogoId>(&json).unwrap(), id);
        assert_eq!(serde_json::from_str::<LogoId>(r#""no-such-brand""#).unwrap(), MISSING);
    }
}
