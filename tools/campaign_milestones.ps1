function Get-CampaignMilestoneStatus {
    param(
        [string] $ReportPath,
        [string] $Milestone
    )

    if (-not (Test-Path -LiteralPath $ReportPath)) {
        return [pscustomobject]@{
            Reached = $false
            FurthestAct = 0
            FurthestFloor = 0
            HitCount = 0
            RoundsCompleted = 0
        }
    }

    $Report = Get-Content -LiteralPath $ReportPath -Raw | ConvertFrom-Json
    $Branches = @()
    foreach ($Bucket in @("active", "frozen", "stuck", "victories", "dead", "abandoned")) {
        if ($Report.$Bucket) {
            $Branches += @($Report.$Bucket)
        }
    }

    $FurthestAct = 0
    $FurthestFloor = 0
    $HitCount = 0
    foreach ($Branch in $Branches) {
        if (-not $Branch.summary) {
            continue
        }
        $Act = [int] $Branch.summary.act
        $Floor = [int] $Branch.summary.floor
        if (($Act -gt $FurthestAct) -or (($Act -eq $FurthestAct) -and ($Floor -gt $FurthestFloor))) {
            $FurthestAct = $Act
            $FurthestFloor = $Floor
        }
        $Hit = switch ($Milestone) {
            "Act1Boss" { ($Act -gt 1) -or (($Act -eq 1) -and ($Floor -ge 16)) }
            "Act2Start" { $Act -ge 2 }
            default { $false }
        }
        if ($Hit) {
            $HitCount += 1
        }
    }

    return [pscustomobject]@{
        Reached = $HitCount -gt 0
        FurthestAct = $FurthestAct
        FurthestFloor = $FurthestFloor
        HitCount = $HitCount
        RoundsCompleted = [int] $Report.rounds_completed
    }
}

function New-MilestoneResumeDriverArgs {
    param(
        [string[]] $RunIdentityArgs,
        [string] $CampaignPath,
        [string] $CheckpointPath,
        [int] $StepRounds,
        [object] $OptionContext
    )

    $Args = @($RunIdentityArgs)
    $Args += @(
        "--resume", "$CampaignPath",
        "--resume-checkpoint", "$CheckpointPath",
        "--out", "$CampaignPath",
        "--checkpoint-out", "$CheckpointPath",
        "--rounds", "$StepRounds"
    )
    return Add-CampaignSharedDriverOptions `
        -Arguments $Args `
        -IncludeActiveLineageDiversity $false `
        -IncludeBossRelicAxes $false `
        -IncludeAutoCaptureCombat $false `
        -OptionContext $OptionContext
}

function New-CampaignMilestoneContext {
    param(
        [string] $ReportPath,
        [string] $CheckpointPath,
        [string] $DriverExe,
        [string] $UntilMilestone,
        [string] $ResolvedMilestoneStop,
        [int] $MilestoneStepRounds,
        [int] $MilestoneMaxRounds,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    return [pscustomobject]@{
        ReportPath = $ReportPath
        CheckpointPath = $CheckpointPath
        DriverExe = $DriverExe
        UntilMilestone = $UntilMilestone
        ResolvedMilestoneStop = $ResolvedMilestoneStop
        MilestoneStepRounds = $MilestoneStepRounds
        MilestoneMaxRounds = $MilestoneMaxRounds
        RunIdentityArgs = @($RunIdentityArgs)
        OptionContext = $OptionContext
    }
}

function New-CampaignMilestoneResumeDriverArgs {
    param(
        [object] $MilestoneContext,
        [int] $StepRounds
    )

    return New-MilestoneResumeDriverArgs `
        -RunIdentityArgs $MilestoneContext.RunIdentityArgs `
        -CampaignPath $MilestoneContext.ReportPath `
        -CheckpointPath $MilestoneContext.CheckpointPath `
        -StepRounds $StepRounds `
        -OptionContext $MilestoneContext.OptionContext
}

function Invoke-CampaignUntilMilestone {
    param(
        [object] $MilestoneContext,
        [int] $AlreadySpentRounds = 0,
        [string] $Label = "milestone"
    )

    $SpentRounds = $AlreadySpentRounds
    while ($SpentRounds -lt $MilestoneContext.MilestoneMaxRounds) {
        $Status = Get-CampaignMilestoneStatus `
            -ReportPath $MilestoneContext.ReportPath `
            -Milestone $MilestoneContext.UntilMilestone
        Write-Host "$Label-status target=$($MilestoneContext.UntilMilestone) stop=$($MilestoneContext.ResolvedMilestoneStop) reached=$($Status.Reached) hits=$($Status.HitCount) furthest=A$($Status.FurthestAct)F$($Status.FurthestFloor) report-rounds=$($Status.RoundsCompleted) spent-rounds=$SpentRounds cap=$($MilestoneContext.MilestoneMaxRounds)"
        if ($Status.Reached -and $MilestoneContext.ResolvedMilestoneStop -eq "first_hit") {
            return 0
        }
        $StepRounds = [Math]::Min($MilestoneContext.MilestoneStepRounds, $MilestoneContext.MilestoneMaxRounds - $SpentRounds)
        $ResumeArgs = New-CampaignMilestoneResumeDriverArgs `
            -MilestoneContext $MilestoneContext `
            -StepRounds $StepRounds
        Write-Host "$Label-step target=$($MilestoneContext.UntilMilestone) additional-rounds=$StepRounds"
        & $MilestoneContext.DriverExe @ResumeArgs | ForEach-Object { Write-Host $_ }
        if ($LASTEXITCODE -ne 0) {
            return $LASTEXITCODE
        }
        $SpentRounds += $StepRounds
    }

    $FinalStatus = Get-CampaignMilestoneStatus `
        -ReportPath $MilestoneContext.ReportPath `
        -Milestone $MilestoneContext.UntilMilestone
    Write-Host "$Label-status target=$($MilestoneContext.UntilMilestone) stop=$($MilestoneContext.ResolvedMilestoneStop) reached=$($FinalStatus.Reached) hits=$($FinalStatus.HitCount) furthest=A$($FinalStatus.FurthestAct)F$($FinalStatus.FurthestFloor) report-rounds=$($FinalStatus.RoundsCompleted) spent-rounds=$SpentRounds cap=$($MilestoneContext.MilestoneMaxRounds)"
    return 0
}
