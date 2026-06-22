function Convert-CampaignWrapperParameterValue {
    param(
        [object] $Value
    )

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [System.Management.Automation.SwitchParameter]) {
        return [bool] $Value
    }
    if ($Value -is [System.Array]) {
        $Converted = @()
        foreach ($Item in $Value) {
            $Converted += (Convert-CampaignWrapperParameterValue -Value $Item)
        }
        return ,$Converted
    }
    return $Value
}

function Format-CommandLine {
    param(
        [string] $ExePath,
        [string[]] $Arguments
    )

    $RenderedExe = if ($ExePath -match '^[A-Za-z0-9_./:=\\-]+$') { $ExePath } else { "'$($ExePath -replace "'", "''")'" }
    $RenderedArgs = $Arguments | ForEach-Object {
        if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
    }
    return $RenderedExe + " " + ($RenderedArgs -join " ")
}

function Write-CampaignWrapperManifest {
    param(
        [string] $Path,
        [object] $Manifest
    )

    if (-not $Path) {
        return
    }
    $Parent = Split-Path -Parent $Path
    if ($Parent) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $Manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Path
}

function Write-CampaignPrimaryDriverCommandRecord {
    param(
        [string] $PrimaryDriverCommandLine
    )

    if ($Scratch) {
        Set-Content -LiteralPath $RunCommandPath -Value $PrimaryDriverCommandLine
        Write-Host "scratch-primary-driver-command=$RunCommandPath"
        Write-Host "scratch-manifest=$RunManifestPath"
        return
    }

    Set-Content -LiteralPath $LatestSeedPath -Value $Seed
    Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
    Set-Content -LiteralPath $LatestClassPath -Value $Class
    Set-Content -LiteralPath $LatestModePath -Value $Mode
    Set-Content -LiteralPath $LatestCommandPath -Value $PrimaryDriverCommandLine
}

function Invoke-CampaignLoggedDriverCommand {
    param(
        [string] $ExePath,
        [string[]] $Arguments,
        [string] $LogPath
    )

    $LogParent = Split-Path -Parent $LogPath
    if ($LogParent) {
        New-Item -ItemType Directory -Force -Path $LogParent | Out-Null
    }
    $DriverStderrLogPath = "$LogPath.stderr.tmp"
    Remove-Item -LiteralPath $LogPath, $DriverStderrLogPath -Force -ErrorAction SilentlyContinue
    $PreviousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        & $ExePath @Arguments 2> $DriverStderrLogPath |
            Tee-Object -FilePath $LogPath |
            ForEach-Object { Write-Host $_ }
        $ExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
    if (Test-Path -LiteralPath $DriverStderrLogPath) {
        $DriverStderrText = Get-Content -LiteralPath $DriverStderrLogPath -Raw
        if ($DriverStderrText) {
            Add-Content -LiteralPath $LogPath -Value ""
            Add-Content -LiteralPath $LogPath -Value "[stderr]"
            Add-Content -LiteralPath $LogPath -Value $DriverStderrText
        }
        Remove-Item -LiteralPath $DriverStderrLogPath -Force -ErrorAction SilentlyContinue
    }
    return $ExitCode
}

function New-CampaignWrapperManifestBase {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string] $CommandKind,
        [string[]] $PrimaryDriverArgs,
        [string] $PrimaryDriverCommand
    )

    return [ordered]@{
        schema_name = "CampaignWrapperManifestV1"
        schema_version = 1
        created_at = (Get-Date).ToString("o")
        stage = $Stage
        exit_code = $ExitCode
        wrapper_script = $PSCommandPath
        command_kind = $CommandKind
        mode = $Mode
        seed = $Seed
        ascension = $Ascension
        class = $Class
        build_profile = $BuildProfile
        driver_exe = "$DriverExe"
        scratch = [bool] $Scratch
        scratch_label = $ScratchLabel
        output_report = "$RunOutputCampaignPath"
        output_checkpoint = "$RunOutputCheckpointPath"
        command_file_semantics = "primary_driver_command"
        command_file = "$RunCommandPath"
        manifest_file = "$RunManifestPath"
        wrapper_invocation = [ordered]@{
            line = $CampaignWrapperInvocationLine
            bound_parameters = $CampaignWrapperBoundParameters
        }
        primary_driver = [ordered]@{
            args = @($PrimaryDriverArgs)
            command = $PrimaryDriverCommand
            command_file = "$RunCommandPath"
        }
    }
}
