param(
    [string]$Toolchain = 'stable-x86_64-pc-windows-msvc',
    [double]$MinimumLineCoverage = 90,
    [switch]$IncludeGui
)
$ErrorActionPreference = 'Stop'
if ($IncludeGui -and !$PSBoundParameters.ContainsKey('MinimumLineCoverage')) {
    $MinimumLineCoverage = 92
}
$projectRoot = Split-Path $PSScriptRoot -Parent
$crateRoot = Join-Path $projectRoot 'native/modbus-configurator'
$coverageExe = Join-Path $crateRoot 'target/coverage-tools/bin/cargo-llvm-cov.exe'
if (!(Test-Path -LiteralPath $coverageExe)) {
    $installedCoverage = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
    if (!$installedCoverage) {
        throw 'Install llvm-tools-preview and cargo-llvm-cov as described in docs/RUST-COVERAGE.md.'
    }
    $coverageExe = $installedCoverage.Source
}
$reportRoot = Join-Path $crateRoot 'target/coverage'
New-Item -ItemType Directory -Force -Path $reportRoot | Out-Null
$previousToolchain = $env:RUSTUP_TOOLCHAIN
try {
    $env:RUSTUP_TOOLCHAIN = $Toolchain
    $common = @('--manifest-path', (Join-Path $crateRoot 'Cargo.toml'), '--ignore-filename-regex', '[/\\](tests|examples)[/\\]')
    # --no-report retains profiles for the later GUI merge. Start this run clean
    # so removed tests or previous desktop runs cannot inflate the result.
    & $coverageExe llvm-cov clean --workspace --manifest-path (Join-Path $crateRoot 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Coverage workspace cleanup failed.' }
    & $coverageExe llvm-cov --manifest-path (Join-Path $crateRoot 'Cargo.toml') --locked --offline --no-report
    if ($LASTEXITCODE -ne 0) { throw "Coverage tests or threshold failed ($LASTEXITCODE)." }
    if ($IncludeGui) {
        & $coverageExe llvm-cov report @common --json --output-path (Join-Path $reportRoot 'headless-coverage.json') --fail-under-lines 90
        if ($LASTEXITCODE -ne 0) { throw 'Headless coverage floor failed.' }
        # Merge real desktop startup/rendering with the unit profiles. Offline mode
        # prevents hardware access; isolated storage protects the user's settings.
        $runRoot = Join-Path $reportRoot ('desktop-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $runRoot | Out-Null
        $oldData = $env:LOCALAPPDATA
        try {
            $env:LOCALAPPDATA = Join-Path $runRoot 'data'
            & $coverageExe llvm-cov run --manifest-path (Join-Path $crateRoot 'Cargo.toml') --locked --offline --no-report --bin modbus-configurator -- --offline --capture-views (Join-Path $runRoot 'views') --exit-after-capture
            if ($LASTEXITCODE -ne 0) { throw 'Instrumented offline GUI failed.' }
            $views = @(Get-Content -LiteralPath (Join-Path $runRoot 'views/complete.json') -Raw | ConvertFrom-Json)
            if ($views.Count -lt 23) { throw 'Instrumented GUI did not capture every view.' }
            foreach ($view in $views) {
                $bytes = [IO.File]::ReadAllBytes((Join-Path $runRoot "views/$view"))
                if ($bytes.Length -lt 100 -or [BitConverter]::ToString($bytes[0..7]) -ne '89-50-4E-47-0D-0A-1A-0A') { throw "Invalid captured image: $view" }
            }
            Write-Output "Instrumented offline GUI: $($views.Count) views captured in $runRoot"
        } finally { $env:LOCALAPPDATA = $oldData }
    }
    & $coverageExe llvm-cov report @common --json --output-path (Join-Path $reportRoot 'coverage.json') --fail-under-lines $MinimumLineCoverage
    if ($LASTEXITCODE -ne 0) { throw 'Coverage report or threshold failed.' }
    & $coverageExe llvm-cov report @common --html --output-dir $reportRoot
    if ($LASTEXITCODE -ne 0) { throw 'HTML report generation failed.' }
    & $coverageExe llvm-cov report @common --lcov --output-path (Join-Path $reportRoot 'lcov.info')
    if ($LASTEXITCODE -ne 0) { throw 'LCOV export failed.' }
    $coverage = Get-Content -LiteralPath (Join-Path $reportRoot 'coverage.json') -Raw | ConvertFrom-Json
    $coverage.data[0].files | ForEach-Object {
        [PSCustomObject]@{Module=(Split-Path $_.filename -Leaf); Lines=('{0:F2}%' -f $_.summary.lines.percent); Functions=('{0:F2}%' -f $_.summary.functions.percent)}
    } | Format-Table -AutoSize
    Write-Output ('Overall line coverage: {0:F2}%' -f $coverage.data[0].totals.lines.percent)
    Write-Output "HTML report: $reportRoot/html/index.html"
} finally {
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

