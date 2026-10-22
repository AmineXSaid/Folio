//! The emoji picker for sheets: about two hundred emoji in tabs, the
//! sheet's one mark (sheets have no colour of their own).

use gpui::{div, prelude::*, px, Context, SharedString};

use super::app::Folio;
use crate::theme::Theme;

pub const EMOJI: [(&str, &[&str]); 8] = [
    ("Work", &[
        "💼", "📊", "📈", "📉", "🗂️", "📁", "📂", "🗃️", "📋", "📌", "📍", "📎", "🖇️", "✂️", "🗓️", "📅", "⏰", "⏳",
        "💡", "🧠", "🎯", "🚀", "🏁", "🏆", "📣", "💬", "🗣️", "🤝", "🧾", "💰", "💳", "🏦", "⚖️", "🔒", "🔑", "🛠️",
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
                .child(list[0])
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
                .text_size(px(19.))
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
            .child(div().flex().flex_wrap().gap(px(2.)).children(grid))
    }
}

/// The emoji for the `n`th new sheet, before you choose one.
pub fn default_icon(n: usize) -> &'static str {
    const START: [&str; 12] = ["📝", "💡", "🎯", "📚", "🏠", "🌱", "✈️", "🎨", "💼", "🍳", "🎵", "⭐"];
    START[n % START.len()]
}
