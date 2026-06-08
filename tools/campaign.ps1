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
Reuses the last seed with deeper defaults.

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
    [switch] $More,
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

$DriverArgs = @(
    "run", "--quiet", "--bin", "branch_campaign_driver", "--",
    "--preset", "$Mode",
    "--seed", "$Seed"
)

$CampaignBoundParameters = @{}
foreach ($ParameterName in $PSBoundParameters.Keys) {
    $CampaignBoundParameters[$ParameterName] = $true
}

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
Write-Host "run-more=.\tools\campaign.ps1 -More"

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
