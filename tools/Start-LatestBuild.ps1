# Resolve the newest complete portable package at launch time, independent of its folder name.
param([switch]$ResolveOnly)
$ErrorActionPreference = 'Stop'
try {
    $releaseRoot = Join-Path (Split-Path $PSScriptRoot -Parent) 'artifacts/releases'
    $candidates = @(Get-ChildItem -LiteralPath $releaseRoot -Directory | ForEach-Object {
        $exe = Join-Path $_.FullName 'Polygon Device Configurator.exe'
        $infoPath = Join-Path $_.FullName 'build-info.json'
        if ((Test-Path -LiteralPath $exe) -and (Test-Path -LiteralPath $infoPath)) {
            try {
                $info = Get-Content -LiteralPath $infoPath -Raw | ConvertFrom-Json
                [pscustomobject]@{Exe=$exe;Built=[DateTimeOffset]::Parse($info.build_utc);Hash=$info.exe_sha256}
            } catch { } # An incomplete package is not a launch candidate.
        }
    } | Sort-Object Built -Descending)
    $latest = $candidates | Select-Object -First 1
    if (!$latest) { throw 'No packaged build found. Run tools/Build-WindowsPackage.ps1 first.' }
    if ((Get-FileHash -LiteralPath $latest.Exe -Algorithm SHA256).Hash -ne $latest.Hash) {
        throw 'The latest executable does not match its build checksum. Rebuild the package before launching.'
    }
    if ($ResolveOnly) { Write-Output $latest.Exe; return }
    Start-Process -FilePath $latest.Exe -WorkingDirectory (Split-Path $latest.Exe -Parent) -WindowStyle Normal
} catch {
    if ($ResolveOnly) { throw }
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Polygon Device Configurator') | Out-Null
    exit 1
}
