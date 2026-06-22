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
        [int] $StepRounds,
        [object] $OptionContext
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
    $Args += @(
        "--resume", "$RunOutputCampaignPath",
        "--resume-checkpoint", "$RunOutputCheckpointPath",
        "--out", "$RunOutputCampaignPath",
        "--checkpoint-out", "$RunOutputCheckpointPath",
        "--rounds", "$StepRounds"
    )
    return Add-CampaignSharedDriverOptions `
        -Arguments $Args `
        -IncludeActiveLineageDiversity $false `
        -IncludeBossRelicAxes $false `
        -IncludeAutoCaptureCombat $false `
        -OptionContext $OptionContext
}

function Invoke-CampaignUntilMilestone {
    param(
        [int] $AlreadySpentRounds = 0,
        [object] $OptionContext
    )

    $script:CampaignMilestoneExitCode = 0
    $SpentRounds = $AlreadySpentRounds
    while ($SpentRounds -lt $MilestoneMaxRounds) {
        $Status = Get-CampaignMilestoneStatus -ReportPath $RunOutputCampaignPath -Milestone $UntilMilestone
        Write-Host "milestone-status target=$UntilMilestone stop=$ResolvedMilestoneStop reached=$($Status.Reached) hits=$($Status.HitCount) furthest=A$($Status.FurthestAct)F$($Status.FurthestFloor) report-rounds=$($Status.RoundsCompleted) spent-rounds=$SpentRounds cap=$MilestoneMaxRounds"
        if ($Status.Reached -and $ResolvedMilestoneStop -eq "first_hit") {
            $script:CampaignMilestoneExitCode = 0
            return
        }
        $StepRounds = [Math]::Min($MilestoneStepRounds, $MilestoneMaxRounds - $SpentRounds)
        $ResumeArgs = New-MilestoneResumeDriverArgs -StepRounds $StepRounds -OptionContext $OptionContext
        Write-Host "milestone-step target=$UntilMilestone additional-rounds=$StepRounds"
        & $DriverExe @ResumeArgs
        if ($LASTEXITCODE -ne 0) {
            $script:CampaignMilestoneExitCode = $LASTEXITCODE
            return
        }
        $SpentRounds += $StepRounds
    }

    $FinalStatus = Get-CampaignMilestoneStatus -ReportPath $RunOutputCampaignPath -Milestone $UntilMilestone
    Write-Host "milestone-status target=$UntilMilestone stop=$ResolvedMilestoneStop reached=$($FinalStatus.Reached) hits=$($FinalStatus.HitCount) furthest=A$($FinalStatus.FurthestAct)F$($FinalStatus.FurthestFloor) report-rounds=$($FinalStatus.RoundsCompleted) spent-rounds=$SpentRounds cap=$MilestoneMaxRounds"
    $script:CampaignMilestoneExitCode = 0
}
