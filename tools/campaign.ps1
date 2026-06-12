<#
.SYNOPSIS
Runs the focused branch campaign with baby-friendly defaults.

.EXAMPLE
.\tools\campaign.ps1
Runs a focused campaign on a random seed.

.EXAMPLE
.\tools\campaign.ps1 521
Runs the same focused campaign on seed 521.

.EXAMPLE
.\tools\campaign.ps1 -Last
Reuses the last non-dry-run campaign seed.

.EXAMPLE
.\tools\campaign.ps1 -More
Resumes the latest saved campaign report with deeper defaults.

.EXAMPLE
.\tools\campaign.ps1 -Mode quick
Runs a shorter random-seed campaign for fast smoke testing.

.EXAMPLE
.\tools\campaign.ps1 -Mode deep
Runs a larger random-seed campaign when you want to leave it working longer.

.EXAMPLE
.\tools\campaign.ps1 -More -VictoryHpPercent 50
Resumes the latest campaign but keeps exploring until it finds a victory at 50% HP or better.

.EXAMPLE
.\tools\campaign.ps1 -DryRun
Prints the cargo command without updating the last seed or running it.

.EXAMPLE
.\tools\campaign.ps1 -NoProgress
Runs without coarse campaign progress messages.

.EXAMPLE
.\tools\campaign.ps1 -NoBossSegments
Compatibility switch; boss combats already stay on complete-win search by default.

.EXAMPLE
.\tools\campaign.ps1 -BossSegments
Allows turn-segment continuation inside boss combats. This is slower, but can push through bosses while debugging combat strategy.

.EXAMPLE
.\tools\campaign.ps1 -DebugBuild
Runs the slower debug build when you are debugging compilation or assertions.

.EXAMPLE
.\tools\campaign.ps1 -Build
Rebuilds the branch campaign driver before running it.
#>
param(
    [Parameter(Position = 0)]
    [long] $Seed = 0,

    [switch] $Last,
    [switch] $More,
    [switch] $DryRun,
    [switch] $NoProgress,
    [switch] $NoBossSegments,
    [switch] $BossSegments,
    [switch] $DebugBuild,
    [switch] $Build,

    [ValidateSet("quick", "focused", "deep")]
    [string] $Mode = "focused",

    [int] $MaxRounds = 6,
    [int] $ExperimentWallMs = 10000,
    [int] $SearchWallMs = 300,
    [int] $SearchMaxNodes = 50000,
    [int] $BranchExamples = 4,
    [ValidateRange(0, 100)]
    [int] $VictoryHpPercent = 20,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$CampaignDir = Join-Path $RepoRoot "tools\artifacts\campaigns"
$LatestSeedPath = Join-Path $CampaignDir "latest.seed.txt"
$LatestCommandPath = Join-Path $CampaignDir "latest.command.txt"
$LatestCampaignPath = Join-Path $CampaignDir "latest.campaign.json"
$LatestCheckpointPath = Join-Path $CampaignDir "latest.checkpoint.json"

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

if ($More) {
    $Last = $true
    if (-not $PSBoundParameters.ContainsKey("Mode")) {
        $Mode = "deep"
    }
}

if ($Last) {
    if (-not (Test-Path $LatestSeedPath)) {
        throw "No previous campaign seed found at $LatestSeedPath. Run .\tools\campaign.ps1 first."
    }
    $SeedText = (Get-Content -LiteralPath $LatestSeedPath -Raw).Trim()
    if (-not [long]::TryParse($SeedText, [ref] $Seed)) {
        throw "Invalid previous campaign seed in $LatestSeedPath`: $SeedText"
    }
} elseif ($Seed -le 0) {
    $Seed = Get-Random -Minimum 1 -Maximum 2147483647
}

$BuildProfile = if ($DebugBuild) { "debug" } else { "release" }
$DriverExe = Join-Path $RepoRoot "target\$BuildProfile\branch_campaign_driver.exe"
$BuildArgs = @("build", "--quiet", "--bin", "branch_campaign_driver")
if (-not $DebugBuild) {
    $BuildArgs += "--release"
}

$DriverArgs = @(
    "--preset", "$Mode",
    "--seed", "$Seed"
)

$CampaignBoundParameters = @{}
foreach ($ParameterName in $PSBoundParameters.Keys) {
    $CampaignBoundParameters[$ParameterName] = $true
}

$ResumeCampaignPath = $null
$ResumeCheckpointPath = $null
$ResumeRoundsCompleted = $null
$TargetRounds = $null
if ($More) {
    if (-not (Test-Path $LatestCampaignPath)) {
        throw "No previous campaign report found at $LatestCampaignPath. Run .\tools\campaign.ps1 first, or use -Last to rerun the previous seed from the start."
    }
    $ResumeCampaignPath = $LatestCampaignPath
    $ResumeReport = Get-Content -LiteralPath $ResumeCampaignPath -Raw | ConvertFrom-Json
    $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
    if ($CampaignBoundParameters.ContainsKey("MaxRounds")) {
        $TargetRounds = $MaxRounds
        $MaxRounds = [Math]::Max(0, $TargetRounds - $ResumeRoundsCompleted)
    }
    $DriverArgs += @("--resume", "$ResumeCampaignPath")
    if (Test-Path $LatestCheckpointPath) {
        $ResumeCheckpointPath = $LatestCheckpointPath
        $DriverArgs += @("--resume-checkpoint", "$ResumeCheckpointPath")
    }
}

$DriverArgs += @("--out", "$LatestCampaignPath", "--checkpoint-out", "$LatestCheckpointPath")

function Add-DriverArgIfBound {
    param(
        [string] $ParameterName,
        [string] $Flag,
        [object] $Value
    )

    if ($CampaignBoundParameters.ContainsKey($ParameterName)) {
        $script:DriverArgs += @($Flag, "$Value")
    }
}

Add-DriverArgIfBound "MaxRounds" "--max-rounds" $MaxRounds
Add-DriverArgIfBound "ExperimentWallMs" "--experiment-wall-ms" $ExperimentWallMs
Add-DriverArgIfBound "SearchWallMs" "--search-wall-ms" $SearchWallMs
Add-DriverArgIfBound "SearchMaxNodes" "--search-max-nodes" $SearchMaxNodes
Add-DriverArgIfBound "BranchExamples" "--branch-examples" $BranchExamples
Add-DriverArgIfBound "VictoryHpPercent" "--min-acceptable-victory-hp-percent" $VictoryHpPercent

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

if ($BossSegments -and $NoBossSegments) {
    throw "-BossSegments and -NoBossSegments conflict; choose one segment policy."
}

$CombatSegmentMode = "custom"
if (-not (Test-ExtraCombatOptionKey -Tokens $ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
    if ($BossSegments) {
        $DriverArgs += @("--combat-search-option", "segment=turn")
        $CombatSegmentMode = "turn"
    } else {
        $DriverArgs += @("--combat-search-option", "segment=non_boss_turn")
        $CombatSegmentMode = "non_boss_turn"
    }
}

if (-not $NoProgress) {
    $DriverArgs += "--progress"
}

if ($ExtraArgs) {
    $DriverArgs += $ExtraArgs
}

function Test-DriverNeedsBuild {
    param(
        [string] $ExePath
    )

    if (-not (Test-Path -LiteralPath $ExePath)) {
        return $true
    }

    $ExeTime = (Get-Item -LiteralPath $ExePath).LastWriteTimeUtc
    foreach ($Path in @("Cargo.toml", "Cargo.lock")) {
        $FullPath = Join-Path $RepoRoot $Path
        if ((Test-Path -LiteralPath $FullPath) -and (Get-Item -LiteralPath $FullPath).LastWriteTimeUtc -gt $ExeTime) {
            return $true
        }
    }
    foreach ($SourceFile in Get-ChildItem -LiteralPath (Join-Path $RepoRoot "src") -Recurse -File -Filter *.rs) {
        if ($SourceFile.LastWriteTimeUtc -gt $ExeTime) {
            return $true
        }
    }
    return $false
}

Write-Host "seed=$Seed"
$RenderedArgs = $DriverArgs | ForEach-Object {
    if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
}
$RenderedExe = if ($DriverExe -match '^[A-Za-z0-9_./:=\\-]+$') { $DriverExe } else { "'$($DriverExe -replace "'", "''")'" }
$RenderedCommand = $RenderedExe + " " + ($RenderedArgs -join " ")

Write-Host "mode=$Mode branch campaign"
$NeedsBuild = $Build -or (Test-DriverNeedsBuild $DriverExe)
Write-Host "build=$BuildProfile exe=$DriverExe"
if ($NeedsBuild) {
    Write-Host "build-needed=yes"
} else {
    Write-Host "build-needed=no"
}
Write-Host "rerun-last=.\tools\campaign.ps1 -Last"
Write-Host "run-more=.\tools\campaign.ps1 -More"
Write-Host "report=$LatestCampaignPath"
Write-Host "checkpoint=$LatestCheckpointPath"
Write-Host "combat-segment=$CombatSegmentMode"
if ($ResumeCampaignPath) {
    Write-Host "resume=$ResumeCampaignPath"
    Write-Host "resume-rounds=$ResumeRoundsCompleted"
    if ($CampaignBoundParameters.ContainsKey("MaxRounds")) {
        Write-Host "target-rounds=$TargetRounds additional-rounds=$MaxRounds"
    }
    if ($ResumeCheckpointPath) {
        Write-Host "resume-checkpoint=$ResumeCheckpointPath"
    } else {
        Write-Host "resume-checkpoint=missing; falling back to replay"
    }
}

if ($More -and $CampaignBoundParameters.ContainsKey("MaxRounds") -and $MaxRounds -eq 0) {
    Write-Host "already-at-target-rounds=yes; nothing to run"
    exit 0
}

if ($DryRun) {
    if ($NeedsBuild) {
        $RenderedBuildArgs = $BuildArgs | ForEach-Object {
            if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
        }
        Write-Host ("cargo " + ($RenderedBuildArgs -join " "))
    }
    Write-Host $RenderedCommand
    exit 0
}

Set-Content -LiteralPath $LatestSeedPath -Value $Seed
Set-Content -LiteralPath $LatestCommandPath -Value $RenderedCommand

Push-Location $RepoRoot
try {
    if ($NeedsBuild) {
        & cargo @BuildArgs
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    & $DriverExe @DriverArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
