//! Folio's own line icons for the sheet (network, devices, people, office,
//! software), drawn on a 24-unit grid as SVG path data and traced here into
//! polylines. Being outlines like any other shape, they turn, hit-test and
//! export the same way. Plain geometry only; nothing here knows about GPUI.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::model::{v2, Vec2};

/// The grid an icon is drawn on.
pub const GRID: f32 = 24.;

macro_rules! icons {
    ($($name:ident: $label:literal, $lib:ident, $keys:literal, $d:literal;)*) => {
        /// An icon to put on the sheet.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Icon { $($name),* }

        impl Icon {
            pub const ALL: &'static [Icon] = &[$(Icon::$name),*];
            pub fn name(self) -> &'static str {
                match self { $(Icon::$name => $label),* }
            }
            pub fn library(self) -> IconLibrary {
                match self { $(Icon::$name => IconLibrary::$lib),* }
            }
            /// Other words it's found by.
            pub fn keywords(self) -> &'static str {
                match self { $(Icon::$name => $keys),* }
            }
            fn path(self) -> &'static str {
                match self { $(Icon::$name => $d),* }
            }
        }
    };
}

/// The families icons are kept in, as the shapes panel shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconLibrary {
    Network,
    Devices,
    People,
    Software,
}

icons! {
    Wifi: "Wi-Fi", Network, "wireless wlan signal", "M5 12.5a10 10 0 0 1 14 0 M8.5 16a5 5 0 0 1 7 0 M2 9a15 15 0 0 1 20 0 M12 20h.01";
    Antenna: "Antenna", Network, "radio aerial signal wireless", "M12 10v11 M9 21h6 M8.5 6.5a5 5 0 0 0 0 7 M15.5 6.5a5 5 0 0 1 0 7 M5.6 3.6a9 9 0 0 0 0 12.8 M18.4 3.6a9 9 0 0 1 0 12.8 M12 9a1 1 0 1 0 0 2a1 1 0 1 0 0-2";
    Router: "Router", Network, "wifi gateway modem", "M3 13h18v7H3z M7 16.5h.01 M11 16.5h.01 M17 13V8 M14.5 6a3.5 3.5 0 0 1 5 0 M12.5 4a6.5 6.5 0 0 1 9 0";
    AccessPoint: "Access point", Network, "wifi wap wireless hotspot", "M4 15h16v4H4z M8 17h.01 M12 11v4 M8.5 8.5a5 5 0 0 1 7 0 M6 6a8.5 8.5 0 0 1 12 0";
    Switch: "Switch", Network, "hub ethernet lan ports", "M2 9h20v6H2z M6 12h.01 M10 12h.01 M14 12h.01 M18 12h.01";
    Server: "Server", Network, "rack host nas", "M3 3h18v7H3z M3 14h18v7H3z M7 6.5h.01 M7 17.5h.01";
    Firewall: "Firewall", Network, "shield security", "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z";
    Internet: "Internet", Network, "web globe world wan", "M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20 M2 12h20 M12 2a15 15 0 0 1 0 20a15 15 0 0 1 0-20";
    CloudIcon: "Cloud", Network, "internet saas hosting", "M17.5 19H8a5 5 0 1 1 1.2-9.9A6 6 0 0 1 20.5 11a4 4 0 0 1-3 8z";
    CellTower: "Cell tower", Network, "mobile 4g 5g antenna signal", "M12 9v13 M8 22l4-13 4 13 M9.5 17h5 M8 4.5a5 5 0 0 0 0 7 M16 4.5a5 5 0 0 1 0 7 M12 7a1 1 0 1 0 0 2a1 1 0 1 0 0-2";
    Dish: "Satellite dish", Network, "antenna signal satellite", "M4 10a7.3 7.3 0 0 0 10 10z M9 15l3-3 M17 13a6 6 0 0 0-6-6 M21 13A10 10 0 0 0 11 3";
    Bluetooth: "Bluetooth", Network, "wireless pairing", "M7 7l10 10-5 5V2l5 5L7 17";
    Vpn: "VPN", Network, "lock secure tunnel", "M5 11h14v10H5z M8 11V7a4 4 0 0 1 8 0v4";
    DatabaseIcon: "Database", Network, "sql storage db", "M4 5c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5 M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3";
    LoadBalancer: "Load balancer", Network, "split traffic proxy", "M12 3v6 M12 9l-7 6 M12 9l7 6 M12 9v6 M3 15h4v5H3z M10 15h4v5h-4z M17 15h4v5h-4z";
    Ethernet: "Ethernet port", Network, "lan cable rj45 socket", "M3 6h18v13H3z M7 15v-4h10v4 M9 11V9h6v2";
    Laptop: "Laptop", Devices, "computer notebook pc", "M5 5h14v10H5z M2 19h20";
    Desktop: "Desktop", Devices, "computer pc screen monitor", "M3 4h18v12H3z M8 20h8 M12 16v4";
    Phone: "Phone", Devices, "mobile smartphone", "M7 2h10v20H7z M11 18h2";
    Tablet: "Tablet", Devices, "ipad slate", "M4 2h16v20H4z M11 18h2";
    Printer: "Printer", Devices, "print", "M6 9V2h12v7 M6 18H4v-7h16v7h-2 M6 14h12v8H6z";
    Chip: "Chip", Devices, "cpu processor iot", "M6 6h12v12H6z M9 9h6v6H9z M9 2v4 M15 2v4 M9 18v4 M15 18v4 M2 9h4 M2 15h4 M18 9h4 M18 15h4";
    Drive: "Drive", Devices, "disk storage nas hdd", "M2 14h20v6H2z M5 14l3-9h8l3 9 M6 17h.01 M10 17h.01";
    Sensor: "Sensor", Devices, "iot smart signal", "M12 10a2 2 0 1 0 0 4a2 2 0 1 0 0-4 M7.8 7.8a6 6 0 0 0 0 8.4 M16.2 7.8a6 6 0 0 1 0 8.4";
    Camera: "Camera", Devices, "cctv photo webcam", "M3 7h4l2-3h6l2 3h4v13H3z M12 10a3.5 3.5 0 1 0 0 7a3.5 3.5 0 1 0 0-7";
    Keyboard: "Keyboard", Devices, "input typing", "M2 6h20v12H2z M6 10h.01 M10 10h.01 M14 10h.01 M18 10h.01 M7 14h10";
    Watch: "Watch", Devices, "wearable smartwatch", "M7 6h10v12H7z M9 6l1-4h4l1 4 M9 18l1 4h4l1-4 M12 10v2l1.5 1.5";
    Person: "Person", People, "user actor customer", "M12 3a4 4 0 1 0 0 8a4 4 0 1 0 0-8 M4 21a8 8 0 0 1 16 0";
    Team: "Team", People, "users group people", "M9 4a3.5 3.5 0 1 0 0 7a3.5 3.5 0 1 0 0-7 M2 20a7 7 0 0 1 14 0 M16 4.5a3.5 3.5 0 0 1 0 6.5 M18 14a7 7 0 0 1 4 6";
    Mail: "Mail", People, "email envelope message", "M3 5h18v14H3z M3 6l9 7 9-7";
    Chat: "Chat", People, "message talk comment", "M4 4h16v12H9l-5 4z";
    Calendar: "Calendar", People, "date meeting event", "M3 5h18v16H3z M3 10h18 M8 3v4 M16 3v4";
    Clock: "Clock", People, "time schedule", "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18 M12 7v5l3 2";
    FileIcon: "File", People, "document page", "M6 2h8l5 5v15H6z M14 2v5h5";
    Folder: "Folder", People, "directory files", "M3 6h6l2 2h10v11H3z";
    Home: "Home", People, "house building", "M3 11l9-8 9 8 M5 9v12h14V9 M10 21v-6h4v6";
    Building: "Office", People, "building company", "M4 21V3h11v18 M15 9h5v12 M2 21h20 M7 7h2 M11 7h2 M7 11h2 M11 11h2 M7 15h2 M11 15h2";
    Bell: "Bell", People, "notification alert", "M6 16v-5a6 6 0 0 1 12 0v5l2 2H4z M10 21h4";
    Cart: "Cart", People, "shop basket checkout", "M3 4h3l2 11h11l2-8H7 M9 19a1 1 0 1 0 0 2a1 1 0 1 0 0-2 M18 19a1 1 0 1 0 0 2a1 1 0 1 0 0-2";
    Code: "Code", Software, "program develop", "M8 7l-5 5 5 5 M16 7l5 5-5 5 M14 4l-4 16";
    Terminal: "Terminal", Software, "console shell command", "M3 4h18v16H3z M7 9l3 3-3 3 M12 15h5";
    Gear: "Settings", Software, "gear cog config", "M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6 M12 2v3 M12 19v3 M2 12h3 M19 12h3 M4.9 4.9l2.1 2.1 M17 17l2.1 2.1 M4.9 19.1l2.1-2.1 M17 7l2.1-2.1";
    Key: "Key", Software, "password auth login", "M8 11a4 4 0 1 0 0 8a4 4 0 1 0 0-8 M11 13l9-9 M17 7l3 3 M15 9l2 2";
    Bug: "Bug", Software, "issue defect", "M8 8h8v8a4 4 0 0 1-8 0z M9 8a3 3 0 0 1 6 0 M3 12h5 M16 12h5 M4 7l4 2 M20 7l-4 2 M4 18l4-2 M20 18l-4-2";
    Api: "API", Software, "plug integration connect", "M9 2v5 M15 2v5 M7 7h10v4a5 5 0 0 1-10 0z M12 16v6";
    Window: "App window", Software, "application browser ui", "M3 4h18v16H3z M3 9h18 M6 6.5h.01 M9 6.5h.01";
    Queue: "Queue", Software, "message broker stream", "M3 8h18v8H3z M8 8v8 M13 8v8 M18 8v8";
    Container: "Container", Software, "docker package box", "M3 7l9-4 9 4v10l-9 4-9-4z M3 7l9 4 9-4 M12 11v10";
}

/// An icon's lines on its 24-unit grid, top-left at 0,0: each a polyline,
/// closed or not. Traced once.
pub fn lines(icon: Icon) -> &'static [(Vec<Vec2>, bool)] {
    static TRACED: OnceLock<HashMap<Icon, Vec<(Vec<Vec2>, bool)>>> = OnceLock::new();
    TRACED.get_or_init(|| Icon::ALL.iter().map(|&i| (i, trace(i.path()))).collect()).get(&icon).map_or(&[], Vec::as_slice)
}

/// SVG path data traced into polylines: M L H V C S Q A Z, absolute and
/// relative. A run that goes nowhere ("h.01", a dot) becomes a small ring.
pub fn trace(d: &str) -> Vec<(Vec<Vec2>, bool)> {
    let mut out: Vec<(Vec<Vec2>, bool)> = Vec::new();
    let mut run: Vec<Vec2> = Vec::new();
    let (mut at, mut start, mut last_ctrl) = (v2(0., 0.), v2(0., 0.), None::<Vec2>);
    let mut nums = Numbers::new(d);
    let mut cmd = 'M';
    let finish = |run: &mut Vec<Vec2>, closed: bool, out: &mut Vec<(Vec<Vec2>, bool)>| {
        if run.len() > 1 {
            let span = run.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |(a, b, c, e), p| (a.min(p.x), b.min(p.y), c.max(p.x), e.max(p.y)));
            if span.2 - span.0 < 0.2 && span.3 - span.1 < 0.2 {
                // A dot: a ring as wide as a line.
                let c = run[0];
                *run = (0..8).map(|i| {
                    let a = TAU * i as f32 / 8.;
                    v2(c.x + 0.6 * a.cos(), c.y + 0.6 * a.sin())
                })
                .collect();
                out.push((std::mem::take(run), true));
                return;
            }
            out.push((std::mem::take(run), closed));
        }
        run.clear();
    };
    while let Some(c) = nums.command() {
        if c.is_ascii_alphabetic() {
            cmd = c;
        }
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { at } else { v2(0., 0.) };
        match cmd.to_ascii_uppercase() {
            'M' => {
                let Some(p) = nums.point() else { break };
                finish(&mut run, false, &mut out);
                at = base.add(p);
                start = at;
                run.push(at);
                // Pairs after a move are lines.
                cmd = if rel { 'l' } else { 'L' };
                last_ctrl = None;
            }
            'L' => {
                let Some(p) = nums.point() else { break };
                at = base.add(p);
                run.push(at);
                last_ctrl = None;
            }
            'H' => {
                let Some(x) = nums.number() else { break };
                at = v2(if rel { at.x + x } else { x }, at.y);
                run.push(at);
                last_ctrl = None;
            }
            'V' => {
                let Some(y) = nums.number() else { break };
                at = v2(at.x, if rel { at.y + y } else { y });
                run.push(at);
                last_ctrl = None;
            }
            'C' | 'S' => {
                let smooth = cmd.eq_ignore_ascii_case(&'S');
                let c1 = if smooth {
                    last_ctrl.map_or(at, |k| v2(2. * at.x - k.x, 2. * at.y - k.y))
                } else {
                    let Some(p) = nums.point() else { break };
                    base.add(p)
                };
                let (Some(c2), Some(e)) = (nums.point(), nums.point()) else { break };
                let (c2, e) = (base.add(c2), base.add(e));
                for i in 1..=10 {
                    let t = i as f32 / 10.;
                    let u = 1. - t;
                    let (a, b, c, d) = (u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t);
                    run.push(v2(a * at.x + b * c1.x + c * c2.x + d * e.x, a * at.y + b * c1.y + c * c2.y + d * e.y));
                }
                last_ctrl = Some(c2);
                at = e;
            }
            'Q' => {
                let (Some(c1), Some(e)) = (nums.point(), nums.point()) else { break };
                let (c1, e) = (base.add(c1), base.add(e));
                for i in 1..=8 {
                    let t = i as f32 / 8.;
                    let u = 1. - t;
                    run.push(v2(u * u * at.x + 2. * u * t * c1.x + t * t * e.x, u * u * at.y + 2. * u * t * c1.y + t * t * e.y));
                }
                last_ctrl = None;
                at = e;
            }
            'A' => {
                let (Some(rx), Some(ry), Some(rot), Some(large), Some(sweep), Some(e)) =
                    (nums.number(), nums.number(), nums.number(), nums.flag(), nums.flag(), nums.point())
                else {
                    break;
                };
                let e = base.add(e);
                arc(&mut run, at, e, rx, ry, rot, large, sweep);
                at = e;
                last_ctrl = None;
            }
            'Z' => {
                finish(&mut run, true, &mut out);
                at = start;
                run.push(at);
                last_ctrl = None;
                // Z takes no numbers: read the next command.
                cmd = ' ';
            }
            _ => break,
        }
    }
    finish(&mut run, false, &mut out);
    out
}

/// An elliptical arc from `a` to `b`, as SVG gives it (endpoints, radii,
/// flags), sampled into `run`.
#[allow(clippy::too_many_arguments)]
fn arc(run: &mut Vec<Vec2>, a: Vec2, b: Vec2, rx: f32, ry: f32, rot_deg: f32, large: bool, sweep: bool) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-4 || ry < 1e-4 || a.sub(b).len() < 1e-5 {
        run.push(b);
        return;
    }
    let (s, c) = rot_deg.to_radians().sin_cos();
    let (dx, dy) = ((a.x - b.x) / 2., (a.y - b.y) / 2.);
    let (x1, y1) = (c * dx + s * dy, -s * dx + c * dy);
    let k = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if k > 1. {
        rx *= k.sqrt();
        ry *= k.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut f = (num / den).sqrt();
    if large == sweep {
        f = -f;
    }
    let (cx1, cy1) = (f * rx * y1 / ry, -f * ry * x1 / rx);
    let (cx, cy) = (c * cx1 - s * cy1 + (a.x + b.x) / 2., s * cx1 + c * cy1 + (a.y + b.y) / 2.);
    let angle = |ux: f32, uy: f32| uy.atan2(ux);
    let t1 = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry) - t1;
    if sweep && dt < 0. {
        dt += TAU;
    } else if !sweep && dt > 0. {
        dt -= TAU;
    }
    let n = ((dt.abs() / TAU) * 48.).ceil().max(2.) as usize;
    for i in 1..=n {
        let t = t1 + dt * i as f32 / n as f32;
        let (x, y) = (rx * t.cos(), ry * t.sin());
        run.push(v2(c * x - s * y + cx, s * x + c * y + cy));
    }
}

/// Reads path data: commands, numbers ("-3.5", ".5.5", "1e2") and arc flags.
struct Numbers<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Numbers<'a> {
    fn new(d: &'a str) -> Self {
        Numbers { s: d.as_bytes(), i: 0 }
    }
    fn skip(&mut self) {
        while self.i < self.s.len() && (self.s[self.i].is_ascii_whitespace() || self.s[self.i] == b',') {
            self.i += 1;
        }
    }
    /// The next command letter, or a space when numbers carry on the last one.
    fn command(&mut self) -> Option<char> {
        self.skip();
        let c = *self.s.get(self.i)? as char;
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            self.i += 1;
            Some(c)
        } else {
            Some(' ')
        }
    }
    fn number(&mut self) -> Option<f32> {
        self.skip();
        let start = self.i;
        let mut dot = false;
        if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        while let Some(&b) = self.s.get(self.i) {
            match b {
                b'0'..=b'9' => self.i += 1,
                b'.' if !dot => {
                    dot = true;
                    self.i += 1;
                }
                b'e' | b'E' => {
                    self.i += 1;
                    if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
                        self.i += 1;
                    }
                }
                _ => break,
            }
        }
        std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()
    }
    fn point(&mut self) -> Option<Vec2> {
        Some(v2(self.number()?, self.number()?))
    }
    /// An arc flag: one digit, which may sit against the next number.
    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let b = *self.s.get(self.i)?;
        self.i += 1;
        match b {
            b'0' => Some(false),
            b'1' => Some(true),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_traces_onto_its_grid() {
        for &i in Icon::ALL {
            let l = lines(i);
            assert!(!l.is_empty(), "{i:?}");
            for p in l.iter().flat_map(|(p, _)| p) {
                assert!(p.x > -1. && p.x < GRID + 1. && p.y > -1. && p.y < GRID + 1., "{i:?} {p:?}");
            }
        }
    }

    #[test]
    fn paths_read_as_svg_does() {
        // A box: absolute, relative and closed.
        let b = trace("M3 4h18v7H3z");
        assert_eq!(b.len(), 1);
        assert!(b[0].1);
        assert_eq!(b[0].0, vec![v2(3., 4.), v2(21., 4.), v2(21., 11.), v2(3., 11.)]);
        // A dot becomes a small ring; a circle from two arcs stays round.
        let d = trace("M12 20h.01");
        assert!(d[0].1 && d[0].0.len() == 8);
        let c = trace("M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20");
        assert!(c[0].0.iter().all(|p| (p.sub(v2(12., 12.)).len() - 10.).abs() < 0.05));
        // Numbers run together: "-3-8" and ".5.5".
        let n = trace("M0 0l-3-8l.5.5");
        assert_eq!(n[0].0, vec![v2(0., 0.), v2(-3., -8.), v2(-2.5, -7.5)]);
    }
}
