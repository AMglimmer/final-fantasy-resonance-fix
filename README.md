# FINAL FANTASY RESONANCE Fix

## Steam retail portable release

This is AMglimmer's retail compatibility fork of PolarWizard's original fix.
Download the Windows x64 portable ZIP from this fork's **Releases** page. It
needs no Rust or Python: extract it, exit the game, run `Install.cmd`, and point
it at the Steam retail game root or `FFRS\Binaries\Win64`.

Supported retail build: **24250053**, tested at **3440×1440**. Gameplay scenes
and the world map have been confirmed by the user; SDK/HUD/movie callbacks are
present in runtime logs. Full CG and all UI screens are still awaiting complete
visual validation. The first portable release is marked as a prerelease.

The retail fix resolves SDK addresses from unique signatures and validates
object layouts before applying corrections. The original Rust/C++ architecture,
movie handling and Hor+ logic are retained. Game EXEs, Paks, saves and the Demo
are not modified. Updates to the game require compatibility revalidation.

See [portable instructions](portable/README.md), [中文说明](portable/README.zh-CN.md)
and [technical compatibility notes](RETAIL_COMPATIBILITY.md).

Default portable configuration enables centered 16:9 HUD, movie aspect handling,
and original post-processing; custom fonts remain disabled. Backups and optional
Disable/Enable/Uninstall scripts are included.

The original project documentation follows.

![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/PolarWizard/final-fantasy-resonance-fix/total)

***This project is designed exclusively for Windows due to its reliance on Windows-specific APIs.***

## Fixes
- Remove black bars in ultrawide resolutions
- Movies are constrained to their original 16:9 format
- Add ability to constrain HUD
- Add ability to configure various post processing effects
- Add ability to load and use a custom font

## Build and Install
Requires:
- [Rust](https://www.rust-lang.org/tools/install)
- [Python 3](https://www.python.org/downloads/)

1. Clone:
```ps1
git clone https://github.com/PolarWizard/final-fantasy-resonance-fix.git
cd final-fantasy-resonance-fix
```
2. Install [Ultimate ASI Loader](https://github.com/ThirteenAG/Ultimate-ASI-Loader) into the game folder (only needed once):
```ps1
python scripts/loader.py "<path to 'FINAL FANTASY RESONANCE\FFRS\Binaries\Win64' folder>"
```
3. Build and install the fix:
```ps1
python scripts/install.py "<path to 'FINAL FANTASY RESONANCE\FFRS\Binaries\Win64' folder>"
```
This builds a release DLL with `cargo` and copies it to `<game folder>/scripts/final_fantasy_resonance_fix.asi`, along with a default config on first install. Pass `--debug` to build and install a debug build instead.

If you're using VS Code, the same steps are wired up as tasks in `.vscode/tasks.json` (`loader`, `install`, `build and install`) -- edit the game folder path in there to match your install first.

## Configuration
- Edit `<game folder>/scripts/final_fantasy_resonance_fix.toml`

## Screenshots
| Ultrawide |
| :---: |
| ![Ultrawide demo](images/final-fantasy-resonance-fix-1.gif) |
| Fix disabled → Fix enabled |

| HUD Constraining |
| :---: |
| ![HUD constraining demo](images/final-fantasy-resonance-fix-2.gif) |
| 32:9 → 21:9 → 16:9 |

## License
Distributed under the MIT License. See [LICENSE](LICENSE) for more information. Third-party license text for every dependency can be generated with `python scripts/licenses.py`, which writes it out to `LICENSES`.

## Special Thanks
- Thank you [Dumper-7: Unreal Engine SDK Generator](https://github.com/Encryqed/Dumper-7) project for making this possible!
