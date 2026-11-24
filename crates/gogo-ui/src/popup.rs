//! `Popup`: what a control opens beside itself, a menu or a strip of
//! choices.
//!
//! The file writes a Popup inside the node it belongs to. gogo draws it
//! above the whole page, sizes it by what it holds, places it against
//! that node's rectangle and keeps it inside the page. It shows and
//! hides with `visibility=`, like any node.
//!
//! How, with what the engine already has:
//!
//! - **Above the page.** A resolve seats every Popup under the page's
//!   root, after the root's own children (`runtime::settle_seats`), as a
//!   `when` seats a node in a `seat`. So layout, drawing and the search
//!   for the node under a finger meet it last among the root's children:
//!   drawn last, asked first, cut and turned by no node between it and
//!   the root.
//! - **Its size.** The root places it as any free child that hugs, at
//!   the page's top left corner.
//! - **Its place.** Each frame, after layout, `seat_popups` moves the
//!   rectangles of a shown Popup and of what it holds from that corner
//!   to their place against the parent's rectangle.

use crate::layout::Rect;
use crate::page::{NodeId, Page, PropKey};
use crate::runtime::Resolved;

/// The side of its parent's rectangle a Popup opens on: the word after
/// `side=`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl PopupSide {
    /// Every side, in the order the docs list them.
    pub const ALL: [PopupSide; 4] = [PopupSide::Top, PopupSide::Bottom, PopupSide::Left, PopupSide::Right];

    /// The word a file writes after `side=`.
    pub fn word(self) -> &'static str {
        match self {
            PopupSide::Top => "top",
            PopupSide::Bottom => "bottom",
            PopupSide::Left => "left",
            PopupSide::Right => "right",
        }
    }

    /// The side `word` names, if it names one.
    pub fn from_word(word: &str) -> Option<PopupSide> {
        PopupSide::ALL.into_iter().find(|side| side.word() == word)
    }
}

/// Where a Popup sits along the side it opens on: the word after
/// `align=`. Along a top or bottom side `Start` is the parent's left
/// edge, along a left or right side its top edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupAlign {
    Start,
    Center,
    End,
}

impl PopupAlign {
    /// Every alignment, in the order the docs list them.
    pub const ALL: [PopupAlign; 3] = [PopupAlign::Start, PopupAlign::Center, PopupAlign::End];

    /// The word a file writes after `align=`.
    pub fn word(self) -> &'static str {
        match self {
            PopupAlign::Start => "start",
            PopupAlign::Center => "center",
            PopupAlign::End => "end",
        }
    }

    /// The alignment `word` names, if it names one.
    pub fn from_word(word: &str) -> Option<PopupAlign> {
        PopupAlign::ALL.into_iter().find(|align| align.word() == word)
    }
}

/// Where a Popup of `size` goes: against `parent` on `side`, at `align`
/// along that side, then moved the least that puts it inside `bounds`.
///
/// A Popup larger than `bounds` starts at the left or top edge of
/// `bounds` and runs past the other: its start is what a user reads
/// first.
///
/// Returns the Popup's top left corner.
pub fn popup_corner(parent: Rect, size: (f32, f32), side: PopupSide, align: PopupAlign, bounds: Rect) -> (f32, f32) {
    let along = |start: f32, length: f32, own: f32| -> f32 {
        match align {
            PopupAlign::Start => start,
            PopupAlign::Center => start + (length - own) / 2.0,
            PopupAlign::End => start + length - own,
        }
    };
    let (x, y): (f32, f32) = match side {
        PopupSide::Top => (along(parent.x, parent.w, size.0), parent.y - size.1),
        PopupSide::Bottom => (along(parent.x, parent.w, size.0), parent.bottom()),
        PopupSide::Left => (parent.x - size.0, along(parent.y, parent.h, size.1)),
        PopupSide::Right => (parent.right(), along(parent.y, parent.h, size.1)),
    };
    let inside = |at: f32, own: f32, start: f32, length: f32| -> f32 { at.min(start + length - own).max(start) };
    (inside(x, size.0, bounds.x, bounds.w), inside(y, size.1, bounds.y, bounds.h))
}

/// The side a Popup's line names. `Bottom` for a word that names none:
/// the compiler refuses such a file, a write from code may hold any word.
fn side_of(page: &Page, look: &Resolved, popup: NodeId) -> PopupSide {
    look.ident(page, popup, PropKey::Side).and_then(PopupSide::from_word).unwrap_or(PopupSide::Bottom)
}

/// The alignment a Popup's line names, `Center` when it names none.
fn align_of(page: &Page, look: &Resolved, popup: NodeId) -> PopupAlign {
    look.ident(page, popup, PropKey::Align).and_then(PopupAlign::from_word).unwrap_or(PopupAlign::Center)
}

/// Moves each shown Popup of the page, with what it holds, from where
/// layout put it (the page's top left corner) to its place: against the
/// rectangle of the node it is written in, inside `viewport` less the
/// safe insets.
///
/// Needed after layout, on the rectangles a frame draws and hits with:
/// a Popup's place follows another node's rectangle, which no placement
/// word can say. A move costs no layout, so a Popup also follows a
/// parent that a Scroll pans.
///
/// A Popup inside a Popup works: the outer one is earlier in
/// `Page::popups` and has moved its content, the inner one's parent
/// with it, when the inner one is placed.
///
/// WARNING: `rects` must come straight from layout. Called twice on the
/// same rectangles it moves nothing the second time, as each Popup is
/// then where it belongs; called on rectangles of another look it reads
/// the wrong parent.
pub(crate) fn seat_popups(page: &Page, look: &Resolved, viewport: Rect, rects: &mut [Rect]) {
    let bounds: Rect = viewport.inset(look.safe[0], look.safe[1], look.safe[2], look.safe[3]);
    for at in 0..page.popups.len() {
        let popup: NodeId = page.popups[at];
        let parent: NodeId = page.declared_parent(popup);
        if popup.index() >= rects.len() || parent.index() >= rects.len() || look.gone(page, popup) {
            continue;
        }
        let own: Rect = rects[popup.index()];
        let (x, y): (f32, f32) = popup_corner(rects[parent.index()], (own.w, own.h), side_of(page, look, popup), align_of(page, look, popup), bounds);
        let (dx, dy): (f32, f32) = (x - own.x, y - own.y);
        if dx == 0.0 && dy == 0.0 {
            continue;
        }
        for node in look.tree.subtree(popup) {
            let rect: &mut Rect = &mut rects[node.index()];
            rect.x += dx;
            rect.y += dy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: Rect = Rect { x: 0.0, y: 0.0, w: 320.0, h: 680.0 };
    const BUTTON: Rect = Rect { x: 200.0, y: 600.0, w: 60.0, h: 50.0 };

    #[test]
    fn a_popup_sits_against_the_side_its_line_names() {
        assert_eq!(popup_corner(BUTTON, (70.0, 300.0), PopupSide::Top, PopupAlign::Center, PAGE), (195.0, 300.0), "above, centered: its bottom on the button's top");
        assert_eq!(popup_corner(BUTTON, (70.0, 20.0), PopupSide::Bottom, PopupAlign::Start, PAGE), (200.0, 650.0), "under it, left edges in line");
        assert_eq!(popup_corner(BUTTON, (70.0, 20.0), PopupSide::Top, PopupAlign::End, PAGE), (190.0, 580.0), "above it, right edges in line");
        assert_eq!(popup_corner(BUTTON, (40.0, 20.0), PopupSide::Left, PopupAlign::Start, PAGE), (160.0, 600.0), "left of it, top edges in line");
        assert_eq!(popup_corner(BUTTON, (40.0, 20.0), PopupSide::Right, PopupAlign::Center, PAGE), (260.0, 615.0), "right of it, centered");
        assert_eq!(popup_corner(BUTTON, (40.0, 20.0), PopupSide::Left, PopupAlign::End, PAGE), (160.0, 630.0), "left of it, bottom edges in line");
    }

    #[test]
    fn a_popup_is_moved_the_least_that_keeps_it_inside_the_page() {
        assert_eq!(popup_corner(BUTTON, (200.0, 100.0), PopupSide::Top, PopupAlign::Center, PAGE), (120.0, 500.0), "wider than the room to the right: its right edge on the page's");
        assert_eq!(popup_corner(BUTTON, (70.0, 100.0), PopupSide::Bottom, PopupAlign::Center, PAGE), (195.0, 580.0), "no room under the button: it covers the button");
        assert_eq!(popup_corner(Rect::new(4.0, 4.0, 20.0, 20.0), (100.0, 40.0), PopupSide::Top, PopupAlign::Center, PAGE), (0.0, 0.0), "past the top left corner: back to it");
        assert_eq!(popup_corner(BUTTON, (400.0, 100.0), PopupSide::Top, PopupAlign::Center, PAGE).0, 0.0, "wider than the page: from the left edge, the rest runs past the right");
        let safe: Rect = PAGE.inset(0.0, 30.0, 0.0, 20.0);
        assert_eq!(popup_corner(Rect::new(100.0, 40.0, 60.0, 20.0), (60.0, 30.0), PopupSide::Top, PopupAlign::Center, safe), (100.0, 30.0), "the page's safe area is the room, not the window");
    }

    #[test]
    fn a_side_and_an_alignment_have_one_word_each() {
        for side in PopupSide::ALL {
            assert_eq!(PopupSide::from_word(side.word()), Some(side));
        }
        for align in PopupAlign::ALL {
            assert_eq!(PopupAlign::from_word(align.word()), Some(align));
        }
        assert_eq!((PopupSide::from_word("above"), PopupAlign::from_word("middle")), (None, None));
    }
}
