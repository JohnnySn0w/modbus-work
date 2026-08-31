param([string]$PortName = 'COM5')

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
    $banner = Read-Serial 1000
    if ($banner -notmatch 'Password:') {
        $port.Write("`r")
        $banner += Read-Serial 2500
    }
    $match = [regex]::Match($banner, 'DevEui:\s*([0-9A-Fa-f-]+)')
    if (-not $match.Success) { throw 'Unable to read DevEUI. Cycle USB and retry.' }
    $password = ($match.Groups[1].Value -replace '-', '').Substring(12, 4)
    $port.Write("$password`r")
    $menu = Read-Serial 2500
    if ($menu -notmatch 'enLink Main Menu') { throw 'Unable to authenticate.' }
    Write-Host $menu
    Write-Host 'Authenticated console active. Ctrl+] exits locally.'

    while ($true) {
        if ($port.BytesToRead -gt 0) { [Console]::Write($port.ReadExisting()) }
        while ([Console]::KeyAvailable) {
            $key = [Console]::ReadKey($true)
            if ($key.KeyChar -eq [char]29) { return }
            if ($key.Key -eq [ConsoleKey]::Enter) { $port.Write("`r") }
            elseif ($key.KeyChar -ne [char]0) { $port.Write($key.KeyChar.ToString()) }
        }
        Start-Sleep -Milliseconds 20
    }
}
finally {
    if ($port.IsOpen) { $port.Close() }
    $port.Dispose()
}
