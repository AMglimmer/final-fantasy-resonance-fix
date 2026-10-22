use std::ffi::c_void;
use std::fs::File;
use std::path::Path;

use grapnel::{Context, MidHook};
use windows::Win32::Foundation::{HMODULE, SYSTEMTIME};
use windows::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
    PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS, PAGE_PROTECTION_FLAGS, VirtualProtect,
    VirtualQuery,
};
use windows::Win32::System::ProcessStatus::{GetModuleInformation, MODULEINFO};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::GetCurrentProcess;

/// Formats a broken-down time: `YYYY-MM-DDTHH:MM:SS.mmm`
fn format_systemtime(t: &SYSTEMTIME) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

/// Installs a `fern` dispatch as the process-wide [`log`] sink, writing every
/// level into a freshly truncated file at `path`. Each record is emitted as
/// `[YYYY-MM-DDTHH:MM:SS.mmm] [LEVEL] message`.
///
/// The timestamp comes from `GetLocalTime` rather than a UTC clock so a line
/// lines up with the wall clock of whoever is reading the log next to the game
/// they just ran.
///
/// Dependencies log here too -- `grapnel` reports a hook it could not install
/// -- so a line from anything but this crate is tagged with its target. To
/// silence one without touching it, add
/// `.level_for("grapnel", log::LevelFilter::Off)` below; the level is a
/// runtime choice, so nothing needs recompiling to change it.
///
/// Both failures here are swallowed, because neither should stop a fix from
/// applying: the file not opening, and `apply` rejecting a second logger for
/// the process. The `log` macros are no-ops while no logger is installed, so
/// call sites need no guard of their own.
pub fn init_logging(path: &str) {
    let Ok(file) = File::create(path) else { return };
    let _ = fern::Dispatch::new()
        .format(|out, message, record| {
            let t = unsafe { GetLocalTime() };
            // A line from another crate carries its target, so grapnel
            // reporting a hook it could not install is not mistaken for this
            // mod's own reporting. This crate's lines stay untagged.
            if record.target().starts_with(env!("CARGO_CRATE_NAME")) {
                out.finish(format_args!(
                    "[{}] [{}] {message}",
                    format_systemtime(&t),
                    record.level()
                ))
            } else {
                out.finish(format_args!(
                    "[{}] [{}] [{}] {message}",
                    format_systemtime(&t),
                    record.level(),
                    record.target()
                ))
            }
        })
        .level(log::LevelFilter::Debug)
        .chain(file)
        .apply();
}

/// Logs a correction's report, if it made one. The C++ side returns an empty
/// string when a correction found nothing to change, or when its report repeats
/// the last one under the same key, which keeps a callback that runs every
/// frame from writing a line every frame.
pub fn log_report(report: String) {
    if !report.is_empty() {
        log::info!("{report}");
    }
}

/// A scanned/patched module: its base address plus everything worth logging
/// about the file backing it.
pub struct ModuleInfo {
    /// Base virtual address.
    pub address: HMODULE,
    /// Name of the module.
    pub name: String,
    /// Path of the filesystem to the exe.
    pub path: String,
    /// On-disk file size in bytes.
    pub size: u64,
}

impl ModuleInfo {
    /// Builds a `ModuleInfo` from a module handle, resolving its file name,
    /// size, and modified time for logging.
    pub fn new(address: HMODULE) -> Self {
        let mut buf = [0u16; 260];
        let len = unsafe { GetModuleFileNameW(Some(address), &mut buf) } as usize;
        let path = String::from_utf16_lossy(&buf[..len]);
        let name = Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Self {
            address,
            name,
            path,
            size,
        }
    }
}

/// A mid-function hook keyed off a pattern scan.
pub struct SignatureHook {
    /// Name identifier for the hook.
    pub tag: &'static str,
    /// IDA-style byte string describing where the hook should be placed.
    pub signature: &'static str,
    /// Byte offset from the pattern match to the hook point.
    pub offset: usize,
}

/// Converts memory bytes into an IDA-style byte string, e.g. `[0x40, 0x63]` -> `"40 63"`.
pub fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parses an IDA-style byte pattern (e.g. `"48 8B 05 ?? ?? ?? ?? 48 85 C0"`) into a
/// sequence of optional bytes, where `None` is a wildcard.
fn parse_pattern(pattern: &str) -> Vec<Option<u8>> {
    pattern
        .split_whitespace()
        .map(|byte| {
            if byte == "??" {
                None
            } else {
                Some(u8::from_str_radix(byte, 16).expect("invalid byte in AOB pattern"))
            }
        })
        .collect()
}

/// Overwrites memory at `address` with `pattern` (an IDA-style byte string,
/// e.g. `"DE AD BE EF"`), temporarily marking the region writable, then
/// restoring its original protection.
pub fn patch(address: usize, pattern: &str) -> bool {
    let bytes: Vec<u8> = pattern
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("invalid byte in patch pattern"))
        .collect();

    unsafe {
        let mut old_protect = PAGE_PROTECTION_FLAGS(0);
        if VirtualProtect(
            address as *const c_void,
            bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old_protect,
        )
        .is_err()
        {
            log::error!("Patch: VirtualProtect failed @ {address:X}; write skipped");
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
        let flushed = FlushInstructionCache(
            GetCurrentProcess(),
            Some(address as *const c_void),
            bytes.len(),
        );
        if let Err(error) = &flushed {
            log::error!("Patch: instruction cache flush failed @ {address:X}: {error}");
        }
        let mut discard = PAGE_PROTECTION_FLAGS(0);
        let restored = VirtualProtect(
            address as *const c_void,
            bytes.len(),
            old_protect,
            &mut discard,
        );
        if let Err(error) = &restored {
            log::error!("Patch: protection restore failed @ {address:X}: {error}");
        }
        flushed.is_ok() && restored.is_ok()
    }
}

/// Returns the base address and size of `module`'s loaded image.
fn get_module_range(module: HMODULE) -> Option<(*const u8, usize)> {
    unsafe {
        let process = GetCurrentProcess();
        let mut mod_info = MODULEINFO::default();
        GetModuleInformation(
            process,
            module,
            &mut mod_info,
            std::mem::size_of::<MODULEINFO>() as u32,
        )
        .ok()?;
        Some((
            mod_info.lpBaseOfDll as *const u8,
            mod_info.SizeOfImage as usize,
        ))
    }
}

/// Slides a window over `data` looking for the first match against `pattern`
/// (`None` entries are wildcards), returning the offset of the match.
fn aob_scan(data: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }
    data.windows(pattern.len()).position(|window| {
        window
            .iter()
            .zip(pattern.iter())
            .all(|(byte, pat)| pat.is_none_or(|p| *byte == p))
    })
}

/// Scans `module`'s memory for `signature`, returning the absolute address of the
/// unique match, if any. Ambiguous matches are rejected.
pub fn pattern_scan(module: HMODULE, signature: &str) -> Option<usize> {
    let hits = pattern_scan_all(module, signature);
    if hits.len() != 1 {
        log::error!(
            "Signature must match once, found {}: {signature}",
            hits.len()
        );
        return None;
    }
    Some(hits[0])
}

/// Scans `module` for every occurrence of `signature`, returning all matching
/// absolute addresses.
///
/// Only committed, readable executable regions are searched.
pub fn pattern_scan_all(module: HMODULE, signature: &str) -> Vec<usize> {
    let Some((base, size)) = get_module_range(module) else {
        return Vec::new();
    };
    scan_code_range(base as usize, size, signature)
}

fn scan_code_range(base: usize, size: usize, signature: &str) -> Vec<usize> {
    let pattern = parse_pattern(signature);
    let mut hits = Vec::new();
    if pattern.is_empty() {
        return hits;
    }

    // A SizeOfImage-wide slice can span guard/no-access or uncommitted pages.
    // These signatures identify code, so scan only readable executable regions.
    let mut at = base;
    let end = at + size;
    while at < end {
        let mut memory = MEMORY_BASIC_INFORMATION::default();
        if unsafe {
            VirtualQuery(
                Some(at as *const c_void),
                &mut memory,
                std::mem::size_of_val(&memory),
            )
        } == 0
        {
            break;
        }
        let next = (memory.BaseAddress as usize + memory.RegionSize).min(end);
        if next <= at {
            break;
        }
        let executable = memory.Protect.0
            & (PAGE_EXECUTE_READ.0 | PAGE_EXECUTE_READWRITE.0 | PAGE_EXECUTE_WRITECOPY.0)
            != 0;
        if memory.State == MEM_COMMIT
            && executable
            && memory.Protect.0 & (PAGE_GUARD.0 | PAGE_NOACCESS.0) == 0
        {
            let data = unsafe { std::slice::from_raw_parts(at as *const u8, next - at) };
            let mut offset = 0;
            while let Some(found) = aob_scan(&data[offset..], &pattern) {
                hits.push(at + offset + found);
                offset += found + pattern.len();
            }
        }
        at = next;
    }
    hits
}

/// Installs a mid-function hook permanently at a fixed `offset` from
/// `module`'s base, bypassing signature scanning.
///
/// Useful for investigations that need to probe an address before it is worth
/// a signature at all. No shipped fix should call this: an absolute offset
/// moves whenever a game update shifts the code around it, so a confirmed hook
/// is converted to a signature before it ships. Kept, and deliberately not
/// removed as dead code, for the next investigation.
pub fn inject_hook_at<F>(module: &ModuleInfo, tag: &str, offset: usize, callback: F) -> bool
where
    F: FnMut(&mut Context) + Send + 'static,
{
    let hook_addr = (module.address.0 as usize + offset) as *mut u8;
    match MidHook::install(hook_addr, callback) {
        Ok(_hook) => {
            log::info!("{tag} : Hooked @ {}+{offset:x}", module.name);
            true
        }
        Err(e) => {
            log::error!(
                "{tag} : Failed to install hook @ {}+{offset:x}: {e}",
                module.name
            );
            false
        }
    }
}

/// Scans `module` for `sh.signature` and, if found, installs a mid-function
/// hook permanently for the duration of the main process at the address plus
/// `sh.offset` that jumps to the `callback`. Reports whether the hook went in.
pub fn inject_hook<F>(module: &ModuleInfo, sh: &SignatureHook, mut callback: F) -> bool
where
    F: FnMut(&mut Context) + Send + 'static,
{
    match pattern_scan(module.address, sh.signature) {
        Some(addr) => {
            let rel_addr = addr - module.address.0 as usize;
            log::info!(
                "{} : Found '{}' @ {}+{:x}",
                sh.tag,
                sh.signature,
                module.name,
                rel_addr
            );

            let hook_addr = (addr + sh.offset) as *mut u8;
            let mut last_status = u32::MAX;
            let tag = sh.tag;
            match MidHook::install(hook_addr, move |ctx| {
                if !crate::runtime::installed() {
                    return;
                }
                let status = crate::bridge::ffi::sdk_ready();
                if status != last_status {
                    log::info!(
                        "{tag}: SDK callback state {status} (0=waiting, 1=validated, 2=layout rejected)"
                    );
                    last_status = status;
                }
                if status == 1 {
                    crate::fixes::on_validated_callback();
                    if crate::runtime::installed() {
                        callback(ctx);
                    }
                }
            }) {
                Ok(_hook) => {
                    log::info!(
                        "{} : Hooked @ {}+{:x}",
                        sh.tag,
                        module.name,
                        rel_addr + sh.offset
                    );
                    true
                }
                Err(e) => {
                    crate::runtime::reject_install();
                    log::error!(
                        "{} : Failed to install hook for '{}': {e}",
                        sh.tag,
                        sh.signature
                    );
                    false
                }
            }
        }
        None => {
            crate::runtime::reject_install();
            log::error!("{} : Did not find '{}'", sh.tag, sh.signature);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{aob_scan, parse_pattern, scan_code_range};
    #[test]
    fn reference_scanner_handles_wildcards_and_bounds() {
        let pattern = parse_pattern("40 ?? 57");
        assert_eq!(aob_scan(&[0, 0x40, 0x55, 0x57], &pattern), Some(1));
        assert_eq!(aob_scan(&[0x40, 0x55], &pattern), None);
        assert_eq!(aob_scan(&[0x40, 0x55, 0x58], &pattern), None);
        assert_eq!(aob_scan(&[0x40], &[]), None);
    }

    #[test]
    fn code_scan_skips_inaccessible_pages_and_finds_ambiguous_matches() {
        use std::ffi::c_void;
        use windows::Win32::System::Memory::*;
        unsafe {
            let base = VirtualAlloc(
                None,
                0x3000,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_EXECUTE_READWRITE,
            );
            assert!(!base.is_null());
            let first = base as usize + 32;
            let second = base as usize + 0x2000 + 64;
            let bytes = [0x40u8, 0x55, 0x57, 0x66, 0x90];
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), first as *mut u8, bytes.len());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), second as *mut u8, bytes.len());
            let mut old = PAGE_PROTECTION_FLAGS(0);
            VirtualProtect(
                (base as usize + 0x1000) as *const c_void,
                0x1000,
                PAGE_NOACCESS,
                &mut old,
            )
            .unwrap();
            let hits = scan_code_range(base as usize, 0x3000, "40 ?? 57 66 90");
            assert_eq!(hits, vec![first, second]);
            VirtualProtect(
                (base as usize + 0x2000) as *const c_void,
                0x1000,
                PAGE_READWRITE,
                &mut old,
            )
            .unwrap();
            assert_eq!(
                scan_code_range(base as usize, 0x3000, "40 ?? 57 66 90"),
                vec![first]
            );
            VirtualFree(base, 0, MEM_RELEASE).unwrap();
        }
    }
}
