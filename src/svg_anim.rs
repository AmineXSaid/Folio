//! Animated SVG: the SMIL animations a drawing can carry (`<animate>`,
//! `<animateTransform>` and `<set>`), worked out at a moment in time into a
//! still SVG that resvg can draw. The app draws a run of those as the
//! frames of a moving picture. CSS animations (`@keyframes`) and
//! `<animateMotion>` aren't played: the drawing shows as it stands.
//! Nothing here knows about GPUI.

use std::fmt::Write;

use resvg::usvg::roxmltree::{Document, Node, NodeType};

/// One animation in the drawing: what it changes, how, and when.
#[derive(Debug, Clone, PartialEq)]
struct Anim {
    /// `animate`, `animateTransform` or `set`.
    tag: String,
    attribute: String,
    /// For `animateTransform`: translate, scale, rotate, skewX or skewY.
    kind: String,
    values: Vec<String>,
    begin: f32,
    dur: f32,
    /// None: for ever.
    repeat: Option<f32>,
    freeze: bool,
    additive: bool,
}

/// Seconds in a SMIL clock value: "2s", "500ms", "1.5", "0:01".
fn seconds(v: &str) -> Option<f32> {
    let v = v.trim();
    if let Some(ms) = v.strip_suffix("ms") {
        return ms.trim().parse::<f32>().ok().map(|x| x / 1000.);
    }
    if let Some(m) = v.strip_suffix("min") {
        return m.trim().parse::<f32>().ok().map(|x| x * 60.);
    }
    if let Some(s) = v.strip_suffix('s') {
        return s.trim().parse().ok();
    }
    if let Some(h) = v.strip_suffix('h') {
        return h.trim().parse::<f32>().ok().map(|x| x * 3600.);
    }
    if v.contains(':') {
        return v.rsplit(':').enumerate().try_fold(0., |acc, (i, p)| p.parse::<f32>().ok().map(|x| acc + x * 60f32.powi(i as i32)));
    }
    v.parse().ok()
}

fn anim_of(n: Node) -> Option<Anim> {
    let tag = n.tag_name().name();
    if !matches!(tag, "animate" | "animateTransform" | "set") {
        return None;
    }
    let attribute = n.attribute("attributeName").unwrap_or(if tag == "animateTransform" { "transform" } else { "" }).to_string();
    if attribute.is_empty() {
        return None;
    }
    let values: Vec<String> = match (n.attribute("values"), n.attribute("from"), n.attribute("to")) {
        (Some(v), _, _) => v.split(';').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        (None, Some(f), Some(t)) => vec![f.to_string(), t.to_string()],
        (None, None, Some(t)) => vec![t.to_string()],
        _ => return None,
    };
    if values.is_empty() {
        return None;
    }
    // Only a clock time starts it: an event ("click") counts as the start.
    let begin = n.attribute("begin").and_then(|b| b.split(';').next()).and_then(seconds).unwrap_or(0.);
    let dur = n.attribute("dur").and_then(seconds).filter(|d| *d > 0.).unwrap_or(if tag == "set" { 0.001 } else { 1. });
    let repeat = match n.attribute("repeatCount") {
        Some("indefinite") => None,
        Some(c) => c.parse::<f32>().ok().or(Some(1.)),
        None if n.attribute("repeatDur") == Some("indefinite") => None,
        None => Some(1.),
    };
    Some(Anim {
        tag: tag.to_string(),
        attribute,
        kind: n.attribute("type").unwrap_or("translate").to_string(),
        values,
        begin,
        dur,
        repeat,
        freeze: n.attribute("fill") == Some("freeze") || tag == "set",
        additive: n.attribute("additive") == Some("sum"),
    })
}

/// Whether the drawing moves at all.
pub fn is_animated(svg: &str) -> bool {
    (svg.contains("<animate") || svg.contains("<set")) && Document::parse(svg).is_ok_and(|d| d.descendants().any(|n| anim_of(n).is_some()))
}

/// How long one round of the drawing's movement takes, for playing it in
/// a loop: the longest of its animations (begin and one run), at most a
/// minute; 0 for a still drawing.
pub fn period(svg: &str) -> f32 {
    let Ok(doc) = Document::parse(svg) else { return 0. };
    doc.descendants().filter_map(anim_of).map(|a| a.begin + a.dur * a.repeat.unwrap_or(1.).min(4.)).fold(0., f32::max).min(60.)
}

/// The numbers in `s`, and the text around them, so two values of the
/// same shape ("10 20", "rotate(0 50 50)") can be blended number by number.
fn numbers(s: &str) -> (Vec<f32>, Vec<String>) {
    let mut nums = Vec::new();
    let mut gaps = vec![String::new()];
    let mut cur = String::new();
    let flush = |cur: &mut String, nums: &mut Vec<f32>, gaps: &mut Vec<String>| {
        if !cur.is_empty() {
            if let Ok(v) = cur.parse::<f32>() {
                nums.push(v);
                gaps.push(String::new());
            } else if let Some(g) = gaps.last_mut() {
                g.push_str(cur);
            }
            cur.clear();
        }
    };
    for c in s.chars() {
        let starts = c == '-' && !cur.is_empty() && !cur.ends_with(['e', 'E']);
        if starts {
            flush(&mut cur, &mut nums, &mut gaps);
        }
        if c.is_ascii_digit() || c == '.' || c == '-' || ((c == 'e' || c == 'E') && !cur.is_empty()) {
            cur.push(c);
        } else {
            flush(&mut cur, &mut nums, &mut gaps);
            if let Some(g) = gaps.last_mut() {
                g.push(c);
            }
        }
    }
    flush(&mut cur, &mut nums, &mut gaps);
    (nums, gaps)
}

/// A colour as red, green, blue (0–255): `#rgb` or `#rrggbb`.
fn colour(s: &str) -> Option<[f32; 3]> {
    let h = s.trim().strip_prefix('#')?;
    let d = |a: &str| u8::from_str_radix(a, 16).ok().map(f32::from);
    match h.len() {
        3 => Some([d(&h[0..1])? * 17., d(&h[1..2])? * 17., d(&h[2..3])? * 17.]),
        6 => Some([d(&h[0..2])?, d(&h[2..4])?, d(&h[4..6])?]),
        _ => None,
    }
}

/// `a` blended toward `b` by `k` (0 to 1): colour by colour, number by
/// number when they have the same shape; otherwise `a` until halfway.
fn blend(a: &str, b: &str, k: f32) -> String {
    if let (Some(x), Some(y)) = (colour(a), colour(b)) {
        let c: Vec<u8> = (0..3).map(|i| (x[i] + (y[i] - x[i]) * k).round().clamp(0., 255.) as u8).collect();
        return format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    }
    let ((na, ga), (nb, _)) = (numbers(a), numbers(b));
    if !na.is_empty() && na.len() == nb.len() {
        let mut out = String::new();
        for (i, g) in ga.iter().enumerate() {
            out.push_str(g);
            if let (Some(x), Some(y)) = (na.get(i), nb.get(i)) {
                let v = x + (y - x) * k;
                let _ = write!(out, "{}", (v * 1000.).round() / 1000.);
            }
        }
        return out;
    }
    if k < 0.5 { a } else { b }.to_string()
}

/// What `a` sets its attribute to at `t` seconds, if it's acting then.
fn value_at(a: &Anim, t: f32) -> Option<String> {
    let local = t - a.begin;
    if local < 0. {
        return None;
    }
    let runs = local / a.dur;
    let ended = a.repeat.is_some_and(|r| runs >= r);
    if ended && !a.freeze {
        return None;
    }
    let k = if ended { a.repeat.map_or(1., |r| if r.fract() == 0. { 1. } else { r.fract() }) } else { runs.fract() };
    if a.tag == "set" || a.values.len() == 1 {
        return a.values.last().cloned();
    }
    // Evenly spaced values: which two it's between, and how far.
    let steps = (a.values.len() - 1) as f32;
    let at = (k * steps).min(steps);
    let i = (at.floor() as usize).min(a.values.len() - 2);
    let v = blend(&a.values[i], &a.values[i + 1], at - i as f32);
    Some(if a.tag == "animateTransform" { format!("{}({})", a.kind, v.replace(',', " ")) } else { v })
}

/// The drawing as it stands at `t` seconds: each animation's value written
/// onto what it animates, the animations themselves left out.
pub fn at(svg: &str, t: f32) -> String {
    let Ok(doc) = Document::parse(svg) else { return svg.to_string() };
    let mut out = String::with_capacity(svg.len());
    write_node(doc.root(), t, &mut out);
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn write_node(n: Node, t: f32, out: &mut String) {
    match n.node_type() {
        NodeType::Root => n.children().for_each(|c| write_node(c, t, out)),
        NodeType::Text => out.push_str(&esc(n.text().unwrap_or(""))),
        NodeType::Element => {
            if anim_of(n).is_some() || matches!(n.tag_name().name(), "animate" | "animateTransform" | "set" | "animateMotion") {
                return;
            }
            // This element's own attributes, less those its animations set now.
            let mut attrs: Vec<(String, String)> = n
                .attributes()
                .map(|a| {
                    let name = match a.namespace() {
                        Some("http://www.w3.org/1999/xlink") => format!("xlink:{}", a.name()),
                        Some("http://www.w3.org/XML/1998/namespace") => format!("xml:{}", a.name()),
                        _ => a.name().to_string(),
                    };
                    (name, a.value().to_string())
                })
                .collect();
            for a in n.children().filter_map(anim_of) {
                let Some(v) = value_at(&a, t) else { continue };
                match attrs.iter_mut().find(|(k, _)| *k == a.attribute) {
                    Some((_, old)) if a.tag == "animateTransform" && a.additive => *old = format!("{old} {v}"),
                    Some((_, old)) => *old = v,
                    None => attrs.push((a.attribute.clone(), v)),
                }
            }
            let name = n.tag_name().name();
            let _ = write!(out, "<{name}");
            if n.parent().is_some_and(|p| p.is_root()) {
                out.push_str(r#" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink""#);
            }
            for (k, v) in &attrs {
                if k == "xmlns" || k.starts_with("xmlns:") {
                    continue;
                }
                let _ = write!(out, r#" {k}="{}""#, esc(v));
            }
            out.push('>');
            n.children().for_each(|c| write_node(c, t, out));
            let _ = write!(out, "</{name}>");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPIN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <circle cx="50" cy="50" r="10" fill="#ff0000">
    <animate attributeName="r" values="10;40;10" dur="2s" repeatCount="indefinite"/>
    <animate attributeName="fill" from="#ff0000" to="#0000ff" dur="2s" fill="freeze"/>
  </circle>
  <rect x="0" y="0" width="10" height="10">
    <animateTransform attributeName="transform" type="rotate" from="0 50 50" to="360 50 50" dur="4s" repeatCount="indefinite"/>
  </rect>
</svg>"##;

    #[test]
    fn a_moving_drawing_is_worked_out_at_any_moment() {
        assert!(is_animated(SPIN));
        assert!(!is_animated(r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="1" height="1"/></svg>"#));
        assert_eq!(period(SPIN), 4.);
        let half = at(SPIN, 1.);
        assert!(half.contains(r#"r="40""#), "{half}");
        assert!(half.contains(r##"fill="#800080""##) || half.contains(r##"fill="#7f007f""##) || half.contains("#80007f"), "{half}");
        assert!(half.contains(r#"transform="rotate(90 50 50)""#), "{half}");
        assert!(!half.contains("<animate"));
        // Frozen at its end once it's run.
        assert!(at(SPIN, 3.).contains(r##"fill="#0000ff""##));
        // Each moment draws.
        let tree = resvg::usvg::Tree::from_str(&at(SPIN, 0.5), &resvg::usvg::Options::default());
        assert!(tree.is_ok());
    }

    #[test]
    fn clock_values_read_as_seconds() {
        assert_eq!(seconds("2s"), Some(2.));
        assert_eq!(seconds("500ms"), Some(0.5));
        assert_eq!(seconds("1.5"), Some(1.5));
        assert_eq!(seconds("0:01:30"), Some(90.));
        assert_eq!(blend("0 10", "10 30", 0.5), "5 20");
    }
}
