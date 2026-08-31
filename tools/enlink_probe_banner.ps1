param([string]$PortName = 'COM5')

$port = [System.IO.Ports.SerialPort]::new($PortName, 115200, 'None', 8, 'One')
$port.ReadTimeout = 300
$port.WriteTimeout = 300
$port.DtrEnable = $true
$port.RtsEnable = $false
try {
    $port.Open()
    $deadline = [DateTime]::UtcNow.AddMilliseconds(1800)
    $text = ''
    while ([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 100
        $text += $port.ReadExisting()
    }
    Write-Output $text
}
finally {
    if ($port.IsOpen) { $port.Close() }
    $port.Dispose()
}
