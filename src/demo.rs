//! The board a first launch opens on: three sheets that show what a sheet
//! can hold, so there's something to move around straight away.

use crate::model::*;

fn note(b: &mut Board, rect: (f32, f32, f32, f32), paper: Paper, title: &str, body: &str) -> Sticker {
    Sticker {
        id: b.fresh_id(),
        rect: Rect { x: rect.0, y: rect.1, w: rect.2, h: rect.3 },
        paper,
        title: title.into(),
        body: body.into(),
        image: None,
        text_size: TextSize::Medium,
        edited: 0,
    }
}

pub fn board() -> Board {
    let mut b = Board { version: FORMAT_VERSION, categories: Vec::new(), active: 0, dark: false, next_id: 0 };

    b.add_sheet("Studio projects");
    let refresh = note(
        &mut b,
        (0., 0., 230., 170.),
        Paper::Butter,
        "Website refresh",
        "Keep it simple. Let the work speak first. Explore a **warmer, more human** direction for the landing page.",
    );
    let explore = note(
        &mut b,
        (320., 60., 240., 190.),
        Paper::Sage,
        "Things to explore",
        "- [x] Soft, tactile colour palette\n- [ ] Editorial typography pairing\n- [ ] Motion that feels <mark color=\"yellow\">considered</mark>",
    );
    let launch = note(
        &mut b,
        (40., 300., 280., 180.),
        Paper::Sky,
        "Launch checklist",
        "- [x] Finalize homepage copy\n- [ ] Review mobile layouts\n- [ ] Share with the team",
    );
    let reminder = note(
        &mut b,
        (420., 340., 230., 150.),
        Paper::Blush,
        "Little reminder",
        "Make it useful. Make it feel good. *Then remove one more thing.*",
    );
    let plan = note(
        &mut b,
        (720., 300., 260., 200.),
        Paper::Plain,
        "Content plan",
        "| Week | Focus |\n|---|---|\n| 01 | Introduce |\n| 02 | Behind the scenes |\n| 03 | Launch |",
    );
    let ideas = note(
        &mut b,
        (700., 20., 240., 180.),
        Paper::Lilac,
        "Shortcuts",
        "Double-click the sheet for a new note.\n\n`V` select · `H` pan · `P` pen · `M` marker · `T` text · `C` connect · `E` eraser",
    );
    let (r1, r2) = (refresh.id, explore.id);
    let (l1, l2) = (launch.id, reminder.id);
    let link_a = b.fresh_id();
    let link_b = b.fresh_id();
    let label = b.fresh_id();
    let stroke = b.fresh_id();
    let sheet = b.sheet_mut();
    sheet.blurb = "Ideas, references & things in progress".into();
    sheet.stickers = vec![refresh, explore, launch, reminder, plan, ideas];
    sheet.links = vec![
        Link { id: link_a, from: r1, to: r2, ink: Ink::Forest },
        Link { id: link_b, from: l1, to: l2, ink: Ink::Forest },
    ];
    sheet.labels = vec![Label {
        id: label,
        at: v2(470., 500.),
        text: "remember to leave room for the unexpected".into(),
        ink: Ink::Graphite,
    }];
    // A hand-drawn underline beneath the label.
    let points = (0..=40)
        .map(|i| {
            let t = i as f32 / 40.;
            v2(466. + t * 300., 526. + (t * 9.).sin() * 2.5 + t * 4.)
        })
        .collect();
    sheet.strokes = vec![Stroke { id: stroke, ink: Ink::Coral, width: 2.5, marker: false, points }];

    b.add_sheet("Personal");
    let a = note(&mut b, (0., 0., 240., 160.), Paper::Sage, "This week", "- [ ] Call the bank\n- [ ] Book dentist\n- [x] Water the plants");
    let c = note(&mut b, (300., 30., 230., 140.), Paper::Butter, "Groceries", "Oat milk, lemons, basil, sourdough, <mark color=\"orange\">coffee!</mark>");
    let sheet = b.sheet_mut();
    sheet.blurb = "Life admin and small joys".into();
    sheet.stickers = vec![a, c];

    b.add_sheet("Reading list");
    let a = note(
        &mut b,
        (0., 0., 300., 200.),
        Paper::Plain,
        "Up next",
        "| Title | Author |\n|---|---|\n| The Timeless Way of Building | Alexander |\n| Thinking with Type | Lupton |\n| Shape Up | Singer |",
    );
    let sheet = b.sheet_mut();
    sheet.blurb = "Books and long reads".into();
    sheet.stickers = vec![a];

    b.active = 0;
    b
}
