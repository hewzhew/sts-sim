param(
    [string]$ConfigPath = $(Join-Path $env:LOCALAPPDATA "ModTheSpire\CommunicationMod\config.properties"),
    [string]$LauncherPath = $(Join-Path $PSScriptRoot "launch_live_comm.ps1"),
    [bool]$RunAtGameStart = $true,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Set-PropertyLine {
    param(
        [string[]]$Lines,
        [string]$Key,
        [string]$Value
    )

    $pattern = '^\s*' + [regex]::Escape($Key) + '='
    $updated = $false
    $result = New-Object System.Collections.Generic.List[string]
    foreach ($line in $Lines) {
        if ($line -match $pattern) {
            $result.Add("$Key=$Value")
            $updated = $true
        } else {
            $result.Add($line)
        }
    }
    if (-not $updated) {
        $result.Add("$Key=$Value")
    }
    return ,$result.ToArray()
}

if (-not (Test-Path -LiteralPath $LauncherPath)) {
    throw "Launcher script not found: $LauncherPath"
}

$resolvedLauncherPath = (Resolve-Path -LiteralPath $LauncherPath).Path
$normalizedLauncherPath = $resolvedLauncherPath -replace '\\', '/'
$commandValue = "powershell -ExecutionPolicy Bypass -File $normalizedLauncherPath"

$lines = @()
if (Test-Path -LiteralPath $ConfigPath) {
    $lines = Get-Content -LiteralPath $ConfigPath
}

$lines = Set-PropertyLine -Lines $lines -Key "command" -Value $commandValue
$lines = Set-PropertyLine -Lines $lines -Key "runAtGameStart" -Value ($RunAtGameStart.ToString().ToLowerInvariant())

if ($DryRun) {
    [ordered]@{
        config_path = $ConfigPath
        command = $commandValue
        runAtGameStart = $RunAtGameStart
        lines = $lines
    } | ConvertTo-Json -Depth 4
    exit 0
}

$configDir = Split-Path -Parent $ConfigPath
if (-not (Test-Path -LiteralPath $configDir)) {
    New-Item -ItemType Directory -Path $configDir | Out-Null
}

if (Test-Path -LiteralPath $ConfigPath) {
    $backupPath = "$ConfigPath.bak"
    Copy-Item -LiteralPath $ConfigPath -Destination $backupPath -Force
}

Set-Content -LiteralPath $ConfigPath -Value $lines -Encoding UTF8
Write-Output "Updated $ConfigPath"
