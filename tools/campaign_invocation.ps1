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

function Resolve-CampaignBoundParameterContext {
    param(
        [System.Collections.IDictionary] $BoundParameters,
        [string] $InvocationLine,
        [string] $UntilMilestone
    )

    $CampaignBoundParameters = @{}
    foreach ($ParameterName in $BoundParameters.Keys) {
        $CampaignBoundParameters[$ParameterName] = $true
    }

    $CampaignWrapperBoundParameters = [ordered]@{}
    foreach ($ParameterName in ($BoundParameters.Keys | Sort-Object)) {
        $CampaignWrapperBoundParameters[$ParameterName] =
            Convert-CampaignWrapperParameterValue -Value $BoundParameters[$ParameterName]
    }

    return [pscustomobject]@{
        CampaignBoundParameters = $CampaignBoundParameters
        WrapperInvocationLine = if ($InvocationLine) { $InvocationLine.Trim() } else { "" }
        WrapperBoundParameters = $CampaignWrapperBoundParameters
        RoundsBound = $CampaignBoundParameters.ContainsKey("Rounds")
        UntilRoundBound = $CampaignBoundParameters.ContainsKey("UntilRound")
        UntilMilestoneBound = $CampaignBoundParameters.ContainsKey("UntilMilestone") -and $UntilMilestone
        MaxRoundsBound = $CampaignBoundParameters.ContainsKey("MaxRounds")
    }
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

function New-CampaignRunDriverIdentityArgs {
    param(
        [string] $Mode,
        [long] $Seed,
        [int] $Ascension,
        [string] $Class
    )

    $Args = @(
        "run",
        "--preset", "$Mode",
        "--seed", "$Seed",
        "--ascension", "$Ascension",
        "--class", "$Class"
    )
    if (@(0, 10, 15, 17, 20) -contains $Ascension) {
        $Args += @("--ascension-domain", "a$Ascension")
    }
    return $Args
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

function New-CampaignSharedDriverOptionContext {
    param(
        [System.Collections.IDictionary] $CampaignBoundParameters,
        [int] $ExperimentWallMs,
        [int] $SearchWallMs,
        [int] $SearchMaxNodes,
        [int] $ActiveLineageDiversity,
        [bool] $BossRelicAxes,
        [int] $CombatRetryWallMs,
        [int] $BranchExamples,
        [int] $VictoryHpPercent,
        [bool] $AutoCaptureCombat,
        [string] $AutoCaptureRoot,
        [string[]] $ExtraArgs,
        [bool] $BossSegments,
        [bool] $NoProgress,
        [bool] $VerboseProgress,
        [bool] $Perf,
        [bool] $Diagnose
    )

    $ResolvedExtraArgs = @()
    if ($ExtraArgs) {
        $ResolvedExtraArgs = @($ExtraArgs)
    }

    return [pscustomobject]@{
        BoundParameters = $CampaignBoundParameters
        ExperimentWallMs = $ExperimentWallMs
        SearchWallMs = $SearchWallMs
        SearchMaxNodes = $SearchMaxNodes
        ActiveLineageDiversity = $ActiveLineageDiversity
        BossRelicAxes = [bool] $BossRelicAxes
        CombatRetryWallMs = $CombatRetryWallMs
        BranchExamples = $BranchExamples
        VictoryHpPercent = $VictoryHpPercent
        AutoCaptureCombat = [bool] $AutoCaptureCombat
        AutoCaptureRoot = $AutoCaptureRoot
        ExtraArgs = $ResolvedExtraArgs
        BossSegments = [bool] $BossSegments
        NoProgress = [bool] $NoProgress
        VerboseProgress = [bool] $VerboseProgress
        Perf = [bool] $Perf
        Diagnose = [bool] $Diagnose
    }
}

function Test-CampaignSharedDriverOptionBound {
    param(
        [object] $OptionContext,
        [string] $Name
    )

    return $OptionContext.BoundParameters -and $OptionContext.BoundParameters.ContainsKey($Name)
}

function Add-CampaignSharedDriverOptions {
    param(
        [string[]] $Arguments,
        [bool] $IncludeActiveLineageDiversity = $false,
        [bool] $IncludeBossRelicAxes = $false,
        [bool] $IncludeAutoCaptureCombat = $true,
        [object] $OptionContext
    )

    if ($null -eq $OptionContext) {
        throw "Internal error: campaign shared driver option context was not initialized."
    }

    $Args = @($Arguments)
    if (Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "ExperimentWallMs") {
        $Args += @("--experiment-wall-ms", "$($OptionContext.ExperimentWallMs)")
    }
    if (Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "SearchWallMs") {
        $Args += @("--search-wall-ms", "$($OptionContext.SearchWallMs)")
    }
    if (Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "SearchMaxNodes") {
        $Args += @("--search-max-nodes", "$($OptionContext.SearchMaxNodes)")
    }
    if ($IncludeActiveLineageDiversity -and (Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "ActiveLineageDiversity") -and $OptionContext.ActiveLineageDiversity -ge 0) {
        $Args += @("--active-lineage-diversity", "$($OptionContext.ActiveLineageDiversity)")
    }
    if ($IncludeBossRelicAxes -and $OptionContext.BossRelicAxes) {
        $Args += "--boss-relic-axes"
    }
    if ((Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "CombatRetryWallMs") -and $OptionContext.CombatRetryWallMs -gt 0) {
        $Args += @("--combat-retry-wall-ms", "$($OptionContext.CombatRetryWallMs)")
    }
    if (Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "BranchExamples") {
        $Args += @("--branch-examples", "$($OptionContext.BranchExamples)")
    }
    if (Test-CampaignSharedDriverOptionBound -OptionContext $OptionContext -Name "VictoryHpPercent") {
        $Args += @("--min-acceptable-victory-hp-percent", "$($OptionContext.VictoryHpPercent)")
    }
    if ($IncludeAutoCaptureCombat -and $OptionContext.AutoCaptureCombat) {
        $Args += "--auto-capture-combat"
        if ($OptionContext.AutoCaptureRoot) {
            $Args += @("--auto-capture-root", "$($OptionContext.AutoCaptureRoot)")
        }
    }
    if (-not (Test-ExtraCombatOptionKey -Tokens $OptionContext.ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
        if ($OptionContext.BossSegments) {
            $Args += @("--combat-search-option", "segment=turn")
        } else {
            $Args += @("--combat-search-option", "segment=non_boss_turn")
        }
    }
    if (-not $OptionContext.NoProgress) {
        $Args += "--progress"
        if ($OptionContext.VerboseProgress) {
            $Args += @("--progress-detail", "verbose")
        }
    }
    if ($OptionContext.Perf) {
        $Args += @("--report-detail", "perf")
    } elseif ($OptionContext.Diagnose) {
        $Args += @("--report-detail", "diagnose")
    }
    if ($OptionContext.ExtraArgs) {
        $Args += $OptionContext.ExtraArgs
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

function Write-CampaignRunPreflight {
    Write-Host "seed=$Seed"
    Write-Host "ascension=A$Ascension domain=a$Ascension class=$Class"
    Write-Host "mode=$Mode branch campaign"
    Write-Host "build=$BuildProfile exe=$DriverExe"
    if ($NeedsBuild) {
        Write-Host "build-needed=yes"
    } else {
        Write-Host "build-needed=no"
    }
    if ($BossRelicAxes) {
        Write-Host "boss-relic-axes=on active/frozen budgets are per boss relic lineage"
    }
    Write-Host "rerun-last=.\tools\campaign.ps1 -Last"
    Write-Host "continue-latest=.\tools\campaign.ps1 -From latest -Continue"
    Write-Host "continue-one-round=.\tools\campaign.ps1 -From latest -Continue -Rounds 1"
    Write-Host "report=$RunOutputCampaignPath"
    Write-Host "checkpoint=$RunOutputCheckpointPath"
    Write-Host "manifest=$RunManifestPath"
    if ($Log) {
        Write-Host "log=$RunLogPath"
    }
    Write-Host "combat-segment=$CombatSegmentMode"
    if ($ResumeCampaignPath) {
        Write-Host "resume=$ResumeCampaignPath"
        Write-Host "resume-rounds=$ResumeRoundsCompleted"
        if ($TargetRounds -ne $null) {
            Write-Host "round-budget=$RoundBudgetSource target-rounds=$TargetRounds additional-rounds=$MaxRounds"
        } elseif ($RoundBudgetSource -ne "preset") {
            Write-Host "round-budget=$RoundBudgetSource additional-rounds=$MaxRounds"
        } else {
            Write-Host "round-budget=preset additional-rounds=mode-default"
        }
        if ($ResumeCheckpointPath) {
            Write-Host "resume-checkpoint=$ResumeCheckpointPath"
        } else {
            Write-Host "resume-checkpoint=missing; falling back to replay"
        }
    }
    if ($UntilMilestoneBound) {
        Write-Host "until-milestone=$UntilMilestone step-rounds=$MilestoneStepRounds max-additional-rounds=$MilestoneMaxRounds"
    }
}

function New-CampaignRunWrapperManifest {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    $Manifest = New-CampaignWrapperManifestBase `
        -ExitCode $ExitCode `
        -Stage $Stage `
        -CommandKind "campaign_run" `
        -PrimaryDriverArgs $DriverArgs `
        -PrimaryDriverCommand $RenderedCommand
    $Manifest["resume_report"] = if ($ResumeCampaignPath) { "$ResumeCampaignPath" } else { "" }
    $Manifest["resume_checkpoint"] = if ($ResumeCheckpointPath) { "$ResumeCheckpointPath" } else { "" }
    $Manifest["log_file"] = if ($Log) { "$RunLogPath" } else { "" }
    $Manifest["round_budget"] = [ordered]@{
        source = $RoundBudgetSource
        target_rounds = $TargetRounds
        additional_rounds = $MaxRounds
    }

    if ($UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs -RunIdentityArgs $RunIdentityArgs -StepRounds $MilestoneStepRounds -OptionContext $OptionContext
        $Manifest["milestone"] = [ordered]@{
            target = $UntilMilestone
            stop = $ResolvedMilestoneStop
            step_rounds = $MilestoneStepRounds
            max_additional_rounds = $MilestoneMaxRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $DriverExe -Arguments $MilestoneResumeArgs)
        }
    }

    return $Manifest
}

function Invoke-CampaignRunCommand {
    param(
        [bool] $DryRun,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    if ($ContinueCampaign -and $TargetRounds -ne $null -and $MaxRounds -eq 0) {
        Write-Host "already-at-target-rounds=yes; nothing to run"
        return 0
    }
    if ($ContinueCampaign -and $UntilMilestoneBound) {
        $InitialMilestoneStatus = Get-CampaignMilestoneStatus -ReportPath $ResumeCampaignPath -Milestone $UntilMilestone
        if ($InitialMilestoneStatus.Reached) {
            Write-Host "already-at-milestone=yes target=$UntilMilestone hits=$($InitialMilestoneStatus.HitCount) furthest=A$($InitialMilestoneStatus.FurthestAct)F$($InitialMilestoneStatus.FurthestFloor)"
            return 0
        }
    }

    if ($DryRun) {
        if ($NeedsBuild) {
            Write-CampaignBuildCommandPreview -BuildArgs $BuildArgs
        }
        Write-Host $RenderedCommand
        if ($UntilMilestoneBound) {
            Write-Host "milestone-loop-command-template:"
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-MilestoneResumeDriverArgs -RunIdentityArgs $RunIdentityArgs -StepRounds $MilestoneStepRounds -OptionContext $OptionContext))
        }
        return 0
    }

    Push-Location $RepoRoot
    try {
        if ($NeedsBuild) {
            & cargo @BuildArgs
            if ($LASTEXITCODE -ne 0) {
                return $LASTEXITCODE
            }
        }
        if ($Log) {
            $DriverExitCode = Invoke-CampaignLoggedDriverCommand -ExePath $DriverExe -Arguments $DriverArgs -LogPath $RunLogPath
        } else {
            & $DriverExe @DriverArgs
            $DriverExitCode = $LASTEXITCODE
        }
        if ($DriverExitCode -eq 0) {
            Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine $RenderedCommand
            Write-CampaignWrapperManifest `
                -Path $RunManifestPath `
                -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage "initial_driver_completed" -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext)
            if ($UntilMilestoneBound) {
                Invoke-CampaignUntilMilestone -AlreadySpentRounds $MaxRounds -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext
                $DriverExitCode = $script:CampaignMilestoneExitCode
            }
            $ManifestStage = if ($UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
            Write-CampaignWrapperManifest `
                -Path $RunManifestPath `
                -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage $ManifestStage -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext)
        }
        return $DriverExitCode
    } finally {
        Pop-Location
    }
}
