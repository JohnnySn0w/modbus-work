param(
    [string]$Toolchain = 'stable-x86_64-pc-windows-msvc',
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._-]*$')][string]$Version
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$crateRoot = Join-Path $projectRoot 'native/modbus-configurator'
$buildRoot = Join-Path $crateRoot 'target/portable-build'
$releaseRoot = Join-Path $projectRoot 'artifacts/releases'
$stamp = if ($Version) { $Version } else { Get-Date -Format 'yyyyMMdd-HHmmss' }
$packageName = "Polygon-Device-Configurator-windows-x64-$stamp"
$packageRoot = Join-Path $releaseRoot $packageName
if (Test-Path -LiteralPath $packageRoot) { throw 'Package directory already exists; choose a new version.' }
$sourceCommit = & git -C $projectRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Could not identify source commit.' }
$sourceChanges = @(& git -C $projectRoot status --porcelain)
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect source changes.' }
$sourceDirty = $sourceChanges.Count -gt 0
$previousFlags = $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS
$previousBuild = $env:POLYGON_BUILD_ID
try {
    $env:POLYGON_BUILD_ID = "$stamp | commit $sourceCommit | modified source $sourceDirty"
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
    & cargo "+$Toolchain" build --release --locked --offline --target x86_64-pc-windows-msvc --manifest-path (Join-Path $crateRoot 'Cargo.toml') --target-dir $buildRoot
    if ($LASTEXITCODE -ne 0) { throw 'Portable release build failed.' }
    $exe = Join-Path $buildRoot 'x86_64-pc-windows-msvc/release/modbus-configurator.exe'
    $pdb = Join-Path $buildRoot 'x86_64-pc-windows-msvc/release/modbus_configurator.pdb'
    if (!(Test-Path -LiteralPath $pdb) -or (Get-Item -LiteralPath $pdb).Length -eq 0) { throw 'Release debug symbols are missing.' }
    $sysroot = & rustup run $Toolchain rustc --print sysroot
    if ($LASTEXITCODE -ne 0) { throw 'Could not locate the Rust toolchain.' }
    $readobj = Join-Path $sysroot 'lib/rustlib/x86_64-pc-windows-msvc/bin/llvm-readobj.exe'
    if (!(Test-Path -LiteralPath $readobj)) { throw 'Install llvm-tools-preview for this toolchain before packaging.' }
    # IMAGE_SUBSYSTEM_WINDOWS_GUI (2), independent of diagnostic build settings.
    $peBytes = [System.IO.File]::ReadAllBytes($exe)
    $peOffset = [BitConverter]::ToInt32($peBytes, 60)
    if ([BitConverter]::ToUInt16($peBytes, $peOffset + 24 + 68) -ne 2) {
        throw 'The application must use the Windows GUI subsystem, not open a console.'
    }
    $imports = & $readobj --coff-imports $exe
    if ($LASTEXITCODE -ne 0) { throw 'Dependency inspection failed.' }
    $dlls = @($imports | Select-String '^  Name: ' | ForEach-Object { $_.Line.Substring(8).Trim() } | Sort-Object -Unique)
    if ($dlls | Where-Object { $_ -match '(?i)vcruntime|msvcp|python|powershell' }) { throw 'Unexpected runtime dependency in portable executable.' }
    New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null
    Copy-Item -LiteralPath $exe -Destination (Join-Path $packageRoot 'Polygon Device Configurator.exe')
    Copy-Item -LiteralPath $pdb -Destination (Join-Path $packageRoot 'modbus_configurator.pdb')
    Copy-Item -LiteralPath (Join-Path $projectRoot 'docs/PORTABLE-README.txt') -Destination (Join-Path $packageRoot 'README.txt')
    $metadataText = & cargo "+$Toolchain" metadata --locked --offline --filter-platform x86_64-pc-windows-msvc --format-version 1 --manifest-path (Join-Path $crateRoot 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Dependency metadata inspection failed.' }
    $metadata = $metadataText | ConvertFrom-Json
    $noticeRoot = Join-Path $packageRoot 'licenses'
    New-Item -ItemType Directory -Path $noticeRoot | Out-Null
    $notices = @()
    foreach ($dependency in $metadata.packages | Sort-Object name,version) {
        $notices += "$($dependency.name) $($dependency.version) : $($dependency.license)"
        $sourceRoot = Split-Path $dependency.manifest_path -Parent
        $licenseFiles = @(Get-ChildItem -LiteralPath $sourceRoot -File | Where-Object { $_.Name -match '^(LICENSE|COPYING|NOTICE)([.-]|$)' })
        if ($licenseFiles.Count -gt 0) {
            $destination = Join-Path $noticeRoot "$($dependency.name)-$($dependency.version)"
            New-Item -ItemType Directory -Path $destination | Out-Null
            foreach ($license in $licenseFiles) { Copy-Item -LiteralPath $license.FullName -Destination $destination }
        }
    }
    $notices | Set-Content -LiteralPath (Join-Path $packageRoot 'DEPENDENCIES.txt') -Encoding utf8
    $digest = (Get-FileHash -LiteralPath (Join-Path $packageRoot 'Polygon Device Configurator.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    $symbolsDigest = (Get-FileHash -LiteralPath $pdb -Algorithm SHA256).Hash.ToLowerInvariant()
    [ordered]@{build_utc=[DateTime]::UtcNow.ToString('o');source_commit=$sourceCommit;source_dirty=$sourceDirty;version=$stamp;toolchain=$Toolchain;target='x86_64-pc-windows-msvc';static_crt=$true;debug_symbols='full';debug_assertions=$true;overflow_checks=$true;pdb_file='modbus_configurator.pdb';pdb_sha256=$symbolsDigest;exe_sha256=$digest;imported_dlls=$dlls} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $packageRoot 'build-info.json') -Encoding utf8
    Compress-Archive -LiteralPath $packageRoot -DestinationPath "$packageRoot.zip"
    $zipDigest = (Get-FileHash -LiteralPath "$packageRoot.zip" -Algorithm SHA256).Hash.ToLowerInvariant()
    "$zipDigest  $packageName.zip" | Set-Content -LiteralPath "$packageRoot.zip.sha256" -Encoding ascii
    Write-Output "Package: $packageRoot.zip"
    Write-Output "Executable: $packageRoot/Polygon Device Configurator.exe"
    Write-Output "Imports: $($dlls -join ', ')"
} finally {
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = $previousFlags
    $env:POLYGON_BUILD_ID = $previousBuild
}
