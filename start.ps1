Add-Type -AssemblyName System.Windows.Forms

$configFile = "$PSScriptRoot\blur.config"

function Get-SavedPath {
    if (Test-Path $configFile) {
        return Get-Content $configFile -ErrorAction SilentlyContinue
    }
    return $null
}

function Save-Path($path) {
    Set-Content -Path $configFile -Value $path
}

$gamePath = Get-SavedPath

if (-not $gamePath -or -not (Test-Path $gamePath)) {

    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    $dialog.Filter = "Blur.exe|Blur.exe|All files|*.*"
    $dialog.Title = "Select Blur.exe"

    if ($dialog.ShowDialog() -eq "OK") {
        $gamePath = $dialog.FileName
        Save-Path $gamePath
        Write-Host "[+] Saved Blur path for future runs" -ForegroundColor Green
    } else {
        exit
    }
}

function Log($msg) {
    Write-Host "[+] $msg" -ForegroundColor Cyan
}

function Disable-NonWiFi {
    Log "Disabling non-WiFi adapters..."

    Get-NetAdapter |
    Where-Object { $_.Name -ne "Wi-Fi" -and $_.Status -eq "Up" } |
    ForEach-Object {
        Log "Disabling: $($_.Name)"
        Disable-NetAdapter -Name $_.Name -Confirm:$false
    }
}

function Enable-All {
    Log "Re-enabling adapters..."

    Get-NetAdapter |
    Where-Object { $_.Status -eq "Disabled" } |
    ForEach-Object {
        Log "Enabling: $($_.Name)"
        Enable-NetAdapter -Name $_.Name -Confirm:$false
    }
}

Log "=== BLUR LAN MODE START ==="

# 1. disable first
Disable-NonWiFi

# 2. strict wait (your requirement)
Log "Waiting 5 seconds for network to settle..."
Start-Sleep -Seconds 5

# 3. launch game
Log "Launching Blur..."
$process = Start-Process $gamePath -WorkingDirectory (Split-Path $gamePath) -PassThru

Log "Game PID: $($process.Id)"
Log "Waiting for game to close..."

# 4. wait for exit
Wait-Process -Id $process.Id

# 5. restore
Enable-All

Log "=== BLUR LAN MODE END ==="
