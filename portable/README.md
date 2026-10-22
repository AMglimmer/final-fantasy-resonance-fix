# FF Resonance Ultrawide Fix — Windows x64 portable

Based on [PolarWizard/final-fantasy-resonance-fix](https://github.com/PolarWizard/final-fantasy-resonance-fix).
This fork fixes SDK address resolution for Steam retail build **24250053**.

No Rust, Python, UE4SS or installer is required by this download. It uses
Windows PowerShell already included with Windows. Keep this folder to manage
the installation and preserve its backups.

## Install

1. Extract this ZIP into a writable folder outside the game installation.
2. Exit FF Resonance.
3. Run `Install.cmd`. Paste either the Steam game root or its
   `FFRS\Binaries\Win64` directory when prompted. Admin rights are normally
   unnecessary for a writable Steam library.
4. Start the game normally through Steam and select your display's native
   resolution. The tested setup is **3440×1440**.

The script checks the retail EXE and package hashes, backs up existing managed
files and copies only `winmm.dll` plus the ASI and TOML in `scripts`. Existing
TOML settings are preserved. Unsupported EXEs, including the Demo, are rejected.
Other proxy loaders are not replaced automatically.

Default configuration: ultrawide scene rendering, maintain-Y FOV (Hor+),
centered 16:9 HUD, original 16:9 movie handling, original post-processing,
custom fonts disabled. At 3440×1440 the HUD uses the centered 2560×1440 area.

## Disable, enable or uninstall

Exit the game, then run `Disable.cmd`, `Enable.cmd` or `Uninstall.cmd` from this
same extracted folder. The installation record and backups stay here, not in
your save directory. Do not delete them until you no longer need restoration.
Edited configs are preserved in a separate backup on uninstall.

## Manual installation / existing ASI Loader

If you already use another ASI Loader, use its documented ASI loading location
and copy only the two files in `payload/scripts`. Do not overwrite its proxy DLL.
For a clean installation, back up any destination files first, then copy the
contents of `payload` into `FFRS\Binaries\Win64`. The optional scripts manage
only installations they recorded; manual installs are removed manually.

Manual disable: rename `scripts/final_fantasy_resonance_fix.asi` to
`final_fantasy_resonance_fix.asi.disabled` while the game is closed.

Config/log: `FFRS\Binaries\Win64\scripts\final_fantasy_resonance_fix.toml`
and `final_fantasy_resonance_fix.log`.

## Validation and limits

The binary uses the runtime source tested locally, rebuilt with local build paths
remapped for public distribution. The user confirmed
ultrawide gameplay scenes and the world map. Runtime logs show successful SDK
validation and HUD/movie callbacks. Sixteen automated tests passed. Full CG,
all UI screens, same-camera Hor+ image comparison and long-session stability
have not been exhaustively validated. This first retail release is a prerelease.

Only the documented build is supported; game updates require revalidation.
This does not patch the on-disk game EXE, Paks, saves, DRM or anti-cheat.

## Credits

Original fix: PolarWizard (MIT). Retail adaptation and portable packaging:
AMglimmer's fork. ASI Loader: ThirteenAG / Ultimate ASI Loader, x64 10.0.0
(`01eb152`). See `LICENSE`, `LICENSES` and `LICENSE-UAL.txt` for notices.
