param(
    [string]$ConfigPath = $(Join-Path $env:LOCALAPPDATA "ModTheSpire\CommunicationMod\config.properties"),
    [bool]$RunAtGameStart = $true,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$launcherPath = Join-Path $PSScriptRoot "launch_manual_client.ps1"
$installerPath = Join-Path $PSScriptRoot "install_communicationmod_config.ps1"

& $installerPath `
    -ConfigPath $ConfigPath `
    -LauncherPath $launcherPath `
    -RunAtGameStart:$RunAtGameStart `
    -DryRun:$DryRun
exit $LASTEXITCODE
