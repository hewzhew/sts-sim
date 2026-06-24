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

function New-CampaignRunDriverArgsContext {
    param(
        [string[]] $RunIdentityArgs,
        [object] $Request,
        [object] $RunOutputContext,
        [object] $RunRoundContext,
        [object] $OptionContext,
        [string] $ExportLearningDataset
    )

    $Args = @($RunIdentityArgs)
    $Args += @($RunRoundContext.ResumeDriverArgs)
    if ($RunOutputContext.WritesCampaignOutput) {
        $Args += @("--out", "$($RunOutputContext.CampaignPath)", "--checkpoint-out", "$($RunOutputContext.CheckpointPath)")
    }
    if ($RunRoundContext.DriverRoundBudgetArgs.Count -gt 0) {
        $Args += @($RunRoundContext.DriverRoundBudgetArgs)
    }
    if ($ExportLearningDataset -and -not $Request.Inspect) {
        $Args += @("--export-learning-dataset", "$ExportLearningDataset")
    }
    $Args = Add-CampaignSharedDriverOptions `
        -Arguments $Args `
        -IncludeActiveLineageDiversity $true `
        -IncludeBossRelicAxes $true `
        -IncludeAutoCaptureCombat $true `
        -OptionContext $OptionContext

    return [pscustomobject]@{
        DriverArgs = @($Args)
        CombatSegmentMode = Resolve-CampaignCombatSegmentMode -OptionContext $OptionContext
    }
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
        [object] $DriverPassthroughContext,
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
        DriverPassthroughContext = $DriverPassthroughContext
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

function New-CampaignRunMilestoneContext {
    param(
        [object] $Context,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    return New-CampaignMilestoneContext `
        -ReportPath $Context.RunOutputCampaignPath `
        -CheckpointPath $Context.RunOutputCheckpointPath `
        -DriverExe $Context.DriverExe `
        -UntilMilestone $Context.UntilMilestone `
        -ResolvedMilestoneStop $Context.ResolvedMilestoneStop `
        -MilestoneStepRounds $Context.MilestoneStepRounds `
        -MilestoneMaxRounds $Context.MilestoneMaxRounds `
        -RunIdentityArgs $RunIdentityArgs `
        -OptionContext $OptionContext
}

function Write-CampaignRunDryRunCommandSet {
    param(
        [object] $Context,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    if ($Context.NeedsBuild) {
        Write-CampaignBuildCommandPreview -BuildArgs $Context.BuildArgs
    }
    Write-Host $Context.RenderedCommand
    if ($Context.UntilMilestoneBound) {
        $MilestoneContext = New-CampaignRunMilestoneContext `
            -Context $Context `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext
        Write-CampaignMilestoneLoopCommandPreview `
            -DriverExe $Context.DriverExe `
            -MilestoneContext $MilestoneContext
    }
}

function Invoke-CampaignInitialDriverCommand {
    param(
        [object] $Context
    )

    if ($Context.Log) {
        return Invoke-CampaignLoggedDriverCommand -ExePath $Context.DriverExe -Arguments $Context.DriverArgs -LogPath $Context.RunLogPath
    }

    & $Context.DriverExe @($Context.DriverArgs) | ForEach-Object { Write-Host $_ }
    return $LASTEXITCODE
}

function Invoke-CampaignRunMilestoneLoop {
    param(
        [object] $Context,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    if (-not $Context.UntilMilestoneBound) {
        return 0
    }

    $MilestoneContext = New-CampaignRunMilestoneContext `
        -Context $Context `
        -RunIdentityArgs $RunIdentityArgs `
        -OptionContext $OptionContext
    return Invoke-CampaignUntilMilestone `
        -MilestoneContext $MilestoneContext `
        -AlreadySpentRounds $Context.MaxRounds
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
        Write-CampaignRunDryRunCommandSet `
            -Context $Context `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext
        return 0
    }

    Push-Location $Context.RepoRoot
    try {
        if ($Context.NeedsBuild) {
            & cargo @($Context.BuildArgs) | ForEach-Object { Write-Host $_ }
            if ($LASTEXITCODE -ne 0) {
                return $LASTEXITCODE
            }
        }
        $DriverExitCode = Invoke-CampaignInitialDriverCommand -Context $Context
        if ($DriverExitCode -eq 0) {
            Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine $Context.RenderedCommand -Context $Context
            Write-CampaignWrapperManifest `
                -Path $Context.RunManifestPath `
                -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage "initial_driver_completed" -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext -Context $Context) `
                -Context $Context
            $MilestoneExitCode = Invoke-CampaignRunMilestoneLoop `
                -Context $Context `
                -RunIdentityArgs $RunIdentityArgs `
                -OptionContext $OptionContext
            if ($Context.UntilMilestoneBound) {
                $DriverExitCode = $MilestoneExitCode
            }
            $ManifestStage = if ($Context.UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
            Write-CampaignWrapperManifest `
                -Path $Context.RunManifestPath `
                -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage $ManifestStage -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext -Context $Context) `
                -Context $Context
        }
        return $DriverExitCode
    } finally {
        Pop-Location
    }
}
