param(
    [Parameter(Mandatory = $true)][string]$PortName,
    [ValidateSet('Export', 'Apply')][string]$Action = 'Export',
    [string]$ConfigPath = '',
    [string]$BackupPath = ''
)

$ErrorActionPreference = 'Stop'
$port = [System.IO.Ports.SerialPort]::new($PortName, 115200, 'None', 8, 'One')
$port.ReadTimeout = 300
$port.WriteTimeout = 500
$port.DtrEnable = $true
$port.RtsEnable = $false

function Read-Serial([int]$Milliseconds) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($Milliseconds)
    $value = ''
    while ([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 50
        $value += $port.ReadExisting()
    }
    return $value
}

function Send-Line([string]$Value, [int]$Milliseconds = 1800) {
    $port.Write("$Value`r")
    return Read-Serial $Milliseconds
}

function Reopen-AndRead([int]$Milliseconds) {
    if ($port.IsOpen) { $port.Close() }
    Start-Sleep -Milliseconds 350
    $port.Open()
    return Read-Serial $Milliseconds
}

function Require-Text([string]$Text, [string]$Pattern, [string]$Step) {
    if ($Text -notmatch $Pattern) { throw "$Step failed: expected console pattern '$Pattern'." }
}

function Open-MainMenu {
    $latest = Read-Serial 1600
    $text = $latest
    $devEui = ''
    for ($attempt = 0; $attempt -lt 10 -and $text -notmatch 'enLink Main Menu:'; $attempt++) {
        $eui = [regex]::Match($text, 'DevEui:\s*([0-9A-Fa-f-]+)')
        if ($eui.Success) { $devEui = $eui.Groups[1].Value }
        if ($latest -match 'Password:') {
            if (-not $devEui) { throw 'Bridge requested a password before providing a DevEUI.' }
            $normalized = $devEui -replace '-', ''
            $password = $normalized.Substring($normalized.Length - 4, 4).ToLowerInvariant()
            $latest = Send-Line $password 2500
        }
        elseif ($latest -match 'Press a key to continue|Finish the import with an empty line') {
            $latest = Send-Line '' 1800
        }
        elseif ($latest -match 'Menu:') {
            $latest = Send-Line 'X' 1800
        }
        else {
            $latest = Send-Line '' 2400
        }
        $text += $latest
    }
    Require-Text $text 'enLink Main Menu:' 'Authentication'
    $model = [regex]::Match($text, 'Model Number:\s*(\S+)').Groups[1].Value
    $firmware = [regex]::Match($text, 'Firmware Ver:\s*(\S+)').Groups[1].Value
    if ($model -ne 'ENL-MOD-32' -or $firmware -ne '3.6') {
        throw "Unsupported bridge compatibility identity: $model firmware $firmware"
    }
    return @{ Text = $text; Model = $model; Firmware = $firmware }
}

function Enter-ImportExport {
    $text = Send-Line 'C' 1800
    Require-Text $text 'Modbus Configuration Menu:' 'Open Modbus configuration'
    $text = Send-Line 'M' 1800
    Require-Text $text 'Modbus Import/Export Menu:' 'Open Import/Export'
}

function Parse-Rows([string]$Text) {
    return @($Text -split "`r?`n" | Where-Object {
        $_ -match '^\d+\t\d+\t(?:Hold|Input)\t\d+\t\S+\t\S+\t\S+\t\S+$'
    })
}

function Export-Rows {
    $text = Send-Line 'E' 3000
    Require-Text $text 'Press a key to continue' 'Export point table'
    $rows = @(Parse-Rows $text)
    [void](Send-Line '' 1200)
    return $rows
}

function Submit-Rows([string[]]$Rows, [string]$SuccessText) {
    $text = Send-Line 'I' 1800
    Require-Text $text 'Finish the import with an empty line' 'Open tab-delimited import'
    $acks = @()
    foreach ($row in $Rows) {
        $response = Send-Line $row 900
        Require-Text $response ([regex]::Escape($SuccessText)) "Submit item $($row.Split("`t")[0])"
        $acks += ($response -split "`r?`n" | Where-Object { $_ -match [regex]::Escape($SuccessText) } | Select-Object -Last 1).Trim()
    }
    $finished = Send-Line '' 1800
    Require-Text $finished 'Import Finished' 'Finish import'
    [void](Send-Line '' 1200)
    return $acks
}

function Write-Table([string]$Path, [string[]]$Rows) {
    $header = "Item`tID`tReg`tAddr`tData`tWord`tMult`tRead"
    [System.IO.File]::WriteAllText($Path, $header + "`r`n" + ($Rows -join "`r`n") + "`r`n")
}

try {
    $port.Open()
    $identity = Open-MainMenu
    Enter-ImportExport
    $backupRows = @(Export-Rows)
    if ($BackupPath) { Write-Table $BackupPath $backupRows }

    if ($Action -eq 'Export') {
        [pscustomobject]@{
            ok = $true; model = $identity.Model; firmware = $identity.Firmware
            backupRows = $backupRows; appliedRows = @(); readbackRows = @()
            acknowledgements = @(); readSummary = ''
        } | ConvertTo-Json -Depth 4 -Compress
        exit 0
    }

    if (-not $ConfigPath) { throw 'Apply requires ConfigPath.' }
    $targetRows = @(Get-Content -LiteralPath $ConfigPath | Select-Object -Skip 1 | Where-Object { $_.Trim() })
    if ($targetRows.Count -lt 1 -or $targetRows.Count -gt 32) { throw 'Configuration must contain 1..32 rows.' }

    $deleteRows = @($backupRows | ForEach-Object {
        $columns = $_.Split("`t")
        $columns[1] = '0'
        $columns -join "`t"
    })
    if ($deleteRows.Count) { [void](Submit-Rows $deleteRows 'deleted OK') }
    $acks = @(Submit-Rows $targetRows 'imported OK')
    $readback = @(Export-Rows)
    if (($readback -join "`n") -ne ($targetRows -join "`n")) {
        throw 'Bridge exported readback does not match the selected configuration.'
    }

    $menu = Send-Line 'X' 1500
    Require-Text $menu 'Modbus Configuration Menu:' 'Return to Modbus configuration'
    $read = Send-Line 'A' 12000
    if ($read -notmatch 'Modbus read completed') {
        $read += Reopen-AndRead 6000
    }
    Require-Text $read 'Modbus read completed' 'Read All Data Points'
    $menuAfterRead = Send-Line '' 1800
    if ($menuAfterRead -notmatch 'Modbus Configuration Menu:') {
        $menuAfterRead += Reopen-AndRead 3000
    }
    $summaryMatches = [regex]::Matches($menuAfterRead, '\d+/\d+\s*\(OK/Exceptions\)')
    if ($summaryMatches.Count -eq 0) { throw 'Read All Data Points did not return a summary.' }
    $summary = $summaryMatches[$summaryMatches.Count - 1].Value
    $expected = "$($targetRows.Count)/0 (OK/Exceptions)"
    if ($summary -ne $expected) { throw "Read All verification returned $summary; expected $expected." }

    [pscustomobject]@{
        ok = $true; model = $identity.Model; firmware = $identity.Firmware
        backupRows = $backupRows; appliedRows = $targetRows; readbackRows = $readback
        acknowledgements = $acks; readSummary = $summary
    } | ConvertTo-Json -Depth 4 -Compress
}
finally {
    if ($port.IsOpen) { $port.Close() }
    $port.Dispose()
}
