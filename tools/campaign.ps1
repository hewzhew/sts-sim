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
.\tools\campaign.ps1 -Mode quick
Runs a shorter random-seed campaign for fast smoke testing.

.EXAMPLE
.\tools\campaign.ps1 -Mode deep
Runs a larger random-seed campaign when you want to leave it working longer.

.EXAMPLE
.\tools\campaign.ps1 -DryRun
Prints the cargo command without updating the last seed or running it.
#>
param(
    [Parameter(Position = 0)]
    [long] $Seed = 0,

    [switch] $Last,
    [switch] $DryRun,

    [ValidateSet("quick", "focused", "deep")]
    [string] $Mode = "focused",

    [int] $MaxRounds = 6,
    [int] $ExperimentWallMs = 10000,
    [int] $SearchWallMs = 300,
    [int] $SearchMaxNodes = 50000,
    [int] $BranchExamples = 4,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$CampaignDir = Join-Path $RepoRoot "tools\artifacts\campaigns"
$LatestSeedPath = Join-Path $CampaignDir "latest.seed.txt"
$LatestCommandPath = Join-Path $CampaignDir "latest.command.txt"

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

switch ($Mode) {
    "quick" {
        if (-not $PSBoundParameters.ContainsKey("MaxRounds")) { $MaxRounds = 3 }
        if (-not $PSBoundParameters.ContainsKey("ExperimentWallMs")) { $ExperimentWallMs = 3000 }
        if (-not $PSBoundParameters.ContainsKey("SearchWallMs")) { $SearchWallMs = 50 }
        if (-not $PSBoundParameters.ContainsKey("SearchMaxNodes")) { $SearchMaxNodes = 5000 }
        if (-not $PSBoundParameters.ContainsKey("BranchExamples")) { $BranchExamples = 3 }
    }
    "deep" {
        if (-not $PSBoundParameters.ContainsKey("MaxRounds")) { $MaxRounds = 10 }
        if (-not $PSBoundParameters.ContainsKey("ExperimentWallMs")) { $ExperimentWallMs = 30000 }
        if (-not $PSBoundParameters.ContainsKey("SearchWallMs")) { $SearchWallMs = 1000 }
        if (-not $PSBoundParameters.ContainsKey("SearchMaxNodes")) { $SearchMaxNodes = 200000 }
        if (-not $PSBoundParameters.ContainsKey("BranchExamples")) { $BranchExamples = 6 }
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

$DriverArgs = @(
    "run", "--quiet", "--bin", "branch_campaign_driver", "--",
    "--preset", "focused",
    "--seed", "$Seed",
    "--max-rounds", "$MaxRounds",
    "--experiment-wall-ms", "$ExperimentWallMs",
    "--search-wall-ms", "$SearchWallMs",
    "--search-max-nodes", "$SearchMaxNodes",
    "--branch-examples", "$BranchExamples"
)

if ($ExtraArgs) {
    $DriverArgs += $ExtraArgs
}

Write-Host "seed=$Seed"
$RenderedArgs = $DriverArgs | ForEach-Object {
    if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
}
$RenderedCommand = "cargo " + ($RenderedArgs -join " ")

Write-Host "mode=$Mode branch campaign"
Write-Host "rerun-last=.\tools\campaign.ps1 -Last"

if ($DryRun) {
    Write-Host $RenderedCommand
    exit 0
}

Set-Content -LiteralPath $LatestSeedPath -Value $Seed
Set-Content -LiteralPath $LatestCommandPath -Value $RenderedCommand

Push-Location $RepoRoot
try {
    & cargo @DriverArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
