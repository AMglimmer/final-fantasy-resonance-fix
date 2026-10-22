//! Resolve the SDK entry points from code shared by the demo and retail builds.
//! No demo RVA is used as a retail fallback. See RETAIL_COMPATIBILITY.md.
use crate::{
    bridge::ffi,
    utils::{ModuleInfo, pattern_scan},
};
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALLED: AtomicBool = AtomicBool::new(false);
static FAILED: AtomicBool = AtomicBool::new(false);
pub fn reject_install() {
    FAILED.store(true, Ordering::Release);
    INSTALLED.store(false, Ordering::Release);
}
pub fn finish_install() -> bool {
    let ready = !FAILED.load(Ordering::Acquire);
    INSTALLED.store(ready, Ordering::Release);
    ready
}
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn initialize(module: &ModuleInfo) -> bool {
    // Bound support to the two locally inspected builds. Matching signatures
    // alone do not prove that a future build retains the reflected layouts.
    let base = module.address.0 as usize;
    let nt = unsafe { base + ((base + 0x3c) as *const u32).read_unaligned() as usize };
    let timestamp = unsafe { ((nt + 8) as *const u32).read_unaligned() };
    let image_size = unsafe { ((nt + 24 + 56) as *const u32).read_unaligned() };
    let profile = match (timestamp, image_size) {
        (0x6a469dfa, 0x1da1b000) => "Steam demo build 24032336",
        (0x6a586b2f, 0x1dfb6000) => "Steam retail build 24250053",
        _ => {
            log::error!(
                "Unsupported executable: timestamp={timestamp:X}, SizeOfImage={image_size:X}; no fixes installed"
            );
            return false;
        }
    };
    log::info!("SDK profile: {profile}");
    let append = pattern_scan(
        module.address,
        "48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 20 80 3D ?? ?? ?? ?? 00 48 8B F2 8B 19 48 8B F9 74 ?? 4C 8D 05 ?? ?? ?? ?? EB ?? 48 8D 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? 4C 8B C0 C6 05 ?? ?? ?? ?? 01 8B CB 0F B7 C3 C1 E9 10",
    );
    let event = pattern_scan(
        module.address,
        "40 55 56 57 41 54 41 55 41 56 41 57 48 81 EC 00 01 00 00 48 8D 6C 24 30 48 89 9D 28 01 00 00 48 8B 05 ?? ?? ?? ?? 48 33 C5 48 89 85 C0 00 00 00 8B 41 08 4D 8B F0 C1 E8 1E 48 8B FA F6 D0 4C 8B F9 A8 01",
    );
    let refs = [
        "48 8B 05 ?? ?? ?? ?? 48 8B 0C C8 48 8D 1C D1 0F 1F 80 00 00 00 00 8B 43 08 0F BA E0 15 72 ?? 8B C8 0F BA E9 15 F0 0F B1 4B 08 75 ?? 48 8B 5C 24 30 B0 01 48 83 C4 20",
        "48 8B 05 ?? ?? ?? ?? 48 8B 0C C8 48 8D 04 D1 EB ?? 49 8B C7 8B 40 08 C1 E8 19 A8 01 0F 85 ?? ?? ?? ?? 41 8B 40 0C 3B 05 ?? ?? ?? ?? 7D ?? 8B C8 0F B7 C0 48 C1 E9 10",
    ];
    let globals: Option<Vec<usize>> = refs
        .iter()
        .map(|signature| {
            let at = pattern_scan(module.address, signature)?;
            let displacement = unsafe { ((at + 3) as *const i32).read_unaligned() };
            Some((at + 7).wrapping_add_signed(displacement as isize))
        })
        .collect();
    let (Some(append), Some(event), Some(globals)) = (append, event, globals) else {
        log::error!("SDK: required unique signatures missing; no fixes installed");
        return false;
    };
    if globals[0] != globals[1] {
        log::error!("SDK: GObjects references disagree; no fixes installed");
        return false;
    }
    let objects = globals[0];
    log::info!(
        "SDK resolved: GObjects=+{:X}, AppendString=+{:X}, ProcessEvent=+{:X}",
        objects - base,
        append - base,
        event - base
    );
    unsafe { ffi::initialize_sdk(objects, append, event) }
}
