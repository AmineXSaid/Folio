//! Templates: ready-made layouts of areas, notes, text, shapes and links for
//! the ways people work (retrospectives, brainstorming, strategy canvases,
//! product planning, research, meetings, diagrams, planning your own week).
//! Each is built as a `Piece` laid out from (0, 0), so it can start a new
//! sheet or be dropped into the one you're on. All of Folio's own making:
//! the methods are common knowledge, the layouts are drawn here.
//! Nothing here knows about GPUI.

use crate::model::*;
use crate::shape::Geom;

/// A family of templates, as the gallery groups them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Basics,
    Retrospectives,
    Brainstorming,
    Strategy,
    Product,
    Research,
    Meetings,
    Diagrams,
    Personal,
}

impl Family {
    pub const ALL: [Family; 9] = [
        Family::Basics,
        Family::Retrospectives,
        Family::Brainstorming,
        Family::Strategy,
        Family::Product,
        Family::Research,
        Family::Meetings,
        Family::Diagrams,
        Family::Personal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Basics => "Basics",
            Family::Retrospectives => "Retrospectives",
            Family::Brainstorming => "Brainstorming",
            Family::Strategy => "Strategy",
            Family::Product => "Product",
            Family::Research => "UX & research",
            Family::Meetings => "Meetings & teams",
            Family::Diagrams => "Diagrams",
            Family::Personal => "Personal",
        }
    }
}

macro_rules! templates {
    ($($v:ident: $name:literal, $fam:ident, $blurb:literal, $words:literal;)*) => {
        /// A template: what a new sheet starts from, or what's dropped into one.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Template { $($v,)* }

        impl Template {
            pub const ALL: &'static [Template] = &[$(Template::$v,)*];
            pub fn name(self) -> &'static str { match self { $(Template::$v => $name,)* } }
            pub fn family(self) -> Family { match self { $(Template::$v => Family::$fam,)* } }
            pub fn blurb(self) -> &'static str { match self { $(Template::$v => $blurb,)* } }
            /// Other words it's found by.
            pub fn keywords(self) -> &'static str { match self { $(Template::$v => $words,)* } }
        }
    };
}

templates! {
    Blank: "Blank", Basics, "An empty sheet", "empty new";
    // Retrospectives.
    QuickRetro: "Quick Retrospective", Retrospectives, "Continue, Stop, Invent, Act", "retro sprint review agile";
    StartStopContinue: "Start, Stop, Continue", Retrospectives, "Three columns for the next sprint", "retro sprint";
    FourLs: "4Ls", Retrospectives, "Liked, Learned, Lacked, Longed for", "retro four ls";
    MadSadGlad: "Mad, Sad, Glad", Retrospectives, "How the sprint felt", "retro feelings emotions";
    Starfish: "Starfish", Retrospectives, "Keep, less, more, stop, start", "retro";
    Sailboat: "Sailboat", Retrospectives, "Wind, anchors, rocks and the island", "retro speedboat";
    HotAirBalloon: "Hot-air balloon", Retrospectives, "What lifts us, what weighs us down", "retro balloon";
    RoseThornBud: "Rose, Thorn, Bud", Retrospectives, "Highs, lows and what's growing", "retro reflection";
    // Brainstorming.
    Brainwriting: "Brainwriting 6-3-5", Brainstorming, "Six people, three ideas each, passed round", "ideas ideation participants";
    Crazy8s: "Crazy 8s", Brainstorming, "Eight ideas in eight minutes", "sketch design sprint ideas";
    MindMap: "Mind map", Brainstorming, "One topic, branches all round", "mindmap ideas topic";
    Affinity: "Affinity map", Brainstorming, "Loose ideas sorted into themes", "cluster group themes";
    Brainstorm: "Brainstorm", Brainstorming, "One idea in the middle, thoughts around it", "ideas";
    HowMightWe: "How might we", Brainstorming, "Turn a problem into questions to answer", "hmw problem questions";
    RoundRobin: "Round robin", Brainstorming, "Idea, why it fails, how to fix it", "ideation critique";
    Lotus: "Lotus blossom", Brainstorming, "A topic and eight ideas round it", "lotus ideas";
    // Strategy.
    Swot: "SWOT", Strategy, "Strengths, weaknesses, opportunities, threats", "analysis strategy";
    BusinessModel: "Business Model Canvas", Strategy, "Nine blocks of how a business works", "bmc startup canvas";
    LeanCanvas: "Lean Canvas", Strategy, "A one-page plan for a new idea", "startup canvas problem solution";
    Pest: "PEST", Strategy, "Political, economic, social, technological", "pestle analysis environment";
    Okrs: "OKRs", Strategy, "Objectives and their key results", "goals objectives key results";
    FiveForces: "Five forces", Strategy, "What shapes competition in a market", "porter competition market";
    ValueProp: "Value proposition canvas", Strategy, "What customers need, what you offer", "customer jobs pains gains";
    Ansoff: "Ansoff matrix", Strategy, "Four ways to grow", "growth markets products";
    // Product.
    StoryMap: "User story map", Product, "The journey, its steps, and releases", "stories backlog agile";
    Roadmap: "Product roadmap", Product, "Now, next and later, by team", "plan timeline";
    Kanban: "Kanban", Product, "To do, Doing, Done", "board tasks todo";
    ImpactEffort: "Impact / effort", Product, "Quick wins first", "prioritize priority matrix";
    Moscow: "MoSCoW", Product, "Must, should, could, won't", "prioritize priority";
    Rice: "RICE scoring", Product, "Reach, impact, confidence, effort", "prioritize score features";
    ReleasePlan: "Release plan", Product, "Sprint by sprint to a release", "sprints release";
    SprintPlanning: "Sprint planning", Product, "Goal, capacity and what's in", "sprint scrum agile";
    // UX & research.
    Journey: "Customer journey", Research, "Stages, and what people do, think and feel", "touchpoint map experience";
    EmpathyMap: "Empathy map", Research, "Says, thinks, does, feels", "user empathy";
    Persona: "Persona", Research, "Who you're designing for", "user profile";
    Blueprint: "Service blueprint", Research, "What customers see and what's behind it", "service frontstage backstage";
    Interview: "Interview notes", Research, "Questions, quotes and what you learned", "user research interview";
    Jtbd: "Jobs to be done", Research, "When, I want to, so I can", "jtbd jobs";
    Usability: "Usability findings", Research, "Problems found, by how bad", "usability test findings";
    // Meetings & teams.
    IceBreaker: "Ice breaker", Meetings, "Two truths and a lie", "warm up team game";
    Meeting: "Meeting notes", Meetings, "Agenda, notes, decisions, actions", "meeting agenda";
    TeamSync: "Weekly team sync", Meetings, "Wins, blockers, this week", "team weekly status";
    DecisionLog: "Decision log", Meetings, "What was decided, why and by whom", "decisions record";
    TeamCharter: "Team charter", Meetings, "Purpose, values and how we work", "team agreement";
    Raci: "RACI", Meetings, "Who's responsible, accountable, consulted, informed", "roles responsibilities";
    Kickoff: "Project kickoff", Meetings, "Goal, scope, team, timeline, risks", "project start";
    Standup: "Stand-up", Meetings, "Yesterday, today, blockers", "daily scrum";
    // Diagrams.
    Flowchart: "Flowchart", Diagrams, "Steps and decisions, start to end", "flow process diagram";
    OrgChart: "Org chart", Diagrams, "Who reports to whom", "organization team structure";
    Timeline: "Timeline", Diagrams, "Milestones along a line", "milestones history schedule";
    Fishbone: "Fishbone", Diagrams, "Causes of a problem, by kind", "ishikawa cause effect";
    Venn2: "Venn, two circles", Diagrams, "What two things share", "overlap compare";
    Venn3: "Venn, three circles", Diagrams, "What three things share", "overlap compare";
    Swimlanes: "Swimlanes", Diagrams, "Who does which step", "process lanes";
    ConceptMap: "Concept map", Diagrams, "Ideas and how they connect", "concepts relations";
    // Personal.
    Week: "Weekly planner", Personal, "A column for every day", "week days plan";
    Reading: "Reading list", Personal, "Books and articles to get to", "books";
    Eisenhower: "Eisenhower matrix", Personal, "Urgent, important, or neither", "priority tasks";
    Habits: "Habit tracker", Personal, "A tick for every day", "habits routine";
    Quarters: "Goals by quarter", Personal, "A year in four parts", "goals year plan";
    ProsCons: "Pros and cons", Personal, "Both sides of a decision", "decision compare";
    StudyPlan: "Study plan", Personal, "Topics, this week, review", "study exam learning";
}

/// A template laid out from (0, 0): what it puts on a sheet.
#[derive(Clone, Debug, Default)]
pub struct Piece {
    pub blurb: String,
    pub frames: Vec<Frame>,
    pub stickers: Vec<Sticker>,
    pub shapes: Vec<Shape>,
    pub labels: Vec<Label>,
    pub links: Vec<Link>,
}

impl Piece {
    /// The box round everything in it.
    pub fn bounds(&self) -> Option<Rect> {
        self.frames
            .iter()
            .map(|f| f.rect)
            .chain(self.stickers.iter().map(|s| s.rect))
            .chain(self.shapes.iter().map(Shape::bounds))
            .chain(self.labels.iter().map(label_rect))
            .reduce(|a, b| a.union(&b))
    }

    /// Everything moved by `d`.
    pub fn translate(&mut self, d: Vec2) {
        self.frames.iter_mut().for_each(|f| {
            f.rect.x += d.x;
            f.rect.y += d.y;
        });
        self.stickers.iter_mut().for_each(|s| {
            s.rect.x += d.x;
            s.rect.y += d.y;
        });
        self.shapes.iter_mut().for_each(|s| {
            s.rect.x += d.x;
            s.rect.y += d.y;
        });
        self.labels.iter_mut().for_each(|l| l.at = l.at.add(d));
    }

    /// It as a sheet of its own, for a preview.
    pub fn as_sheet(&self) -> Category {
        let mut c = Category::new(0, "", Accent::Slate);
        c.frames = self.frames.clone();
        c.stickers = self.stickers.clone();
        c.shapes = self.shapes.clone();
        c.labels = self.labels.clone();
        c.links = self.links.clone();
        c
    }
}

/// An area's colour, and the paper its notes are on.
const ACCENTS: [(Accent, Paper); 5] =
    [(Accent::Moss, Paper::Leaf), (Accent::Rose, Paper::Bubblegum), (Accent::Amber, Paper::Lemon), (Accent::Slate, Paper::Azure), (Accent::Plum, Paper::Grape)];

/// The papers a grid of notes runs through, column by column.
const GRID: [Paper; 7] = [Paper::Lemon, Paper::Azure, Paper::Sky, Paper::Leaf, Paper::Tangerine, Paper::Bubblegum, Paper::Grape];

/// An idea note's side: templates' notes are square.
const SQ: f32 = 150.;

/// The height of an area's title bar, where nothing else goes.
const BAR: f32 = 40.;
/// A note's height as templates place it.
const NOTE_H: f32 = 110.;

/// Builds a piece: places things and hands out ids.
struct B<'a> {
    p: Piece,
    ids: &'a mut dyn FnMut() -> Id,
    now: u64,
    /// How the areas it makes look.
    style: AreaStyle,
}

impl B<'_> {
    fn id(&mut self) -> Id {
        (self.ids)()
    }

    fn area(&mut self, x: f32, y: f32, w: f32, h: f32, title: &str, accent: Accent) {
        let id = self.id();
        let style = self.style;
        self.p.frames.push(Frame { id, rect: Rect { x, y, w, h }, title: title.into(), accent, style });
    }

    /// Small slanted words under an area's title saying what goes in it.
    fn hint(&mut self, x: f32, y: f32, w: f32, text: &str) {
        let id = self.id();
        self.p.labels.push(Label { text: text.into(), size: LabelSize::S, width: Some(w), italic: true, ..Label::new(id, v2(x, y), Ink::Slate) });
    }

    /// The same, centred under a filled area's centred title.
    fn hint_centred(&mut self, x: f32, y: f32, w: f32, text: &str) {
        let id = self.id();
        self.p.labels.push(Label { text: text.into(), size: LabelSize::S, width: Some(w), italic: true, align: Align::Center, ..Label::new(id, v2(x, y), Ink::Slate) });
    }

    /// A white board with its title above it, and a grid of square notes.
    fn board(&mut self, title: &str, cols: usize, rows: usize, side: f32, top: f32, title_of: impl Fn(usize, usize) -> String) {
        let gap = 24.;
        let w = cols as f32 * (side + gap) + gap;
        let h = top + rows as f32 * (side + gap) + gap;
        let was = self.style;
        self.style = AreaStyle::Board;
        self.area(0., 0., w, h, title, Accent::Slate);
        self.style = was;
        self.grid(gap, top + gap, cols, rows, side, side, title_of);
    }

    /// Words on the sheet, as a heading or a note to the reader.
    fn text(&mut self, x: f32, y: f32, w: Option<f32>, size: LabelSize, text: &str) {
        let id = self.id();
        self.p.labels.push(Label { text: text.into(), size, width: w, ..Label::new(id, v2(x, y), Ink::Graphite) });
    }

    fn note_sized(&mut self, x: f32, y: f32, w: f32, h: f32, paper: Paper, title: &str, body: &str) -> Id {
        let id = self.id();
        self.p.stickers.push(Sticker { id, rect: Rect { x, y, w, h }, paper, title: title.into(), body: body.into(), edited: self.now, ..Default::default() });
        id
    }

    fn note(&mut self, x: f32, y: f32, w: f32, paper: Paper, title: &str, body: &str) -> Id {
        self.note_sized(x, y, w, NOTE_H, paper, title, body)
    }

    fn shape(&mut self, geom: Geom, x: f32, y: f32, w: f32, h: f32, text: &str, fill: Option<Paper>, ink: Ink) -> Id {
        let id = self.id();
        let mut s = Shape::new(id, geom, v2(0., 0.));
        s.rect = Rect { x, y, w, h };
        s.text = text.into();
        s.fill = fill.filter(|_| geom.fills());
        s.ink = ink;
        self.p.shapes.push(s);
        id
    }

    fn link(&mut self, a: Id, b: Id, style: LinkStyle, arrow: bool) {
        let id = self.id();
        self.p.links.push(Link { style, ends: if arrow { Ends::End } else { Ends::None }, ..Link::new(id, a, b, Ink::Slate) });
    }

    /// An area with its hint, and a first note in it to write on.
    fn zone(&mut self, x: f32, y: f32, w: f32, h: f32, title: &str, k: usize, hint: &str, starter: bool) {
        let (accent, paper) = ACCENTS[k % ACCENTS.len()];
        self.area(x, y, w, h, title, accent);
        if !hint.is_empty() {
            match self.style {
                AreaStyle::Filled => self.hint_centred(x + 20., y + BAR + 4., w - 40., hint),
                _ => self.hint(x + 20., y + BAR + 10., w - 40., hint),
            }
        }
        if starter {
            let side = (w - 48.).min(SQ);
            self.note_sized(x + 24., y + BAR + 50., side, side, paper, "", "");
        }
    }

    /// Areas side by side, each with its hint and a first note.
    fn columns(&mut self, x: f32, y: f32, w: f32, h: f32, cols: &[(&str, &str)]) {
        for (i, (title, hint)) in cols.iter().enumerate() {
            self.zone(x + i as f32 * (w + 24.), y, w, h, title, i, hint, true);
        }
    }

    /// Four areas, two by two.
    fn matrix(&mut self, x: f32, y: f32, w: f32, h: f32, cells: [(&str, &str); 4]) {
        for (i, (title, hint)) in cells.iter().enumerate() {
            let (cx, cy) = ((i % 2) as f32 * (w + 24.), (i / 2) as f32 * (h + 24.));
            self.zone(x + cx, y + cy, w, h, title, i, hint, true);
        }
    }

    /// A grid of notes, each column in its own paper.
    fn grid(&mut self, x: f32, y: f32, cols: usize, rows: usize, w: f32, h: f32, title: impl Fn(usize, usize) -> String) {
        let papers = GRID;
        for r in 0..rows {
            for c in 0..cols {
                let t = title(c, r);
                self.note_sized(x + c as f32 * (w + 16.), y + r as f32 * (h + 16.), w, h, papers[c % papers.len()], &t, "");
            }
        }
    }

    /// A note holding a Markdown table with these columns and empty rows.
    fn table(&mut self, x: f32, y: f32, w: f32, title: &str, heads: &[&str], rows: usize, paper: Paper) {
        let mut t = format!("| {} |\n|{}\n", heads.join(" | "), " --- |".repeat(heads.len()));
        for _ in 0..rows {
            t.push_str(&format!("|{}\n", "  |".repeat(heads.len())));
        }
        self.note_sized(x, y, w, 60. + 36. * rows as f32, paper, title, t.trim_end());
    }

    /// A canvas of blocks: three tall on the outside, two stacked in the
    /// second and fourth columns, a wide one in the middle, and two along
    /// the foot (the Business Model and Lean canvases).
    fn canvas(&mut self, blocks: [(&str, &str); 10]) {
        let (cw, top, foot) = (260., 640., 260.);
        let mut k = 0;
        let mut put = |b: &mut Self, x: f32, y: f32, w: f32, h: f32, i: usize| {
            let (title, hint) = blocks[i];
            b.zone(x, y, w, h, title, k, hint, false);
            k += 1;
        };
        put(self, 0., 0., cw, top, 0);
        put(self, cw + 16., 0., cw, top / 2. - 8., 1);
        put(self, cw + 16., top / 2. + 8., cw, top / 2. - 8., 2);
        put(self, 2. * (cw + 16.), 0., cw, top, 3);
        put(self, 3. * (cw + 16.), 0., cw, top / 2. - 8., 4);
        put(self, 3. * (cw + 16.), top / 2. + 8., cw, top / 2. - 8., 5);
        put(self, 4. * (cw + 16.), 0., cw, top, 6);
        let half = (5. * cw + 4. * 16.) / 2. - 8.;
        put(self, 0., top + 16., half, foot, 7);
        put(self, half + 16., top + 16., half, foot, 8);
        let _ = blocks[9];
    }
}

/// Builds template `t` from (0, 0), taking new ids from `ids`.
pub fn build(t: Template, ids: &mut dyn FnMut() -> Id, now: u64) -> Piece {
    let mut b = B { p: Piece::default(), ids, now, style: AreaStyle::Filled };
    b.p.blurb = t.blurb().to_string();
    let (s, l) = (LabelSize::S, LabelSize::L);
    match t {
        Template::Blank => {}
        // ---------- Retrospectives ----------
        Template::QuickRetro => b.matrix(
            0.,
            0.,
            520.,
            400.,
            [
                ("Continue ✅", "What helped us move forward?"),
                ("Stop 🛑", "What held us back?"),
                ("Invent 💡", "How could we do things differently?"),
                ("Act 👍", "What should we do next?"),
            ],
        ),
        Template::StartStopContinue => b.columns(
            0.,
            0.,
            320.,
            560.,
            &[("Start 🚀", "What should we begin doing?"), ("Stop ✋", "What isn't helping?"), ("Continue 🔁", "What's working well?")],
        ),
        Template::FourLs => b.matrix(
            0.,
            0.,
            460.,
            360.,
            [("Liked 💚", "What did you enjoy?"), ("Learned 📘", "What did you find out?"), ("Lacked 🕳️", "What was missing?"), ("Longed for 🌟", "What did you wish for?")],
        ),
        Template::MadSadGlad => {
            b.columns(0., 0., 320., 520., &[("Mad 😠", "What frustrated you?"), ("Sad 😢", "What disappointed you?"), ("Glad 😊", "What made you happy?")])
        }
        Template::Starfish => b.columns(
            0.,
            0.,
            260.,
            520.,
            &[
                ("Keep doing", "Working, keep it up"),
                ("Less of", "Some value, but too much"),
                ("More of", "Worth doing more"),
                ("Stop doing", "Not bringing anything"),
                ("Start doing", "New things to try"),
            ],
        ),
        Template::Sailboat => {
            b.matrix(
                0.,
                0.,
                480.,
                360.,
                [
                    ("Wind 🌬️", "What pushes us forward?"),
                    ("Island 🏝️", "Where are we heading?"),
                    ("Anchors ⚓", "What slows us down?"),
                    ("Rocks 🪨", "What risks lie ahead?"),
                ],
            );
        }
        Template::HotAirBalloon => b.columns(
            0.,
            0.,
            280.,
            520.,
            &[
                ("Hot air 🔥", "What lifts us up?"),
                ("Sandbags", "What weighs us down?"),
                ("Storms ⛈️", "What could blow us off course?"),
                ("Sunny skies ☀️", "What's going well?"),
            ],
        ),
        Template::RoseThornBud => b.columns(
            0.,
            0.,
            320.,
            520.,
            &[("Rose 🌹", "What went well?"), ("Thorn 🌵", "What hurt?"), ("Bud 🌱", "What could grow into something?")],
        ),
        // ---------- Brainstorming ----------
        Template::Brainwriting => b.board("Brainwriting 6-3-5", 6, 6, 120., 0., |c, r| match r {
            0 => format!("Participant {}", c + 1),
            1 if c < 3 => format!("Idea {}", c + 1),
            _ => String::new(),
        }),
        Template::Crazy8s => {
            b.board("Crazy 8s", 4, 2, 200., 40., |c, r| (r * 4 + c + 1).to_string());
            b.hint(24., 12., 600., "Eight ideas in eight minutes: one a minute, rough is fine.");
        }
        Template::MindMap => {
            let centre = b.note(420., 260., 240., Paper::Bubblegum, "Main topic", "");
            let spots = [(0., 0.), (420., -40.), (840., 0.), (0., 520.), (420., 560.), (840., 520.)];
            for (i, (x, y)) in spots.iter().enumerate() {
                let n = b.note(*x, *y, 220., ACCENTS[i % 5].1, &format!("Branch {}", i + 1), "");
                b.link(centre, n, LinkStyle::Curve, false);
            }
        }
        Template::Affinity => {
            b.area(0., 0., 520., 520., "Ideas", Accent::Slate);
            b.hint(20., BAR + 10., 480., "Put every idea here, then sort them into themes.");
            b.grid(20., BAR + 56., 2, 3, 220., 110., |_, _| String::new());
            for (i, t) in ["Theme A", "Theme B", "Theme C"].iter().enumerate() {
                b.zone(560. + i as f32 * 300., 0., 280., 520., t, i, "", false);
            }
        }
        Template::Brainstorm => {
            let centre = b.note(300., 220., 240., Paper::Tangerine, "The big question", "What are we trying to figure out?");
            let spots = [(0., 0.), (600., 0.), (0., 440.), (600., 440.)];
            for (i, (x, y)) in spots.iter().enumerate() {
                let n = b.note(*x, *y, 220., [Paper::Lemon, Paper::Leaf, Paper::Azure, Paper::Grape][i], "", "An idea…");
                b.link(centre, n, LinkStyle::Curve, false);
            }
        }
        Template::HowMightWe => {
            b.note(0., 0., 520., Paper::Bubblegum, "The problem", "Write it as people feel it.");
            b.area(0., 150., 1060., 440., "How might we…?", Accent::Moss);
            b.hint(20., 150. + BAR + 10., 900., "Turn the problem into questions that start with \"How might we\".");
            b.grid(20., 150. + BAR + 56., 4, 2, 240., 130., |_, _| "How might we…".to_string());
        }
        Template::RoundRobin => {
            let cols = [("Idea 💡", 0), ("Why it might fail", 1), ("How to fix it", 2), ("The idea, better", 3)];
            for (i, (t, k)) in cols.iter().enumerate() {
                b.zone(i as f32 * 284., 0., 260., 560., t, *k, "", false);
                for r in 0..3 {
                    b.note(i as f32 * 284. + 20., BAR + 20. + r as f32 * 160., 220., ACCENTS[*k].1, "", "");
                }
            }
        }
        Template::Lotus => {
            b.grid(0., 0., 3, 3, 220., 160., |c, r| if (c, r) == (1, 1) { "Topic".into() } else { String::new() });
        }
        // ---------- Strategy ----------
        Template::Swot => b.matrix(
            0.,
            0.,
            480.,
            380.,
            [
                ("Strengths 💪", "What do we do well?"),
                ("Weaknesses", "Where do we fall short?"),
                ("Opportunities 🌱", "What could we take advantage of?"),
                ("Threats ⚠️", "What could hurt us?"),
            ],
        ),
        Template::BusinessModel => b.canvas([
            ("Key partners", "Who helps us?"),
            ("Key activities", "What do we do?"),
            ("Key resources", "What do we need?"),
            ("Value propositions", "What do we offer, and why does it matter?"),
            ("Customer relationships", "How do we keep in touch?"),
            ("Channels", "How do we reach them?"),
            ("Customer segments", "Who is it for?"),
            ("Cost structure", "What does it cost?"),
            ("Revenue streams", "How does money come in?"),
            ("", ""),
        ]),
        Template::LeanCanvas => b.canvas([
            ("Problem", "The top three problems"),
            ("Solution", "A solution to each"),
            ("Key metrics", "The numbers that matter"),
            ("Unique value proposition", "Why you, in one sentence"),
            ("Unfair advantage", "What can't easily be copied"),
            ("Channels", "Paths to customers"),
            ("Customer segments", "Who has the problem"),
            ("Cost structure", "What it costs"),
            ("Revenue streams", "How it makes money"),
            ("", ""),
        ]),
        Template::Pest => b.matrix(
            0.,
            0.,
            460.,
            360.,
            [
                ("Political 🏛️", "Rules, policy, elections"),
                ("Economic 💶", "Prices, growth, spending"),
                ("Social 👥", "Habits, trends, people"),
                ("Technological 💻", "New tools and changes"),
            ],
        ),
        Template::Okrs => {
            for i in 0..3 {
                let y = i as f32 * 320.;
                b.zone(0., y, 1000., 300., &format!("Objective {}", i + 1), i, "Something you want to reach, said plainly", false);
                for k in 0..3 {
                    b.note(20. + k as f32 * 320., y + BAR + 56., 300., ACCENTS[i].1, &format!("Key result {}", k + 1), "From __ to __");
                }
            }
        }
        Template::FiveForces => {
            b.zone(340., 300., 320., 260., "Rivalry ⚔️", 0, "How fierce is competition now?", false);
            b.zone(340., 0., 320., 260., "New entrants", 1, "How easy is it to come in?", false);
            b.zone(700., 300., 320., 260., "Buyers", 2, "How much say do customers have?", false);
            b.zone(340., 600., 320., 260., "Substitutes", 3, "What else could do the job?", false);
            b.zone(-20., 300., 320., 260., "Suppliers", 4, "How much say do suppliers have?", false);
        }
        Template::ValueProp => {
            b.area(0., 0., 600., 560., "Value map", Accent::Plum);
            b.area(640., 0., 600., 560., "Customer profile", Accent::Moss);
            for (i, t) in ["Products & services", "Pain relievers", "Gain creators"].iter().enumerate() {
                b.note(20., BAR + 20. + i as f32 * 160., 560., Paper::Grape, t, "");
            }
            for (i, t) in ["Customer jobs", "Pains", "Gains"].iter().enumerate() {
                b.note(660., BAR + 20. + i as f32 * 160., 560., Paper::Leaf, t, "");
            }
        }
        Template::Ansoff => {
            b.matrix(
                0.,
                60.,
                440.,
                320.,
                [
                    ("Market penetration", "Existing products, existing markets"),
                    ("Product development", "New products, existing markets"),
                    ("Market development", "Existing products, new markets"),
                    ("Diversification", "New products, new markets"),
                ],
            );
            b.text(0., 0., None, LabelSize::M, "**Products** → existing · new      **Markets** ↓ existing · new");
        }
        // ---------- Product ----------
        Template::StoryMap => {
            b.text(0., 0., None, l, "**User story map**");
            for i in 0..5 {
                let x = i as f32 * 250.;
                b.note(x, 70., 230., Paper::Azure, &format!("Activity {}", i + 1), "What the user wants to get done");
                b.note(x, 200., 230., Paper::Lemon, &format!("Step {}", i + 1), "");
            }
            for (r, t) in ["Release 1", "Release 2"].iter().enumerate() {
                let y = 340. + r as f32 * 200.;
                b.zone(0., y, 1230., 180., t, r + 2, "", false);
                for i in 0..3 {
                    b.note(20. + i as f32 * 250., y + BAR + 10., 230., Paper::Leaf, "", "As a … I want … so that …");
                }
            }
        }
        Template::Roadmap => {
            let lanes = ["Design", "Build", "Launch"];
            for (c, t) in ["Now", "Next", "Later"].iter().enumerate() {
                b.zone(140. + c as f32 * 340., 0., 320., 60. + 3. * 160., t, c, "", false);
                for r in 0..lanes.len() {
                    b.note(160. + c as f32 * 340., BAR + 20. + r as f32 * 160., 280., ACCENTS[c].1, "", "");
                }
            }
            for (r, t) in lanes.iter().enumerate() {
                b.text(0., BAR + 60. + r as f32 * 160., Some(120.), LabelSize::M, &format!("**{t}**"));
            }
        }
        Template::Kanban => {
            for (i, (t, a)) in [("To do", Accent::Slate), ("Doing", Accent::Amber), ("Done", Accent::Moss)].iter().enumerate() {
                b.area(i as f32 * 320., 0., 296., 560., t, *a);
            }
            b.note_sized(24., 64., SQ, SQ, Paper::Lemon, "First task", "Drag me to Doing when you start.");
        }
        Template::ImpactEffort => {
            b.matrix(
                40.,
                40.,
                440.,
                320.,
                [
                    ("Quick wins ⚡", "High impact, low effort: do these first"),
                    ("Big bets 🎯", "High impact, high effort: plan them"),
                    ("Fill-ins", "Low impact, low effort: when there's time"),
                    ("Time sinks 🕳️", "Low impact, high effort: avoid"),
                ],
            );
            b.text(40., 0., None, s, "Effort → low · high");
            b.text(0., 120., Some(30.), s, "Impact ↓");
        }
        Template::Moscow => b.columns(
            0.,
            0.,
            280.,
            560.,
            &[
                ("Must have", "It doesn't work without these"),
                ("Should have", "Important, not vital"),
                ("Could have", "Nice if there's room"),
                ("Won't have", "Not this time"),
            ],
        ),
        Template::Rice => {
            b.table(0., 0., 760., "RICE scoring", &["Feature", "Reach", "Impact", "Confidence", "Effort", "Score"], 6, Paper::Plain);
            b.hint(0., 360., 760., "Score = Reach × Impact × Confidence ÷ Effort. The highest score goes first.");
        }
        Template::ReleasePlan => {
            b.note(0., 0., 520., Paper::Bubblegum, "Release goal", "What's true for users when it ships?");
            for i in 0..4 {
                b.zone(i as f32 * 284., 150., 260., 420., &format!("Sprint {}", i + 1), i, "", false);
                b.note(i as f32 * 284. + 20., 150. + BAR + 20., 220., ACCENTS[i].1, "", "");
            }
        }
        Template::SprintPlanning => {
            b.note(0., 0., 400., Paper::Bubblegum, "Sprint goal", "");
            b.note(420., 0., 300., Paper::Tangerine, "Capacity", "People and days");
            b.columns(0., 150., 300., 460., &[("Backlog", "Ready to pick from"), ("In this sprint", "What we commit to"), ("Risks ⚠️", "What could get in the way")]);
        }
        // ---------- UX & research ----------
        Template::Journey => {
            let stages = ["Aware", "Consider", "Buy", "Use", "Recommend"];
            let rows = ["Doing", "Thinking", "Feeling 🙂😐🙁", "Touchpoints", "Ideas"];
            for (c, t) in stages.iter().enumerate() {
                let x = 140. + c as f32 * 244.;
                b.zone(x, 0., 230., BAR + 20. + rows.len() as f32 * 130., t, c, "", false);
                for r in 0..rows.len() {
                    b.note_sized(x + 15., BAR + 20. + r as f32 * 130., 200., 110., ACCENTS[c].1, "", "");
                }
            }
            for (r, t) in rows.iter().enumerate() {
                b.text(0., BAR + 60. + r as f32 * 130., Some(120.), LabelSize::S, &format!("**{t}**"));
            }
        }
        Template::EmpathyMap => {
            b.note(380., 0., 240., Paper::Bubblegum, "Who", "The person, in a line");
            b.matrix(0., 150., 480., 320., [("Says 💬", "Their words"), ("Thinks 💭", "What's on their mind"), ("Does 🏃", "What they do"), ("Feels ❤️", "How they feel")]);
            b.zone(0., 850., 492., 260., "Pains", 1, "Fears, frustrations", false);
            b.zone(512., 850., 492., 260., "Gains", 0, "Wants, needs, hopes", false);
        }
        Template::Persona => {
            b.note_sized(0., 0., 300., 260., Paper::Tangerine, "Name", "Age:\nJob:\nWhere they live:\n\nA line in their own words.");
            b.matrix(330., 0., 380., 260., [("Goals 🎯", "What they want"), ("Frustrations", "What gets in the way"), ("Motivations", "Why they act"), ("Habits & tools", "How they do it now")]);
        }
        Template::Blueprint => {
            let lanes = ["Customer actions", "Frontstage", "Backstage", "Support processes"];
            for (r, t) in lanes.iter().enumerate() {
                let y = r as f32 * 180.;
                b.zone(0., y, 1180., 164., t, r, "", false);
                for c in 0..4 {
                    b.note(180. + c as f32 * 250., y + BAR + 8., 230., ACCENTS[r].1, if r == 0 { "Step" } else { "" }, "");
                }
            }
        }
        Template::Interview => {
            b.note(0., 0., 360., Paper::Tangerine, "Who and when", "Name:\nRole:\nDate:");
            b.note_sized(0., 140., 360., 260., Paper::Azure, "Questions", "1. \n2. \n3. \n4. \n5. ");
            b.columns(390., 0., 300., 400., &[("Quotes 💬", "Their words, as said"), ("Insights 💡", "What it tells us"), ("Follow-ups", "What to ask or do next")]);
        }
        Template::Jtbd => {
            for i in 0..3 {
                b.note(i as f32 * 340., 0., 320., Paper::Lemon, &format!("Job {}", i + 1), "When ___,\nI want to ___,\nso I can ___.");
            }
            b.columns(0., 150., 312., 380., &[("Functional", "What they need to get done"), ("Emotional", "How they want to feel"), ("Social", "How they want to be seen")]);
        }
        Template::Usability => b.columns(
            0.,
            0.,
            320.,
            520.,
            &[("Critical 🔴", "Stops people finishing"), ("Major 🟠", "Slows them down a lot"), ("Minor 🟡", "Small annoyances")],
        ),
        // ---------- Meetings & teams ----------
        Template::IceBreaker => {
            b.board("Ice breaker", 4, 2, 150., 50., |_, _| "Name".to_string());
            b.text(24., 14., None, LabelSize::M, "**Two truths and a lie**: everyone writes three things, the others guess the lie.");
        }
        Template::Meeting => {
            b.note(0., 0., 260., Paper::Azure, "Agenda", "1. \n2. \n3. ");
            b.note(290., 0., 300., Paper::Plain, "Notes", "");
            b.note(0., 200., 260., Paper::Leaf, "Decisions", "- ");
            b.note(290., 200., 300., Paper::Lemon, "Actions", "- [ ] \n- [ ] ");
        }
        Template::TeamSync => b.matrix(
            0.,
            0.,
            440.,
            320.,
            [("Wins 🎉", "What went well"), ("Blockers 🚧", "What's stuck"), ("This week", "What we're on"), ("Shout-outs 🙌", "Thanks to…")],
        ),
        Template::DecisionLog => b.table(0., 0., 820., "Decision log", &["Date", "Decision", "Why", "Who decided"], 6, Paper::Plain),
        Template::TeamCharter => {
            let cells = [
                ("Purpose", "Why this team exists"),
                ("Values", "What we hold to"),
                ("Roles", "Who does what"),
                ("Ways of working", "How we work together"),
                ("Rituals", "Meetings and habits"),
                ("Success looks like", "How we'll know"),
            ];
            for (i, (t, h)) in cells.iter().enumerate() {
                b.zone((i % 3) as f32 * 344., (i / 3) as f32 * 344., 320., 320., t, i, h, true);
            }
        }
        Template::Raci => {
            b.table(0., 0., 820., "RACI", &["Task", "Responsible", "Accountable", "Consulted", "Informed"], 6, Paper::Plain);
            b.hint(0., 360., 820., "Responsible does it · Accountable signs it off · Consulted is asked · Informed is told.");
        }
        Template::Kickoff => {
            let cells = [
                ("Goal 🎯", "What we're here to do"),
                ("Scope", "In, and out"),
                ("Team 👥", "Who's on it"),
                ("Timeline 📅", "Key dates"),
                ("Risks ⚠️", "What could go wrong"),
                ("Next steps", "Who does what first"),
            ];
            for (i, (t, h)) in cells.iter().enumerate() {
                b.zone((i % 3) as f32 * 344., (i / 3) as f32 * 344., 320., 320., t, i, h, true);
            }
        }
        Template::Standup => b.columns(
            0.,
            0.,
            300.,
            480.,
            &[("Yesterday", "What I finished"), ("Today", "What I'm on"), ("Blockers 🚧", "What's in my way")],
        ),
        // ---------- Diagrams ----------
        Template::Flowchart => {
            let start = b.shape(Geom::Terminator, 0., 20., 160., 64., "Start", Some(Paper::Leaf), Ink::Forest);
            let step = b.shape(Geom::Rect, 240., 12., 180., 80., "Do the thing", Some(Paper::Azure), Ink::Cobalt);
            let ask = b.shape(Geom::Diamond, 500., 0., 180., 104., "Done?", Some(Paper::Lemon), Ink::Marigold);
            let end = b.shape(Geom::Terminator, 760., 20., 160., 64., "End", Some(Paper::Bubblegum), Ink::Ruby);
            let fix = b.shape(Geom::Rect, 500., 200., 180., 80., "Fix what's left", Some(Paper::Grape), Ink::Violet);
            b.link(start, step, LinkStyle::Straight, true);
            b.link(step, ask, LinkStyle::Straight, true);
            b.link(ask, end, LinkStyle::Straight, true);
            b.link(ask, fix, LinkStyle::Straight, true);
            b.link(fix, step, LinkStyle::Elbow, true);
            b.text(700., 10., None, s, "yes");
            b.text(600., 130., None, s, "no");
        }
        Template::OrgChart => {
            let top = b.shape(Geom::Rounded, 360., 0., 200., 70., "Lead", Some(Paper::Bubblegum), Ink::Ruby);
            for i in 0..3 {
                let x = i as f32 * 320.;
                let head = b.shape(Geom::Rounded, x + 40., 140., 200., 70., &format!("Head {}", i + 1), Some(Paper::Azure), Ink::Cobalt);
                b.link(top, head, LinkStyle::Elbow, false);
                for j in 0..2 {
                    let m = b.shape(Geom::Rounded, x + j as f32 * 150., 280., 140., 60., "Member", Some(Paper::Plain), Ink::Slate);
                    b.link(head, m, LinkStyle::Elbow, false);
                }
            }
        }
        Template::Timeline => {
            b.shape(Geom::Arrow, 0., 200., 1100., 24., "", None, Ink::Graphite);
            for i in 0..5 {
                let x = 60. + i as f32 * 220.;
                b.shape(Geom::Circle, x + 60., 196., 32., 32., "", Some(Paper::Bubblegum), Ink::Ruby);
                let y = if i % 2 == 0 { 40. } else { 260. };
                b.note(x, y, 180., ACCENTS[i % 5].1, &format!("Milestone {}", i + 1), "Date");
            }
        }
        Template::Fishbone => {
            b.shape(Geom::Arrow, 0., 288., 900., 24., "", None, Ink::Graphite);
            b.shape(Geom::Rect, 920., 260., 200., 80., "The problem", Some(Paper::Bubblegum), Ink::Ruby);
            let kinds = ["People", "Process", "Tools", "Materials", "Environment", "Measurement"];
            for (i, k) in kinds.iter().enumerate() {
                let x = 80. + (i % 3) as f32 * 280.;
                let up = i < 3;
                let mut bone = Shape::new(b.id(), Geom::Line, v2(0., 0.));
                bone.rect = Rect { x: x - 20., y: if up { 160. } else { 420. }, w: 260., h: 24. };
                bone.rot = if up { 60. } else { -60. };
                bone.ink = Ink::Slate;
                b.p.shapes.push(bone);
                b.note(x - 20., if up { 0. } else { 470. }, 200., ACCENTS[i % 5].1, k, "");
            }
        }
        Template::Venn2 => {
            b.shape(Geom::Circle, 0., 0., 400., 400., "", None, Ink::Cobalt);
            b.shape(Geom::Circle, 240., 0., 400., 400., "", None, Ink::Ruby);
            b.text(80., 180., None, LabelSize::M, "**A**");
            b.text(500., 180., None, LabelSize::M, "**B**");
            b.text(285., 180., None, LabelSize::M, "**Both**");
        }
        Template::Venn3 => {
            b.shape(Geom::Circle, 0., 0., 380., 380., "", None, Ink::Cobalt);
            b.shape(Geom::Circle, 220., 0., 380., 380., "", None, Ink::Ruby);
            b.shape(Geom::Circle, 110., 190., 380., 380., "", None, Ink::Forest);
            b.text(70., 120., None, LabelSize::M, "**A**");
            b.text(500., 120., None, LabelSize::M, "**B**");
            b.text(285., 480., None, LabelSize::M, "**C**");
            b.text(275., 250., None, LabelSize::S, "**All**");
        }
        Template::Swimlanes => {
            let lanes = ["Customer", "Sales", "Support"];
            let mut prev = None;
            for (r, t) in lanes.iter().enumerate() {
                let y = r as f32 * 200.;
                b.zone(0., y, 1100., 180., t, r, "", false);
                let step = b.shape(Geom::Rounded, 200. + r as f32 * 280., y + BAR + 30., 200., 70., &format!("Step {}", r + 1), Some(ACCENTS[r].1), Ink::Graphite);
                if let Some(p) = prev {
                    b.link(p, step, LinkStyle::Elbow, true);
                }
                prev = Some(step);
            }
        }
        Template::ConceptMap => {
            let main = b.shape(Geom::Ellipse, 380., 220., 220., 120., "Main idea", Some(Paper::Bubblegum), Ink::Ruby);
            let spots = [(0., 0.), (760., 0.), (0., 460.), (760., 460.)];
            for (i, (x, y)) in spots.iter().enumerate() {
                let c = b.shape(Geom::Rounded, *x, *y, 200., 80., &format!("Concept {}", i + 1), Some(ACCENTS[i].1), Ink::Graphite);
                b.link(main, c, LinkStyle::Straight, true);
            }
        }
        // ---------- Personal ----------
        Template::Week => {
            for (i, d) in ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"].iter().enumerate() {
                let a = if i >= 5 { Accent::Rose } else { Accent::Slate };
                b.area((i % 4) as f32 * 280., (i / 4) as f32 * 360., 260., 340., d, a);
            }
        }
        Template::Reading => {
            b.note(0., 0., 320., Paper::Plain, "Up next", "| Title | Author |\n|---|---|\n|  |  |");
            b.note(350., 0., 240., Paper::Grape, "Reading now", "");
            b.note(350., 160., 240., Paper::Leaf, "Finished", "- [x] ");
        }
        Template::Eisenhower => b.matrix(
            0.,
            0.,
            440.,
            320.,
            [
                ("Do now 🔥", "Urgent and important"),
                ("Schedule 📅", "Important, not urgent"),
                ("Delegate 🤝", "Urgent, not important"),
                ("Drop 🗑️", "Neither"),
            ],
        ),
        Template::Habits => b.table(0., 0., 760., "Habit tracker", &["Habit", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], 5, Paper::Plain),
        Template::Quarters => b.columns(0., 0., 260., 520., &[("Q1 ❄️", "Jan – Mar"), ("Q2 🌸", "Apr – Jun"), ("Q3 ☀️", "Jul – Sep"), ("Q4 🍂", "Oct – Dec")]),
        Template::ProsCons => {
            b.note(0., 0., 640., Paper::Tangerine, "The decision", "");
            b.columns(0., 150., 308., 440., &[("Pros 👍", "For it"), ("Cons 👎", "Against it")]);
        }
        Template::StudyPlan => {
            b.note(0., 0., 400., Paper::Bubblegum, "Exam or goal", "Date:");
            b.columns(0., 150., 300., 460., &[("Topics 📚", "Everything to cover"), ("This week", "What I'll study now"), ("Review 🔁", "Go over again")]);
        }
    }
    b.p
}

impl Board {
    /// What a new sheet from template `t` holds: the piece as built.
    pub fn fill_from_template(&mut self, t: Template, now: u64) {
        let mut next = self.next_id;
        let piece = build(t, &mut || {
            next += 1;
            next
        }, now);
        self.next_id = next;
        let c = self.sheet_mut();
        if !piece.blurb.is_empty() && t != Template::Blank {
            c.blurb = piece.blurb;
        }
        c.frames.extend(piece.frames);
        c.stickers.extend(piece.stickers);
        c.shapes.extend(piece.shapes);
        c.labels.extend(piece.labels);
        c.links.extend(piece.links);
    }

    /// Drops template `t` into the open sheet, centred on `centre` (or,
    /// where it would cover what's there, beside everything to the right):
    /// its notes, text and shapes one group. Returns what landed.
    pub fn drop_template(&mut self, t: Template, centre: Vec2, now: u64) -> Piece {
        let mut next = self.next_id;
        let mut piece = build(t, &mut || {
            next += 1;
            next
        }, now);
        let group = next + 1;
        self.next_id = next + 1;
        let Some(b) = piece.bounds() else { return piece };
        let mut at = Rect { x: centre.x - b.w / 2., y: centre.y - b.h / 2., ..b };
        let c = self.sheet();
        let taken: Vec<Rect> = c
            .stickers
            .iter()
            .map(|s| s.rect)
            .chain(c.frames.iter().map(|f| f.rect))
            .chain(c.shapes.iter().map(Shape::bounds))
            .collect();
        if taken.iter().any(|r| r.intersects(&at)) {
            if let Some(all) = taken.iter().copied().reduce(|a, b| a.union(&b)) {
                at.x = all.x + all.w + 120.;
                at.y = all.y;
            }
        }
        piece.translate(v2(at.x - b.x, at.y - b.y));
        piece.stickers.iter_mut().for_each(|s| s.group = Some(group));
        piece.shapes.iter_mut().for_each(|s| s.group = Some(group));
        piece.labels.iter_mut().for_each(|l| l.group = Some(group));
        let c = self.sheet_mut();
        // Areas go behind what's already there.
        for f in piece.frames.iter().rev() {
            c.frames.insert(0, f.clone());
        }
        c.stickers.extend(piece.stickers.clone());
        c.shapes.extend(piece.shapes.clone());
        c.labels.extend(piece.labels.clone());
        c.links.extend(piece.links.clone());
        c.edited = now;
        piece
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn built(t: Template) -> Piece {
        let mut n = 0;
        build(t, &mut || {
            n += 1;
            n
        }, 0)
    }

    #[test]
    fn there_are_sixty_templates_in_every_family() {
        assert!(Template::ALL.len() >= 61, "{}", Template::ALL.len());
        for f in Family::ALL {
            assert!(Template::ALL.iter().any(|t| t.family() == f), "{f:?}");
        }
    }

    #[test]
    fn every_template_builds_sound() {
        for &t in Template::ALL.iter().filter(|t| **t != Template::Blank) {
            let p = built(t);
            assert!(p.bounds().is_some(), "{t:?} is empty");
            // Ids are all different.
            let mut ids: Vec<Id> = p.frames.iter().map(|x| x.id).chain(p.stickers.iter().map(|x| x.id)).chain(p.shapes.iter().map(|x| x.id)).chain(p.labels.iter().map(|x| x.id)).chain(p.links.iter().map(|x| x.id)).collect();
            let n = ids.len();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), n, "{t:?} repeats an id");
            // Notes don't cover each other.
            for (i, a) in p.stickers.iter().enumerate() {
                for b in &p.stickers[i + 1..] {
                    let inner = Rect { x: a.rect.x + 1., y: a.rect.y + 1., w: a.rect.w - 2., h: a.rect.h - 2. };
                    assert!(!inner.intersects(&b.rect), "{t:?}: '{}' covers '{}'", a.title, b.title);
                }
            }
            // A note in an area's column sits inside it, below its title.
            for s in &p.stickers {
                for f in p.frames.iter().filter(|f| f.style != AreaStyle::Board) {
                    let r = f.rect;
                    let centre = s.rect.center();
                    if centre.x > r.x && centre.x < r.x + r.w && centre.y > r.y && centre.y < r.y + r.h {
                        assert!(s.rect.y >= r.y + BAR - 0.5, "{t:?}: '{}' over the title of '{}'", s.title, f.title);
                        assert!(s.rect.x + s.rect.w <= r.x + r.w + 0.5 && s.rect.y + s.rect.h <= r.y + r.h + 0.5, "{t:?}: '{}' spills out of '{}'", s.title, f.title);
                    }
                }
            }
            // Links join things in the piece.
            for l in &p.links {
                let has = |id: Id| p.stickers.iter().any(|s| s.id == id) || p.shapes.iter().any(|s| s.id == id);
                assert!(has(l.from) && has(l.to), "{t:?}: a link to nothing");
            }
        }
    }

    #[test]
    fn a_template_dropped_lands_centred_as_one_group() {
        let mut b = Board::empty();
        b.sheet_mut().stickers.clear();
        b.sheet_mut().frames.clear();
        let before = b.next_id;
        let p = b.drop_template(Template::QuickRetro, v2(1000., 500.), 0);
        let r = p.bounds().unwrap();
        assert!((r.center().x - 1000.).abs() < 0.5 && (r.center().y - 500.).abs() < 0.5);
        let g = p.stickers[0].group;
        assert!(g.is_some() && p.stickers.iter().all(|s| s.group == g) && p.labels.iter().all(|l| l.group == g));
        assert!(b.next_id > before + p.stickers.len() as Id);
        assert_eq!(b.sheet().frames.len(), 4);
        // Dropped again on the same spot, it goes beside what's there.
        let q = b.drop_template(Template::QuickRetro, v2(1000., 500.), 0);
        assert!(q.bounds().unwrap().x > r.x + r.w);
    }
}
