param(
    [ValidateSet('Install', 'Disable', 'Enable', 'Uninstall', 'Status')]
    [string]$Action = 'Install',
    [string]$GameDirectory,
    [switch]$NonInteractive
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$bundleRoot = $PSScriptRoot
$statePath = Join-Path $bundleRoot '.ffrs-portable-state.json'
$package = Get-Content -LiteralPath (Join-Path $bundleRoot 'package.json') -Raw | ConvertFrom-Json
$allowedFiles = @('winmm.dll', 'scripts/final_fantasy_resonance_fix.asi', 'scripts/final_fantasy_resonance_fix.toml')

function Get-Digest([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $hasher.Dispose() }
}
function Get-Target([string]$Root, [string]$Relative) {
    if ($allowedFiles -notcontains $Relative) { throw "Unexpected managed file: $Relative" }
    $prefix = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    $path = [IO.Path]::GetFullPath((Join-Path $Root $Relative))
    if (-not $path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Target escaped the game directory.' }
    $scriptsDirectory = Join-Path $Root 'scripts'
    if ((Test-Path -LiteralPath $scriptsDirectory) -and ((Get-Item -LiteralPath $scriptsDirectory).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'The scripts directory is a link; use manual installation.'
    }
    return $path
}
function Save-State($State) {
    $State | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statePath -Encoding UTF8
}
function Copy-Safe([string]$Source, [string]$Destination) {
    $null = New-Item -ItemType Directory -Force -Path (Split-Path $Destination -Parent)
    $temporary = $Destination + '.ffrs-portable-tmp'
    if (Test-Path -LiteralPath $temporary) { throw "Temporary file already exists: $temporary" }
    Copy-Item -LiteralPath $Source -Destination $temporary
    Move-Item -LiteralPath $temporary -Destination $Destination -Force
}
function Restore-State($State) {
    foreach ($entry in $State.files) {
        $target = Get-Target $State.gameDirectory $entry.relative
        if ($entry.existed) {
            $backup = Join-Path $bundleRoot $entry.backup
            if ((Get-Digest $backup) -ne $entry.originalSha256) { throw "Backup checksum mismatch: $($entry.relative)" }
        }
    }
    foreach ($entry in $State.files) {
        $target = Get-Target $State.gameDirectory $entry.relative
        if ($entry.existed) { Copy-Safe (Join-Path $bundleRoot $entry.backup) $target }
        elseif (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target }
        if ($entry.relative.EndsWith('.asi') -and (Test-Path -LiteralPath ($target + '.disabled'))) {
            Remove-Item -LiteralPath ($target + '.disabled')
        }
    }
}

try {
    if ($Action -eq 'Status') {
        if (Test-Path -LiteralPath $statePath) { Get-Content -LiteralPath $statePath }
        else { Write-Output 'This portable folder has no installation record.' }
        exit 0
    }
    if (Get-Process -Name 'FFRS', 'FFRS-Win64-Shipping' -ErrorAction SilentlyContinue) {
        throw 'Exit Final Fantasy Resonance before changing patch files.'
    }
    if ($Action -eq 'Install') {
        if (Test-Path -LiteralPath $statePath) {
            $oldState = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
            if ($oldState.status -in @('installed', 'disabled', 'preparing')) {
                throw 'This portable folder already manages an installation. Use Enable, Status or Uninstall.'
            }
        }
        if (-not $GameDirectory) {
            if ($NonInteractive) { throw 'Provide -GameDirectory.' }
            $GameDirectory = Read-Host 'Steam retail game root OR FFRS\Binaries\Win64 directory'
        }
        $GameDirectory = $GameDirectory.Trim().Trim('"')
        if (-not (Test-Path -LiteralPath (Join-Path $GameDirectory 'FFRS-Win64-Shipping.exe'))) {
            $GameDirectory = Join-Path $GameDirectory 'FFRS\Binaries\Win64'
        }
        $gameRoot = (Resolve-Path -LiteralPath $GameDirectory).Path
        $exe = Join-Path $gameRoot 'FFRS-Win64-Shipping.exe'
        if ((Get-Digest $exe) -ne $package.retailExeSha256) {
            throw 'Unsupported executable. This package supports only the documented Steam retail build, not the Demo.'
        }
        foreach ($relative in $allowedFiles) {
            $expected = $package.files.PSObject.Properties[$relative].Value
            $source = Join-Path (Join-Path $bundleRoot 'payload') $relative
            if ((Get-Digest $source) -ne $expected) { throw "Package checksum mismatch: $relative" }
        }
        foreach ($proxy in @('dinput8.dll', 'dsound.dll', 'version.dll', 'dxgi.dll', 'd3d9.dll', 'xinput1_3.dll')) {
            if (Test-Path -LiteralPath (Join-Path $gameRoot $proxy)) {
                throw "Existing proxy loader detected: $proxy. See the manual-install notes; no files changed."
            }
        }
        $loader = Join-Path $gameRoot 'winmm.dll'
        if ((Test-Path -LiteralPath $loader) -and (Get-Digest $loader) -ne $package.files.'winmm.dll') {
            throw 'A different winmm.dll already exists. See manual installation; no files changed.'
        }
        if (Test-Path -LiteralPath (Join-Path $gameRoot 'scripts/final_fantasy_resonance_fix.asi.disabled')) {
            throw 'A disabled copy of the patch already exists. Restore it using its original manager or use manual installation.'
        }
        $backupDirectory = '.ffrs-backups/' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
        $entries = @()
        foreach ($relative in $allowedFiles) {
            $target = Get-Target $gameRoot $relative
            if ((Test-Path -LiteralPath $target) -and ((Get-Item -LiteralPath $target).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
                throw "Managed file is a link: $relative"
            }
            $existed = Test-Path -LiteralPath $target
            $backup = $backupDirectory + '/' + $relative
            $originalSha = $null
            if ($existed) {
                $originalSha = Get-Digest $target
                Copy-Safe $target (Join-Path $bundleRoot $backup)
            }
            $installedSha = $package.files.PSObject.Properties[$relative].Value
            if ($existed -and $relative.EndsWith('.toml')) { $installedSha = $originalSha }
            $entries += [pscustomobject]@{ relative = $relative; existed = [bool]$existed; backup = $backup; originalSha256 = $originalSha; installedSha256 = $installedSha }
        }
        $state = [pscustomobject]@{ version = $package.version; gameDirectory = $gameRoot; status = 'preparing'; created = (Get-Date).ToString('o'); files = $entries }
        Save-State $state
        try {
            foreach ($entry in $entries) {
                $target = Get-Target $gameRoot $entry.relative
                if (-not ($entry.existed -and $entry.relative.EndsWith('.toml'))) {
                    Copy-Safe (Join-Path (Join-Path $bundleRoot 'payload') $entry.relative) $target
                }
                if ((Get-Digest $target) -ne $entry.installedSha256) { throw "Installed checksum mismatch: $($entry.relative)" }
            }
            $state.status = 'installed'
        } catch {
            Restore-State $state
            $state.status = 'rolled-back'
            Save-State $state
            throw
        }
    } else {
        if (-not (Test-Path -LiteralPath $statePath)) { throw 'No installation record. Manual-install users should follow README.md.' }
        $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
        if ($state.status -notin @('installed', 'disabled')) { throw "Installation is not active: $($state.status)" }
        $asiEntry = @($state.files | Where-Object { $_.relative.EndsWith('.asi') })[0]
        $asi = Get-Target $state.gameDirectory $asiEntry.relative
        $disabled = $asi + '.disabled'
        if ($Action -in @('Disable', 'Enable')) {
            $source = $asi; $destination = $disabled
            if ($Action -eq 'Enable') { $source = $disabled; $destination = $asi }
            if ((Test-Path -LiteralPath $destination) -and -not (Test-Path -LiteralPath $source)) {
                Write-Output 'Already in the requested state.'; exit 0
            }
            if (Test-Path -LiteralPath $destination) { throw 'Both enabled and disabled ASI files exist.' }
            if ((Get-Digest $source) -ne $asiEntry.installedSha256) { throw 'ASI changed; refusing to rename.' }
            Move-Item -LiteralPath $source -Destination $destination
            $state.status = 'disabled'
            if ($Action -eq 'Enable') { $state.status = 'installed' }
        } else {
            # Check every managed file before removing or restoring anything.
            foreach ($entry in $state.files) {
                $target = Get-Target $state.gameDirectory $entry.relative
                if ($entry.relative.EndsWith('.asi') -and -not (Test-Path -LiteralPath $target)) { $target += '.disabled' }
                if (Test-Path -LiteralPath $target) {
                    if ((Get-Digest $target) -ne $entry.installedSha256) {
                        if ($entry.relative.EndsWith('.toml')) {
                            Copy-Safe $target (Join-Path $bundleRoot ('.ffrs-backups/user-config-' + [Guid]::NewGuid().ToString('N') + '.toml'))
                        } else { throw "Managed file changed; refusing to remove it: $($entry.relative)" }
                    }
                }
            }
            Restore-State $state
            $state.status = 'uninstalled'
        }
    }
    Save-State $state
    Write-Output ("{0}: {1}" -f $state.status, $state.gameDirectory)
    Write-Output 'Game EXE, Paks, saves and the Demo were not modified.'
} catch {
    Write-Error $_.Exception.Message -ErrorAction Continue
    exit 1
}
