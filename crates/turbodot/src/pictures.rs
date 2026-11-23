//! Pictures a program's pages name: their sizes when the program starts,
//! their pixels when a page first draws them.
//!
//! - The build puts every picture a page names into the program
//!   (`res://assets/x.png`), so no file is read while it runs.
//! - [`image_table`] reads each picture's header at start. Pages lay out
//!   at their pictures' sizes before a pixel is decoded, with no GPU.
//! - [`PictureLoader`] (cargo feature `gpu`) decodes and uploads a
//!   picture in the first frame a page draws it. A picture every line
//!   marks `loading=lazy` is decoded on a thread of its own and shows a
//!   frame or more later, so a page full of them holds no frame.
//! - A picture every line marks `release=unused` leaves the GPU once no
//!   live page names it, and is decoded again at its next show.
//!
//! Decodes png and jpeg. Cargo features `webp` and `bmp` add those.

use gogo_ui::reload::{BuildTables, BuiltPage};
use gogo_ui::{ImageKey, Images, Kind, PropKey, Value};

use crate::pages::ImageTable;

/// Picture table of a program at its start: every picture the build
/// holds, with its file bytes still to decode, its size read from the
/// file's header and whether it loads late. `release_unused`: the build's
/// `RELEASE_UNUSED`.
///
/// Needed so a page lays out at its pictures' sizes in its first frame:
/// without the sizes an Image with no written size is 0 wide until its
/// pixels are decoded, and the page jumps when they are.
pub fn image_table(tables: &'static BuildTables<'static>, release_unused: &[ImageKey]) -> ImageTable {
    let keys: Images = Images::from_names(tables.images.iter().map(|(name, _)| *name));
    let is_lazy: Vec<bool> = lazy_pictures(&keys, tables.pages);
    let mut pending: Vec<Option<&'static [u8]>> = Vec::with_capacity(tables.images.len());
    let mut sizes: Vec<Option<(u32, u32)>> = Vec::with_capacity(tables.images.len());
    for (_, bytes) in tables.images {
        sizes.push(bytes.and_then(picture_size));
        pending.push(*bytes);
    }
    let mut table: ImageTable = ImageTable::new(keys, pending, is_lazy, sizes);
    table.set_release_unused(release_unused);
    table
}

/// Width and height of a picture from its file's header alone: no pixel
/// is decoded.
///
/// Returns `None` for a file that is no picture this build can decode.
pub fn picture_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if let Some(size) = gogo_ui::atlas::picture_size(bytes) {
        return Some(size);
    }
    other_format_size(bytes)
}

/// Size of a picture in a format gogo's own header reader does not know
/// (it reads png and jpeg).
#[cfg(any(feature = "webp", feature = "bmp"))]
fn other_format_size(bytes: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?.into_dimensions().ok()
}

#[cfg(not(any(feature = "webp", feature = "bmp")))]
fn other_format_size(_bytes: &[u8]) -> Option<(u32, u32)> {
    None
}

/// By `ImageKey`: true when every Image line that names the picture marks
/// it `loading=lazy`. An atlas slice counts for its whole picture: the
/// picture is what decodes.
///
/// Needed because a picture one page shows at once is wanted at that
/// page's first frame: one line without the mark makes it load at once
/// everywhere.
///
/// Reads the built pages' tables in place: nothing is copied.
fn lazy_pictures(images: &Images, pages: &[BuiltPage<'_>]) -> Vec<bool> {
    let mut is_lazy: Vec<bool> = vec![false; images.len()];
    let mut is_shown_at_once: Vec<bool> = vec![false; images.len()];
    for built in pages {
        let page = built.page;
        for node in page.nodes {
            if node.kind != Kind::Image {
                continue;
            }
            let props = &page.props[node.props.0 as usize..(node.props.0 + node.props.1) as usize];
            let source: Option<ImageKey> = props.iter().find_map(|p| match (p.key, p.value) {
                (PropKey::Source, Value::VImage(key)) => {
                    let picture: ImageKey = if key.is_slice() { page.slices.get(key.slice_index()).map_or(key, |slice| slice.picture) } else { key };
                    (picture.index() < images.len()).then_some(picture)
                }
                _ => None,
            });
            let Some(source) = source else { continue };
            let says_lazy: bool = props.iter().any(|p| matches!((p.key, p.value), (PropKey::Loading, Value::VIdent(word)) if page.strings.get(word.index()) == Some(&"lazy")));
            if says_lazy {
                is_lazy[source.index()] = true;
            } else {
                is_shown_at_once[source.index()] = true;
            }
        }
    }
    for (lazy, at_once) in is_lazy.iter_mut().zip(&is_shown_at_once) {
        *lazy = *lazy && !*at_once;
    }
    is_lazy
}

#[cfg(feature = "gpu")]
pub use loader::{LazyPictures, PictureLoader};

#[cfg(feature = "gpu")]
mod loader {
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::time::Instant;

    use gogo_ui::ImageKey;
    use gogo_wgpu::UiRenderer;

    use crate::pages::ImageTable;

    /// A decoded picture: width, height, then straight-alpha sRGB bytes,
    /// four per pixel (R, G, B, A), row by row.
    type Decoded = (u32, u32, Vec<u8>);

    /// Decodes a picture file to pixels. `name` is for the warning.
    ///
    /// Returns `None`, after a warning in the log, for a file that does
    /// not decode.
    fn decode(name: &str, bytes: &[u8]) -> Option<Decoded> {
        match image::load_from_memory(bytes) {
            Ok(picture) => {
                let rgba = picture.to_rgba8();
                Some((rgba.width(), rgba.height(), rgba.into_raw()))
            }
            Err(e) => {
                log::warn!("TURBODOT_PICTURE {name}: {e}");
                None
            }
        }
    }

    /// What a frame does with a picture marked `loading=lazy`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LazyPictures {
        /// Sends it to the decode thread: it shows a frame or more later.
        /// A window's frames.
        OnThread,
        /// Decodes it in this frame, like any other picture. For a frame
        /// that must show everything: a picture of the pages for a test.
        Now,
    }

    /// Decodes and uploads the pictures a program's pages draw, and frees
    /// the ones no page shows any more. One per renderer.
    #[derive(Default)]
    pub struct PictureLoader {
        /// Pictures this frame's draw commands name. Kept between frames
        /// for its memory.
        wanted: Vec<ImageKey>,
        lazy_decoder: LazyDecoder,
        /// By key index: a live page names the picture. Kept for its
        /// memory.
        live: Vec<bool>,
        /// Pictures the last `release_unshown` freed. Kept for its memory.
        freed: Vec<ImageKey>,
    }

    impl PictureLoader {
        /// Decodes and uploads every picture this frame's draw commands
        /// name that the renderer does not have yet. `add_wanted` adds
        /// the keys those commands name (`PageStack::wanted_images`).
        /// Call it before the renderer prepares the pages.
        ///
        /// Needed because the renderer draws nothing for a picture it
        /// does not have: without this call an Image shows as an empty
        /// place.
        ///
        /// A lazy picture sent to the thread stays "to decode" until its
        /// pixels are uploaded here, in a later frame.
        ///
        /// Returns how many pictures it uploaded.
        pub fn upload_for_draws(&mut self, images: &mut ImageTable, renderer: &mut UiRenderer, device: &wgpu::Device, queue: &wgpu::Queue, lazy: LazyPictures, add_wanted: impl FnOnce(&mut Vec<ImageKey>)) -> usize {
            if images.pending_count() == 0 {
                return 0;
            }
            let mut uploaded: usize = 0;
            for landed in self.lazy_decoder.take_decoded() {
                let Some(_) = images.take_pending(landed.key) else { continue };
                let Some((width, height, rgba)) = landed.pixels else { continue };
                let upload_start: Instant = Instant::now();
                images.upload_image(renderer, device, queue, landed.key, &rgba, width, height);
                log::info!(
                    "TURBODOT_PICTURE lazy {} {width}x{height} decoded in {} ms on the decode thread, uploaded in {:.1} ms, {} ms after the request",
                    images.name(landed.key),
                    landed.decode_ms,
                    upload_start.elapsed().as_secs_f64() * 1000.0,
                    landed.sent_at.elapsed().as_millis()
                );
                uploaded += 1;
            }
            self.wanted.clear();
            add_wanted(&mut self.wanted);
            for i in 0..self.wanted.len() {
                let key: ImageKey = self.wanted[i];
                let Some(bytes) = images.pending_bytes(key) else { continue };
                if lazy == LazyPictures::OnThread && images.is_lazy(key) {
                    // Sent once; it uploads above when it lands.
                    self.lazy_decoder.request(key, bytes);
                    continue;
                }
                images.take_pending(key);
                let start: Instant = Instant::now();
                let Some((width, height, rgba)) = decode(images.name(key), bytes) else { continue };
                images.upload_image(renderer, device, queue, key, &rgba, width, height);
                log::info!("TURBODOT_PICTURE {} {width}x{height} decoded in {} ms", images.name(key), start.elapsed().as_millis());
                uploaded += 1;
            }
            uploaded
        }

        /// Frees the GPU copy of every `release=unused` picture no live
        /// page names. `mark_live` marks, by key index, the pictures live
        /// pages name (`PageStack::mark_pictures`).
        ///
        /// Call it after a page was freed, not every frame: `mark_live`
        /// reads every live page's pictures.
        ///
        /// Returns how many pictures it freed.
        pub fn release_unshown(&mut self, images: &mut ImageTable, renderer: &mut UiRenderer, device: &wgpu::Device, mark_live: impl FnOnce(&mut [bool])) -> usize {
            self.live.clear();
            self.live.resize(images.len(), false);
            mark_live(&mut self.live);
            self.freed.clear();
            images.release_unmarked(renderer, device, &self.live, &mut self.freed);
            let mut bytes: u64 = 0;
            for key in &self.freed {
                // Its next show sends it to the thread again.
                self.lazy_decoder.forget(*key);
                bytes += images.size(*key).map_or(0, |(width, height)| width as u64 * height as u64 * 4);
            }
            if !self.freed.is_empty() {
                log::info!("TURBODOT_PICTURE freed {} pictures no page shows ({:.1} MB)", self.freed.len(), bytes as f64 / (1024.0 * 1024.0));
            }
            self.freed.len()
        }

        /// True while pictures are out on the decode thread.
        ///
        /// Needed by a loop that sleeps while nothing moves: it must keep
        /// drawing frames until they land, or they show only at the next
        /// touch.
        pub fn is_decoding(&self) -> bool {
            self.lazy_decoder.is_decoding()
        }
    }

    /// One picture to decode: its key and its file bytes.
    struct Request {
        key: ImageKey,
        bytes: &'static [u8],
        sent_at: Instant,
    }

    /// One picture back from the thread. `pixels` is `None` for a file
    /// that did not decode.
    struct Landed {
        key: ImageKey,
        pixels: Option<Decoded>,
        /// How long the decode itself took on the thread.
        decode_ms: u128,
        /// When the frame asked for it.
        sent_at: Instant,
    }

    /// Decode thread for `loading=lazy` pictures, with its two channels.
    /// First request starts the thread.
    ///
    /// Needed because a page can name many lazy pictures: decoding them
    /// all in the frame that first drew the page froze a slow phone for
    /// most of that frame.
    #[derive(Default)]
    struct LazyDecoder {
        worker: Option<(Sender<Request>, Receiver<Landed>)>,
        /// By key index: sent already (on the thread or landed), so a
        /// picture named every frame is sent once.
        is_requested: Vec<bool>,
        in_flight: usize,
    }

    impl LazyDecoder {
        /// Sends `key` to the thread, unless it went already.
        fn request(&mut self, key: ImageKey, bytes: &'static [u8]) {
            let index: usize = key.index();
            if self.is_requested.get(index).copied().unwrap_or(false) {
                return;
            }
            if self.is_requested.len() <= index {
                self.is_requested.resize(index + 1, false);
            }
            self.is_requested[index] = true;
            let (sender, _) = self.worker.get_or_insert_with(spawn_worker);
            if sender.send(Request { key, bytes, sent_at: Instant::now() }).is_ok() {
                self.in_flight += 1;
            }
        }

        /// Lets `key` be sent again: its picture was freed after it
        /// landed, and its next show decodes it anew.
        fn forget(&mut self, key: ImageKey) {
            if let Some(is_requested) = self.is_requested.get_mut(key.index()) {
                *is_requested = false;
            }
        }

        /// Every picture decoded since last call. Never waits.
        fn take_decoded(&mut self) -> Vec<Landed> {
            let Some((_, receiver)) = &self.worker else { return Vec::new() };
            let landed: Vec<Landed> = receiver.try_iter().collect();
            self.in_flight -= landed.len();
            landed
        }

        fn is_decoding(&self) -> bool {
            self.in_flight > 0
        }
    }

    fn spawn_worker() -> (Sender<Request>, Receiver<Landed>) {
        let (request_sender, request_receiver) = mpsc::channel::<Request>();
        let (landed_sender, landed_receiver) = mpsc::channel::<Landed>();
        std::thread::Builder::new()
            .name("turbodot-lazy-pictures".to_string())
            .spawn(move || {
                for request in request_receiver {
                    let start: Instant = Instant::now();
                    let pixels: Option<Decoded> = decode("a lazy picture", request.bytes);
                    let landed: Landed = Landed { key: request.key, pixels, decode_ms: start.elapsed().as_millis(), sent_at: request.sent_at };
                    if landed_sender.send(landed).is_err() {
                        return; // the program is gone
                    }
                }
            })
            .expect("start the thread that decodes lazy pictures");
        (request_sender, landed_receiver)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// A 2x2 png, encoded here.
        fn png_bytes() -> &'static [u8] {
            let mut bytes: Vec<u8> = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().expect("a png header");
                writer.write_image_data(&[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 9]).expect("the pixels");
            }
            Box::leak(bytes.into_boxed_slice())
        }

        /// Waits (test only) until `count` pictures have landed, 5 s cap.
        fn take_all(decoder: &mut LazyDecoder, count: usize) -> Vec<Landed> {
            let deadline: Instant = Instant::now() + std::time::Duration::from_secs(5);
            let mut all: Vec<Landed> = Vec::new();
            while all.len() < count && Instant::now() < deadline {
                all.extend(decoder.take_decoded());
                std::thread::yield_now();
            }
            all
        }

        /// Picture named every frame is sent once, comes back decoded in
        /// the order asked, and `is_decoding` stays true until it is
        /// taken: what a loop that sleeps reads.
        #[test]
        fn a_lazy_picture_is_sent_once_and_decoding_lasts_until_it_lands() {
            let mut decoder: LazyDecoder = LazyDecoder::default();
            assert!(!decoder.is_decoding(), "nothing sent yet");
            assert!(decoder.take_decoded().is_empty(), "no thread before the first request");
            let (first, second): (ImageKey, ImageKey) = (ImageKey::from_index(3), ImageKey::from_index(7));
            let file: &'static [u8] = png_bytes();
            for _frame in 0..3 {
                decoder.request(first, file);
                decoder.request(second, file);
            }
            assert!(decoder.is_decoding(), "two pictures on the thread");
            let landed: Vec<Landed> = take_all(&mut decoder, 2);
            assert_eq!(landed.len(), 2, "each picture decoded once, however many frames named it");
            assert_eq!((landed[0].key, landed[1].key), (first, second), "the order they were asked in");
            let (width, height, rgba) = landed[1].pixels.as_ref().expect("the png decodes");
            assert_eq!((*width, *height, &rgba[..8]), (2, 2, &[255, 0, 0, 255, 0, 255, 0, 255][..]));
            assert!(!decoder.is_decoding(), "both taken");
            std::thread::sleep(std::time::Duration::from_millis(20));
            assert!(decoder.take_decoded().is_empty(), "nothing sent twice");

            // Freed picture is sent again at its next show.
            decoder.forget(first);
            decoder.request(first, file);
            assert_eq!(take_all(&mut decoder, 1).len(), 1);
        }

        /// Lazy picture a frame names goes to the thread and is uploaded by
        /// a later frame; freed while no page names it, its next show
        /// decodes it again; a frame that asks for everything now decodes
        /// it at once.
        ///
        /// Ignored unless asked for: needs a GPU adapter.
        #[test]
        #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
        fn a_lazy_picture_is_uploaded_by_a_later_frame_and_again_after_it_was_freed() {
            let gpu: crate::gpu::Gpu = crate::gpu::Gpu::new().expect("this test needs a GPU adapter");
            let mut renderer: UiRenderer = UiRenderer::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
            let keys = gogo_ui::Images::from_names(["res://assets/late.png"]);
            let key: ImageKey = keys.get("res://assets/late.png").expect("the key");
            let mut images: ImageTable = ImageTable::new(keys, vec![Some(png_bytes())], vec![true], vec![Some((2, 2))]);
            images.set_release_unused(&[key]);
            let mut loader: PictureLoader = PictureLoader::default();
            let frame = |loader: &mut PictureLoader, images: &mut ImageTable, renderer: &mut UiRenderer, lazy: LazyPictures| -> usize { loader.upload_for_draws(images, renderer, &gpu.device, &gpu.queue, lazy, |wanted| wanted.push(key)) };
            let frames_until_uploaded = |loader: &mut PictureLoader, images: &mut ImageTable, renderer: &mut UiRenderer| {
                assert_eq!(frame(loader, images, renderer, LazyPictures::OnThread), 0, "the frame that first names it uploads nothing");
                assert!(loader.is_decoding() && images.pending_bytes(key).is_some(), "on the thread, still to decode");
                let deadline: Instant = Instant::now() + std::time::Duration::from_secs(5);
                while frame(loader, images, renderer, LazyPictures::OnThread) == 0 {
                    assert!(Instant::now() < deadline, "the picture never landed");
                    std::thread::yield_now();
                }
                assert!(!loader.is_decoding() && images.pending_bytes(key).is_none(), "uploaded");
            };
            frames_until_uploaded(&mut loader, &mut images, &mut renderer);
            assert_eq!(frame(&mut loader, &mut images, &mut renderer, LazyPictures::OnThread), 0, "uploaded once");

            assert_eq!(loader.release_unshown(&mut images, &mut renderer, &gpu.device, |live| live[key.index()] = true), 0, "a live page names it");
            assert_eq!(loader.release_unshown(&mut images, &mut renderer, &gpu.device, |_| {}), 1, "no live page names it");
            assert!(images.pending_bytes(key).is_some(), "to decode again");
            frames_until_uploaded(&mut loader, &mut images, &mut renderer);

            images.forget_uploads();
            assert_eq!(frame(&mut loader, &mut images, &mut renderer, LazyPictures::Now), 1, "decoded in the frame that asks for everything now");
        }

        /// File that does not decode still lands, with no pixels, so a
        /// loop is not kept awake for it for good.
        #[test]
        fn a_picture_that_does_not_decode_still_lands() {
            let mut decoder: LazyDecoder = LazyDecoder::default();
            decoder.request(ImageKey::from_index(0), b"no picture");
            let landed: Vec<Landed> = take_all(&mut decoder, 1);
            assert_eq!(landed.len(), 1);
            assert!(landed[0].pixels.is_none());
            assert!(!decoder.is_decoding());
        }
    }
}
