# Steam retail compatibility investigation — 2026-10-23

Based on upstream `585a57d` (0.2.0). The user has since confirmed ultrawide
gameplay scenes and the world map at 3440x1440. Runtime logs confirm SDK
validation and HUD/movie callbacks. Full CG, every UI screen, same-camera
Hor+ image comparison and long-session validation remain incomplete.

## Evidence

| Item | Demo | Retail |
|---|---|---|
| Steam app/build | 4474710 / 24032336 | 3259780 / 24250053 |
| Shipping EXE bytes | 489829120 | 495708416 |
| PE timestamp | 6A469DFA | 6A586B2F |
| SizeOfImage | 1DA1B000 | 1DFB6000 |
| Engine string | Unreal Engine 5.6.1 | Unreal Engine 5.6.1 |
| GObjects RVA | 09D76D50 | 09D84DD0 |
| FName::AppendString RVA | 01376360 | 013763A0 |
| ProcessEvent RVA | 01592D00 | 01592E30 |
| ProcessEvent vtable index | 4C | 4C |

Both executables are AMD64, PE32+, image base 140000000. Their file/product
version resources both say 1.0.0.0; the observed retail title screen says 1.0.1.0.
All 14 accompanying DLLs in the targeted inventory have identical SHA-256
hashes between installations, as do the sets of shipping EXE import names.
The EXEs have different protected section layouts; section names should not
be assumed to be conventional `.text`/`.rdata` names.

SHA-256:

* Demo: `e06401cdf4e64d55f652043bdd2012267d5529a00c3dbc36ee4365e0ce49caad`
* Retail: `b8c588e46ebadbe37feca7bdf2da0ae63a8538099d14a53ec0c2241b086e5ffa`

All 16 upstream code signatures match once in each on-disk executable.
The 4 added SDK resolver signatures also match once in each inspected build.
The new, tighter aspect signatures retain the same two patch sites.
The 8-byte axis instruction remains `movzx edx, byte ptr [r15+0xb8]`;
replacing it with `mov edx,0` and NOPs selects MaintainYFOV (enum value 0).
This is the existing Hor+ mechanism, not a viewport stretch or an FOV multiplier.

FName::AppendString and ProcessEvent were located by normalized instruction
signatures, wildcarding relative displacements. Six sampled GObjects references
(including five individually unique code sites) resolve to retail 09D84DD0.
The runtime resolver requires two independent unique references to agree.

ReadProcessMemory on the normally launched retail process confirmed:

* Demo GObjects location: 32 zero bytes, including NumElements == 0.
* Retail GObjects: a plausible chunked object array (74414 and 57784 objects in
  two observations at different startup stages).
* Object zero: `/Script/CoreUObject`, index 0, vtable[4C] == base+01592E30.
* 24 relevant native classes/structs and 535 reflected property offsets and
  sizes match the supplied demo SDK, including UMG movie classes and battle
  sequence parameters. This does not validate every type in the entire SDK.
* All 16 hook/patch sites are in PAGE_EXECUTE_READ regions. The observed
  loaded image had no inaccessible regions; scanner page safety is preventive
  hardening, not an established cause of this local incompatibility.

## Failure mechanism and limits

The SDK lazily falls back to fixed demo RVAs. In retail, that makes class
lookups iterate an empty array, returning null; generated wrappers such as
Conv_StringToName dereference StaticClass()/function results without a null
check. Independently, calling demo AppendString RVA 01376360 enters unrelated
retail instructions, inside a function rather than at its entry. C++ access
violations are not caught by Rust catch_unwind.

This establishes concrete SDK incompatibility on the inspected retail binary.
Issue #1 contains only "Starts up and crashes instantly", no stack or dump.
Its exact first fault cannot be attributed conclusively without that evidence.
We did not deliberately install the unsafe original build to provoke a crash.
Pre-existing local Windows events in Steam crashhandler64.dll (C0000409)
were captured separately and must not be treated as evidence from this patch.

## Implementation

* `src/runtime.rs`: known-build gate using timestamp/SizeOfImage; unique code
  signatures; RIP-relative GObjects resolution with cross-check; no retail
  fallback to any fixed demo address.
* `cpp/runtime.cpp`, bridge and build wiring: initialize the SDK using its
  existing InitManually APIs; validate array bounds, first object's vtable,
  and 12 key reflected properties on the first engine callback. Unready
  objects are retryable; a mismatch disables callbacks.
* `src/utils.rs`: scan only committed readable executable regions; require
  unique matches; gate SDK callbacks until installation is complete; reject
  partial hook installations; check VirtualProtect and flush instruction cache.
* `src/fixes/aspect_ratio.rs`: more specific signatures and deferred writes
  after SDK validation. The original unconstrain/maintain-Y byte edits remain.
* `src/fixes/mod.rs`, `src/lib.rs`: wire validation and diagnostic phases;
  retain the existing movie, HUD, capture and post-processing implementations.
* `scripts/check_compatibility.py`: read-only file hash and signature preflight.
* `scripts/manage_retail.py`: exact retail target, stopped-process check,
  package checksums, backups, deployment manifest, disable/enable/uninstall.

The bundled generated SDK, original game EXEs, Paks and saves are not modified.
Demo is read-only throughout the investigation and is not used for installation.
No protection/DRM/anti-cheat code is patched or bypassed.

## Build and test

Retain the original x64 MSVC Rust/cxx build:

```powershell
cargo build --locked --release
cargo test --locked --release
python scripts/check_compatibility.py '<retail shipping executable>'
```

Needs Rust, Python, Visual C++ x64 tools and a Windows SDK. In this workspace,
Rust is installed in the task's work/toolchain directory, MSVC Build Tools in
C:\BuildTools, and Windows SDK 10.0.26100.0 in the standard Windows Kits path.
The original loader.py fetched and checksum-verified Ultimate ASI Loader x64
10.0.0.0-01eb152 into a staging directory before deployment.

An added Windows allocation test covers inaccessible pages, non-executable
pages and multiple matches without reading invalid memory. Existing tests
cover layout, movie aspect arithmetic, post-processing and configuration.

## Local configuration and pending validation

Retail already requests 3440x1440, fullscreen mode 1; GameUserSettings.ini does
not need editing. Deployment enables movies and a centered 16:9 HUD. All
post-processing switches remain true; custom fonts stay disabled.

After next launch, inspect scripts/final_fantasy_resonance_fix.log for:

1. SDK profile and resolved retail RVAs.
2. Successful hook installation.
3. SDK callback state 1 and "Aspect correction applied after SDK validation".

These establish initialization, not correct rendering. Still verify real-time
scene black bars, same-scene 16:9 vs 3440x1440 Hor+, HUD, menu/map/tooltip,
pre-rendered CG and summon transitions. Compare unchanged vertical content and
extra horizontal scene content; a title-screen screenshot alone cannot prove Hor+.

Disable by renaming only scripts/final_fantasy_resonance_fix.asi to
final_fantasy_resonance_fix.asi.disabled while the game is closed. The generated
deployment shortcuts use the manager and its manifest; uninstall restores
original files and preserves a user-edited TOML in the backup directory.
# Capture-transition lifetime fix (2026-10-26)

Four retail crash dumps identify the same cached scratch-target name check in
`cpp/encounter.cpp::bounce_target`: the mod kept a render-target UObject only as
a native pointer across transitions, without a GC-visible owner. Checking its
full name after collection could dereference an invalid Outer chain.

Scratch render targets are now scoped to one capture, with GPU resources
released by the existing engine `ReleaseRenderTarget2D` wrapper on success and
draw failure. Capture requests are consumed once, including failures; 16:9
captures allocate no scratch. This removes that cached-pointer/name check
without changing aspect or movie/HUD settings.

Run `python scripts/test_capture_lifetime.py` in an x64 MSVC developer shell to
exercise the actual encounter source with a small rendering fixture. It checks
repeated transitions after target destruction, crop geometry, allocation/draw
failures, one-shot requests and the 16:9 no-op. `--baseline` tests the published
`f0595ed` baseline and demonstrates failure of its cached implementation.

Build and offline tests passed. Retail startup validated the SDK and applied
Hor+. The patched game process then remained running for about 64 monitored
minutes without another captured crash, and the user subsequently reported
stable gameplay. This supports the first stable retail release, without claiming
exhaustive CG/UI or every-scene validation. The public binary uses the same
runtime source with build paths remapped. Private crash dumps and logs are not
distributed.
