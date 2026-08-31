param(
    [string]$PortName = 'COM5',
    [int]$BaudRate = 115200
)

$port = [System.IO.Ports.SerialPort]::new(
    $PortName,
    $BaudRate,
    [System.IO.Ports.Parity]::None,
    8,
    [System.IO.Ports.StopBits]::One
)
$port.ReadTimeout = 100
$port.WriteTimeout = 500
$port.DtrEnable = $true
$port.RtsEnable = $false

try {
    $port.Open()
    Write-Host "Connected to $PortName. Keystrokes pass directly to the bridge. Ctrl+C exits."

    while ($true) {
        if ($port.BytesToRead -gt 0) {
            [Console]::Write($port.ReadExisting())
        }

        while ([Console]::KeyAvailable) {
            $key = [Console]::ReadKey($true)
            if ($key.Key -eq [ConsoleKey]::Enter) {
                $port.Write("`r")
            }
            elseif ($key.KeyChar -ne [char]0) {
                $port.Write($key.KeyChar.ToString())
            }
        }

        Start-Sleep -Milliseconds 20
    }
}
finally {
    if ($port.IsOpen) {
        $port.Close()
    }
    $port.Dispose()
}
