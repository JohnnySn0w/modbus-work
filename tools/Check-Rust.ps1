param([string]$Toolchain = 'stable-x86_64-pc-windows-msvc', [switch]$IncludeGui)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$manifest = Join-Path $projectRoot 'native/modbus-configurator/Cargo.toml'
& cargo "+$Toolchain" fmt --manifest-path $manifest --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed; run cargo fmt.' }
& cargo "+$Toolchain" clippy --locked --offline --manifest-path $manifest --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'Rust lint checks failed.' }
# Coverage runs the complete test suite once and enforces the maintained floor.
& (Join-Path $PSScriptRoot 'Measure-RustCoverage.ps1') -Toolchain $Toolchain -IncludeGui:$IncludeGui
Write-Output 'Rust checks passed: formatting, Clippy, tests and coverage floor.'
