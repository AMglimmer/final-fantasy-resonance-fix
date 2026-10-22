# Run from an x64 MSVC developer shell with Rust on PATH.
param([string]$TargetDirectory)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not $TargetDirectory) { $TargetDirectory = Join-Path $repoRoot 'target/public' }
$userProfilePath = [Environment]::GetFolderPath('UserProfile')
$flags = @()
if ($env:CARGO_ENCODED_RUSTFLAGS) { $flags += $env:CARGO_ENCODED_RUSTFLAGS.Split([char]31) }
$flags += ('--remap-path-prefix=' + $userProfilePath + '=/build')
$flags += '-Clink-arg=/PDBALTPATH:final_fantasy_resonance_fix.pdb'
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
$env:CC_SHELL_ESCAPED_FLAGS = '1'
$env:CXXFLAGS = ($env:CXXFLAGS + ' /experimental:deterministic "/pathmap:' + $userProfilePath + '=/build"').Trim()
$env:CARGO_TARGET_DIR = $TargetDirectory
& cargo build --locked --release --manifest-path (Join-Path $repoRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo test --locked --release --manifest-path (Join-Path $repoRoot 'Cargo.toml')
exit $LASTEXITCODE
