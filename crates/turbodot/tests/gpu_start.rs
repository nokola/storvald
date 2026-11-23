//! Vulkan drivers known bad: the list names the phones it exists for and
//! nobody else, and never a phone on GL.
#![cfg(feature = "gpu")]

use turbodot::gpu_start::{GpuDriver, KnownBad, VulkanNumbers, dequeues_inside_make_current, known_bad_driver};
use turbodot::wgpu;

const IMG_TEC: u32 = 0x1010;
const QUALCOMM: u32 = 0x5143;
const ARM: u32 = 0x13B5;
const NVIDIA: u32 = 0x10DE;
const HUAWEI: u32 = 0x19E5;
const SAMSUNG: u32 = 0x144D;

/// A Vulkan GPU as a driver reports it; the Vulkan version is 1.1 and the
/// driver version 0 unless a test says otherwise.
fn vulkan(vendor: u32, name: &'static str) -> GpuDriver<'static> {
    GpuDriver { name, driver: "", driver_info: "", vendor, device: 0, vulkan: Some(VulkanNumbers { api_version: (1, 1), driver_version: 0 }), backend: wgpu::Backend::Vulkan }
}

#[test]
fn the_known_bad_list_names_the_powervr_phones_before_cxt() {
    // Nokia 3.1 Plus, in its driver's own words: noise on screen, every
    // off-screen draw right.
    let nokia_3_1_plus: GpuDriver = GpuDriver { driver: "PowerVR Rogue Vulkan Driver", driver_info: "1.11@5425693", ..vulkan(IMG_TEC, "PowerVR Rogue GE8320") };
    assert_eq!(known_bad_driver(&nokia_3_1_plus), Some(KnownBad::PowerVrBeforeCxt), "the Nokia 3.1 Plus must start on GL");
    // Nokia C32: its driver crashed at every launch.
    assert_eq!(known_bad_driver(&vulkan(IMG_TEC, "PowerVR Rogue GE8322")), Some(KnownBad::PowerVrBeforeCxt), "the Nokia C32 must start on GL");
    // Whatever the driver version: the rule reads the series, as Flutter's does.
    assert_eq!(known_bad_driver(&GpuDriver { driver_info: "1.13@5776728", ..vulkan(IMG_TEC, "PowerVR Rogue GE8320") }), Some(KnownBad::PowerVrBeforeCxt));
    for name in ["PowerVR B-Series BXE-4-32", "PowerVR B-Series BXM-8-256", "PowerVR B-Series BXS-32-1024", "PowerVR B-Series BXT-32-1024", "some PowerVR nobody has named yet"] {
        assert_eq!(known_bad_driver(&vulkan(IMG_TEC, name)), Some(KnownBad::PowerVrBeforeCxt), "{name}");
    }
    for name in ["PowerVR C-Series CXT-48-1536", "PowerVR D-Series DXT-48-1536"] {
        assert_eq!(known_bad_driver(&vulkan(IMG_TEC, name)), None, "{name}: CXT is the first series with good Vulkan drivers");
    }
    // GL is where a listed phone is sent: it must never be listed there.
    assert_eq!(known_bad_driver(&GpuDriver { backend: wgpu::Backend::Gl, vulkan: None, ..nokia_3_1_plus }), None, "the same phone on GL is fine");
    // The rule reads the maker's id, not the name alone.
    assert_eq!(known_bad_driver(&vulkan(NVIDIA, "PowerVR Rogue GE8320")), None);
}

/// A GL GPU as wgpu's GL backend reports it: no Vulkan numbers, and the
/// maker id only when wgpu knows the vendor string.
fn gl(vendor: u32, name: &'static str) -> GpuDriver<'static> {
    GpuDriver { name, driver: "", driver_info: "", vendor, device: 0, vulkan: None, backend: wgpu::Backend::Gl }
}

#[test]
fn powervr_on_gl_dequeues_inside_make_current() {
    // Galaxy A04e ("isn't responding"): on GL wgpu leaves Imagination's
    // maker id at 0, so the name alone must match.
    assert!(dequeues_inside_make_current(&gl(0, "PowerVR Rogue GE8320")), "the Galaxy A04e");
    assert!(dequeues_inside_make_current(&gl(IMG_TEC, "PowerVR Rogue GE8320")));
    assert!(dequeues_inside_make_current(&gl(IMG_TEC, "some PowerVR nobody has named yet")));
    // Mali and Adreno take the buffer in eglSwapBuffers, outside the lock.
    for gpu in [gl(ARM, "Mali-G52 MC2"), gl(QUALCOMM, "Adreno (TM) 330"), gl(0, "Adreno (TM) 610")] {
        assert!(!dequeues_inside_make_current(&gpu), "{}: takes the buffer in eglSwapBuffers", gpu.name);
    }
    // Vulkan has no EGL.
    assert!(!dequeues_inside_make_current(&vulkan(IMG_TEC, "PowerVR Rogue GE8320")));
}

#[test]
fn the_known_bad_list_names_an_old_pixel_10_driver() {
    const PIXEL_10: u32 = 0x71061212;
    let pixel = |driver_version: u32| GpuDriver { device: PIXEL_10, vulkan: Some(VulkanNumbers { api_version: (1, 3), driver_version }), ..vulkan(IMG_TEC, "PowerVR D-Series DXT-48-1536") };
    assert_eq!(known_bad_driver(&pixel(6794073)), Some(KnownBad::Pixel10OldDriver));
    assert_eq!(known_bad_driver(&pixel(6794074)), None, "25.1@6794074 is the first good driver");
    assert_eq!(known_bad_driver(&GpuDriver { vulkan: None, ..pixel(0) }), None, "a driver version nobody can read keeps Vulkan");
    assert_eq!(known_bad_driver(&GpuDriver { device: 1, ..pixel(1) }), None, "another DXT phone is not the Pixel 10");
}

#[test]
fn the_known_bad_list_names_adreno_up_to_650() {
    for name in ["Adreno (TM) 504", "Adreno (TM) 540", "Adreno (TM) 610", "Adreno (TM) 619L", "Adreno (TM) 642L", "Adreno (TM) 650"] {
        assert_eq!(known_bad_driver(&vulkan(QUALCOMM, name)), Some(KnownBad::AdrenoUpTo650), "{name}");
    }
    // Galaxy S24 (Adreno 750) and every model after the 650 keep Vulkan, and
    // so does a model nobody has listed: it is likely newer.
    for name in ["Adreno (TM) 660", "Adreno (TM) 702", "Adreno (TM) 750", "Adreno (TM) 830", "Adreno (TM) 6500", "Qualcomm GPU"] {
        assert_eq!(known_bad_driver(&vulkan(QUALCOMM, name)), None, "{name}");
    }
    for vendor in [0x168C, 0x17CB, 0x1969, 0x5143] {
        assert_eq!(known_bad_driver(&vulkan(vendor, "Adreno (TM) 650")), Some(KnownBad::AdrenoUpTo650), "Qualcomm id {vendor:#06X}");
    }
    assert_eq!(known_bad_driver(&vulkan(ARM, "Adreno (TM) 650")), None, "the rule reads the maker's id, not the name alone");
    assert_eq!(known_bad_driver(&GpuDriver { backend: wgpu::Backend::Gl, vulkan: None, ..vulkan(QUALCOMM, "Adreno (TM) 650") }), None, "never on GL");
}

#[test]
fn the_known_bad_list_names_huawei_and_the_first_xclipse() {
    assert_eq!(known_bad_driver(&vulkan(HUAWEI, "Maleoon 910")), Some(KnownBad::Huawei));
    let xclipse = |api_version: (u32, u32)| GpuDriver { vulkan: Some(VulkanNumbers { api_version, driver_version: 0 }), ..vulkan(SAMSUNG, "Samsung Xclipse 920") };
    assert_eq!(known_bad_driver(&xclipse((1, 1))), Some(KnownBad::SamsungFirstXclipse));
    assert_eq!(known_bad_driver(&xclipse((1, 2))), Some(KnownBad::SamsungFirstXclipse));
    assert_eq!(known_bad_driver(&xclipse((1, 3))), None, "a later Xclipse reports Vulkan 1.3");
    assert_eq!(known_bad_driver(&xclipse((2, 0))), None);
    assert_eq!(known_bad_driver(&GpuDriver { vulkan: None, ..xclipse((1, 1)) }), None, "a Vulkan version nobody can read keeps Vulkan");
}

#[test]
fn the_known_bad_list_names_nobody_else() {
    for gpu in [vulkan(ARM, "Mali-G57"), vulkan(ARM, "Mali-T830"), vulkan(NVIDIA, "NVIDIA GeForce RTX 4070"), vulkan(0x8086, "Intel(R) UHD Graphics"), vulkan(0, "")] {
        assert_eq!(known_bad_driver(&gpu), None, "{} is not on the list", gpu.name);
    }
    for bad in [KnownBad::Pixel10OldDriver, KnownBad::AdrenoUpTo650, KnownBad::Huawei, KnownBad::SamsungFirstXclipse, KnownBad::PowerVrBeforeCxt] {
        assert!(!bad.why().is_empty(), "{bad:?}: the log line says why");
    }
}
