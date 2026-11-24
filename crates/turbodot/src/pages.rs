//! Several gogo pages in one window: laid out, drawn and touched in one
//! order.
//!
//! - [`UiAssets`] is what every page of an app shares: the text engine,
//!   the pictures and the language.
//! - [`PageUi`] is one page with its state, its input and its last
//!   frame.
//! - [`PageStack`] holds the pages of one window. It draws them bottom
//!   first, and gives a touch to the topmost page that takes it.
//! - [`PagePlatform`] is what a page reads from the screen: the safe
//!   insets, the orientation and size class, the language.
//!
//! Needed because a page is more than its `Page`: its state, its input and
//! its last layout belong together, and "which page takes this finger"
//! needs all of them, in draw order.
//!
//! Nothing here needs a window or a GPU. The calls that write a picture to
//! the GPU or draw a page to a texture need the cargo feature `gpu`.

use std::cell::RefCell;
use std::sync::Arc;

use gogo_text::{EpxMeasure, TextEngine};
use gogo_ui::page::{Action, NO_NODE, NO_STR, StaticPage, StrId};
use gogo_ui::reload::{Refused, Reloader};
use gogo_ui::{ArrangementId, Buttons, Draw, Event, EventList, ImageKey, Images, Input, KeptFrame, Key, Kind, Locale, MOUSE_ID, Mods, NodeId, Orientation, Page, Phase, PropKey, Rect, Runtime, ScreenClass, Touch, Value, hit};

/// Every picture an app's pages name or its code shows, with what the host
/// keeps about each: its size, whether it loads late, and the file bytes
/// still to decode.
///
/// Needed because a picture's size is an input of layout (`aspect=natural`
/// works one side out from the other), so every key a page can name needs
/// a size row. With the columns in separate lists, a key that code made
/// while the app ran had no size row, and its Image was laid out 0 wide:
/// the user saw an empty place where a thumbnail belonged. Here `intern`
/// grows every column, and `upload_image` writes the pixels and the size
/// together.
pub struct ImageTable {
    keys: Images,
    /// (width, height) in pixels: read from the file's header when the app
    /// starts, so `aspect=natural` lays out before the decode, or written
    /// by `upload_image` for a picture that code shows. `None` = not known
    /// yet.
    sizes: Vec<Option<(u32, u32)>>,
    /// Whether every line that names the picture says `loading=lazy`.
    is_lazy: Vec<bool>,
    /// The file bytes still to decode. `None` = uploaded, or a picture of
    /// the app's code, which has no file.
    pending: Vec<Option<&'static [u8]>>,
    /// How many of `pending` are still `Some`.
    pending_count: usize,
    /// The file bytes, kept after the upload: a picture `release_unmarked`
    /// freed decodes from them again when a page shows it the next time.
    files: Vec<Option<&'static [u8]>>,
    /// Whether a page marked the picture `release=unused`. Only these may
    /// `release_unmarked` free.
    can_release: Vec<bool>,
}

impl ImageTable {
    /// The table of an app at its start: the keys its build made, with one
    /// row per key in each of the three columns.
    ///
    /// WARNING: panics when a column is not as long as `keys`.
    pub fn new(keys: Images, pending: Vec<Option<&'static [u8]>>, is_lazy: Vec<bool>, sizes: Vec<Option<(u32, u32)>>) -> ImageTable {
        assert!(pending.len() == keys.len() && is_lazy.len() == keys.len() && sizes.len() == keys.len(), "one row per key: {} keys, {} pending, {} lazy, {} sizes", keys.len(), pending.len(), is_lazy.len(), sizes.len());
        let pending_count: usize = pending.iter().filter(|bytes| bytes.is_some()).count();
        let files: Vec<Option<&'static [u8]>> = pending.clone();
        let can_release: Vec<bool> = vec![false; files.len()];
        ImageTable { keys, sizes, is_lazy, pending, pending_count, files, can_release }
    }

    /// A table of the keys alone, for a test or a page on its own: no bytes
    /// to decode, nothing that loads late, no size before an upload.
    pub fn from_keys(keys: Images) -> ImageTable {
        let count: usize = keys.len();
        ImageTable::new(keys, vec![None; count], vec![false; count], vec![None; count])
    }

    /// The key of the picture named `name`, made now if the name is new.
    /// Every column gets its row here, so a key never points past the
    /// table.
    pub fn intern(&mut self, name: &str) -> ImageKey {
        let key: ImageKey = self.keys.intern(name);
        if key.index() >= self.sizes.len() {
            self.sizes.push(None);
            self.is_lazy.push(false);
            self.pending.push(None);
            self.files.push(None);
            self.can_release.push(false);
        }
        key
    }

    /// Writes the size of a picture whose pixels come later, for layout.
    ///
    /// Needed for a picture code makes: the page lays out at its size in
    /// the frame it was made, before a GPU frame uploads it, and with no
    /// GPU at all.
    ///
    /// WARNING: panics for a key that is not of this table.
    ///
    /// Returns true when the size changed: page that shows it is laid
    /// out again.
    pub fn set_size(&mut self, key: ImageKey, width: u32, height: u32) -> bool {
        let row: &mut Option<(u32, u32)> = &mut self.sizes[key.index()];
        let has_changed: bool = *row != Some((width, height));
        *row = Some((width, height));
        has_changed
    }

    /// The key `name` already has. Never makes one.
    pub fn get(&self, name: &str) -> Option<ImageKey> {
        self.keys.get(name)
    }

    pub fn name(&self, key: ImageKey) -> &str {
        self.keys.name(key)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.len() == 0
    }

    /// (width, height) in pixels of the picture; `None` until it is known.
    pub fn size(&self, key: ImageKey) -> Option<(u32, u32)> {
        self.sizes.get(key.index()).copied().flatten()
    }

    /// The sizes by key: what layout measures `aspect=natural` against
    /// (`EpxMeasure::images`).
    pub fn sizes(&self) -> &[Option<(u32, u32)>] {
        &self.sizes
    }

    pub fn is_lazy(&self, key: ImageKey) -> bool {
        self.is_lazy.get(key.index()).copied().unwrap_or(false)
    }

    /// The file bytes the picture still has to decode; `None` once it is
    /// uploaded, and for a picture of the app's code.
    pub fn pending_bytes(&self, key: ImageKey) -> Option<&'static [u8]> {
        self.pending.get(key.index()).copied().flatten()
    }

    /// Takes the bytes to decode, for a caller that decodes and uploads
    /// them.
    ///
    /// Returns `None` when nothing is pending for `key`.
    pub fn take_pending(&mut self, key: ImageKey) -> Option<&'static [u8]> {
        let bytes: &'static [u8] = self.pending.get_mut(key.index())?.take()?;
        self.pending_count -= 1;
        Some(bytes)
    }

    /// How many pictures still have file bytes to decode.
    pub fn pending_count(&self) -> usize {
        self.pending_count
    }

    /// Marks the pictures the pages declared `release=unused`. Only these
    /// may `release_unmarked` free.
    pub fn set_release_unused(&mut self, keys: &[ImageKey]) {
        for key in keys {
            if let Some(can_release) = self.can_release.get_mut(key.index()) {
                *can_release = true;
            }
        }
    }
}

#[cfg(feature = "gpu")]
impl ImageTable {
    /// Marks every picture that has a file as still to decode.
    ///
    /// Needed for a renderer that was made new: it has none of the
    /// pictures the table counts as uploaded, and a page would show an
    /// empty place where each belongs.
    ///
    /// A picture code uploaded has no file: code uploads it again.
    pub fn forget_uploads(&mut self) {
        for i in 0..self.files.len() {
            if self.pending[i].is_none() && self.files[i].is_some() {
                self.pending[i] = self.files[i];
                self.pending_count += 1;
            }
        }
    }

    /// Frees the GPU copy of every `release=unused` picture that `live`
    /// (by key index) does not mark. Each decodes again from its file when
    /// a page shows it the next time.
    ///
    /// Needed because a big picture that a page shows now and then would
    /// stay on the GPU for as long as the app runs.
    ///
    /// Every other picture stays: one without `release=unused`, and one
    /// the app's code uploaded, which has no file to decode again.
    ///
    /// Adds the keys it freed to `freed`, so the host lets its decoder
    /// send them again.
    pub fn release_unmarked(&mut self, renderer: &mut gogo_wgpu::UiRenderer, device: &wgpu::Device, live: &[bool], freed: &mut Vec<ImageKey>) {
        for i in 0..self.files.len() {
            let Some(file) = self.files[i] else { continue };
            if !self.can_release[i] || self.pending[i].is_some() || live.get(i).copied().unwrap_or(false) {
                continue;
            }
            let key: ImageKey = ImageKey::new(i as u32);
            if !renderer.release_image(device, key) {
                continue;
            }
            self.pending[i] = Some(file);
            self.pending_count += 1;
            freed.push(key);
        }
    }

    /// The one way a picture's pixels reach the screen: writes the texture
    /// and records the size for layout, together.
    ///
    /// Needed because a texture whose size layout does not know is drawn
    /// into a box 0 wide: the user sees nothing where the picture belongs.
    ///
    /// WARNING: panics for a key that is not of this table. `intern` it
    /// here first.
    ///
    /// Returns whether the size changed. A host that laid the page out
    /// before this call then marks its runtime dirty, or the Image keeps
    /// its old width.
    pub fn upload_image(&mut self, renderer: &mut gogo_wgpu::UiRenderer, device: &wgpu::Device, queue: &wgpu::Queue, key: ImageKey, rgba: &[u8], width: u32, height: u32) -> bool {
        let row: &mut Option<(u32, u32)> = self.sizes.get_mut(key.index()).unwrap_or_else(|| panic!("upload_image: key {} is not in this table ({} keys): intern it here first", key.index(), self.keys.len()));
        let has_changed: bool = *row != Some((width, height));
        *row = Some((width, height));
        renderer.write_image_texture_only(device, queue, key, rgba, width, height);
        has_changed
    }
}

/// Marks in `live`, by key index, every picture `page` can show without
/// code writing it again: each Image's `source=`, every source code wrote,
/// and a picture a `when` shows right now. An atlas slice marks its whole
/// picture.
///
/// A picture that only an inactive `when` names is not marked: freeing it
/// costs one decode when that state shows it.
pub fn mark_page_pictures(page: &Page, runtime: &Runtime, live: &mut [bool]) {
    let mut mark = |value: Value| {
        if let Value::VImage(key) = gogo_ui::runtime::deref(page, runtime.variant, value) {
            if let Some(is_live) = live.get_mut(page.picture_of(key).index()) {
                *is_live = true;
            }
        }
    };
    for (i, node) in page.nodes.iter().enumerate() {
        if node.kind == Kind::Image {
            if let Some(value) = page.prop(NodeId::new(i as u32), PropKey::Source) {
                mark(value);
            }
        }
    }
    for (_, prop) in &runtime.writes {
        if prop.key == PropKey::Source {
            mark(prop.value);
        }
    }
    for (_, prop) in &runtime.resolved().overrides {
        if prop.key == PropKey::Source {
            mark(prop.value);
        }
    }
}

/// What every page of an app lays out and draws against.
pub struct UiAssets {
    /// Shapes text and keeps the glyphs. In a `RefCell` because layout
    /// measures text through a shared reference.
    pub text: RefCell<TextEngine>,
    /// Every picture the pages name or code shows.
    pub images: ImageTable,
    /// The language every page shows: one table, shared by every page's
    /// runtime and handed to the renderer. `None` = the texts as the
    /// files have them.
    pub locale: Option<Arc<Locale>>,
    /// True from a layout that left a page with a `Surface
    /// draw=commands` to write, until `take_surface_asks`.
    ///
    /// Needed here, in what every page lays out against, so a frame in
    /// which no page is laid out asks no page for it: a still frame costs
    /// one read. In a `Cell` because layout has a shared reference.
    has_surface_asks: std::cell::Cell<bool>,
}

impl UiAssets {
    pub fn new(text: TextEngine, images: ImageTable) -> UiAssets {
        UiAssets { text: RefCell::new(text), images, locale: None, has_surface_asks: std::cell::Cell::new(false) }
    }

    /// True when a layout since the last call left a page with a
    /// `Surface draw=commands` to write (`Runtime::next_surface_to_draw`).
    /// A host with pages of its own that have such Surfaces calls it
    /// once a frame, and asks its pages only on true.
    pub fn take_surface_asks(&self) -> bool {
        self.has_surface_asks.replace(false)
    }

    /// Sets the language: `bcp47` for the text engine (where a line
    /// breaks, which font a script falls back to), and `locale`, the table
    /// a `PageStack` hands its pages.
    pub fn set_locale(&mut self, locale: Option<Arc<Locale>>, bcp47: &str) {
        self.text.borrow_mut().set_locale(bcp47);
        self.locale = locale;
    }

    /// What layout measures text and pictures with, at `epx` device pixels
    /// per epx.
    pub fn measure(&self, epx: f32) -> EpxMeasure<'_> {
        EpxMeasure { engine: &self.text, epx, font: None, images: self.images.sizes() }
    }
}

/// The three things a page reads from the screen it is on: the safe insets
/// (`padding_top=safe`), the orientation and the size class (`when
/// screen.landscape`), and the language (`text=/key`).
///
/// Needed as one value so a host that hands a page one hands it all three:
/// a page that was given the insets and not the language showed its texts
/// in English inside a translated app.
#[derive(Clone, Debug)]
pub struct PagePlatform {
    /// What the notch and the on-screen keyboard cover at each edge, in
    /// epx: left, top, right, bottom.
    pub safe: [f32; 4],
    pub orientation: Orientation,
    pub class: ScreenClass,
    /// The language table; `None` = the texts as the files have them.
    pub locale: Option<Arc<Locale>>,
}

impl PagePlatform {
    pub fn new(safe: [f32; 4], orientation: Orientation, class: ScreenClass, locale: Option<Arc<Locale>>) -> PagePlatform {
        PagePlatform { safe, orientation, class, locale }
    }

    /// Replaces the insets, the orientation and the size class. The
    /// language stays.
    pub fn set_screen_and_insets(&mut self, safe: [f32; 4], orientation: Orientation, class: ScreenClass) {
        self.safe = safe;
        self.orientation = orientation;
        self.class = class;
    }

    /// Writes all three into `runtime`, and marks it dirty only when one
    /// of them changed.
    ///
    /// Needed because a host calls this for every page on every frame, and
    /// a runtime marked dirty lays its page out again.
    pub fn apply(&self, page: &Page, runtime: &mut Runtime) {
        if runtime.safe != self.safe {
            runtime.safe = self.safe;
            runtime.dirty = true;
        }
        runtime.set_screen(page, self.orientation, self.class);
        if !is_same_locale(&runtime.locale, &self.locale) {
            runtime.set_locale(self.locale.clone());
        }
    }

    /// The platform for a child page laid out at `rect` inside `window`,
    /// both in epx: at each edge, the part of the inset that the distance
    /// from `rect` to the window's edge does not already cover.
    ///
    /// Needed so nothing is inset twice: a child at the window's edge gets
    /// the whole inset, and a child inside a padded parent gets none.
    pub fn residual(&self, window: Rect, rect: Rect) -> PagePlatform {
        let [left, top, right, bottom] = self.safe;
        let safe: [f32; 4] = [
            (left - (rect.x - window.x)).max(0.0),
            (top - (rect.y - window.y)).max(0.0),
            (right - (window.right() - rect.right())).max(0.0),
            (bottom - (window.bottom() - rect.bottom())).max(0.0),
        ];
        PagePlatform { safe, orientation: self.orientation, class: self.class, locale: self.locale.clone() }
    }
}

/// Whether `a` and `b` are the same language table, or both none. A
/// `Locale` cannot be compared, and a table is one shared allocation, so
/// the two are the same when they point at the same one.
fn is_same_locale(a: &Option<Arc<Locale>>, b: &Option<Arc<Locale>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}

/// Where a page is drawn this frame against where it is laid out:
/// moved by the offset, in epx, and faded to `alpha`. A host's own
/// slide sets it. A program sets it from the anim on page's root
/// (`on:show`, `on:hide`, `on:uncover`), and for a page placed in a
/// node that an anim moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PagePose {
    pub offset_x: f32,
    pub offset_y: f32,
    pub alpha: f32,
}

impl PagePose {
    /// Two poses as one: offsets added, alphas multiplied. `None`: in
    /// place, not faded.
    pub fn both(first: Option<PagePose>, second: Option<PagePose>) -> Option<PagePose> {
        match (first, second) {
            (Some(a), Some(b)) => Some(PagePose { offset_x: a.offset_x + b.offset_x, offset_y: a.offset_y + b.offset_y, alpha: a.alpha * b.alpha }),
            (a, b) => a.or(b),
        }
    }
}

/// Where `PageUi::draw_to_texture` puts a node and its children on the
/// texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TexturePlacement {
    /// Texture pixels per epx of the page. It need not be the screen's
    /// scale: a picture for a file is drawn at the file's.
    pub px_per_epx: f32,
    /// The point of the page, in epx, at the texture's top left.
    pub origin_epx: (f32, f32),
    pub size_px: (u32, u32),
    /// `Some((degrees, pivot_epx))`: the node is drawn at this angle in
    /// place of its own rotation; a node with no rotation turns about the
    /// pivot. `None`: as the page shows it.
    pub turn: Option<(f32, (f32, f32))>,
}

/// How far a plate's edge may sit inside page's edge and still count as
/// covering it, in epx: less than a device pixel on any screen.
const COVER_SLACK_EPX: f32 = 0.01;

/// One gogo page: its state, its input and its last frame. It is laid out
/// again only when a state, the window's size or the scale changed.
pub struct PageUi {
    pub page: Page,
    pub runtime: Runtime,
    pub input: Input,
    /// The page's last frame: where every node sat and how it looked.
    ///
    /// Needed as one value so a hit test or a drawing never pairs the look
    /// of one frame with the rectangles of another.
    pub kept: KeptFrame,
    /// The last frame's draw commands, in paint order.
    pub draws: Vec<Draw>,
    /// A node that is painted above this page's child pages, with
    /// everything inside it. `None` = the whole page is painted below
    /// them.
    ///
    /// Needed because a child page is painted after its whole parent, and
    /// a parent may have a part that belongs on top of the child: a back
    /// button over a screen the child fills. `draws` leaves that node out,
    /// `top_draws` holds it, and a press on it goes to this page
    /// (`hits_above_children`).
    pub above_children: Option<NodeId>,
    /// The draw commands of `above_children`; empty without one.
    pub top_draws: Vec<Draw>,
    /// What the last layout was for: the size in device pixels, the scale
    /// and the viewport.
    laid_out_for: ((u32, u32), f32, Option<Rect>),
    /// Whether the page takes a touch that lands on none of its nodes. A
    /// dialog does: a tap beside its buttons stays with it. A toolbar does
    /// not: the finger goes on to what is under it.
    pub is_modal: bool,
    /// Where the page is drawn this frame; `None` = in place, not faded.
    pub pose: Option<PagePose>,
    /// The rectangle the page lays out in, in epx; `None` = the whole
    /// window.
    pub viewport: Option<Rect>,
    /// Pictures page's code made (`set_picture`), one per Surface.
    pub(crate) code_pictures: Vec<CodePicture>,
    /// Textures page's code handed over (`set_picture_texture`), one per
    /// Surface.
    #[cfg(feature = "gpu")]
    pub(crate) picture_textures: Vec<PictureTexture>,
    /// Format of the pass page's GPU frame is drawn in
    /// (`PageUi::target_format`). `None` before page's first GPU frame.
    #[cfg(feature = "gpu")]
    pub(crate) target_format: Option<wgpu::TextureFormat>,
    /// Format of the depth buffer of that pass (`PageUi::depth_format`).
    /// `None`: the pass has none, or page had no GPU frame yet.
    #[cfg(feature = "gpu")]
    pub(crate) depth_format: Option<wgpu::TextureFormat>,
}

/// Picture a page's code made for one `Surface draw=picture`: its
/// pixels, kept until a GPU frame uploads them, and after, so a renderer
/// made new gets them again.
pub(crate) struct CodePicture {
    pub(crate) node: NodeId,
    /// Key the Surface shows it under. `None` until program's next frame.
    pub(crate) key: Option<ImageKey>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Straight alpha, sRGB, four bytes a pixel (R, G, B, A), row by row.
    pub(crate) rgba: Vec<u8>,
    /// True from `set_picture` until program's next frame, which hands
    /// layout the size.
    pub(crate) is_size_new: bool,
    /// True once a GPU frame uploaded these pixels.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    pub(crate) is_uploaded: bool,
}

/// Texture a page's code handed over for one `Surface draw=picture`.
/// The renderer samples it where the Surface is: no copy.
#[cfg(feature = "gpu")]
pub(crate) struct PictureTexture {
    pub(crate) node: NodeId,
    /// Key the Surface shows it under. `None` until the GPU frame that
    /// hands the texture to the renderer.
    pub(crate) key: Option<ImageKey>,
    /// Page's texture: a handle to it, not a copy of its pixels.
    pub(crate) texture: wgpu::Texture,
    /// True from `set_picture_texture` with another texture until a GPU
    /// frame handed it to the renderer.
    pub(crate) is_new: bool,
}

impl PageUi {
    pub fn new(page: Page) -> PageUi {
        let runtime: Runtime = Runtime::new(&page);
        PageUi {
            page,
            runtime,
            input: Input::new(),
            kept: KeptFrame::default(),
            draws: Vec::new(),
            above_children: None,
            top_draws: Vec::new(),
            laid_out_for: ((0, 0), 0.0, None),
            is_modal: false,
            pose: None,
            viewport: None,
            code_pictures: Vec::new(),
            #[cfg(feature = "gpu")]
            picture_textures: Vec::new(),
            #[cfg(feature = "gpu")]
            target_format: None,
            #[cfg(feature = "gpu")]
            depth_format: None,
        }
    }

    /// A page from what `gogo_ui::build_pages` wrote when the program was
    /// built.
    pub fn from_static(page: &'static StaticPage) -> PageUi {
        PageUi::new(Page::from_static(page))
    }

    /// Brings the page in step with its `.gogo` file, when the file says
    /// something else than the page shows. The page keeps its state: its
    /// scroll positions, its arrangement, what code wrote, its list rows.
    ///
    /// A host calls it for every page before a frame. With nothing saved
    /// it costs one comparison, and without the cargo feature `hot-reload`
    /// nothing.
    ///
    /// Returns `Ok(true)` when the page shows its file again, and why a
    /// save cannot be shown, with the file and the line. The page then
    /// stays as it was.
    pub fn refresh(&mut self, reload: &mut Reloader) -> Result<bool, Refused> {
        let is_in_step: bool = reload.refresh(&mut self.page, &mut self.runtime)?;
        if is_in_step {
            self.laid_out_for = ((0, 0), 0.0, None);
        }
        Ok(is_in_step)
    }

    /// Marks in `live` every picture this page can show without code
    /// writing it again (`mark_page_pictures`).
    pub fn mark_pictures(&self, live: &mut [bool]) {
        mark_page_pictures(&self.page, &self.runtime, live);
    }

    /// Lays the page out and fills `draws`, when something changed since
    /// the last time. `size` is in device pixels, `epx` is device pixels
    /// per epx.
    pub fn relayout(&mut self, size: (u32, u32), epx: f32, assets: &UiAssets) {
        if !self.runtime.dirty && self.laid_out_for == (size, epx, self.viewport) {
            return;
        }
        let viewport: Rect = self.viewport.unwrap_or(Rect::new(0.0, 0.0, size.0 as f32 / epx, size.1 as f32 / epx));
        let measure: EpxMeasure<'_> = assets.measure(epx);
        self.kept.frame_retained(&mut self.runtime, &self.page, viewport, &measure, &mut self.draws, self.above_children.map(|top| (top, &mut self.top_draws)));
        if self.runtime.has_surface_to_draw() {
            assets.has_surface_asks.set(true);
        }
        self.laid_out_for = (size, epx, self.viewport);
    }

    /// One frame of this page, `dt` seconds after the last: page takes
    /// `platform`, moves its animations, and lays out again if something
    /// changed. `size` is in device pixels, `epx` is device pixels per
    /// epx.
    ///
    /// Returns true while something still moves.
    pub fn frame(&mut self, dt: f32, size: (u32, u32), epx: f32, platform: &PagePlatform, assets: &UiAssets) -> bool {
        platform.apply(&self.page, &mut self.runtime);
        let mut is_moving: bool = self.runtime.tick(&mut self.page, dt);
        if self.kept.is_laid_out() {
            is_moving |= self.input.tick(&self.page, &mut self.runtime, self.kept.frame(), dt);
        }
        self.relayout(size, epx, assets);
        // Asked after the layout: a glide that this frame's write
        // started there must keep the frames coming.
        is_moving | self.runtime.is_moving()
    }

    /// Where `node` is drawn this frame against where it is laid out:
    /// page's own pose, and what anims on node and on the nodes it sits
    /// under do together. `None`: in place, not faded, or page has no
    /// frame yet.
    ///
    /// Needed for what is drawn in a node's rectangle and is not the
    /// page's own node: a page placed there.
    pub fn drawn_pose(&self, node: NodeId) -> Option<PagePose> {
        if !self.kept.is_laid_out() {
            return None;
        }
        let shift: gogo_ui::DrawnShift = gogo_ui::drawn_shift(&self.page, self.kept.frame(), node);
        let within_page: Option<PagePose> = if shift == gogo_ui::DrawnShift::NONE { None } else { Some(PagePose { offset_x: shift.offset_x, offset_y: shift.offset_y, alpha: shift.opacity }) };
        PagePose::both(self.pose, within_page)
    }

    /// Where anims have page's root now, and how solid. `None`: in
    /// place, not faded.
    ///
    /// Needed by a program: it moves page's finished picture there,
    /// where gogo would draw every node again on each frame.
    pub fn root_pose(&self) -> Option<PagePose> {
        let shift: gogo_ui::DrawnShift = self.runtime.root_pose();
        if shift == gogo_ui::DrawnShift::NONE { None } else { Some(PagePose { offset_x: shift.offset_x, offset_y: shift.offset_y, alpha: shift.opacity }) }
    }

    /// Whether the page's root shows at all.
    pub fn is_shown(&self) -> bool {
        self.kept.is_laid_out() && !self.kept.look().gone(&self.page, NodeId::new(0)) && !self.kept.look().hidden(&self.page, NodeId::new(0))
    }

    /// Whether anything of this page is under (`x`, `y`), in epx. The
    /// page's root counts.
    pub fn covers(&self, x: f32, y: f32) -> bool {
        self.is_shown() && self.kept.rects()[0].contains(x, y)
    }

    /// True when page's root paints an opaque plate over page's whole
    /// rectangle: nothing under page shows through it.
    ///
    /// Reads last frame's first draw command: root's plate, when root has
    /// `background=`. False before first frame, for root with rounded
    /// corners, and for root that is turned or clipped.
    pub fn covers_viewport(&self) -> bool {
        let Some(Draw::Quad { rect, color, corner, .. }) = self.draws.first() else { return false };
        if !self.kept.is_laid_out() || *corner != 0.0 || color.to_f32()[3] < 1.0 {
            return false;
        }
        let ((width_px, height_px), epx, _) = self.laid_out_for;
        let whole: Rect = self.viewport.unwrap_or(Rect::new(0.0, 0.0, width_px as f32 / epx, height_px as f32 / epx));
        rect.x <= whole.x + COVER_SLACK_EPX && rect.y <= whole.y + COVER_SLACK_EPX && rect.x + rect.w >= whole.x + whole.w - COVER_SLACK_EPX && rect.y + rect.h >= whole.y + whole.h - COVER_SLACK_EPX
    }

    /// True when a `Surface` of the page covers page's whole rectangle:
    /// the program's code draws the page's picture there.
    ///
    /// Needed by the debug check of a page's `KIND`: gogo cannot see what
    /// code draws, so a full-screen page that says so with a Surface
    /// over all of it is taken at its word. False before first frame.
    pub fn has_surface_over_viewport(&self) -> bool {
        if !self.kept.is_laid_out() {
            return false;
        }
        let ((width_px, height_px), epx, _) = self.laid_out_for;
        let whole: Rect = self.viewport.unwrap_or(Rect::new(0.0, 0.0, width_px as f32 / epx, height_px as f32 / epx));
        let look = self.kept.look();
        self.page.node_ids().filter(|node| self.page.nodes[*node].kind == Kind::Surface && !look.gone(&self.page, *node) && !look.hidden(&self.page, *node)).any(|node| {
            let rect: Rect = self.kept.rects()[node.index()];
            rect.x <= whole.x + COVER_SLACK_EPX && rect.y <= whole.y + COVER_SLACK_EPX && rect.x + rect.w >= whole.x + whole.w - COVER_SLACK_EPX && rect.y + rect.h >= whole.y + whole.h - COVER_SLACK_EPX
        })
    }

    /// The node under (`x`, `y`) that takes input, if any.
    pub fn hit(&self, x: f32, y: f32) -> Option<NodeId> {
        hit(&self.page, self.kept.frame(), x, y, false)
    }

    /// Whether (`x`, `y`) is on the `above_children` node or inside it.
    ///
    /// Needed so a finger that lands on a part painted over a child page
    /// goes to this page, and not to the child whose rectangle covers the
    /// same spot.
    pub fn hits_above_children(&self, x: f32, y: f32) -> bool {
        match self.above_children {
            Some(top) => self.hit(x, y).is_some_and(|node| self.page.is_under(node, top)),
            None => false,
        }
    }

    /// The name of the handler an event is for: the name its `on:` action
    /// gives, else the name of the nearest node at or above the event's
    /// node that the page's file names.
    ///
    /// Returns `None` for an action gogo carried out itself (a change of
    /// arrangement, an anim), and for an event with no named node above
    /// it.
    pub fn handler_name(&self, event: &Event) -> Option<&str> {
        self.handler_name_id(event).map(|id| self.page.str(id))
    }

    /// `handler_name` as id of that name in page's strings.
    ///
    /// Needed so an engine finds a page's handler by number: by text, 30
    /// handler names cost 15 to 22 ns an event (measured, release build).
    pub fn handler_name_id(&self, event: &Event) -> Option<StrId> {
        self.handler_name_id_with(event, |name| self.page.str(name).contains('/'))
    }

    /// `handler_name_id`, with `is_inside_widget` saying which names belong
    /// to a node inside a widget (`volume/thumb`): walk passes those.
    ///
    /// Needed so a caller with a table by string id reads no name's text.
    pub fn handler_name_id_with(&self, event: &Event, is_inside_widget: impl Fn(StrId) -> bool) -> Option<StrId> {
        match event.action {
            Some(Action::Named(id)) | Some(Action::Emit(id)) => Some(id),
            Some(Action::Go(_)) | Some(Action::ToggleArrangement(_)) | Some(Action::Play(_)) => None,
            None => {
                let mut at: NodeId = event.node;
                loop {
                    let node = &self.page.nodes[at.index()];
                    if node.name != NO_STR && !is_inside_widget(node.name) {
                        return Some(node.name);
                    }
                    if node.parent == NO_NODE {
                        return None;
                    }
                    at = node.parent;
                }
            }
        }
    }
}

#[cfg(feature = "gpu")]
impl PageUi {
    /// Draws `node` and everything inside it to a new texture, as the last
    /// `relayout` laid it out. A node that is hidden on screen is drawn
    /// too.
    ///
    /// Needed so a picture made for a file is drawn by the same walk and
    /// the same renderer as the screen: what the user sees is what the
    /// file gets.
    ///
    /// WARNING: lay the page out first (`relayout`). A write since then is
    /// not drawn.
    ///
    /// Returns a tile with colors multiplied by alpha, transparent where
    /// nothing was drawn.
    pub fn draw_to_texture(&self, renderer: &mut gogo_wgpu::UiRenderer, device: &wgpu::Device, queue: &wgpu::Queue, assets: &UiAssets, node: NodeId, place: TexturePlacement) -> gogo_wgpu::Tex_Tile_Premul_Srgb_4xU8_RGBA {
        debug_assert!(self.kept.is_laid_out(), "draw_to_texture before the page's first relayout");
        let mut draws: Vec<Draw> = Vec::new();
        self.kept.subtree_draws(&self.page, node, &mut draws);
        if let Some((angle_deg, pivot_epx)) = place.turn {
            turn_draws(&mut draws, pivot_epx, angle_deg);
        }
        let mut text = assets.text.borrow_mut();
        renderer.render_to_texture(device, queue, &draws, &self.page, &mut text, assets.locale.as_deref(), place.px_per_epx, place.origin_epx, place.size_px)
    }
}

/// Turns the draw commands of one node to `angle_deg`: a node with a
/// rotation of its own gets this angle in its place, and a node without
/// one turns about `pivot_epx`.
///
/// Needed because the renderer keeps one rotation per quad, the innermost,
/// so a second rotation around the list would not add to the node's own.
#[cfg(feature = "gpu")]
fn turn_draws(draws: &mut Vec<Draw>, pivot_epx: (f32, f32), angle_deg: f32) {
    if let Some(Draw::RotatePush { angle, .. }) = draws.first_mut() {
        *angle = angle_deg;
    } else if angle_deg != 0.0 {
        draws.insert(0, Draw::RotatePush { angle: angle_deg, cx: pivot_epx.0, cy: pivot_epx.1 });
        draws.push(Draw::RotatePop);
    }
}

/// A page's id in its stack. It stays the same while other pages are
/// added and removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId(usize);

impl PageId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// The pages of one window, drawn in `order`, bottom first, and touched
/// from the top.
pub struct PageStack {
    pages: Vec<Option<PageUi>>,
    /// The pages that show this frame, bottom first. A page that is not in
    /// it stays alive, and is neither drawn nor touched.
    pub order: Vec<PageId>,
    /// The page that took each finger that is down, by the finger's id.
    touched: Vec<(u64, PageId)>,
    /// Whether a page was removed since `take_freed` was last asked.
    was_page_removed: bool,
}

/// Pages a host lays out and draws itself, beside a program's pages.
///
/// Needed because one renderer and one picture table serve both: a
/// program's picture loader uploads what the host's pages draw, and
/// keeps a `release=unused` picture a live page of the host names
/// (`Program::gpu_draw_beside`).
///
/// `PageStack` is one. A host with several stacks writes its own.
pub trait HostPages {
    /// Adds to `wanted` key of every picture the host's drawn pages show
    /// this frame.
    fn wanted_images(&self, wanted: &mut Vec<ImageKey>);
    /// Marks in `live`, by key index, every picture a live page of the
    /// host can show.
    fn mark_pictures(&self, live: &mut [bool]);
}

impl HostPages for PageStack {
    fn wanted_images(&self, wanted: &mut Vec<ImageKey>) {
        PageStack::wanted_images(self, wanted);
    }

    fn mark_pictures(&self, live: &mut [bool]) {
        PageStack::mark_pictures(self, live);
    }
}

impl Default for PageStack {
    fn default() -> PageStack {
        PageStack::new()
    }
}

impl PageStack {
    pub fn new() -> PageStack {
        PageStack { pages: Vec::new(), order: Vec::new(), touched: Vec::new(), was_page_removed: false }
    }

    /// Adds a page on top of the order.
    pub fn add(&mut self, page: PageUi) -> PageId {
        let id: PageId = match self.pages.iter().position(|slot| slot.is_none()) {
            Some(i) => {
                self.pages[i] = Some(page);
                PageId(i)
            }
            None => {
                self.pages.push(Some(page));
                PageId(self.pages.len() - 1)
            }
        };
        self.order.push(id);
        id
    }

    /// Drops a page for good.
    pub fn remove(&mut self, id: PageId) {
        self.pages[id.0] = None;
        self.order.retain(|other| *other != id);
        self.touched.retain(|(_, other)| *other != id);
        self.was_page_removed = true;
    }

    /// Whether a page was removed since the last call. A host then frees
    /// the pictures no page shows any more.
    pub fn take_freed(&mut self) -> bool {
        std::mem::take(&mut self.was_page_removed)
    }

    /// Marks in `live` every picture a page of the stack can show
    /// (`PageUi::mark_pictures`). A page out of the order counts: it
    /// comes back without a decode.
    pub fn mark_pictures(&self, live: &mut [bool]) {
        for (_, page) in self.iter() {
            page.mark_pictures(live);
        }
    }

    pub fn contains(&self, id: PageId) -> bool {
        self.pages.get(id.0).is_some_and(|slot| slot.is_some())
    }

    /// WARNING: panics for a page that was removed.
    pub fn get(&self, id: PageId) -> &PageUi {
        self.pages[id.0].as_ref().expect("a live page")
    }

    /// WARNING: panics for a page that was removed.
    pub fn get_mut(&mut self, id: PageId) -> &mut PageUi {
        self.pages[id.0].as_mut().expect("a live page")
    }

    /// Two pages at once.
    ///
    /// WARNING: panics when `a` and `b` are the same page.
    pub fn get2_mut(&mut self, a: PageId, b: PageId) -> (&mut PageUi, &mut PageUi) {
        assert_ne!(a, b);
        let (low, high): (usize, usize) = if a.0 < b.0 { (a.0, b.0) } else { (b.0, a.0) };
        let (left, right) = self.pages.split_at_mut(high);
        let low_page: &mut PageUi = left[low].as_mut().expect("a live page");
        let high_page: &mut PageUi = right[0].as_mut().expect("a live page");
        if a.0 < b.0 { (low_page, high_page) } else { (high_page, low_page) }
    }

    /// Every page of the stack, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (PageId, &PageUi)> {
        self.pages.iter().enumerate().filter_map(|(i, slot)| slot.as_ref().map(|page| (PageId(i), page)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (PageId, &mut PageUi)> {
        self.pages.iter_mut().enumerate().filter_map(|(i, slot)| slot.as_mut().map(|page| (PageId(i), page)))
    }

    /// Lays out again every page that code wrote into since this frame's
    /// `frame`.
    ///
    /// Needed for code that writes after it read the layout (a scroll
    /// position set from the height the layout just gave): its write shows
    /// in this frame's drawing, where the old value would show for one
    /// frame. A page nobody wrote into costs nothing.
    pub fn relayout_dirty(&mut self, size: (u32, u32), epx: f32, assets: &UiAssets) {
        for (_, page) in self.iter_mut() {
            page.relayout(size, epx, assets);
        }
    }

    /// Moves `id` in the draw order to just below `above`. A page that is
    /// not in the order joins it there.
    pub fn order_below(&mut self, id: PageId, above: PageId) {
        self.order.retain(|other| *other != id);
        let at: usize = self.order.iter().position(|other| *other == above).unwrap_or(self.order.len());
        self.order.insert(at, id);
    }

    /// Moves `id` to the top of the draw order. A page that is not in the
    /// order joins it there.
    pub fn order_top(&mut self, id: PageId) {
        self.order.retain(|other| *other != id);
        self.order.push(id);
    }

    /// Takes `id` out of the draw order: it is drawn and touched no more,
    /// and stays alive.
    pub fn order_remove(&mut self, id: PageId) {
        self.order.retain(|other| *other != id);
        self.touched.retain(|(_, other)| *other != id);
    }

    pub fn is_ordered(&self, id: PageId) -> bool {
        self.order.contains(&id)
    }

    /// Hands every page's runtime `locale` now. Each page lays out once
    /// for it. A stack that gets a `PagePlatform` at every `frame` needs
    /// no call: the platform carries the language.
    pub fn set_locale(&mut self, locale: Option<Arc<Locale>>) {
        for (_, page) in self.iter_mut() {
            page.runtime.set_locale(locale.clone());
        }
    }

    /// One frame, `dt` seconds after the last: every page takes
    /// `platform`, moves its animations, and lays out again if something
    /// changed. `size` is in device pixels, `epx` is device pixels per
    /// epx.
    ///
    /// Returns true while something still moves.
    pub fn frame(&mut self, dt: f32, size: (u32, u32), epx: f32, platform: &PagePlatform, assets: &UiAssets) -> bool {
        let mut is_moving: bool = false;
        for (_, page) in self.iter_mut() {
            is_moving |= page.frame(dt, size, epx, platform, assets);
        }
        is_moving
    }

    /// Lays out one page now, for a host that reads its rectangles between
    /// two frames.
    pub fn relayout(&mut self, id: PageId, size: (u32, u32), epx: f32, assets: &UiAssets) {
        self.get_mut(id).relayout(size, epx, assets);
    }

    /// Calls `f` with the draw commands of each page in draw order, with
    /// the page that owns their texts and with its pose. Allocates
    /// nothing, so a frame goes this way.
    pub fn for_each_draws<'a>(&'a self, f: &mut dyn FnMut(&[Draw], &'a Page, Option<PagePose>)) {
        for page in self.order.iter().filter_map(|id| self.pages[id.0].as_ref()) {
            f(page.draws.as_slice(), &page.page, page.pose);
        }
    }

    /// `for_each_draws` for the `above_children` commands of the pages
    /// that have some.
    pub fn for_each_top_draws<'a>(&'a self, f: &mut dyn FnMut(&[Draw], &'a Page, Option<PagePose>)) {
        for page in self.order.iter().filter_map(|id| self.pages[id.0].as_ref()).filter(|page| !page.top_draws.is_empty()) {
            f(page.top_draws.as_slice(), &page.page, page.pose);
        }
    }

    /// What `for_each_draws` walks, as a new list, for a test.
    pub fn draws(&self) -> Vec<(&[Draw], &Page, Option<PagePose>)> {
        self.order.iter().filter_map(|id| self.pages[id.0].as_ref()).map(|page| (page.draws.as_slice(), &page.page, page.pose)).collect()
    }

    /// What `for_each_top_draws` walks, as a new list, for a test.
    pub fn top_draws(&self) -> Vec<(&[Draw], &Page, Option<PagePose>)> {
        self.order.iter().filter_map(|id| self.pages[id.0].as_ref()).filter(|page| !page.top_draws.is_empty()).map(|page| (page.top_draws.as_slice(), &page.page, page.pose)).collect()
    }

    /// Adds to `out` the key of every picture the drawn pages show this
    /// frame, each once.
    pub fn wanted_images(&self, out: &mut Vec<ImageKey>) {
        for id in &self.order {
            let Some(page) = self.pages[id.0].as_ref() else { continue };
            for draw in &page.draws {
                if let Draw::Image { image, .. } = draw {
                    if !out.contains(image) {
                        out.push(*image);
                    }
                }
            }
        }
    }

    /// How many `loading=lazy` pictures the drawn pages show this frame
    /// whose pixels are not on the GPU yet.
    pub fn lazy_pending(&self, assets: &UiAssets) -> usize {
        let mut seen: Vec<ImageKey> = Vec::new();
        for id in &self.order {
            let Some(page) = self.pages[id.0].as_ref() else { continue };
            for draw in &page.draws {
                if let Draw::Image { image, .. } = draw {
                    if assets.images.is_lazy(*image) && assets.images.pending_bytes(*image).is_some() && !seen.contains(image) {
                        seen.push(*image);
                    }
                }
            }
        }
        seen.len()
    }

    /// The topmost drawn page that would take a press at (`x`, `y`), in
    /// epx: one with a node that takes input there, or a modal page that
    /// covers the point.
    pub fn pick(&self, x: f32, y: f32) -> Option<PageId> {
        self.order.iter().rev().copied().find(|id| {
            let page: &PageUi = self.get(*id);
            page.covers(x, y) && (page.hit(x, y).is_some() || page.is_modal)
        })
    }

    /// The topmost drawn page that has the keyboard, if any.
    pub fn focused(&self) -> Option<PageId> {
        self.order.iter().rev().copied().find(|id| self.get(*id).input.focused().is_some())
    }

    /// The page and the field that hold the text caret, which is where an
    /// on-screen keyboard types.
    pub fn caret_holder(&self) -> Option<(PageId, NodeId)> {
        self.iter().find_map(|(id, page)| page.runtime.caret.map(|caret| (id, caret.0)))
    }

    /// The page that raised gogo's edit bar (cut, copy, paste) over one of
    /// its fields, and the field.
    pub fn edit_bar_raised(&self) -> Option<(PageId, NodeId)> {
        self.order.iter().rev().find_map(|id| self.get(*id).runtime.edit_bar.map(|field| (*id, field)))
    }

    /// Takes the edit bar down on whichever page raised it.
    pub fn dismiss_edit_bar(&mut self) {
        for (_, page) in self.iter_mut() {
            if page.runtime.edit_bar.take().is_some() {
                page.runtime.dirty = true;
            }
        }
    }

    /// The page that took `finger` when it came down.
    pub fn touched_by(&self, finger: u64) -> Option<PageId> {
        self.touched.iter().find(|(other, _)| *other == finger).map(|(_, page)| *page)
    }

    /// Hands one sample of a pointer that presses, in epx, to the page
    /// `id`: the one `pick` chose when the pointer came down, or the one
    /// that has held it since. A pointer that presses nothing goes to
    /// `hover`.
    ///
    /// A `Down` of a pointer that page already holds stays with that
    /// page: a second button of the mouse.
    ///
    /// Returns the events the sample caused on that page.
    pub fn pointer_to(&mut self, id: PageId, touch: Touch, epx: f32, assets: &UiAssets) -> EventList {
        let finger: u64 = touch.id;
        if touch.phase == Phase::Down {
            self.touched.retain(|(other, _)| *other != finger);
            self.touched.push((finger, id));
        }
        let measure: EpxMeasure<'_> = assets.measure(epx);
        let page: &mut PageUi = self.get_mut(id);
        let events: EventList = page.input.touch(&page.page, &mut page.runtime, page.kept.frame(), &measure, touch);
        if matches!(touch.phase, Phase::Up | Phase::Cancel) {
            self.touched.retain(|(other, _)| *other != finger);
        }
        events
    }

    /// `pointer_to` for a sample that says no more than a plain touch
    /// screen does: a finger, or the mouse's primary button (`finger` is
    /// `MOUSE_ID`).
    #[allow(clippy::too_many_arguments)]
    pub fn touch_to(&mut self, id: PageId, phase: Phase, x: f32, y: f32, t: f64, finger: u64, epx: f32, assets: &UiAssets) -> EventList {
        self.pointer_to(id, plain_touch(phase, x, y, t, finger), epx, assets)
    }

    /// Hands a touch to the page it belongs to. A finger that comes down
    /// goes to the topmost page that takes it (`pick`); its later moves
    /// and its lift follow that page.
    ///
    /// A press that no page takes still goes to the page whose field holds
    /// the text caret, so that the field ends its edit. A focused button
    /// does not get it: the press belongs to what is under the pages.
    ///
    /// Returns the page and the events, or `None` when no page took the
    /// touch.
    #[allow(clippy::too_many_arguments)]
    pub fn touch(&mut self, phase: Phase, x: f32, y: f32, t: f64, finger: u64, epx: f32, assets: &UiAssets) -> Option<(PageId, EventList)> {
        self.pointer(plain_touch(phase, x, y, t, finger), epx, assets)
    }

    /// `touch` with the whole sample: tool, buttons, pressure, a pen's
    /// lean, the keys held.
    ///
    /// A `Down` of a pointer that a page already holds goes to that page:
    /// a second button of the mouse is a move of the press it has.
    pub fn pointer(&mut self, touch: Touch, epx: f32, assets: &UiAssets) -> Option<(PageId, EventList)> {
        let held_by: Option<PageId> = self.touched_by(touch.id);
        let id: PageId = if touch.phase == Phase::Down && held_by.is_none() { self.pick(touch.x, touch.y).or_else(|| self.caret_holder().map(|(id, _)| id)) } else { held_by }?;
        Some((id, self.pointer_to(id, touch, epx, assets)))
    }

    /// Hands a key to the page that has the keyboard. When no page has
    /// it, Tab goes to the topmost page with a node that can take it, and
    /// any other key goes nowhere.
    ///
    /// Returns the events, each with its page.
    pub fn key(&mut self, key: Key, mods: Mods) -> Vec<(PageId, Event)> {
        let mut out: Vec<(PageId, Event)> = Vec::new();
        let targets: Vec<PageId> = match self.focused() {
            Some(id) => vec![id],
            None if matches!(key, Key::Tab | Key::TabBack) => self.order.iter().rev().copied().collect(),
            None => Vec::new(),
        };
        for id in targets {
            let page: &mut PageUi = self.get_mut(id);
            let events: EventList = page.input.key(&mut page.page, &mut page.runtime, page.kept.frame(), key, mods);
            let has_taken_it: bool = page.input.focused().is_some();
            for event in events {
                out.push((id, event));
            }
            if has_taken_it {
                break;
            }
        }
        out
    }

    /// The mouse moved with no button held: the page `top` shows the node
    /// under the mouse as hovered, and every other page shows none.
    ///
    /// `top` is the topmost page of this stack under the mouse, or `None`
    /// when a page of another stack is above it. A finger never comes
    /// here: a touch does not hover.
    ///
    /// `sample`: place in epx; its phase is not read. `heard` gets the
    /// `on:hover=` events, each with its page: `Over` for the box under
    /// the mouse, `Left` for the box it was over before.
    pub fn hover(&mut self, top: Option<PageId>, sample: Touch, heard: &mut Vec<(PageId, Event)>) {
        for (id, page) in self.iter_mut() {
            let events: EventList = if Some(id) == top { page.input.hover_sample(&page.page, &mut page.runtime, page.kept.frame(), sample) } else { page.input.unhover(&page.page, &mut page.runtime) };
            heard.extend(events.into_iter().map(|event| (id, event)));
        }
    }

    /// The topmost drawn page under (`x`, `y`).
    pub fn top_under(&self, x: f32, y: f32) -> Option<PageId> {
        self.order.iter().rev().copied().find(|id| self.get(*id).covers(x, y))
    }

    /// The mouse left the window: no page shows a hovered node. `heard`
    /// gets `Left` for the `on:hover=` box it was over.
    pub fn unhover(&mut self, heard: &mut Vec<(PageId, Event)>) {
        for (id, page) in self.iter_mut() {
            let events: EventList = page.input.unhover(&page.page, &mut page.runtime);
            heard.extend(events.into_iter().map(|event| (id, event)));
        }
    }

    /// Mouse wheel turned by `notches` at (`x`, `y`) on the page `id`,
    /// with `mods` held. It goes to a box with `on:wheel=` under the
    /// mouse, else it steps a Slider or a Toggle there.
    ///
    /// Returns the events of what took it.
    pub fn wheel_to(&mut self, id: PageId, x: f32, y: f32, notches: f32, mods: Mods) -> EventList {
        let page: &mut PageUi = self.get_mut(id);
        page.input.wheel(&page.page, &mut page.runtime, page.kept.frame(), x, y, notches, mods)
    }

    /// The text selected on the page that has the keyboard, for a copy.
    pub fn selected_text(&self) -> Option<String> {
        let id: PageId = self.focused()?;
        let page: &PageUi = self.get(id);
        page.input.selected_text(&page.page, &page.runtime)
    }

    /// Deletes the selection on the page that has the keyboard, for a cut.
    pub fn delete_selection(&mut self) -> Vec<(PageId, Event)> {
        let Some(id) = self.focused() else { return Vec::new() };
        let page: &mut PageUi = self.get_mut(id);
        page.input.delete_selection(&mut page.page, &mut page.runtime).into_iter().map(|event| (id, event)).collect()
    }

    /// Pastes `text` into the page that has the keyboard.
    pub fn paste(&mut self, text: &str) -> Vec<(PageId, Event)> {
        let Some(id) = self.focused() else { return Vec::new() };
        let page: &mut PageUi = self.get_mut(id);
        page.input.paste(&mut page.page, &mut page.runtime, text).into_iter().map(|event| (id, event)).collect()
    }

    /// Selects all the text of the field that has the keyboard.
    pub fn select_all_focused(&mut self) {
        if let Some(id) = self.focused() {
            let page: &mut PageUi = self.get_mut(id);
            page.input.select_all(&page.page, &mut page.runtime);
        }
    }

    /// Switches every page that declares the arrangement `name` to it.
    ///
    /// Returns whether any page declares it.
    pub fn go(&mut self, name: &str) -> bool {
        let mut has_any: bool = false;
        for (_, page) in self.iter_mut() {
            if let Some(i) = page.page.arrangements.iter().position(|arrangement| page.page.str(*arrangement) == name) {
                page.runtime.go_id(ArrangementId::from_index(i));
                has_any = true;
            }
        }
        has_any
    }
}

/// A sample that says no more than a plain touch screen does: a finger,
/// or the mouse with its primary button (`finger` is `MOUSE_ID`).
pub(crate) fn plain_touch(phase: Phase, x: f32, y: f32, t: f64, finger: u64) -> Touch {
    if finger == MOUSE_ID { Touch::mouse(phase, x, y, t, Buttons::PRIMARY) } else { Touch { phase, x, y, t, id: finger, ..Touch::PLAIN } }
}
