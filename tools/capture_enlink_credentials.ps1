param(
    [string]$PortName = 'COM5',
    [string]$OutputDirectory = '.secrets'
)

$port = [System.IO.Ports.SerialPort]::new($PortName, 115200, 'None', 8, 'One')
$port.ReadTimeout = 500
$port.WriteTimeout = 500
$port.DtrEnable = $true
$port.RtsEnable = $false

function Read-Serial([int]$Milliseconds) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
    $text = ''
    while ([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 100
        $text += $port.ReadExisting()
    }
    return $text
}

try {
    $port.Open()
    $banner = Read-Serial 1000
    if ($banner -notmatch 'Password:') {
        $port.Write("`r")
        $banner += Read-Serial 2500
    }

    $devEuiMatch = [regex]::Match($banner, 'DevEui:\s*([0-9A-Fa-f-]+)')
    if (-not $devEuiMatch.Success) {
        throw 'Unable to read DevEUI from bridge banner.'
    }

    $devEui = $devEuiMatch.Groups[1].Value
    $password = ($devEui -replace '-', '').Substring(12, 4)
    $port.Write("$password`r")
    $menu = Read-Serial 2500
    if ($menu -notmatch 'enLink Main Menu') {
        throw 'Unable to log in to bridge.'
    }

    $port.Write("L`r")
    $radio = Read-Serial 3000
    $appEuiMatch = [regex]::Match($radio, 'AppEui\s+([0-9A-Fa-f-]+)')
    $appKeyMatch = [regex]::Match($radio, 'AppKey\s+([0-9A-Fa-f-]+)')
    if (-not $appEuiMatch.Success -or -not $appKeyMatch.Success) {
        throw 'Unable to read AppEUI/AppKey from radio settings.'
    }

    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
    $outputPath = Join-Path $OutputDirectory 'polygon-enl-mod-32-lorawan.env'
    $lines = @(
        '# Polygon ExactAire-E5 / Synetica ENL-MOD-32 LoRaWAN credentials'
        '# Captured locally from firmware 3.6; do not commit or include in technician exports.'
        "LORAWAN_DEV_EUI=$devEui"
        "LORAWAN_APP_EUI=$($appEuiMatch.Groups[1].Value)"
        "LORAWAN_APP_KEY=$($appKeyMatch.Groups[1].Value)"
        '# Firmware 3.6 does not expose a separate provisioned NwkKey in its radio menu.'
        'LORAWAN_NWK_KEY='
    )
    [System.IO.File]::WriteAllLines((Join-Path (Resolve-Path $OutputDirectory).Path 'polygon-enl-mod-32-lorawan.env'), $lines, [System.Text.UTF8Encoding]::new($false))

    Write-Output "Recorded DevEUI, AppEUI, and AppKey in $outputPath. No credential values were printed."
}
finally {
    if ($port.IsOpen) {
        $port.Close()
    }
    $port.Dispose()
}

