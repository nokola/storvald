//! `Surface`: a box of a page that the program's code draws.
//!
//! The file says where the box is and how code fills it (`draw=`). The
//! build gives the Surface's name a type by that word, so code that fills
//! it another way does not compile.

use crate::draw::Draw;
use crate::page::{Kind, NO_STR, NodeId, Page, PropKey, StrId, Value};

/// How code fills a `Surface`: the word after `draw=`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceDraw {
    /// `draw=picture`: code hands over one picture, shown as an `Image`
    /// shows a file.
    Picture,
    /// `draw=commands`: code writes gogo draw commands in the Surface's
    /// own space, and gogo keeps them.
    Commands,
    /// `draw=gpu`: code makes its own GPU draw calls, inside the pass
    /// that draws the pages, with the viewport on the Surface's
    /// rectangle.
    Gpu,
}

impl SurfaceDraw {
    /// The word a file writes after `draw=`.
    pub fn word(self) -> &'static str {
        match self {
            SurfaceDraw::Picture => "picture",
            SurfaceDraw::Commands => "commands",
            SurfaceDraw::Gpu => "gpu",
        }
    }

    /// How the file says `node` is filled.
    ///
    /// Returns `None` for a node that is not a `Surface`.
    pub fn of(page: &Page, node: NodeId) -> Option<SurfaceDraw> {
        if page.nodes.get(node.index())?.kind != Kind::Surface {
            return None;
        }
        match page.prop(node, PropKey::Draw)? {
            Value::VIdent(word) if page.str(word) == "picture" => Some(SurfaceDraw::Picture),
            Value::VIdent(word) if page.str(word) == "commands" => Some(SurfaceDraw::Commands),
            Value::VIdent(word) if page.str(word) == "gpu" => Some(SurfaceDraw::Gpu),
            _ => None,
        }
    }
}

/// True when the file writes `capture_mouse=` for `node`: on its line, or
/// in a `when` that lands on it.
///
/// Needed so a frame asks only these Surfaces whether the word is true
/// now: a page whose Surfaces never write it pays nothing for the mouse.
pub(crate) fn may_capture_mouse(page: &Page, node: NodeId) -> bool {
    let has_the_word = |(first, count): (u32, u32)| page.props[first as usize..(first + count) as usize].iter().any(|prop| prop.key == PropKey::CaptureMouse);
    let n = &page.nodes[node];
    has_the_word(n.props) || page.whens[n.whens.0 as usize..(n.whens.0 + n.whens.1) as usize].iter().any(|when| has_the_word(when.props))
}

/// A `Surface draw=picture` of a page, by its node.
///
/// Needed as a type of its own so a picture cannot be handed to a
/// Surface the file fills another way: `Runtime::set_picture` takes this
/// and nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PictureSurface(NodeId);

/// A `Surface draw=commands` of a page, by its node.
///
/// Needed as a type of its own so draw commands cannot be written for a
/// Surface the file fills another way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommandSurface(NodeId);

/// A `Surface draw=gpu` of a page, by its node.
///
/// Needed as a type of its own so code tells its GPU Surfaces from the
/// ones gogo draws for it: a renderer hands code this name with the
/// place to draw at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuSurface(NodeId);

macro_rules! surface_name {
    ($name:ident, $draw:expr, $word:literal) => {
        impl $name {
            /// The Surface as the build names it. Only the code a build
            /// writes (`ui::<file>::NAMES`) calls this: the build has read
            /// the file's `draw=`.
            #[doc(hidden)]
            pub const fn from_build(node: NodeId) -> $name {
                $name(node)
            }

            /// The Surface's node: its rectangle, its `visibility=`, its
            /// size are a node's.
            pub const fn node(self) -> NodeId {
                self.0
            }

            /// The same Surface in a row cloned from `from` to `to` (a
            /// `repeat`'s row).
            pub const fn rebase(self, from: NodeId, to: NodeId) -> $name {
                $name(self.0.rebase(from, to))
            }

            #[doc = concat!("`node` as a `Surface draw=", $word, "`.")]
            ///
            #[doc = concat!("Returns `None` when `node` is not a Surface, or its file says another `draw=`.")]
            pub fn of(page: &Page, node: NodeId) -> Option<$name> {
                (SurfaceDraw::of(page, node) == Some($draw)).then_some($name(node))
            }

            #[doc = concat!("The `Surface draw=", $word, "` the page's file names `name`.")]
            ///
            /// Needed for a page compiled at run time (a test's page): a
            /// built page has its Surfaces in `NAMES`.
            ///
            /// Returns `None` when no node has the name, or it is not
            /// such a Surface.
            pub fn find(page: &Page, name: &str) -> Option<$name> {
                $name::of(page, page.find(name)?)
            }
        }
    };
}

surface_name!(PictureSurface, SurfaceDraw::Picture, "picture");
surface_name!(CommandSurface, SurfaceDraw::Commands, "commands");
surface_name!(GpuSurface, SurfaceDraw::Gpu, "gpu");

/// One `Surface draw=commands` that needs its draw commands written:
/// it shows for the first time, its size changed, or code said its
/// content changed (`Runtime::surface_changed`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceAsk {
    pub surface: CommandSurface,
    /// The Surface's size, in epx. Commands are written for a box from
    /// (0, 0) to (`width`, `height`).
    pub width: f32,
    pub height: f32,
}

/// The draw commands code wrote for one `Surface draw=commands`, kept by
/// the page from frame to frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceList {
    pub(crate) node: NodeId,
    /// In the Surface's own space: (0, 0) is its top left corner.
    pub(crate) draws: Vec<Draw>,
    /// Size the commands were written for, in epx.
    pub(crate) size: (f32, f32),
    /// True once code wrote the commands at least once.
    pub(crate) is_written: bool,
    /// True from `Runtime::surface_changed` until the next write.
    pub(crate) is_stale: bool,
    /// The strings of the list's texts, by order of
    /// `SurfaceCommands::text` calls in one write. Kept so the next write
    /// puts its texts into the same strings: a text that changes every
    /// frame adds no string to the page.
    pub(crate) texts: Vec<StrId>,
}

impl Page {
    /// The draw commands of `surface` as code wrote them last, in the
    /// Surface's own space. Empty before the first write.
    ///
    /// A renderer reads them where it meets `Draw::Surface`.
    pub fn surface_commands(&self, surface: NodeId) -> &[Draw] {
        self.surface_lists.iter().find(|list| list.node == surface).map_or(&[], |list| list.draws.as_slice())
    }

    /// Starts the draw commands of `ask.surface` anew, for the size `ask`
    /// names, and hands them out to write. `ask`: what
    /// `Runtime::next_surface_to_draw` returned.
    pub fn write_surface(&mut self, ask: SurfaceAsk) -> SurfaceCommands<'_> {
        SurfaceCommands::begin(self, ask)
    }

    /// True when a `Surface` of the page says `depth=true`.
    ///
    /// Needed by a build to know whether its program needs a depth
    /// buffer: one such Surface in any page, and every frame has one
    /// (`build_pages` writes `HAS_DEPTH_SURFACE`).
    pub fn has_depth_surface(&self) -> bool {
        (0..self.nodes.len()).any(|index| self.is_depth_surface(NodeId::from_index(index)))
    }

    /// True when `node` is a `Surface` whose line says `depth=true`.
    ///
    /// Needed by a renderer, where it meets `Draw::GpuSurface`: the
    /// program's draw calls there test and write depth. The file's line
    /// alone says it, so the page is asked and no frame's state.
    pub fn is_depth_surface(&self, node: NodeId) -> bool {
        self.nodes.get(node.index()).is_some_and(|n| n.kind == Kind::Surface) && self.prop(node, PropKey::Depth) == Some(Value::VBool(true))
    }

    /// The kept list of `surface`, if code wrote one.
    pub(crate) fn surface_list(&self, surface: NodeId) -> Option<&SurfaceList> {
        self.surface_lists.iter().find(|list| list.node == surface)
    }

    /// Index of the kept list of `surface`, made empty when there is none.
    pub(crate) fn surface_list_index(&mut self, surface: NodeId) -> usize {
        match self.surface_lists.iter().position(|list| list.node == surface) {
            Some(at) => at,
            None => {
                self.surface_lists.push(SurfaceList { node: surface, draws: Vec::new(), size: (0.0, 0.0), is_written: false, is_stale: false, texts: Vec::new() });
                self.surface_lists.len() - 1
            }
        }
    }
}

/// The draw commands of one `Surface draw=commands`, being written
/// (`Page::write_surface`).
///
/// Commands are in the Surface's own space: (0, 0) is its top left
/// corner, `width` x `height` its size, in epx. gogo moves them to where
/// the Surface is, and fades them with it.
pub struct SurfaceCommands<'p> {
    page: &'p mut Page,
    at: usize,
    texts_used: usize,
}

impl<'p> SurfaceCommands<'p> {
    /// Starts the list of `ask.surface` anew: empty, for the size `ask`
    /// names.
    pub(crate) fn begin(page: &'p mut Page, ask: SurfaceAsk) -> SurfaceCommands<'p> {
        let at: usize = page.surface_list_index(ask.surface.node());
        let list: &mut SurfaceList = &mut page.surface_lists[at];
        list.draws.clear();
        list.size = (ask.width, ask.height);
        list.is_written = true;
        list.is_stale = false;
        SurfaceCommands { page, at, texts_used: 0 }
    }

    /// The Surface's width, in epx.
    pub fn width(&self) -> f32 {
        self.page.surface_lists[self.at].size.0
    }

    /// The Surface's height, in epx.
    pub fn height(&self) -> f32 {
        self.page.surface_lists[self.at].size.1
    }

    /// Adds one draw command, above the ones added before it.
    pub fn draw(&mut self, draw: Draw) {
        self.page.surface_lists[self.at].draws.push(draw);
    }

    /// The id under which a `Draw::Text` of this list shows `text`.
    ///
    /// Needed because a draw command names its text by id, and a text
    /// made new on every write (a score) would add a string to the page
    /// each time. The n-th call of a write puts its text into the string
    /// the n-th call of the write before used.
    pub fn text(&mut self, text: &str) -> StrId {
        let used: usize = self.texts_used;
        self.texts_used += 1;
        if let Some(id) = self.page.surface_lists[self.at].texts.get(used).copied() {
            if self.page.strings[id] != text {
                let slot: &mut String = self.page.strings[id].to_mut();
                slot.clear();
                slot.push_str(text);
            }
            return id;
        }
        if self.page.strings.is_empty() {
            self.page.strings.push(std::borrow::Cow::Borrowed(""));
        }
        self.page.strings.push(std::borrow::Cow::Owned(text.to_string()));
        let id: StrId = StrId::from_index(self.page.strings.len() - 1);
        debug_assert!(id != NO_STR);
        self.page.surface_lists[self.at].texts.push(id);
        id
    }
}
