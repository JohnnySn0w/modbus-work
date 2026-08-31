$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$python = Get-Command python -ErrorAction SilentlyContinue

if (-not $python) {
    $python = Get-Command py -ErrorAction SilentlyContinue
}

if (-not $python) {
    throw 'Python 3 was not found. Install Python 3.11 or newer, then run: pip install -r requirements.txt'
}

Push-Location $root
try {
    & $python.Source -m app.main
}
finally {
    Pop-Location
}
