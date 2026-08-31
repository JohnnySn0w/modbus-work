param(
    [string]$PortName = 'COM5',
    [string[]]$Commands = @(),
    [switch]$ResumeOpenMenu
)

$port = [System.IO.Ports.SerialPort]::new($PortName, 115200, 'None', 8, 'One')
$port.ReadTimeout = 500
$port.WriteTimeout = 500
$port.DtrEnable = $true
$port.RtsEnable = $false

function Read-Serial([int]$Milliseconds) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
    $value = ''
    while ([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 100
        $value += $port.ReadExisting()
    }
    return $value
}

try {
    $port.Open()
    if ($ResumeOpenMenu) {
        $output = ''
    }
    else {
        $banner = Read-Serial 1000
        if ($banner -match 'enlink Main Menu') {
            $output = $banner
        }
        else {
            if ($banner -notmatch 'Password:') {
                $port.Write("`r")
                $banner += Read-Serial 2500
            }

            $devEuiMatch = [regex]::Match($banner, 'DevEui:\s*([0-9A-Fa-f-]+)')
            if (-not $devEuiMatch.Success) {
                throw 'Unable to read DevEUI from enLink banner. Cycle USB and retry.'
            }

            $password = ($devEuiMatch.Groups[1].Value -replace '-', '').Substring(12, 4)
            $port.Write("$password`r")
            $output = Read-Serial 2500
            if ($output -notmatch 'enlink Main Menu') {
                throw 'Unable to log in to enLink device.'
            }
        }
    }

    foreach ($command in $Commands) {
        $port.Write("$command`r")
        $output += Read-Serial 2000
    }

    # Do not print any key values if a traversed menu happens to contain them.
    $output = [regex]::Replace($output, '(?im)^(\s*(?:AppKey|NwkKey)\s+).+$', '$1[REDACTED]')
    Write-Output $output
}
finally {
    if ($port.IsOpen) {
        $port.Close()
    }
    $port.Dispose()
}
