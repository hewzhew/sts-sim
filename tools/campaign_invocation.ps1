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

function Test-ExtraCombatOptionKey {
    param(
        [string[]] $Tokens,
        [string[]] $Keys
    )

    foreach ($Arg in $Tokens) {
        foreach ($Key in $Keys) {
            if ($Arg -match "(^|\s|=)$([regex]::Escape($Key))=") {
                return $true
            }
        }
    }
    return $false
}

function Add-CampaignSharedDriverOptions {
    param(
        [string[]] $Arguments,
        [bool] $IncludeActiveLineageDiversity = $false,
        [bool] $IncludeBossRelicAxes = $false,
        [bool] $IncludeAutoCaptureCombat = $true
    )

    $Args = @($Arguments)
    if ($CampaignBoundParameters.ContainsKey("ExperimentWallMs")) {
        $Args += @("--experiment-wall-ms", "$ExperimentWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("SearchWallMs")) {
        $Args += @("--search-wall-ms", "$SearchWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("SearchMaxNodes")) {
        $Args += @("--search-max-nodes", "$SearchMaxNodes")
    }
    if ($IncludeActiveLineageDiversity -and $CampaignBoundParameters.ContainsKey("ActiveLineageDiversity") -and $ActiveLineageDiversity -ge 0) {
        $Args += @("--active-lineage-diversity", "$ActiveLineageDiversity")
    }
    if ($IncludeBossRelicAxes -and $BossRelicAxes) {
        $Args += "--boss-relic-axes"
    }
    if ($CampaignBoundParameters.ContainsKey("CombatRetryWallMs") -and $CombatRetryWallMs -gt 0) {
        $Args += @("--combat-retry-wall-ms", "$CombatRetryWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("BranchExamples")) {
        $Args += @("--branch-examples", "$BranchExamples")
    }
    if ($CampaignBoundParameters.ContainsKey("VictoryHpPercent")) {
        $Args += @("--min-acceptable-victory-hp-percent", "$VictoryHpPercent")
    }
    if ($IncludeAutoCaptureCombat -and $AutoCaptureCombat) {
        $Args += "--auto-capture-combat"
        if ($AutoCaptureRoot) {
            $Args += @("--auto-capture-root", "$AutoCaptureRoot")
        }
    }
    if (-not (Test-ExtraCombatOptionKey -Tokens $ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
        if ($BossSegments) {
            $Args += @("--combat-search-option", "segment=turn")
        } else {
            $Args += @("--combat-search-option", "segment=non_boss_turn")
        }
    }
    if (-not $NoProgress) {
        $Args += "--progress"
        if ($VerboseProgress) {
            $Args += @("--progress-detail", "verbose")
        }
    }
    if ($Perf) {
        $Args += @("--report-detail", "perf")
    } elseif ($Diagnose) {
        $Args += @("--report-detail", "diagnose")
    }
    if ($ExtraArgs) {
        $Args += $ExtraArgs
    }
    return $Args
}

function Resolve-CampaignAdditionalRoundBudget {
    param(
        [int] $ResumeRoundsCompleted,
        [bool] $UntilMilestoneBound,
        [int] $MilestoneStepRounds,
        [bool] $RoundsBound,
        [int] $Rounds,
        [bool] $UntilRoundBound,
        [int] $UntilRound,
        [bool] $MaxRoundsBound,
        [int] $MaxRounds,
        [string] $MaxRoundsDriverFlag = "--max-rounds"
    )

    $ContinuationRounds = 1
    $RoundBudgetArgs = @("--rounds", "1")
    $TargetRounds = $null
    $Source = "default"
    if ($UntilMilestoneBound) {
        $ContinuationRounds = $MilestoneStepRounds
        $Source = "UntilMilestone"
        $RoundBudgetArgs = @("--rounds", "$MilestoneStepRounds")
    } elseif ($RoundsBound) {
        $ContinuationRounds = $Rounds
        $Source = "Rounds"
        $TargetRounds = $ResumeRoundsCompleted + $Rounds
        $RoundBudgetArgs = @("--rounds", "$Rounds")
    } elseif ($UntilRoundBound) {
        $TargetRounds = $UntilRound
        $ContinuationRounds = [Math]::Max(0, $TargetRounds - $ResumeRoundsCompleted)
        $Source = "UntilRound"
        $RoundBudgetArgs = @("--until-round", "$UntilRound")
    } elseif ($MaxRoundsBound) {
        $ContinuationRounds = $MaxRounds
        $Source = "MaxRounds"
        $TargetRounds = $ResumeRoundsCompleted + $MaxRounds
        $RoundBudgetArgs = @($MaxRoundsDriverFlag, "$ContinuationRounds")
    }

    return [pscustomobject]@{
        AdditionalRounds = $ContinuationRounds
        Args = @($RoundBudgetArgs)
        TargetRounds = $TargetRounds
        Source = $Source
    }
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

    if ($RunOutputArtifact) {
        Set-Content -LiteralPath $RunCommandPath -Value $PrimaryDriverCommandLine
        if ($RunOutputArtifact.Kind -eq "run") {
            Write-CampaignLatestPointer -Artifact $RunOutputArtifact
            Write-Host "latest-pointer=$(Get-CampaignLatestPointerPath)"
        }
        Write-Host "primary-driver-command=$RunCommandPath"
        Write-Host "manifest=$RunManifestPath"
        return
    }

    # Legacy fallback for pre-artifact callers. New wrapper paths should set
    # $RunOutputArtifact and should not write sidecar state.
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
        output_artifact = if ($RunOutputArtifact) { "$($RunOutputArtifact.Label)" } else { "" }
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
