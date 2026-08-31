param(
    [string]$PortName = 'COM5',
    [ValidateSet('quick_start', 'radio', 'configure', 'configure_page2')]
    [string]$Page = 'configure',
    [switch]$RedactSecrets
)

$target = @{
    quick_start = @{ Command = 'Q'; Pattern = 'enlink Quick Start Menu:' }
    radio = @{ Command = 'L'; Pattern = 'LoRa Radio Settings Menu:' }
    configure = @{ Command = 'C'; Pattern = 'Sensor Readings \(Page 1\)' }
    configure_page2 = @{ Command = 'C'; Pattern = 'Sensor Readings \(Page 2\)' }
}[$Page]

function Open-Read([int]$Milliseconds, [string]$WriteValue = '') {
    $port = [System.IO.Ports.SerialPort]::new($PortName, 115200, 'None', 8, 'One')
    $port.ReadTimeout = 300
    $port.WriteTimeout = 500
    $port.DtrEnable = $true
    $port.RtsEnable = $false
    try {
        $port.Open()
        $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
        $value = ''
        while ([DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 100
            $value += $port.ReadExisting()
        }
        if ($WriteValue) {
            if ($WriteValue -eq '__RETURN__') { $port.Write("`r") }
            else { $port.Write("$WriteValue`r") }
            Start-Sleep -Milliseconds 300
        }
        return $value
    }
    finally {
        if ($port.IsOpen) { $port.Close() }
        $port.Dispose()
    }
}

$captured = ''
$devEui = $null
for ($attempt = 0; $attempt -lt 8; $attempt++) {
    $text = Open-Read 1800
    $captured += $text
    if ($captured -match $target.Pattern) { break }

    if ($Page -eq 'configure_page2' -and $text -match 'Sensor Readings \(Page 1\)') {
        [void](Open-Read 300 '__RETURN__')
        Start-Sleep -Milliseconds 500
        continue
    }

    $match = [regex]::Match($captured, 'DevEui:\s*([0-9A-Fa-f-]+)')
    if ($match.Success) { $devEui = $match.Groups[1].Value }

    if ($text -match 'Password:') {
        if (-not $devEui) { throw 'Password prompt found without a parseable DevEUI.' }
        $password = ($devEui -replace '-', '').Substring(12, 4).ToLowerInvariant()
        [void](Open-Read 300 $password)
    }
    elseif ($text -match 'enlink Main Menu') {
        [void](Open-Read 300 $target.Command)
    }
    elseif ($text -match 'Quick Start Menu|LoRa Radio Settings Menu|Device Options:') {
        [void](Open-Read 300 'X')
    }
    Start-Sleep -Milliseconds 500
}

if ($captured -notmatch $target.Pattern) {
    throw "Unable to capture $Page from $PortName after bounded retries."
}

if ($RedactSecrets) {
    $captured = [regex]::Replace(
        $captured,
        '(?im)^(\s*[EK]\s+-\s+(?:AppEui|AppKey)\s+).+$',
        '$1[REDACTED]'
    )
}
Write-Output $captured
