//! Opens the rendered view up past the packaged aspect ratio.

use crate::utils::{ModuleInfo, patch, pattern_scan};
use std::sync::{Once, OnceLock};
static ADDRESSES: OnceLock<(usize, usize)> = OnceLock::new();
static APPLIED: Once = Once::new();

/// Stops the engine constraining the rendered view to the packaged aspect
/// ratio, and replaces the axis constraint the engine would otherwise apply.
///
/// Resolve both code sites without writing. Application waits for the first
/// engine callback after hook installation and SDK layout validation.
pub(super) fn prepare(module: &ModuleInfo) -> bool {
    // The match ends at AND ECX,1 reading the packaged ConstrainAspectRatio
    // flag; its immediate at +15 becomes 0, so the flag never survives the
    // mask and the constrained path is not taken.
    const ASPECT: &str = "89 47 5C 0F B6 8B 69 02 00 00 33 4F 68 83 E1 01 33 4F 68 89 4F 68";

    // At +6, the eight bytes of MOVZX EDX,byte ptr [R15+0xB8] -- the load of
    // AspectRatioAxisConstraint -- become MOV EDX,0 plus three NOPs, which is
    // the maintain-Y constraint: a screen wider than 16:9 shows more to the
    // sides instead of cropping the top and bottom.
    const AXIS: &str =
        "0F 85 ?? ?? ?? ?? 41 0F B6 97 B8 00 00 00 48 8D 8D 70 03 00 00 4C 8B CE 4D 8B C4";
    // Resolve both before changing either: a partial aspect correction can crop.
    let (Some(aspect), Some(axis)) = (
        pattern_scan(module.address, ASPECT),
        pattern_scan(module.address, AXIS),
    ) else {
        log::error!("Aspect correction: required unique signatures not found; skipped");
        return false;
    };
    let _ = ADDRESSES.set((aspect + 15, axis + 6));
    log::info!(
        "Aspect correction prepared: unconstrained @ +{:X}; maintain-Y (Hor+) @ +{:X}",
        aspect + 15 - module.address.0 as usize,
        axis + 6 - module.address.0 as usize
    );
    true
}

pub(super) fn apply_prepared() {
    APPLIED.call_once(|| {
        if let Some(&(aspect, axis)) = ADDRESSES.get() {
            if !patch(axis, "BA 00 00 00 00 90 90 90") || !patch(aspect, "00") {
                let aspect_restored = patch(aspect, "01");
                let axis_restored = patch(axis, "41 0F B6 97 B8 00 00 00");
                crate::runtime::reject_install();
                log::error!("Aspect writes failed; restoration: aspect={aspect_restored}, axis={axis_restored}");
                return;
            }
            log::info!(
                "Aspect correction applied after SDK validation: unconstrained, maintain-Y (Hor+)"
            );
        }
    });
}
