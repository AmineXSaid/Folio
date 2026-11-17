//! Workshop tools on a sheet: dot voting (everyone gets a few dots to put
//! on the notes they back, then the notes are ranked) and private writing
//! (notes stay covered until they're revealed). Voters and writers are
//! known by name, so ballots from different computers merge.
//! Nothing here knows about GPUI.

use serde::{Deserialize, Serialize};

use crate::model::{Category, Id, Sticker};

/// A dot vote on a sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteSession {
    /// Dots each person has.
    pub per_person: u8,
    /// Still voting; once ended, the results show until cleared.
    pub open: bool,
    /// Unix seconds.
    pub started: u64,
}

/// What a writer is called when they've given no name.
pub const SOMEONE: &str = "Someone";

impl Category {
    /// Starts a vote with `per` dots each, clearing any earlier one.
    pub fn start_vote(&mut self, per: u8, now: u64) {
        self.clear_votes();
        self.vote = Some(VoteSession { per_person: per.clamp(1, 20), open: true, started: now });
    }

    /// The dots `voter` has put down.
    pub fn dots_used(&self, voter: &str) -> u32 {
        self.stickers.iter().map(|s| s.votes.get(voter).copied().unwrap_or(0) as u32).sum()
    }

    /// The dots `voter` has still to put down; 0 when no vote is open.
    pub fn dots_left(&self, voter: &str) -> u32 {
        match self.vote {
            Some(v) if v.open => (v.per_person as u32).saturating_sub(self.dots_used(voter)),
            _ => 0,
        }
    }

    /// Puts one of `voter`'s dots on `note`: false when the vote isn't
    /// open, the note isn't here, or they have none left.
    pub fn cast(&mut self, note: Id, voter: &str) -> bool {
        if self.dots_left(voter) == 0 {
            return false;
        }
        let Some(s) = self.stickers.iter_mut().find(|s| s.id == note) else { return false };
        *s.votes.entry(voter.to_string()).or_insert(0) += 1;
        true
    }

    /// Takes one of `voter`'s dots back off `note`: false if there was none.
    pub fn take_back(&mut self, note: Id, voter: &str) -> bool {
        if !self.vote.is_some_and(|v| v.open) {
            return false;
        }
        let Some(s) = self.stickers.iter_mut().find(|s| s.id == note) else { return false };
        let Some(n) = s.votes.get_mut(voter) else { return false };
        *n -= 1;
        if *n == 0 {
            s.votes.remove(voter);
        }
        true
    }

    /// Ends the vote; the results show.
    pub fn end_vote(&mut self) {
        if let Some(v) = self.vote.as_mut() {
            v.open = false;
        }
    }

    /// The notes with dots, most first (equal ones in the sheet's order).
    pub fn results(&self) -> Vec<(Id, u32)> {
        let mut out: Vec<(Id, u32)> = self.stickers.iter().map(|s| (s.id, total(s))).filter(|(_, n)| *n > 0).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out
    }

    /// The people who voted.
    pub fn voters(&self) -> Vec<String> {
        let mut v: Vec<String> = self.stickers.iter().flat_map(|s| s.votes.keys().cloned()).collect();
        v.sort();
        v.dedup();
        v
    }

    /// No vote, and no dots on any note.
    pub fn clear_votes(&mut self) {
        self.vote = None;
        self.stickers.iter_mut().for_each(|s| s.votes.clear());
    }

    /// Notes covered here.
    pub fn hidden_count(&self) -> usize {
        self.stickers.iter().filter(|s| s.hidden_by.is_some()).count()
    }

    /// Uncovers every note and stops writing privately.
    pub fn reveal_all(&mut self) {
        self.private = false;
        self.stickers.iter_mut().for_each(|s| s.hidden_by = None);
    }

    pub fn reveal(&mut self, note: Id) {
        if let Some(s) = self.stickers.iter_mut().find(|s| s.id == note) {
            s.hidden_by = None;
        }
    }

    /// The sheet as it may be shown to others: hidden notes keep their
    /// place and paper, not their words or picture.
    pub fn covered(&self) -> Category {
        let mut c = self.clone();
        for s in c.stickers.iter_mut().filter(|s| s.hidden_by.is_some()) {
            s.title = format!("Hidden · {}", s.hidden_by.clone().unwrap_or_default());
            s.body.clear();
            s.caption.clear();
            s.image = None;
            s.link = None;
        }
        c
    }

    /// Puts a new note on the sheet, covered if writing here is private.
    pub fn add_note(&mut self, mut s: Sticker, writer: &str) {
        if self.private {
            s.hidden_by = Some(if writer.trim().is_empty() { SOMEONE.to_string() } else { writer.to_string() });
        }
        self.stickers.push(s);
    }
}

/// All the dots on a note.
pub fn total(s: &Sticker) -> u32 {
    s.votes.values().map(|n| *n as u32).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Accent;

    fn sheet() -> Category {
        let mut c = Category::new(1, "Retro", Accent::Slate);
        for id in 2..6 {
            c.stickers.push(Sticker { id, ..Default::default() });
        }
        c
    }

    #[test]
    fn each_voter_has_only_their_dots() {
        let mut c = sheet();
        assert!(!c.cast(2, "Sam"), "no vote yet");
        c.start_vote(3, 0);
        assert!(c.cast(2, "Sam") && c.cast(2, "Sam") && c.cast(3, "Sam"));
        assert!(!c.cast(4, "Sam"), "out of dots");
        assert_eq!(c.dots_left("Sam"), 0);
        assert!(c.take_back(2, "Sam"));
        assert_eq!(c.dots_left("Sam"), 1);
        assert!(!c.take_back(5, "Sam"));
        assert!(c.cast(4, "Ana") && c.cast(4, "Ana"));
        assert_eq!(c.dots_left("Ana"), 1);
        c.end_vote();
        assert!(!c.cast(5, "Ana"), "ended");
        assert_eq!(c.results(), vec![(4, 2), (2, 1), (3, 1)]);
        assert_eq!(c.voters(), vec!["Ana".to_string(), "Sam".to_string()]);
        c.clear_votes();
        assert!(c.vote.is_none() && c.results().is_empty());
    }

    #[test]
    fn private_notes_stay_covered_until_revealed() {
        let mut c = sheet();
        c.add_note(Sticker { id: 9, ..Default::default() }, "Sam");
        assert_eq!(c.hidden_count(), 0);
        c.private = true;
        c.add_note(Sticker { id: 10, ..Default::default() }, "Sam");
        c.add_note(Sticker { id: 11, ..Default::default() }, " ");
        assert_eq!(c.stickers.last().unwrap().hidden_by.as_deref(), Some(SOMEONE));
        assert_eq!(c.hidden_count(), 2);
        c.stickers.last_mut().unwrap().body = "secret".into();
        assert!(c.covered().stickers.iter().all(|s| s.body != "secret"));
        c.reveal(10);
        assert_eq!(c.hidden_count(), 1);
        c.reveal_all();
        assert!(!c.private && c.hidden_count() == 0);
    }

    #[test]
    fn ballots_cast_on_two_computers_both_land() {
        let mut base = crate::model::Board::empty();
        base.sheet_mut().start_vote(5, 0);
        let id = base.sheet().stickers.first().map(|s| s.id).unwrap_or_else(|| {
            base.sheet_mut().stickers.push(Sticker { id: 900, ..Default::default() });
            900
        });
        let (mut here, mut there) = (base.clone(), base.clone());
        here.sheet_mut().cast(id, "Sam");
        there.sheet_mut().cast(id, "Ana");
        there.sheet_mut().cast(id, "Ana");
        let (m, _) = crate::merge::merge(Some(&base), &here, &there);
        let s = m.sheet().stickers.iter().find(|s| s.id == id).unwrap();
        assert_eq!((s.votes.get("Sam"), s.votes.get("Ana")), (Some(&1), Some(&2)));
    }
}
