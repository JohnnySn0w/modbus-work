param([Parameter(Mandatory=$true)][string]$ZipPath, [switch]$Dark, [switch]$SkipLaunch)
$ErrorActionPreference = 'Stop'
$zip = (Resolve-Path -LiteralPath $ZipPath).Path
$projectRoot = Split-Path $PSScriptRoot -Parent
$reviewRoot = Join-Path $projectRoot 'artifacts/review'
New-Item -ItemType Directory -Force -Path $reviewRoot | Out-Null
$checkRoot = Join-Path $reviewRoot ('package-check-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $checkRoot | Out-Null
$zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
if (Test-Path -LiteralPath "$zip.sha256") {
    $expectedHash = ((Get-Content -LiteralPath "$zip.sha256" -Raw).Trim() -split '\s+')[0]
    if ($zipHash -ne $expectedHash) { throw 'ZIP checksum mismatch.' }
}
Expand-Archive -LiteralPath $zip -DestinationPath (Join-Path $checkRoot 'extracted')
$executables = @(Get-ChildItem -LiteralPath (Join-Path $checkRoot 'extracted') -Filter "Polygon Device Configurator.exe" -File -Recurse)
if ($executables.Count -ne 1) { throw 'Package must contain exactly one Polygon Device Configurator.exe.' }
$exe = $executables[0].FullName
$packageInfo = Get-Content -LiteralPath (Join-Path $executables[0].DirectoryName 'build-info.json') -Raw | ConvertFrom-Json
if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $packageInfo.exe_sha256) { throw 'Extracted executable checksum mismatch.' }
if ($packageInfo.debug_symbols -ne 'full') { throw 'Package must include full release debug symbols.' }
$symbols = Join-Path $executables[0].DirectoryName 'modbus_configurator.pdb'
if (!(Test-Path -LiteralPath $symbols) -or (Get-FileHash -LiteralPath $symbols -Algorithm SHA256).Hash.ToLowerInvariant() -ne $packageInfo.pdb_sha256) { throw 'Release debug symbols are missing or their checksum does not match.' }
# Headless hosted runners can validate packaging without claiming GUI acceptance.
if ($SkipLaunch) {
    foreach ($file in @('README.txt', 'DEPENDENCIES.txt', 'licenses')) {
        if (!(Test-Path -LiteralPath (Join-Path $executables[0].DirectoryName $file))) { throw "Missing package content: $file" }
    }
    [ordered]@{checked_utc=[DateTime]::UtcNow.ToString('o');zip_sha256=$zipHash;exe_sha256=$packageInfo.exe_sha256;source_commit=$packageInfo.source_commit;scope='Package integrity only; executable was not launched'} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $checkRoot 'integrity.json') -Encoding utf8
    Write-Output "Package integrity passed: $checkRoot"
    return
}
$screenshots = Join-Path $checkRoot 'views'
$runtimeData = Join-Path $checkRoot 'runtime-data'
New-Item -ItemType Directory -Path $runtimeData | Out-Null
$oldPath = $env:PATH
$oldData = $env:LOCALAPPDATA
$oldTemp = $env:TEMP
$oldTmp = $env:TMP
try {
    $env:PATH = Join-Path $env:SystemRoot 'System32'
    $env:LOCALAPPDATA = $runtimeData
    $env:TEMP = $runtimeData
    $env:TMP = $runtimeData
    foreach ($command in @('python.exe','python3.exe','powershell.exe','pwsh.exe','cargo.exe','rustc.exe')) {
        if (Get-Command $command -CommandType Application -ErrorAction SilentlyContinue) { throw "Unexpected command available in runtime PATH: $command" }
    }
    $captureArgs = @('--offline','--capture-views',('"{0}"' -f $screenshots),'--exit-after-capture')
    if ($Dark) { $captureArgs += '--dark' }
    $runtime = Start-Process -FilePath $exe -WorkingDirectory $executables[0].DirectoryName -ArgumentList $captureArgs -WindowStyle Hidden -PassThru -RedirectStandardError (Join-Path $checkRoot "stderr.txt") -RedirectStandardOutput (Join-Path $checkRoot "stdout.txt")
    if (!$runtime.WaitForExit(60000)) {
        Stop-Process -Id $runtime.Id
        throw 'Packaged UI did not finish its offline capture within 60 seconds.'
    }
    if ($runtime.ExitCode -ne 0) { throw "Packaged UI exited with code $($runtime.ExitCode). See $checkRoot/stderr.txt." }
    $views = @(Get-Content -LiteralPath (Join-Path $screenshots 'complete.json') -Raw | ConvertFrom-Json)
    if ($views.Count -lt 23) { throw 'Incomplete view capture.' }
    foreach ($view in $views) {
        $bytes = [IO.File]::ReadAllBytes((Join-Path $screenshots $view))
        if ($bytes.Length -lt 100 -or [BitConverter]::ToString($bytes[0..7]) -ne '89-50-4E-47-0D-0A-1A-0A') { throw "Invalid captured image: $view" }
    }
    [ordered]@{checked_utc=[DateTime]::UtcNow.ToString('o');zip_sha256=$zipHash;exe_sha256=$packageInfo.exe_sha256;exit_code=$runtime.ExitCode;captured_views=$views.Count;hardware_access='disabled by offline backend';runtime_path=$env:PATH;isolated_app_data=$true;scope='Current Windows host; not a clean VM or physical hardware qualification'} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $checkRoot 'result.json') -Encoding utf8
    Write-Output "Package check passed: $checkRoot"
} finally {
    $env:PATH=$oldPath
    $env:LOCALAPPDATA=$oldData
    $env:TEMP=$oldTemp
    $env:TMP=$oldTmp
}
