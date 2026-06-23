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

function Convert-CampaignRequestForManifest {
    param(
        [object] $Request
    )

    if (-not $Request) {
        return [ordered]@{
            schema_name = ""
            kind = ""
            source_intent = ""
            output_intent = ""
            plan_targets = $false
            continue_targets = $false
            plan_coverage_gaps = $false
            continue_coverage_gaps = $false
            reads_campaign_source = $false
            is_continuation_family = $false
            uses_coverage_gap = $false
            uses_legacy_targeted = $false
        }
    }

    return [ordered]@{
        schema_name = $Request.SchemaName
        kind = $Request.Kind
        source_intent = $Request.SourceIntent
        output_intent = $Request.OutputIntent
        plan_targets = [bool] $Request.PlanTargets
        continue_targets = [bool] $Request.ContinueTargets
        plan_coverage_gaps = [bool] $Request.PlanCoverageGaps
        continue_coverage_gaps = [bool] $Request.ContinueCoverageGaps
        reads_campaign_source = [bool] $Request.ReadsCampaignSource
        is_continuation_family = [bool] $Request.IsContinuationFamily
        uses_coverage_gap = [bool] $Request.UsesCoverageGap
        uses_legacy_targeted = [bool] $Request.UsesLegacyTargeted
    }
}

function Write-CampaignPrimaryDriverCommandRecord {
    param(
        [string] $PrimaryDriverCommandLine,
        [object] $Context
    )

    if (-not $Context.OutputArtifact) {
        throw "Primary driver command recording requires an output artifact. Plan-only commands should not call this writer."
    }

    Set-Content -LiteralPath $Context.RunCommandPath -Value $PrimaryDriverCommandLine
    if ($Context.OutputArtifact.Kind -eq "run") {
        Write-CampaignLatestPointer -Artifact $Context.OutputArtifact
        Write-Host "latest-pointer=$(Get-CampaignLatestPointerPath)"
    } elseif ($Context.OutputArtifact.Kind -eq "scratch") {
        Write-CampaignScratchLatestPointer -Artifact $Context.OutputArtifact
        Write-Host "scratch-latest-pointer=$(Get-CampaignScratchLatestPointerPath)"
    }
    Write-Host "primary-driver-command=$($Context.RunCommandPath)"
    Write-Host "manifest=$($Context.RunManifestPath)"
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
        [string] $PrimaryDriverCommand,
        [object] $Context
    )

    return [ordered]@{
        schema_name = "CampaignWrapperManifestV1"
        schema_version = 1
        created_at = (Get-Date).ToString("o")
        stage = $Stage
        exit_code = $ExitCode
        wrapper_script = $Context.WrapperScript
        command_kind = $CommandKind
        request = Convert-CampaignRequestForManifest -Request $Context.CampaignRequest
        mode = $Context.Mode
        seed = $Context.Seed
        ascension = $Context.Ascension
        class = $Context.Class
        build_profile = $Context.BuildProfile
        driver_exe = "$($Context.DriverExe)"
        scratch = [bool] $Context.Scratch
        scratch_label = $Context.ScratchLabel
        output_artifact = if ($Context.OutputArtifact) { "$($Context.OutputArtifact.Label)" } else { "" }
        output_report = "$($Context.RunOutputCampaignPath)"
        output_checkpoint = "$($Context.RunOutputCheckpointPath)"
        command_file_semantics = "primary_driver_command"
        command_file = "$($Context.RunCommandPath)"
        manifest_file = "$($Context.RunManifestPath)"
        wrapper_invocation = [ordered]@{
            line = $Context.WrapperInvocationLine
            bound_parameters = $Context.WrapperBoundParameters
        }
        primary_driver = [ordered]@{
            args = @($PrimaryDriverArgs)
            command = $PrimaryDriverCommand
            command_file = "$($Context.RunCommandPath)"
        }
    }
}

function New-CampaignRunWrapperManifest {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext,
        [object] $Context
    )

    $Manifest = New-CampaignWrapperManifestBase `
        -ExitCode $ExitCode `
        -Stage $Stage `
        -CommandKind "campaign_run" `
        -PrimaryDriverArgs $Context.DriverArgs `
        -PrimaryDriverCommand $Context.RenderedCommand `
        -Context $Context
    $Manifest["resume_report"] = if ($Context.ResumeCampaignPath) { "$($Context.ResumeCampaignPath)" } else { "" }
    $Manifest["resume_checkpoint"] = if ($Context.ResumeCheckpointPath) { "$($Context.ResumeCheckpointPath)" } else { "" }
    $Manifest["log_file"] = if ($Context.Log) { "$($Context.RunLogPath)" } else { "" }
    $Manifest["round_budget"] = [ordered]@{
        source = $Context.RoundBudgetSource
        target_rounds = $Context.TargetRounds
        additional_rounds = $Context.RoundBudgetAdditionalRounds
    }

    if ($Context.UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs -RunIdentityArgs $RunIdentityArgs -StepRounds $Context.MilestoneStepRounds -OptionContext $OptionContext
        $Manifest["milestone"] = [ordered]@{
            target = $Context.UntilMilestone
            stop = $Context.ResolvedMilestoneStop
            step_rounds = $Context.MilestoneStepRounds
            max_additional_rounds = $Context.MilestoneMaxRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $Context.DriverExe -Arguments $MilestoneResumeArgs)
        }
    }

    return $Manifest
}

function New-CampaignRunCommandContext {
    param(
        [object] $CampaignRequest,
        [string] $WrapperScript,
        [string] $RepoRoot,
        [string] $Mode,
        [long] $Seed,
        [int] $Ascension,
        [string] $Class,
        [bool] $Scratch,
        [object] $BuildContext,
        [object] $RunOutputContext,
        [object] $BoundParameterContext,
        [object] $RunRoundContext,
        [string[]] $DriverArgs,
        [bool] $NeedsBuild,
        [bool] $DryRun,
        [bool] $Log,
        [bool] $BossRelicAxes,
        [string] $CombatSegmentMode,
        [string] $UntilMilestone,
        [int] $MilestoneStepRounds,
        [int] $MilestoneMaxRounds,
        [string] $ResolvedMilestoneStop
    )

    if (-not $RunOutputContext.WritesCampaignOutput) {
        throw "Internal error: campaign request '$($CampaignRequest.Kind)' reached run execution without an output artifact."
    }

    return [pscustomobject]@{
        CampaignRequest = $CampaignRequest
        WrapperScript = $WrapperScript
        Mode = $Mode
        Seed = $Seed
        Ascension = $Ascension
        Class = $Class
        BuildProfile = $BuildContext.BuildProfile
        BossRelicAxes = $BossRelicAxes
        Scratch = $Scratch
        ScratchLabel = $RunOutputContext.ScratchLabel
        OutputArtifact = $RunOutputContext.Artifact
        RunOutputCampaignPath = $RunOutputContext.CampaignPath
        RunOutputCheckpointPath = $RunOutputContext.CheckpointPath
        RunCommandPath = $RunOutputContext.CommandPath
        RunManifestPath = $RunOutputContext.ManifestPath
        WrapperInvocationLine = $BoundParameterContext.WrapperInvocationLine
        WrapperBoundParameters = $BoundParameterContext.WrapperBoundParameters
        ContinueCampaign = [bool] $CampaignRequest.ContinueCampaign
        TargetRounds = $RunRoundContext.TargetRounds
        MaxRounds = $RunRoundContext.MaxRounds
        ResumeRoundsCompleted = $RunRoundContext.ResumeRoundsCompleted
        UntilMilestoneBound = $BoundParameterContext.UntilMilestoneBound
        ResumeCampaignPath = $RunRoundContext.ResumeCampaignPath
        ResumeCheckpointPath = $RunRoundContext.ResumeCheckpointPath
        UntilMilestone = $UntilMilestone
        MilestoneStepRounds = $MilestoneStepRounds
        MilestoneMaxRounds = $MilestoneMaxRounds
        ResolvedMilestoneStop = $ResolvedMilestoneStop
        NeedsBuild = $NeedsBuild
        BuildArgs = @($BuildContext.BuildArgs)
        DryRun = $DryRun
        RepoRoot = $RepoRoot
        DriverExe = $BuildContext.DriverExe
        DriverArgs = @($DriverArgs)
        RenderedCommand = Format-CommandLine -ExePath $BuildContext.DriverExe -Arguments $DriverArgs
        Log = $Log
        RunLogPath = $RunOutputContext.LogPath
        CombatSegmentMode = $CombatSegmentMode
        RoundBudgetSource = $RunRoundContext.RoundBudgetSource
        RoundBudgetAdditionalRounds = $RunRoundContext.RoundBudgetAdditionalRounds
    }
}

function Invoke-CampaignRunCommand {
    param(
        [object] $Context,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    if ($Context.ContinueCampaign -and $Context.TargetRounds -ne $null -and $Context.MaxRounds -eq 0) {
        Write-Host "already-at-target-rounds=yes; nothing to run"
        return 0
    }
    if ($Context.ContinueCampaign -and $Context.UntilMilestoneBound) {
        $InitialMilestoneStatus = Get-CampaignMilestoneStatus -ReportPath $Context.ResumeCampaignPath -Milestone $Context.UntilMilestone
        if ($InitialMilestoneStatus.Reached) {
            Write-Host "already-at-milestone=yes target=$($Context.UntilMilestone) hits=$($InitialMilestoneStatus.HitCount) furthest=A$($InitialMilestoneStatus.FurthestAct)F$($InitialMilestoneStatus.FurthestFloor)"
            return 0
        }
    }

    if ($Context.DryRun) {
        if ($Context.NeedsBuild) {
            Write-CampaignBuildCommandPreview -BuildArgs $Context.BuildArgs
        }
        Write-Host $Context.RenderedCommand
        if ($Context.UntilMilestoneBound) {
            Write-Host "milestone-loop-command-template:"
            Write-Host (Format-CommandLine -ExePath $Context.DriverExe -Arguments (New-MilestoneResumeDriverArgs -RunIdentityArgs $RunIdentityArgs -StepRounds $Context.MilestoneStepRounds -OptionContext $OptionContext))
        }
        return 0
    }

    Push-Location $Context.RepoRoot
    try {
        if ($Context.NeedsBuild) {
            & cargo @($Context.BuildArgs)
            if ($LASTEXITCODE -ne 0) {
                return $LASTEXITCODE
            }
        }
        if ($Context.Log) {
            $DriverExitCode = Invoke-CampaignLoggedDriverCommand -ExePath $Context.DriverExe -Arguments $Context.DriverArgs -LogPath $Context.RunLogPath
        } else {
            & $Context.DriverExe @($Context.DriverArgs)
            $DriverExitCode = $LASTEXITCODE
        }
        if ($DriverExitCode -eq 0) {
            Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine $Context.RenderedCommand -Context $Context
            Write-CampaignWrapperManifest `
                -Path $Context.RunManifestPath `
                -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage "initial_driver_completed" -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext -Context $Context)
            if ($Context.UntilMilestoneBound) {
                Invoke-CampaignUntilMilestone -AlreadySpentRounds $Context.MaxRounds -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext
                $DriverExitCode = $script:CampaignMilestoneExitCode
            }
            $ManifestStage = if ($Context.UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
            Write-CampaignWrapperManifest `
                -Path $Context.RunManifestPath `
                -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage $ManifestStage -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext -Context $Context)
        }
        return $DriverExitCode
    } finally {
        Pop-Location
    }
}
