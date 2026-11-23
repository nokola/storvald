//! Several pages in one stack, run with no window: which page takes a
//! finger, which pages are drawn, what a page reads from the screen, and
//! the picture table.
//!
//! Not in the crates.io package (`exclude` in Cargo.toml): it reads a font
//! from `assets/Fonts` at the top of the repository, which a package does
//! not carry.

// Built with no feature (a build script's use), TurboDot has no pages to test.
#![cfg(feature = "ui")]

use gogo_text::TextEngine;
use gogo_ui::{Event, EventKind, EventList, ImageKey, Images, NodeId, Orientation, Page, Phase, Rect, ScreenClass, compile, has_errors};
use turbodot::pages::{PageUi, ImageTable, PageId, PagePlatform, PageStack, UiAssets};

/// A page with one wide button at its top left.
const BOTTOM: &str = r#"
View bottom background=#101010 {
    Button wide text="Wide", width=300, height=200, halign=left, valign=top
}
"#;

/// A page with one small button at its top left, and nothing else that
/// takes a touch.
const TOP: &str = r#"
View top {
    Button small text="Small", width=100, height=50, halign=left, valign=top
}
"#;

const SIZE: (u32, u32) = (400, 300);
const EPX: f32 = 1.0;

fn page(text: &str) -> Page {
    let compiled = compile(text, &|_| None);
    assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
    compiled.page
}

fn assets() -> UiAssets {
    let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
    UiAssets::new(TextEngine::new(&[("Roboto Regular", font)]), ImageTable::from_keys(Images::from_names(std::iter::empty::<&str>())))
}

fn platform() -> PagePlatform {
    PagePlatform::new([0.0; 4], Orientation::from_view(SIZE.0 as f32, SIZE.1 as f32), ScreenClass::from_view(SIZE.0 as f32, SIZE.1 as f32), None)
}

/// The two pages in one stack after a frame: `BOTTOM` below, `TOP` above.
fn two_pages() -> (PageStack, UiAssets, PageId, PageId) {
    let assets: UiAssets = assets();
    let mut stack: PageStack = PageStack::new();
    let bottom: PageId = stack.add(PageUi::new(page(BOTTOM)));
    let top: PageId = stack.add(PageUi::new(page(TOP)));
    stack.frame(0.0, SIZE, EPX, &platform(), &assets);
    (stack, assets, bottom, top)
}

/// The node of the page `id` that its file names `name`.
fn node(stack: &PageStack, id: PageId, name: &str) -> NodeId {
    stack.get(id).page.find(name).unwrap_or_else(|| panic!("no node named `{name}`"))
}

/// A press and a lift at (`x`, `y`).
///
/// Returns the page that took the press, with the events of both.
fn tap(stack: &mut PageStack, assets: &UiAssets, x: f32, y: f32) -> Option<(PageId, EventList)> {
    let (id, mut events) = stack.touch(Phase::Down, x, y, 1.0, 1, EPX, assets)?;
    events.extend(stack.touch(Phase::Up, x, y, 1.05, 1, EPX, assets).expect("the lift follows the press").1);
    Some((id, events))
}

/// Outline names every node that shows, with the rectangle a test reads
/// from the same frame. Empty before first frame.
#[test]
fn the_outline_gives_each_node_its_kind_its_name_and_its_rectangle() {
    let assets: UiAssets = assets();
    let mut stack: PageStack = PageStack::new();
    let id: PageId = stack.add(PageUi::new(page(BOTTOM)));
    assert_eq!(stack.get(id).outline(), "", "no frame yet");
    stack.frame(0.0, SIZE, EPX, &platform(), &assets);
    let outline: String = stack.get(id).outline();
    let lines: Vec<&str> = outline.lines().collect();
    assert_eq!(lines, ["View bottom x=0 y=0 w=400 h=300", "  Button wide x=0 y=0 w=300 h=200"], "{outline}");
    let wide: Rect = stack.get(id).rect(stack.get(id).node("wide"));
    assert_eq!((wide.x, wide.y, wide.w, wide.h), (0.0, 0.0, 300.0, 200.0));
}

/// Page covers its viewport when root paints an opaque plate with square
/// corners over all of it, and from first frame on.
#[test]
fn a_page_covers_its_viewport_when_its_root_is_an_opaque_plate() {
    let assets: UiAssets = assets();
    let mut stack: PageStack = PageStack::new();
    let opaque: PageId = stack.add(PageUi::new(page(BOTTOM)));
    let no_background: PageId = stack.add(PageUi::new(page(TOP)));
    let see_through: PageId = stack.add(PageUi::new(page("View sheet background=black/0.6")));
    let rounded: PageId = stack.add(PageUi::new(page("View card background=#101010, corner_radius=12")));
    let small: PageId = stack.add(PageUi::new(page("View box background=#101010, width=200, height=100")));
    assert!(!stack.get(opaque).covers_viewport(), "no frame yet");
    stack.frame(0.0, SIZE, EPX, &platform(), &assets);
    assert!(stack.get(opaque).covers_viewport());
    assert!(!stack.get(no_background).covers_viewport());
    assert!(!stack.get(see_through).covers_viewport());
    assert!(!stack.get(rounded).covers_viewport());
    assert!(!stack.get(small).covers_viewport());
}

/// A press where both pages have a button goes to the page on top. A
/// press where only the page below has one goes through the page on top
/// to it. A press where neither has one goes to no page.
#[test]
fn a_press_goes_to_the_topmost_page_with_a_node_under_it() {
    let (mut stack, assets, bottom, top) = two_pages();
    let small: Rect = stack.get(top).kept.rects()[node(&stack, top, "small").index()];
    assert_eq!((small.x, small.y, small.w, small.h), (0.0, 0.0, 100.0, 50.0));

    let (id, events) = tap(&mut stack, &assets, 50.0, 25.0).expect("a page takes it");
    assert_eq!(id, top);
    assert!(events.iter().any(|event| event.kind == EventKind::Activate && event.node == node(&stack, top, "small")), "{events:?}");

    let (id, events) = tap(&mut stack, &assets, 250.0, 150.0).expect("a page takes it");
    assert_eq!(id, bottom, "the page on top has nothing there");
    assert!(events.iter().any(|event| event.kind == EventKind::Activate && event.node == node(&stack, bottom, "wide")), "{events:?}");

    assert!(tap(&mut stack, &assets, 350.0, 250.0).is_none(), "no page has a node there");
}

/// A modal page takes a press that lands on none of its nodes, so the
/// page below does not get it.
#[test]
fn a_modal_page_takes_a_press_beside_its_nodes() {
    let (mut stack, assets, _bottom, top) = two_pages();
    stack.get_mut(top).is_modal = true;
    let (id, events) = tap(&mut stack, &assets, 250.0, 150.0).expect("the modal page takes it");
    assert_eq!(id, top);
    assert!(events.is_empty(), "nothing of the page below is pressed: {events:?}");
}

/// A finger that came down on one page stays with it: when it lifts where
/// the page on top has a button, the tap is still the lower page's.
#[test]
fn a_finger_stays_with_the_page_it_came_down_on() {
    let (mut stack, assets, bottom, top) = two_pages();
    let (id, _) = stack.touch(Phase::Down, 250.0, 150.0, 1.0, 7, EPX, &assets).expect("the page below takes it");
    assert_eq!(id, bottom);
    assert_eq!(stack.touched_by(7), Some(bottom));
    let (id, _) = stack.touch(Phase::Move, 50.0, 25.0, 1.1, 7, EPX, &assets).expect("the move follows the finger's page");
    assert_eq!(id, bottom);
    let (id, events) = stack.touch(Phase::Up, 50.0, 25.0, 1.2, 7, EPX, &assets).expect("the lift follows the finger's page");
    assert_eq!(id, bottom);
    assert!(events.iter().any(|event| event.kind == EventKind::Activate && event.node == node(&stack, bottom, "wide")), "the tap is the wide button's: {events:?}");
    assert!(!stack.get(top).runtime.dirty, "the page on top saw nothing of the finger");
    assert_eq!(stack.touched_by(7), None, "a lifted finger belongs to no page");
}

/// A page taken out of the order is neither drawn nor touched, and comes
/// back as it was.
#[test]
fn a_page_out_of_the_order_is_neither_drawn_nor_touched() {
    let (mut stack, assets, bottom, top) = two_pages();
    assert_eq!(stack.draws().len(), 2);
    stack.order_remove(top);
    assert!(stack.contains(top) && !stack.is_ordered(top));
    assert_eq!(stack.draws().len(), 1);
    let (id, _) = tap(&mut stack, &assets, 50.0, 25.0).expect("the page below takes it");
    assert_eq!(id, bottom, "the page on top is out of the order");

    stack.order_top(top);
    assert_eq!(stack.order, [bottom, top]);
    assert_eq!(tap(&mut stack, &assets, 50.0, 25.0).expect("a page takes it").0, top);

    stack.order_below(top, bottom);
    assert_eq!(stack.order, [top, bottom]);
    assert_eq!(tap(&mut stack, &assets, 50.0, 25.0).expect("a page takes it").0, bottom, "the wide button is on top now");

    stack.remove(top);
    assert!(!stack.contains(top));
    assert!(stack.take_freed() && !stack.take_freed(), "a removed page is reported once");
}

/// An event with no `on:` action is for the handler named like the nearest
/// named node above it.
#[test]
fn an_event_names_its_handler() {
    let (mut stack, assets, bottom, _top) = two_pages();
    let (id, events) = tap(&mut stack, &assets, 250.0, 150.0).expect("the page below takes it");
    let activate: &Event = events.iter().find(|event| event.kind == EventKind::Activate).expect("a tap");
    assert_eq!(stack.get(id).handler_name(activate), Some("wide"));
    assert_eq!(id, bottom);
}

/// A child page at the window's edge gets the whole inset, a child that
/// starts inside the window gets what its distance from the edge leaves,
/// and a child far inside gets none.
#[test]
fn a_child_page_gets_the_part_of_the_inset_its_place_does_not_cover() {
    let window: Rect = Rect::new(0.0, 0.0, 400.0, 300.0);
    let mut platform: PagePlatform = platform();
    platform.safe = [10.0, 40.0, 0.0, 20.0];
    assert_eq!(platform.residual(window, window).safe, [10.0, 40.0, 0.0, 20.0]);
    assert_eq!(platform.residual(window, Rect::new(4.0, 30.0, 396.0, 260.0)).safe, [6.0, 10.0, 0.0, 10.0]);
    assert_eq!(platform.residual(window, Rect::new(100.0, 100.0, 100.0, 100.0)).safe, [0.0; 4]);
}

/// A page is laid out again when the insets change, and not when the same
/// platform comes again.
#[test]
fn the_same_platform_twice_lays_nothing_out_again() {
    let (mut stack, assets, bottom, _top) = two_pages();
    let mut platform: PagePlatform = platform();
    stack.frame(0.0, SIZE, EPX, &platform, &assets);
    assert!(!stack.get(bottom).runtime.dirty);
    platform.safe = [0.0, 24.0, 0.0, 0.0];
    let page_ui: &mut PageUi = stack.get_mut(bottom);
    platform.apply(&page_ui.page, &mut page_ui.runtime);
    assert!(page_ui.runtime.dirty, "new insets ask for a layout");
    stack.frame(0.0, SIZE, EPX, &platform, &assets);
    let page_ui: &mut PageUi = stack.get_mut(bottom);
    platform.apply(&page_ui.page, &mut page_ui.runtime);
    assert!(!page_ui.runtime.dirty, "the same insets again ask for none");
}

/// A key that code makes while the app runs gets a row in every column of
/// the picture table, and the bytes to decode count down as they are
/// taken.
///
/// Needed because a key without a size row is laid out 0 wide: the user
/// sees an empty place where the picture belongs.
#[test]
fn a_key_made_later_has_its_rows_and_pending_bytes_count_down() {
    static FILE: [u8; 4] = [1, 2, 3, 4];
    let keys: Images = Images::from_names(["res://a.png", "res://b.png"]);
    let mut table: ImageTable = ImageTable::new(keys, vec![Some(&FILE[..]), None], vec![true, false], vec![Some((7, 9)), None]);
    let (a, b): (ImageKey, ImageKey) = (table.get("res://a.png").expect("a key"), table.get("res://b.png").expect("a key"));
    assert!(table.is_lazy(a) && !table.is_lazy(b));
    assert_eq!((table.size(a), table.size(b)), (Some((7, 9)), None));

    let later: ImageKey = table.intern("code://later");
    assert_eq!(later.index(), 2);
    assert_eq!(table.sizes().len(), 3, "the size column grew with the key");
    assert_eq!((table.size(later), table.is_lazy(later), table.pending_bytes(later)), (None, false, None));
    assert_eq!(table.intern("code://later"), later, "the same name is the same key");
    assert_eq!(table.len(), 3);

    assert_eq!(table.pending_count(), 1);
    assert_eq!(table.take_pending(a), Some(&FILE[..]));
    assert_eq!((table.pending_count(), table.take_pending(a)), (0, None));
}

/// The picture table on a real GPU: an upload lands the size with the
/// pixels and says when the size changed, and only a `release=unused`
/// picture that no page marks leaves the GPU, to be decoded again.
///
/// Needed because both go wrong without a sign in the layout's numbers:
/// with a size that lags the pixels the user sees a picture 0 wide, and
/// with a picture that is never freed the app's memory grows with every
/// page visited.
///
/// Ignored unless asked for: it needs a GPU adapter, which a hosted test
/// machine may not have, and a test that returned early there would count
/// as passed without having run. On a machine with a GPU:
///
///     cargo test -p turbodot -- --include-ignored
#[cfg(feature = "gpu")]
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn an_upload_lands_its_size_and_only_an_unmarked_releasable_picture_is_freed() {
    static FILE_A: [u8; 4] = [1, 2, 3, 4];
    static FILE_B: [u8; 4] = [5, 6, 7, 8];
    static FILE_KEPT: [u8; 4] = [9, 9, 9, 9];
    let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
    let mut renderer: gogo_wgpu::UiRenderer = gogo_wgpu::UiRenderer::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
    let keys: Images = Images::from_names(["res://a.png", "res://b.png", "res://kept.png"]);
    let mut table: ImageTable = ImageTable::new(keys, vec![Some(&FILE_A[..]), Some(&FILE_B[..]), Some(&FILE_KEPT[..])], vec![false; 3], vec![Some((1, 1)); 3]);
    let (a, b, kept): (ImageKey, ImageKey, ImageKey) = (ImageKey::from_index(0), ImageKey::from_index(1), ImageKey::from_index(2));
    table.set_release_unused(&[a, b]);
    let from_code: ImageKey = table.intern("code://c");
    let one_pixel: Vec<u8> = vec![255u8; 4];
    for key in [a, b, kept] {
        assert!(table.take_pending(key).is_some());
        assert!(!table.upload_image(&mut renderer, &gpu.device, &gpu.queue, key, &one_pixel, 1, 1), "the size the header gave stays");
    }
    assert!(table.upload_image(&mut renderer, &gpu.device, &gpu.queue, from_code, &one_pixel, 1, 1), "the first upload of a picture from code changes its size");
    assert_eq!(table.size(from_code), Some((1, 1)));
    let six_pixels: Vec<u8> = vec![255u8; 3 * 2 * 4];
    assert!(table.upload_image(&mut renderer, &gpu.device, &gpu.queue, from_code, &six_pixels, 3, 2), "a new size is a change");
    assert!(!table.upload_image(&mut renderer, &gpu.device, &gpu.queue, from_code, &six_pixels, 3, 2), "the same size again is none");
    assert_eq!(table.size(from_code), Some((3, 2)));
    assert_eq!(table.pending_count(), 0);

    // Only `a` is marked as shown: `b` goes; `a`, the picture without
    // `release=unused` and the picture from code stay.
    let mut live: Vec<bool> = vec![false; table.len()];
    live[a.index()] = true;
    let mut freed: Vec<ImageKey> = Vec::new();
    table.release_unmarked(&mut renderer, &gpu.device, &live, &mut freed);
    assert_eq!(freed, [b]);
    assert!(renderer.has_image(a) && renderer.has_image(from_code) && renderer.has_image(kept));
    assert!(!renderer.has_image(b));
    assert_eq!(table.pending_bytes(b), Some(&FILE_B[..]), "its file is in line to be decoded again");
    assert_eq!((table.pending_count(), table.size(b)), (1, Some((1, 1))), "layout still knows its size");

    freed.clear();
    table.release_unmarked(&mut renderer, &gpu.device, &live, &mut freed);
    assert!(freed.is_empty(), "a freed picture is not freed twice");
}
