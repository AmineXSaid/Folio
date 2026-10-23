//! The emoji picker for sheets: emoji in tabs, the sheet's one mark
//! (sheets have no colour of their own). Where no emoji exists (a CAN bus,
//! Rust, a car maker) the mark is a short monogram, set as a badge.

use gpui::{div, prelude::*, px, Context, SharedString};

use super::app::Folio;
use crate::theme::Theme;

pub const EMOJI: [(&str, &[&str]); 15] = [
    ("Work", &[
        "💼", "📊", "📈", "📉", "🗂️", "📁", "📂", "🗃️", "📋", "📌", "📍", "📎", "🖇️", "✂️", "🗓️", "📅", "⏰", "⏳",
        "💡", "🧠", "🎯", "🚀", "🏁", "🏆", "📣", "💬", "🗣️", "🤝", "🧾", "💰", "💳", "🏦", "⚖️", "🔒", "🔑", "🛠️",
    ]),
    ("Tech", &[
        "💻", "🖥️", "⌨️", "🖱️", "🧑‍💻", "👩‍💻", "👨‍💻", "🗄️", "💾", "💿", "🧮", "🔌", "🔋", "🪫", "🖨️", "📡", "🛰️", "🌐",
        "🔗", "🧰", "🐛", "🐞", "🧪", "⚙️", "🛠️", "🔐", "🔑", "🧱", "☁️", "📦", "🚦", "🧯", "🔬", "📟", "📲", "🛜",
    ]),
    ("AI & agents", &[
        "🤖", "🧠", "✨", "🦾", "🦿", "👾", "🛸", "🧬", "🔮", "💬", "🗨️", "💭", "🕵️", "🧑‍🚀", "🪄", "🎛️", "🧑‍🔬", "⚛️",
        "🌀", "🧭", "🦉", "🐙", "🕸️", "🧩", "♟️", "🎯", "📈", "AI", "ML", "LLM", "RAG", "MCP", "GPT", "NN", "CV", "NLP",
    ]),
    ("Remember", &[
        "🧠", "📌", "📝", "🗒️", "🔖", "📎", "⏰", "🔔", "🗓️", "💭", "🧷", "✅", "☑️", "📍", "🏷️", "🔁", "⭐", "💡",
        "🗃️", "📇", "🧾", "❗", "⚠️", "🚩", "🎗️", "🪢", "🔑", "📬", "⏳", "🫡",
    ]),
    ("Auto", &[
        "🚗", "🚙", "🏎️", "🚘", "🚖", "🚓", "🚑", "🚒", "🚐", "🛻", "🚚", "🚛", "🚜", "🏍️", "🛵", "🚌", "🔋", "⛽",
        "🔧", "🔩", "🛞", "🚦", "🚥", "🛣️", "🅿️", "🚨", "🏁", "⚙️", "🔌", "🧯", "🛢️", "🪝", "ECU", "ADAS", "EV", "BMS",
    ]),
    ("Car brands", &[
        "BMW", "VW", "AUDI", "BENZ", "TESLA", "FORD", "GM", "KIA", "VOLVO", "FIAT", "BYD", "MAZDA", "HONDA", "LEXUS",
        "MINI", "OPEL", "SEAT", "SKODA", "JEEP", "DACIA", "NIO", "RIVN", "LUCID", "ALFA", "JAG", "LR", "PSA", "STLA",
        "TATA", "GEELY", "POLE", "CUPRA", "DS", "SUBA", "TOYO", "NSSN",
    ]),
    ("Code", &[
        "🦀", "🐍", "☕", "🐹", "💎", "🐘", "🐪", "🦫", "RS", "C", "C++", "C#", "PY", "JS", "TS", "GO", "JAVA", "KT",
        "SWIFT", "DART", "SQL", "SH", "PS", "LUA", "PHP", "RB", "R", "JL", "ZIG", "HTML", "CSS", "YAML", "JSON", "MD",
        "ASM", "VHDL", "M", "GIT", "API", "UI",
    ]),
    ("Buses", &[
        "CAN", "CANFD", "CANXL", "LIN", "ETH", "FLEX", "MOST", "SENT", "SPI", "I2C", "UART", "USB", "OBD", "UDS", "DoIP",
        "SOME", "XCP", "CCP", "J1939", "AUTO", "PSI5", "LVDS", "GMSL", "PCIe", "TSN", "AVB", "V2X", "GPS", "BLE", "WIFI",
        "🚌", "🔌", "🧵", "🌐", "📡", "🔗",
    ]),
    ("Writing", &[
        "📝", "✏️", "🖊️", "🖋️", "📓", "📔", "📒", "📕", "📗", "📘", "📙", "📚", "📖", "🔖", "🏷️", "📰", "🗞️", "📜",
        "📄", "📃", "📑", "✉️", "📮", "📦", "🔍", "🔎", "🗒️", "🧷", "📐", "📏",
    ]),
    ("Home", &[
        "🏠", "🏡", "🛋️", "🛏️", "🚿", "🧺", "🧹", "🧼", "🪴", "🌿", "🕯️", "🖼️", "🪑", "🚪", "🔧", "🔨", "🪛", "🧰",
        "👨‍👩‍👧", "👶", "🐶", "🐱", "🐟", "🐦", "🎁", "🎂", "🎈", "💌", "❤️", "🏥", "💊", "🩺",
    ]),
    ("Food", &[
        "🍳", "🥗", "🍝", "🍕", "🍔", "🌮", "🍣", "🍜", "🥐", "🥖", "🧀", "🍎", "🍋", "🍓", "🥑", "🥕", "🌶️", "🧄",
        "🍰", "🍪", "🍫", "☕", "🍵", "🧃", "🍷", "🍺", "🍹", "🛒", "🥘", "🍲",
    ]),
    ("Nature", &[
        "🌱", "🌳", "🌲", "🌵", "🌸", "🌻", "🌷", "🍀", "🍁", "🍂", "🌊", "🏔️", "⛰️", "🏕️", "🏖️", "🌋", "🌙", "⭐",
        "🌟", "☀️", "🌈", "⛅", "❄️", "🔥", "💧", "🐝", "🦋", "🐢", "🦊", "🐻",
    ]),
    ("Travel", &[
        "✈️", "🚆", "🚗", "🚲", "🛵", "🚢", "⛵", "🗺️", "🧭", "🧳", "🎒", "🏨", "🗽", "🗼", "🏰", "⛩️", "🕌", "🎡",
        "🏝️", "🌍", "🌎", "🌏", "📷", "🎟️", "🛂", "⛽", "🚏", "🛤️",
    ]),
    ("Hobbies", &[
        "🎨", "🖌️", "🎵", "🎸", "🎹", "🥁", "🎧", "🎤", "🎬", "📺", "🎮", "🕹️", "🧩", "♟️", "🎲", "🧶", "🧵", "📸",
        "🏃", "🚴", "🏊", "🧘", "🏋️", "⚽", "🏀", "🎾", "⛳", "🎳", "🎣", "🛹",
    ]),
    ("Symbols", &[
        "✅", "☑️", "❗", "❓", "💯", "⚡", "✨", "💎", "🔔", "♻️", "🔴", "🟠", "🟡", "🟢", "🔵", "🟣", "⚫", "⚪",
        "🟥", "🟧", "🟨", "🟩", "🟦", "🟪", "⬛", "🔶", "🔷", "💠", "🌀", "♾️", "🧪", "🧬", "🔭", "🧲",
    ]),
];

impl Folio {
    /// The picker: tabs, then the grid; `pick` gets the emoji chosen.
    pub fn emoji_picker(
        &self,
        selected: &str,
        t: &Theme,
        cx: &mut Context<Self>,
        pick: impl Fn(&mut Folio, &'static str, &mut Context<Folio>) + Clone + 'static,
    ) -> impl IntoElement {
        let tab = self.emoji_tab.min(EMOJI.len() - 1);
        let tabs = EMOJI.iter().enumerate().map(|(i, (name, list))| {
            let on = i == tab;
            div()
                .id(("emoji-tab", i))
                .h(px(28.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(4.))
                .rounded(t.radius.small)
                .text_size(t.text.caption)
                .cursor_pointer()
                .when(on, |d| d.bg(t.colors.text).text_color(t.colors.surface))
                .when(!on, |d| d.text_color(t.colors.text_muted).hover(|d| d.bg(t.colors.hover)))
                .when(!super::is_monogram(list[0]), |d| d.child(list[0]))
                .child(*name)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.emoji_tab = i;
                    cx.notify();
                }))
        });
        let grid = EMOJI[tab].1.iter().map(|&e| {
            let pick = pick.clone();
            let on = e == selected;
            div()
                .id(SharedString::from(format!("emoji-{e}")))
                .size(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.radius.button)
                .text_size(px(if super::is_monogram(e) { 10. } else { 19. }))
                .when(super::is_monogram(e), |d| {
                    d.font_weight(gpui::FontWeight::BOLD).text_color(t.colors.brand_on_tint).child(
                        // Drawn as the badge it becomes.
                        div().absolute().inset(px(3.)).rounded(t.radius.small).bg(t.colors.brand_tint),
                    )
                })
                .relative()
                .cursor_pointer()
                .border_1()
                .border_color(if on { t.colors.brand } else { t.colors.surface })
                .when(on, |d| d.bg(t.colors.brand_tint))
                .when(!on, |d| d.hover(|d| d.bg(t.colors.hover)))
                .child(e)
                .on_click(cx.listener(move |this, _, _, cx| pick(this, e, cx)))
        });
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(div().flex().flex_wrap().gap(px(2.)).children(tabs))
            // Each tab's emoji fade up in place.
            .child(super::motion::enter(div().flex().flex_wrap().gap(px(2.)).children(grid), ("emoji-grid", tab), t.motion.pop_in, t.motion.sink))
    }
}

/// The emoji for the `n`th new sheet, before you choose one.
pub fn default_icon(n: usize) -> &'static str {
    const START: [&str; 12] = ["📝", "💡", "🎯", "📚", "🏠", "🌱", "✈️", "🎨", "💼", "🍳", "🎵", "⭐"];
    START[n % START.len()]
}
