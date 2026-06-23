[CmdletBinding()]
param(
    [switch] $RequireScratchLatest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir
$CampaignScript = Join-Path $ScriptDir "campaign.ps1"
$ScratchLatestPointer = Join-Path (Join-Path (Join-Path $ScriptDir "artifacts") "campaigns\scratch") "latest.json"
$CampaignArtifactDir = Join-Path (Join-Path $ScriptDir "artifacts") "campaigns"
$LegacyLatestCampaignPath = Join-Path $CampaignArtifactDir "latest.campaign.json"
$LegacyLatestCheckpointPath = Join-Path $CampaignArtifactDir "latest.checkpoint.json"

function Get-SmokePowerShellExe {
    $Pwsh = Get-Command pwsh -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($Pwsh) {
        return $Pwsh.Source
    }
    return (Get-Command powershell -ErrorAction Stop | Select-Object -First 1).Source
}

function Invoke-CampaignSmokeCommand {
    param(
        [string[]] $Arguments
    )

    $PowerShellExe = Get-SmokePowerShellExe
    $Output = & $PowerShellExe @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        $CampaignScript
    ) @Arguments 2>&1 | Out-String

    return [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = $Output
    }
}

function Assert-SmokeContains {
    param(
        [string] $Name,
        [string] $Output,
        [string] $Needle
    )
    if (-not $Output.Contains($Needle)) {
        throw "case '$Name' expected output to contain '$Needle'."
    }
}

function Assert-SmokeNotContains {
    param(
        [string] $Name,
        [string] $Output,
        [string] $Needle
    )
    if ($Output.Contains($Needle)) {
        throw "case '$Name' expected output not to contain '$Needle'."
    }
}

function Invoke-CampaignSmokeCase {
    param(
        [string] $Name,
        [string[]] $Arguments,
        [string[]] $Contains = @(),
        [string[]] $NotContains = @()
    )

    $Result = Invoke-CampaignSmokeCommand -Arguments $Arguments
    if ($Result.ExitCode -ne 0) {
        throw "case '$Name' failed with exit code $($Result.ExitCode).`n$($Result.Output)"
    }

    foreach ($Needle in $Contains) {
        Assert-SmokeContains -Name $Name -Output $Result.Output -Needle $Needle
    }
    foreach ($Needle in $NotContains) {
        Assert-SmokeNotContains -Name $Name -Output $Result.Output -Needle $Needle
    }

    Write-Host "campaign-wrapper-smoke: PASS $Name"
}

if (-not (Test-Path -LiteralPath $CampaignScript)) {
    throw "campaign wrapper not found at $CampaignScript"
}

$ScratchLatestExists = Test-Path -LiteralPath $ScratchLatestPointer
if ($RequireScratchLatest -and -not $ScratchLatestExists) {
    throw "scratch latest pointer not found at $ScratchLatestPointer"
}

Push-Location $RepoRoot
try {
    if ($ScratchLatestExists) {
        Invoke-CampaignSmokeCase `
            -Name "FromScratchLatestContinueScratchDryRun" `
            -Arguments @(
                "-FromScratchLatest",
                "-Continue",
                "-Scratch",
                "-Rounds",
                "1",
                "-DebugBuild",
                "-NoProgress",
                "-DryRun"
            ) `
            -Contains @(
                "continue-scratch-latest=.\tools\campaign.ps1 -FromScratchLatest -Continue -Scratch",
                "\tools\artifacts\campaigns\scratch\",
                "round-budget=Rounds"
            ) `
            -NotContains @(
                "continue-latest=.\tools\campaign.ps1 -From latest -Continue",
                "Choose one campaign request kind"
            )

        Invoke-CampaignSmokeCase `
            -Name "FromScratchLatestPlanCoverageGapsDryRun" `
            -Arguments @(
                "-FromScratchLatest",
                "-PlanCoverageGaps",
                "-CoverageGapRoute",
                "-CoverageGapLimit",
                "2",
                "-DebugBuild",
                "-NoProgress",
                "-DryRun"
            ) `
            -Contains @(
                "source=scratch:",
                "coverage-gap-plan=2",
                "--plan-coverage-gap-continuation"
            ) `
            -NotContains @(
                "Choose one campaign request kind"
            )

        Invoke-CampaignSmokeCase `
            -Name "FromScratchLatestCoverageGapMilestoneDryRun" `
            -Arguments @(
                "-FromScratchLatest",
                "-ContinueCoverageGaps",
                "-CoverageGapRoute",
                "-CoverageGapLimit",
                "2",
                "-Scratch",
                "-UntilMilestone",
                "Act1Boss",
                "-MilestoneStepRounds",
                "1",
                "-MilestoneMaxRounds",
                "2",
                "-DebugBuild",
                "-NoProgress",
                "-DryRun"
            ) `
            -Contains @(
                "coverage-gap-continue=2",
                "execution=milestone_continuation",
                "milestone-loop-command-template:",
                "milestone-summary-command:",
                "\tools\artifacts\campaigns\scratch\"
            ) `
            -NotContains @(
                "Choose one campaign request kind"
            )

        Invoke-CampaignSmokeCase `
            -Name "FromScratchLatestInspectDryRun" `
            -Arguments @(
                "-FromScratchLatest",
                "-DryRun"
            ) `
            -Contains @(
                "mode=inspect scratch:",
                "\tools\artifacts\campaigns\scratch\",
                " inspect "
            ) `
            -NotContains @(
                "Choose one campaign request kind"
            )

        Invoke-CampaignSmokeCase `
            -Name "FromScratchLatestProbeDryRun" `
            -Arguments @(
                "-FromScratchLatest",
                "-Probe",
                "shop-evidence",
                "-InspectIndex",
                "0",
                "-DebugBuild",
                "-NoProgress",
                "-DryRun"
            ) `
            -Contains @(
                "mode=inspect scratch:",
                "--inspect-shop-evidence",
                "--inspect-index 0"
            ) `
            -NotContains @(
                "--inspect-summary",
                "branch_campaign_driver.exe run",
                "Choose one campaign request kind"
            )
    } else {
        Write-Host "campaign-wrapper-smoke: SKIP scratch-latest cases; no pointer at $ScratchLatestPointer"
    }

    if ((Test-Path -LiteralPath $LegacyLatestCampaignPath) -and (Test-Path -LiteralPath $LegacyLatestCheckpointPath)) {
        Invoke-CampaignSmokeCase `
            -Name "LegacyLatestInspectDryRun" `
            -Arguments @(
                "-From",
                "legacy-latest",
                "-Inspect",
                "-DebugBuild",
                "-NoProgress",
                "-DryRun"
            ) `
            -Contains @(
                "mode=inspect legacy-latest",
                "\tools\artifacts\campaigns\latest.campaign.json",
                "\tools\artifacts\campaigns\latest.checkpoint.json"
            ) `
            -NotContains @(
                "Choose one campaign request kind"
            )
    } else {
        Write-Host "campaign-wrapper-smoke: SKIP legacy-latest case; missing legacy latest sidecars"
    }

    Invoke-CampaignSmokeCase `
        -Name "NormalScratchRunDryRun" `
        -Arguments @(
            "-Mode",
            "quick",
            "-Scratch",
            "-Rounds",
            "1",
            "-DebugBuild",
            "-NoProgress",
            "-DryRun"
        ) `
        -Contains @(
            "continue-scratch-latest=.\tools\campaign.ps1 -FromScratchLatest -Continue -Scratch",
            "\tools\artifacts\campaigns\scratch\",
            "branch_campaign_driver.exe run"
        ) `
        -NotContains @(
            "continue-latest=.\tools\campaign.ps1 -From latest -Continue",
            "Choose one campaign request kind"
        )

    Invoke-CampaignSmokeCase `
        -Name "NormalScratchMilestoneDryRun" `
        -Arguments @(
            "-Mode",
            "quick",
            "-Scratch",
            "-UntilMilestone",
            "Act1Boss",
            "-MilestoneStepRounds",
            "1",
            "-MilestoneMaxRounds",
            "2",
            "-DebugBuild",
            "-NoProgress",
            "-DryRun"
        ) `
        -Contains @(
            "until-milestone=Act1Boss step-rounds=1 max-additional-rounds=2",
            "milestone-loop-command-template:",
            "\tools\artifacts\campaigns\scratch\"
        ) `
        -NotContains @(
            "Choose one campaign request kind"
        )

    Write-Host "campaign-wrapper-smoke: all checks passed"
} finally {
    Pop-Location
}
