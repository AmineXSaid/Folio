//! Sample sheets that show what a sheet can hold. Folio starts empty; these
//! are for the tests, for screenshots and demos (`FOLIO_DEMO=1` on a new
//! data folder), and to tell the samples older versions started with from
//! sheets people made.

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
        ..Default::default()
    }
}

pub fn board() -> Board {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut b = Board::blank();

    b.add_sheet("Studio projects");
    let refresh = note(
        &mut b,
        (0., 0., 230., 170.),
        Paper::Butter,
        "Website refresh",
        "Keep it simple. Let the work speak first. Explore a **warmer, more human** direction for the landing page. #design",
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
    let mut a = a;
    a.due = Some(now + 2 * 3600);
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

    b.add_sheet("Garden");
    let a = note(&mut b, (0., 120., 210., 130.), Paper::Sage, "Bed A", "- [ ] Tomatoes\n- [x] Basil");
    let c = note(&mut b, (250., 0., 210., 130.), Paper::Sage, "Bed B", "- [ ] Lettuce\n- [ ] Radish");
    let d = note(&mut b, (500., 140., 220., 130.), Paper::Butter, "Seeds to order", "Sweet peas, cosmos, dill. #shopping");
    let (ai, ci) = (a.id, c.id);
    let l = b.fresh_id();
    let sheet = b.sheet_mut();
    sheet.blurb = "Beds, seeds and seasons".into();
    sheet.accent = Accent::Moss;
    sheet.icon = "🌱".into();
    sheet.stickers = vec![a, c, d];
    sheet.links = vec![Link { id: l, from: ai, to: ci, ink: Ink::Forest }];

    b.add_sheet("Japan trip");
    let a = note(&mut b, (0., 0., 240., 150.), Paper::Blush, "Kyoto", "- [ ] Fushimi Inari at dawn\n- [ ] Philosopher's Path\n- [ ] Nishiki market");
    let c = note(&mut b, (280., 60., 220., 120.), Paper::Sky, "Osaka", "Dotonbori at night, okonomiyaki. #food");
    let d = note(&mut b, (540., 0., 230., 150.), Paper::Lilac, "Tokyo", "- [ ] Yanaka\n- [ ] teamLab\n- [ ] Shimokitazawa");
    let sheet = b.sheet_mut();
    sheet.blurb = "Kyoto · Osaka · Tokyo".into();
    sheet.accent = Accent::Plum;
    sheet.icon = "✈️".into();
    sheet.stickers = vec![a, c, d];

    // A cheatsheet made of the three note types: taped notes, cheat cards
    // with a table and Remember boxes, index cards with tasks.
    b.add_sheet("HIL testing");
    let styled = |b: &mut Board, rect, paper, style, title: &str, body: &str| Sticker { style, ..note(b, rect, paper, title, body) };
    let what = styled(
        &mut b,
        (0., 12., 250., 120.),
        Paper::Butter,
        NoteStyle::Taped,
        "What is HIL testing?",
        "- **Hardware-in-the-loop:** real controller hardware against a simulated plant.\n- Reads inputs, runs its algorithm, writes outputs: a **closed loop**.\n- Catches timing and I/O faults *before* the field.",
    );
    let family = styled(
        &mut b,
        (282., 36., 330., 160.),
        Paper::Peach,
        NoteStyle::Cheat,
        "Test-in-the-loop family",
        "| Type | Controller | Plant |\n|---|---|---|\n| MIL | Model | Model |\n| SIL | Real code, host | Model |\n| PIL | Real code, target CPU | Model |\n| HIL | Real ECU | Real-time sim |\n| Vehicle | Real hardware | Real plant |\n> **Remember:** each step makes **one more thing real**.",
    );
    let bench = styled(
        &mut b,
        (652., 54., 262., 120.),
        Paper::Plain,
        NoteStyle::Index,
        "Before the bench session",
        "- [x] Flash the ECU build\n- [ ] Load the plant model\n- [ ] Check the CAN wiring\n- [ ] Log the run",
    );
    let sil = styled(
        &mut b,
        (958., 36., 250., 160.),
        Paper::Sky,
        NoteStyle::Cheat,
        "SIL · Software-in-the-loop",
        "- Controller = **real production code** on a host PC or emulator.\n- Plant = still a **software model**.\n- Built for the host: `make sil`\n> **Remember:** the same code on the **target CPU** is PIL.",
    );
    let pitfalls = styled(
        &mut b,
        (16., 290., 270., 120.),
        Paper::Rose,
        NoteStyle::Taped,
        "Common pitfalls",
        "- “MIL simulates the hardware”: no, both sides are **models**.\n- “System-in-the-loop” isn't standard; say **bench** or **vehicle test**.",
    );
    let mut remember = styled(
        &mut b,
        (672., 326., 240., 100.),
        Paper::Lilac,
        NoteStyle::Index,
        "Remember",
        "MIL → SIL → PIL → HIL: one more real thing at each step.",
    );
    remember.font = NoteFont::Hand;
    remember.pin = Some(Pin { shape: PinShape::Star, size: PinSize::Medium, tone: PinTone::Yellow });
    let (mut family, mut bench, mut sil) = (family, bench, sil);
    family.pin = Some(Pin { shape: PinShape::Push, size: PinSize::Medium, tone: PinTone::Red });
    bench.pin = Some(Pin { shape: PinShape::Ball, size: PinSize::Small, tone: PinTone::Blue });
    sil.pin = Some(Pin { shape: PinShape::Chip, size: PinSize::Medium, tone: PinTone::default() });
    let mut booked = styled(&mut b, (972., 380., 232., 100.), Paper::Mint, NoteStyle::Taped, "Bench booked", "Lab 2, on Sam's rig. Bring the harness.");
    booked.due = Some(now + 2 * 86_400);
    let label = b.fresh_id();
    let stroke = b.fresh_id();
    let sheet = b.sheet_mut();
    sheet.blurb = "Cheatsheet for the test bench".into();
    sheet.accent = Accent::Slate;
    sheet.icon = "🧪".into();
    sheet.stickers = vec![what, family, bench, sil, pitfalls, remember, booked];
    sheet.labels = vec![Label { id: label, at: v2(350., 420.), text: "ask Sam for the CAN logger".into(), ink: Ink::Graphite }];
    let points = (0..=40)
        .map(|i| {
            let t = i as f32 / 40.;
            v2(346. + t * 200., 446. + (t * 7.).sin() * 2. + t * 3.)
        })
        .collect();
    sheet.strokes = vec![Stroke { id: stroke, ink: Ink::Coral, width: 2.5, marker: false, points }];

    b.categories[0].pinned = true;
    b.categories[0].icon = "🎨".into();
    b.categories[1].icon = "🏠".into();
    b.categories[2].icon = "📚".into();
    for c in &mut b.categories {
        c.edited = now;
        for s in &mut c.stickers {
            s.edited = now;
        }
    }
    // The cheatsheet opens first; it was made last so the older samples
    // keep their ids.
    let cheat = b.categories.pop().expect("the cheatsheet");
    b.categories.insert(0, cheat);
    b.active = 0;
    b
}

/// Removes the sample sheets an older Folio started with, if they're as
/// they came: same id and contents (where they're scrolled to, when they
/// were opened and reminders that went off don't count as changes).
pub fn remove_untouched(b: &mut Board) {
    fn plain(c: &Category) -> Category {
        let mut c = c.clone();
        c.camera = Camera::default();
        c.opened = 0;
        c.edited = 0;
        for s in &mut c.stickers {
            s.edited = 0;
            s.due = None;
            s.reminded = false;
            s.popped = None;
            s.snoozed = 0;
        }
        c
    }
    let samples: Vec<Category> = board().categories.iter().map(plain).collect();
    let active = b.categories.get(b.active).map(|c| c.id);
    b.categories.retain(|c| !samples.iter().any(|s| s.id == c.id && *s == plain(c)));
    b.active = active.and_then(|id| b.categories.iter().position(|c| c.id == id)).unwrap_or(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_samples_go_and_edited_ones_stay() {
        let mut old = board();
        old.version = 2;
        old.categories[2].camera.zoom = 2.;
        old.categories[3].stickers[0].body.push_str(" and more");
        let kept = old.categories[3].name.clone();
        old.add_sheet("Mine");
        old.normalize();
        let names: Vec<&str> = old.categories.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, [kept.as_str(), "Mine"]);

        let mut untouched = board();
        untouched.version = 2;
        untouched.normalize();
        assert_eq!(untouched.categories.len(), 1);
        assert_eq!(untouched.categories[0].name, "My notes");
    }

    #[test]
    fn current_boards_are_left_alone() {
        let mut b = board();
        let n = b.categories.len();
        b.normalize();
        assert_eq!(b.categories.len(), n);
    }
}
