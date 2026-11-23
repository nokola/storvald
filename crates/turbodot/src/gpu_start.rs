//! GPU start: which wgpu backends to ask for, which Vulkan drivers not to
//! trust, and the file a phone keeps its answer in.
//!
//! Needed because a bad Vulkan driver reports no error: program draws a
//! full frame and phone shows random pixels, or driver crashes program at
//! every launch. Such a phone draws right on OpenGL.
//!
//! - [`instance`]: wgpu instance on named backends.
//! - [`GpuDriver`], [`known_bad_driver`]: Vulkan drivers known bad, told
//!   before the Vulkan device exists.
//! - [`remembered`], [`remember`]: answer of a driver check, kept between
//!   launches.
//! - [`adapter_line`], [`capability_line`], [`backend_name`]: GPU in
//!   words, for a log and for a user.

use std::path::{Path, PathBuf};

/// Whether wgpu's GL backend makes its debug calls: names of passes,
/// buffers, textures and shaders handed to the driver.
///
/// Needed because one GL driver crashed while copying such a name (Samsung
/// Galaxy S9, Adreno 630 driver v57, Android 10): it reads the name until
/// a zero byte, past the length it was given. Names show only in a GPU
/// capture tool, so a store build turns them off. Same GPU job counts
/// with them off (measured).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlDebugCalls {
    On,
    Off,
}

/// wgpu instance on `backends`. `None`: backend named by `WGPU_BACKEND`
/// (`WGPU_BACKEND=gl cargo test` runs a GPU suite on wgpu's GL backend on a
/// PC), else every backend built in.
///
/// Android GL, debug build: asks for a debug context first, and once more
/// without when GL then lists no adapter. Needed because one EGL 1.4
/// driver lists `KHR_create_context` yet answers the debug bit with
/// `BAD_ATTRIBUTE` (OnePlus A0001, Adreno 330); wgpu-hal retries only over
/// robustness levels, so the whole GL backend comes up empty and program
/// would show nothing. Release build never asks. Validation stays on.
pub fn instance(backends: Option<wgpu::Backends>, gl_debug_calls: GlDebugCalls) -> wgpu::Instance {
    let backends: Option<wgpu::Backends> = backends.or_else(wgpu::Backends::from_env);
    let describe = |flags: Option<wgpu::InstanceFlags>| {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        if let Some(b) = backends {
            desc.backends = b;
        }
        if let Some(f) = flags {
            desc.flags = f;
        }
        if gl_debug_calls == GlDebugCalls::Off {
            desc.backend_options.gl.debug_fns = wgpu::GlDebugFns::Disabled;
        }
        desc
    };
    let first = describe(None);
    let (wants_gl, flags) = (first.backends.contains(wgpu::Backends::GL), first.flags);
    let instance = wgpu::Instance::new(first);
    if cfg!(target_os = "android") && wants_gl && flags.contains(wgpu::InstanceFlags::DEBUG) && pollster::block_on(instance.enumerate_adapters(wgpu::Backends::GL)).is_empty() {
        log::warn!("TURBODOT_GPU GL came up empty with a debug context: once more without InstanceFlags::DEBUG");
        return wgpu::Instance::new(describe(Some(flags - wgpu::InstanceFlags::DEBUG)));
    }
    instance
}

/// Adapter's downlevel picture in one log line: downlevel flags NOT set,
/// shader model, and limits a renderer's choices turn on.
///
/// Needed so a gap on an old GPU class names itself in the start log, in
/// place of a pipeline that never builds and says nothing.
pub fn capability_line(adapter: &wgpu::Adapter) -> String {
    let caps: wgpu::DownlevelCapabilities = adapter.get_downlevel_capabilities();
    let limits: wgpu::Limits = adapter.limits();
    let missing: wgpu::DownlevelFlags = wgpu::DownlevelFlags::all() - caps.flags;
    format!(
        "downlevel missing {:?}; shader model {:?}; vertex storage buffers {}; max uniform buffer binding {} B; max immediates {} B; features {:?}",
        missing, caps.shader_model, limits.max_storage_buffers_per_shader_stage, limits.max_uniform_buffer_binding_size, limits.max_immediate_size, adapter.features()
    )
}

/// Backend's name as a user reads it ("Vulkan", "GL").
///
/// Needed because wgpu's own spelling ("Gl") is not how anyone says it.
pub fn backend_name(backend: wgpu::Backend) -> &'static str {
    match backend {
        wgpu::Backend::Vulkan => "Vulkan",
        wgpu::Backend::Gl => "GL",
        wgpu::Backend::Metal => "Metal",
        wgpu::Backend::Dx12 => "DirectX 12",
        wgpu::Backend::BrowserWebGpu => "WebGPU",
        wgpu::Backend::Noop => "none",
    }
}

/// One line naming GPU, backend and driver. [`remembered`] compares it:
/// a driver update gives another line.
pub fn adapter_line(adapter: &wgpu::Adapter) -> String {
    let info = adapter.get_info();
    format!("{} / {} / {:?} / driver {} {}", info.name, info.vendor, info.backend, info.driver, info.driver_info.lines().next().unwrap_or(""))
}

/// What a rule may ask of a GPU: what its driver says about itself, and
/// which backend program is on.
///
/// Own type (not `wgpu::AdapterInfo`) so a rule is tested with no GPU.
/// Borrows adapter's strings: making one allocates nothing. Rules match
/// words in the case drivers write them.
#[derive(Clone, Copy, Debug)]
pub struct GpuDriver<'a> {
    /// GPU name, e.g. "PowerVR Rogue GE8320", "Adreno (TM) 750".
    pub name: &'a str,
    /// Driver name, e.g. "PowerVR Rogue Vulkan Driver".
    pub driver: &'a str,
    /// Driver version words, e.g. "1.11@5425693".
    pub driver_info: &'a str,
    /// PCI vendor id of GPU maker, e.g. 0x1010 for Imagination.
    pub vendor: u32,
    /// Maker's own id for a GPU model.
    pub device: u32,
    /// `None` off Vulkan, or when driver's own numbers cannot be read: a
    /// rule that needs them then does NOT match (that phone keeps Vulkan).
    pub vulkan: Option<VulkanNumbers>,
    pub backend: wgpu::Backend,
}

/// Two numbers only the Vulkan driver itself reports
/// (`VkPhysicalDeviceProperties`); `wgpu::AdapterInfo` carries neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VulkanNumbers {
    /// Vulkan version driver implements, (major, minor).
    pub api_version: (u32, u32),
    /// `driverVersion`, packed GPU maker's own way: comparable only within
    /// one maker.
    pub driver_version: u32,
}

impl<'a> GpuDriver<'a> {
    /// `info`: `adapter.get_info()`, kept by caller so words can be
    /// borrowed.
    pub fn from_adapter(adapter: &wgpu::Adapter, info: &'a wgpu::AdapterInfo) -> GpuDriver<'a> {
        GpuDriver { name: &info.name, driver: &info.driver, driver_info: &info.driver_info, vendor: info.vendor, device: info.device, vulkan: vulkan_numbers(adapter, info.backend), backend: info.backend }
    }
}

#[cfg(any(windows, target_os = "android", target_os = "linux"))]
fn vulkan_numbers(adapter: &wgpu::Adapter, backend: wgpu::Backend) -> Option<VulkanNumbers> {
    if backend != wgpu::Backend::Vulkan {
        return None;
    }
    // SAFETY: reads a copy of the properties wgpu already fetched; nothing
    // of the adapter is destroyed or kept.
    let raw = unsafe { adapter.as_hal::<wgpu::hal::api::Vulkan>() }?;
    let properties = raw.physical_device_capabilities().properties();
    let api: u32 = properties.api_version;
    // VK_API_VERSION_MAJOR / VK_API_VERSION_MINOR.
    Some(VulkanNumbers { api_version: ((api >> 22) & 0x7F, (api >> 12) & 0x3FF), driver_version: properties.driver_version })
}

#[cfg(not(any(windows, target_os = "android", target_os = "linux")))]
fn vulkan_numbers(_adapter: &wgpu::Adapter, _backend: wgpu::Backend) -> Option<VulkanNumbers> {
    None
}

/// Why a Vulkan driver is on the known-bad list: one rule each of Flutter
/// Impeller's `DriverInfoVK::IsKnownBadDriver`
/// (impeller/renderer/backend/vulkan/driver_info_vk.cc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownBad {
    Pixel10OldDriver,
    AdrenoUpTo650,
    Huawei,
    SamsungFirstXclipse,
    PowerVrBeforeCxt,
}

impl KnownBad {
    /// For a log line and a report.
    pub fn why(self) -> &'static str {
        match self {
            KnownBad::Pixel10OldDriver => "a Pixel 10 PowerVR driver older than 25.1",
            KnownBad::AdrenoUpTo650 => "an Adreno 650 or older, where Flutter needs Vulkan workarounds (flutter/flutter#178285, #178498)",
            KnownBad::Huawei => "a Huawei Maleoon GPU (flutter/flutter#156623)",
            KnownBad::SamsungFirstXclipse => "a first Samsung Xclipse GPU, told by Vulkan older than 1.3 (flutter/flutter#161334)",
            KnownBad::PowerVrBeforeCxt => "a PowerVR older than the CXT series: random pixels on screen (Nokia 3.1 Plus) or a crash in the driver at launch (Nokia C32)",
        }
    }
}

/// GPU makers a rule names, by PCI vendor id (Flutter's `IdentifyVendor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Vendor {
    Qualcomm,
    ImgTec,
    Huawei,
    Samsung,
    Other,
}

impl Vendor {
    fn from_pci_id(vendor: u32) -> Vendor {
        match vendor {
            0x168C | 0x17CB | 0x1969 | 0x5143 => Vendor::Qualcomm,
            0x1010 => Vendor::ImgTec,
            0x19E5 => Vendor::Huawei,
            0x144D => Vendor::Samsung,
            _ => Vendor::Other,
        }
    }
}

/// Pixel 10's PowerVR, by device id (from ANGLE/Chromium, as Flutter has
/// it).
const PIXEL_10_DEVICE_ID: u32 = 0x71061212;

/// PowerVR 25.1@6794074: on this GPU `driverVersion` is driver's build
/// number.
const PIXEL_10_MIN_DRIVER_VERSION: u32 = 6794074;

/// An Adreno names itself "Adreno (TM) <model>".
const ADRENO_NAME_PREFIX: &str = "Adreno (TM) ";

/// Every Adreno model up to the 650, as Flutter lists them. Model not
/// here (newer, or unknown) keeps Vulkan.
const ADRENO_MODELS_UP_TO_650: &[&str] = &[
    "504", "505", "506", "508", "509", "510", "512", "530", "540", // 500s
    "605", "608", "610", "612", "613", "615", "616", "618", "619L", "619", "620", "630", "640", "642L", "642", "643L", "644", "650",
];

/// PowerVR series a name can be told by, and whether its Vulkan drivers
/// are good: CXT is the first good one. PowerVR named none of these (every
/// Rogue, e.g. "PowerVR Rogue GE8320") counts as older.
const POWERVR_SERIES: &[(&str, bool)] = &[("BXE", false), ("BXM", false), ("BXS", false), ("BXT", false), ("CXT", true), ("DXT", true)];

/// Whether this GPU's Vulkan driver is known bad: port of Flutter
/// Impeller's `DriverInfoVK::IsKnownBadDriver`, same rules in same order.
///
/// Needed because a bad driver shows a user random pixels or closes
/// program at every launch, and no test can run on it first: Nokia 3.1
/// Plus (PowerVR GE8320, driver 1.11@5425693, Android 10) draws right off
/// screen and noise on screen; Nokia C32 (GE8322, Android 13) crashes
/// inside `vkQueueSubmit` on first submit that draws. Phone on the list
/// starts on GL before the Vulkan device exists. Every other phone pays a
/// few integer and string compares, no allocation.
///
/// Vulkan only: GL is where such a phone is sent. Rule that needs
/// [`GpuDriver::vulkan`] does not match when it is `None`.
///
/// Returns which rule matched, or `None` for a driver to trust.
pub fn known_bad_driver(gpu: &GpuDriver) -> Option<KnownBad> {
    if gpu.backend != wgpu::Backend::Vulkan {
        return None;
    }
    let vendor: Vendor = Vendor::from_pci_id(gpu.vendor);
    if vendor == Vendor::ImgTec && gpu.device == PIXEL_10_DEVICE_ID && gpu.vulkan.is_some_and(|v| v.driver_version < PIXEL_10_MIN_DRIVER_VERSION) {
        return Some(KnownBad::Pixel10OldDriver);
    }
    if vendor == Vendor::Qualcomm && adreno_model(gpu.name).is_some_and(|model| ADRENO_MODELS_UP_TO_650.contains(&model)) {
        return Some(KnownBad::AdrenoUpTo650);
    }
    if vendor == Vendor::Huawei {
        return Some(KnownBad::Huawei);
    }
    if vendor == Vendor::Samsung {
        // Every Xclipse reports the same driver version, so Flutter tells
        // the first one by its Vulkan version.
        return gpu.vulkan.is_some_and(|v| v.api_version < (1, 3)).then_some(KnownBad::SamsungFirstXclipse);
    }
    if vendor == Vendor::ImgTec && !powervr_has_good_vulkan(gpu.name) {
        return Some(KnownBad::PowerVrBeforeCxt);
    }
    None
}

/// Words after "Adreno (TM) ", e.g. "650"; `None` for a name of another
/// shape.
fn adreno_model(name: &str) -> Option<&str> {
    let at: usize = name.find(ADRENO_NAME_PREFIX)?;
    Some(&name[at + ADRENO_NAME_PREFIX.len()..])
}

/// First series word of [`POWERVR_SERIES`] in the name decides, as in
/// Flutter's `GetPowerVRVersion`.
fn powervr_has_good_vulkan(name: &str) -> bool {
    POWERVR_SERIES.iter().find(|(series, _)| name.contains(series)).is_some_and(|(_, good)| *good)
}

/// Whether this GL driver takes window's next buffer inside
/// `eglMakeCurrent`: PowerVR does (`KEGLGetDrawableParameters` ->
/// `Surface::dequeueBuffer`); Mali and Adreno take it inside
/// `eglSwapBuffers`.
///
/// Needed because Android's own libEGL holds its display lock across
/// driver's `eglMakeCurrent`. When SurfaceFlinger is late releasing a
/// buffer, program's present waits inside that lock, and any other GL
/// user in the process (a WebView) waits for the lock behind it: phone
/// shows "isn't responding" (Galaxy A04e, PowerVR GE8320 on GL). Program
/// with a WebView has it drawn in software on such a driver.
///
/// On GL wgpu reads no maker id from Imagination's vendor string, so the
/// name decides too. Vulkan has no EGL: false there.
pub fn dequeues_inside_make_current(gpu: &GpuDriver) -> bool {
    gpu.backend == wgpu::Backend::Gl && (Vendor::from_pci_id(gpu.vendor) == Vendor::ImgTec || gpu.name.contains("PowerVR"))
}

/// Backend a phone draws with, as [`remember`] writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Vulkan,
    Gl,
}

impl Backend {
    fn word(self) -> &'static str {
        match self {
            Backend::Vulkan => "vulkan",
            Backend::Gl => "gl",
        }
    }
}

/// Name of the file [`remember`] writes, in program's data folder.
pub const BACKEND_FILE: &str = "gpu_backend.txt";

/// How a build that forces a failed driver check spells its reason in the
/// file.
///
/// Needed so a test build's forced failure never leaves the phone on GL
/// for the normal build installed next: such an answer is ignored, and
/// the check runs again.
pub const FORCED_PREFIX: &str = "--driver-probe-fail";

/// Path of [`BACKEND_FILE`] in `data_dir`.
pub fn backend_file(data_dir: &Path) -> PathBuf {
    data_dir.join(BACKEND_FILE)
}

/// Answer an earlier launch left for THIS driver and THIS program
/// version.
///
/// Needed because a driver check costs a GPU read, and a phone that
/// failed it must not pay a whole Vulkan start again at every launch:
/// answer is read before the Vulkan device exists.
///
/// `accept_forced`: whether an answer starting with [`FORCED_PREFIX`]
/// counts. Only a forced build says yes.
///
/// Returns `None` when there is no usable answer: no file, file not in
/// this shape, another adapter line (driver update, backup restored from
/// another phone), answer written by another program version, or forced
/// answer this build does not accept. Caller then checks again: the safe
/// way to be wrong.
pub fn remembered(file: &Path, adapter_line: &str, app_version: &str, accept_forced: bool) -> Option<Backend> {
    let r: Remembered = parse(&std::fs::read_to_string(file).ok()?)?;
    let usable: bool = r.adapter == adapter_line && r.app_version == app_version && (accept_forced || !r.why.starts_with(FORCED_PREFIX));
    usable.then_some(r.backend)
}

/// One answer as the file holds it.
#[derive(Clone, Debug)]
pub struct Remembered {
    pub backend: Backend,
    /// Adapter line ([`adapter_line`]) the answer was made for.
    pub adapter: String,
    /// Program version that wrote it, e.g. "42.0.0".
    pub app_version: String,
    /// Why: a comment for whoever reads the file on a phone.
    pub why: String,
}

/// Version line's word, e.g. `app 42.0.0`.
const APP_WORD: &str = "app";

/// Writes `backend` as the answer for this driver (`adapter`) and this
/// program version, with `why`. File that cannot be written: one log
/// line, and next launch checks again.
pub fn remember(file: &Path, backend: Backend, adapter: &str, app_version: &str, why: &str) {
    let text: String = format!("{}\n{}\n{APP_WORD} {app_version}\n{}\n", backend.word(), adapter.replace('\n', " "), why.replace('\n', " "));
    if let Err(e) = std::fs::write(file, text) {
        log::warn!("TURBODOT_GPU could not remember the GPU backend: {e}");
    }
}

/// Line 1: backend. Line 2: adapter it was decided for. Line 3: program
/// version that wrote it (`app <version>`). Line 4: reason. File without
/// the version line is outdated: no answer.
pub fn parse(text: &str) -> Option<Remembered> {
    let mut lines = text.lines();
    let backend: Backend = match lines.next()?.trim() {
        "vulkan" => Backend::Vulkan,
        "gl" => Backend::Gl,
        _ => return None,
    };
    let adapter: String = lines.next().unwrap_or("").trim().to_string();
    if adapter.is_empty() {
        return None;
    }
    let app_version: String = lines.next()?.trim().strip_prefix(APP_WORD)?.trim().to_string();
    if app_version.is_empty() {
        return None;
    }
    let why: String = lines.next().unwrap_or("").trim().to_string();
    Some(Remembered { backend, adapter, app_version, why })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADAPTER: &str = "PowerVR Rogue GE8320 / 4112 / Vulkan / driver PowerVR Rogue Vulkan Driver 1.11@5425693";
    const APP_VERSION: &str = "42.0.0";

    /// A fresh file per call: tests run in parallel, so two files must
    /// never share a name.
    fn file_with(text: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir: PathBuf = std::env::temp_dir().join(format!("turbodot_gpu_backend_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let file: PathBuf = dir.join(format!("{}.txt", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::write(&file, text).expect("write");
        file
    }

    #[test]
    fn what_was_written_reads_back() {
        let file: PathBuf = file_with("");
        remember(&file, Backend::Gl, ADAPTER, APP_VERSION, "sampled_texture: pixel (5, 3) read #FFFFFF");
        assert_eq!(remembered(&file, ADAPTER, APP_VERSION, false), Some(Backend::Gl));
        let text: String = std::fs::read_to_string(&file).expect("read");
        assert!(text.contains(&format!("\napp {APP_VERSION}\n")), "the file names the program version that wrote it: {text}");
    }

    #[test]
    fn another_driver_is_no_answer() {
        let file: PathBuf = file_with(&format!("gl\n{ADAPTER}\napp {APP_VERSION}\nthe probe failed\n"));
        assert_eq!(remembered(&file, "PowerVR Rogue GE8320 / 4112 / Vulkan / driver PowerVR Rogue Vulkan Driver 1.13@5776728", APP_VERSION, false), None, "a driver update checks again");
    }

    #[test]
    fn an_answer_from_another_program_version_is_no_answer() {
        // A file of three lines, with no version line.
        let no_version: PathBuf = file_with(&format!("vulkan\n{ADAPTER}\nthe driver probe passed\n"));
        assert_eq!(remembered(&no_version, ADAPTER, APP_VERSION, false), None, "an answer with no version is outdated: check again");
        let other: PathBuf = file_with(&format!("vulkan\n{ADAPTER}\napp 41.0.0\nthe driver probe passed\n"));
        assert_eq!(remembered(&other, ADAPTER, APP_VERSION, false), None, "another version's answer is outdated: check again");
    }

    #[test]
    fn a_forced_failure_counts_only_for_a_forced_build() {
        let file: PathBuf = file_with(&format!("gl\n{ADAPTER}\napp {APP_VERSION}\n{FORCED_PREFIX}: pretending this driver is broken\n"));
        assert_eq!(remembered(&file, ADAPTER, APP_VERSION, false), None, "a normal build installed next checks again");
        assert_eq!(remembered(&file, ADAPTER, APP_VERSION, true), Some(Backend::Gl), "the forced build itself shows the remembered switch");
    }

    #[test]
    fn a_file_in_another_shape_is_no_answer() {
        assert!(parse("").is_none());
        assert!(parse("maybe\nadapter\napp 42.0.0\n").is_none(), "an unknown backend word is no answer");
        assert!(parse("gl\n").is_none(), "a backend with no adapter is no answer: check again");
        assert!(parse("gl\n   \n").is_none());
        assert!(parse("gl\nadapter\napp \n").is_none(), "an empty version is no answer");
        assert_eq!(remembered(Path::new("does/not/exist.txt"), ADAPTER, APP_VERSION, false), None);
    }
}
