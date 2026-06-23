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

function Resolve-CampaignDriverPassthroughContext {
    param(
        [string[]] $DriverArgs,
        [string[]] $CompatibilityExtraArgs
    )

    $ResolvedDriverArgs = @()
    if ($DriverArgs) {
        $ResolvedDriverArgs = @($DriverArgs)
    }

    $ResolvedCompatibilityExtraArgs = @()
    if ($CompatibilityExtraArgs) {
        $ResolvedCompatibilityExtraArgs = @($CompatibilityExtraArgs)
    }

    $ResolvedPassthroughArgs = @()
    $ResolvedPassthroughArgs += @($ResolvedDriverArgs)
    $ResolvedPassthroughArgs += @($ResolvedCompatibilityExtraArgs)

    $RetiredWrapperArgs = @(
        "-PlanTargets",
        "-ContinueTargets",
        "-DecisionOutcomeDataset",
        "-TargetedContinuationLimit",
        "-TargetedContinuationCandidatesPerTarget"
    )
    foreach ($Arg in $ResolvedPassthroughArgs) {
        if ($RetiredWrapperArgs -contains $Arg) {
            throw "$Arg was removed from tools/campaign.ps1. Use coverage-gap continuation, or call branch_campaign_driver directly for targeted-continuation archaeology."
        }
        if ($Arg -match '^-[A-Za-z][A-Za-z0-9_-]*(=.*)?$') {
            throw "Unknown wrapper-style argument '$Arg'. Driver passthrough arguments must use Rust-style '--flag' syntax; wrapper switches use declared campaign.ps1 parameters."
        }
    }

    return [pscustomobject]@{
        ExplicitDriverArgs = @($ResolvedDriverArgs)
        CompatibilityExtraArgs = @($ResolvedCompatibilityExtraArgs)
        DriverPassthroughArgs = @($ResolvedPassthroughArgs)
        HasCompatibilityExtraArgs = ($ResolvedCompatibilityExtraArgs.Count -gt 0)
    }
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
        [object] $DriverPassthroughContext,
        [bool] $BossSegments,
        [bool] $NoProgress,
        [bool] $VerboseProgress,
        [bool] $Perf,
        [bool] $Diagnose
    )

    if ($null -eq $DriverPassthroughContext) {
        $DriverPassthroughContext = Resolve-CampaignDriverPassthroughContext
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
        ExplicitDriverArgs = @($DriverPassthroughContext.ExplicitDriverArgs)
        CompatibilityExtraArgs = @($DriverPassthroughContext.CompatibilityExtraArgs)
        DriverPassthroughArgs = @($DriverPassthroughContext.DriverPassthroughArgs)
        HasCompatibilityExtraArgs = [bool] $DriverPassthroughContext.HasCompatibilityExtraArgs
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
    if (-not (Test-ExtraCombatOptionKey -Tokens $OptionContext.DriverPassthroughArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
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
    if ($OptionContext.DriverPassthroughArgs) {
        $Args += $OptionContext.DriverPassthroughArgs
    }
    return $Args
}

function Resolve-CampaignCombatSegmentMode {
    param(
        [object] $OptionContext
    )

    if (Test-ExtraCombatOptionKey -Tokens $OptionContext.DriverPassthroughArgs -Keys @("segment", "segment_mode", "partial", "partial_mode")) {
        return "custom"
    }
    if ($OptionContext.BossSegments) {
        return "turn"
    }
    return "non_boss_turn"
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
