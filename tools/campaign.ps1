<#
.SYNOPSIS
Minimal campaign launcher.

.DESCRIPTION
This wrapper intentionally owns only three lifecycle concepts:
source resolution, output allocation, and minimal continuation.
All campaign strategy, coverage experiments, milestone orchestration, and
artifact semantics belong to the Rust driver.

.EXAMPLE
.\tools\campaign.ps1 -Mode quick
Run a new quick campaign on a random seed.

.EXAMPLE
.\tools\campaign.ps1 -From latest -Continue -Mode quick -Rounds 2
Continue the latest campaign for two additional rounds into a new artifact.

.EXAMPLE
.\tools\campaign.ps1 -From latest -Inspect
Inspect the latest source checkpoint summary.
#>
param(
    [Parameter(Position = 0)]
    [long] $Seed = 0,

    [Alias("Continue")]
    [switch] $ContinueRun,
    [switch] $Inspect,
    [switch] $DryRun,
    [switch] $Build,
    [switch] $Log,
    [switch] $NoProgress,
    [switch] $VerboseProgress,

    [string] $From = "",
    [string] $RunLabel = "",

    [ValidateSet("fast-run", "release-final", "release", "dev-opt", "debug")]
    [string] $BuildProfile = "fast-run",

    [ValidateSet("quick", "focused", "explore", "deep")]
    [string] $Mode = "explore",

    [ValidateSet("", "package", "balanced", "exploration", "survival", "advisory_only")]
    [string] $RetentionProfile = "",

    [ValidateRange(0, 100000)]
    [int] $Rounds = 0,

    [ValidateRange(0, 100000)]
    [int] $UntilRound = 0,

    [ValidateRange(0, 20)]
    [int] $Ascension = 0,

    [ValidateSet("", "a0", "a10", "a15", "a17", "a20")]
    [string] $Domain = "",

    [ValidateSet("ironclad", "silent", "defect", "watcher")]
    [string] $Class = "ironclad",

    [int] $ExperimentWallMs = 10000,
    [int] $SearchWallMs = 300,
    [int] $SearchMaxNodes = 50000,
    [int] $BranchExamples = 4,

    [Alias("Passthrough")]
    [string[]] $DriverArgs = @(),

    [Parameter(ValueFromRemainingArguments = $true, DontShow = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$CampaignDir = Join-Path $PSScriptRoot "artifacts\campaigns"

function Get-DriverExe {
    param(
        [string] $RepoRoot,
        [string] $BuildProfile
    )

    if ($BuildProfile -eq "debug") {
        return Join-Path $RepoRoot "target\debug\branch_campaign_driver.exe"
    }
    return Join-Path $RepoRoot "target\$BuildProfile\branch_campaign_driver.exe"
}

function Invoke-DriverBuild {
    param(
        [string] $RepoRoot,
        [string] $BuildProfile
    )

    Push-Location $RepoRoot
    try {
        $Args = @("build", "--quiet", "--bin", "branch_campaign_driver")
        if ($BuildProfile -ne "debug") {
            $Args += @("--profile", $BuildProfile)
        }
        & cargo @Args
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

function Test-DriverBuildNeeded {
    param(
        [string] $RepoRoot,
        [string] $DriverExe
    )

    if (-not (Test-Path -LiteralPath $DriverExe)) {
        return $true
    }
    $ExeTime = (Get-Item -LiteralPath $DriverExe).LastWriteTimeUtc
    $CargoToml = Join-Path $RepoRoot "Cargo.toml"
    $CargoLock = Join-Path $RepoRoot "Cargo.lock"
    foreach ($Path in @($CargoToml, $CargoLock)) {
        if ((Test-Path -LiteralPath $Path) -and (Get-Item -LiteralPath $Path).LastWriteTimeUtc -gt $ExeTime) {
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

function Invoke-DriverJson {
    param(
        [string] $DriverExe,
        [string[]] $Arguments
    )

    $JsonText = & $DriverExe @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "branch_campaign_driver failed with exit code $LASTEXITCODE while running: $DriverExe $($Arguments -join ' ')"
    }
    try {
        return ($JsonText | ConvertFrom-Json)
    } catch {
        throw "branch_campaign_driver returned invalid JSON for: $DriverExe $($Arguments -join ' ')"
    }
}

function Get-AscensionFromDomain {
    param([string] $Domain)
    switch ($Domain) {
        "a0" { return 0 }
        "a10" { return 10 }
        "a15" { return 15 }
        "a17" { return 17 }
        "a20" { return 20 }
        default { return $null }
    }
}

function Resolve-SourceInfo {
    param(
        [string] $DriverExe,
        [string] $Selector
    )

    if (-not $Selector) {
        $Selector = "latest"
    }
    return Invoke-DriverJson -DriverExe $DriverExe -Arguments @(
        "campaign", "artifacts", "source-info",
        "$Selector",
        "--campaign-dir", "$CampaignDir",
        "--json"
    )
}

function New-OutputArtifact {
    param(
        [string] $DriverExe,
        [string] $Label
    )

    return Invoke-DriverJson -DriverExe $DriverExe -Arguments @(
        "campaign", "artifacts", "allocate",
        "--kind", "run",
        "--label", "$Label",
        "--campaign-dir", "$CampaignDir",
        "--json"
    )
}

function Write-LatestPointer {
    param(
        [string] $DriverExe,
        [string] $ArtifactId
    )

    $UpdatedAt = [DateTime]::UtcNow.ToString("o")
    [void] (Invoke-DriverJson -DriverExe $DriverExe -Arguments @(
        "campaign", "artifacts", "write-latest",
        "--kind", "run",
        "$ArtifactId",
        "--updated-at", "$UpdatedAt",
        "--campaign-dir", "$CampaignDir",
        "--json"
    ))
}

function Format-CommandLine {
    param(
        [string] $ExePath,
        [string[]] $Arguments
    )

    $Parts = @("&", $ExePath)
    foreach ($Arg in $Arguments) {
        if ($Arg -match "\s") {
            $Parts += '"' + ($Arg -replace '"', '\"') + '"'
        } else {
            $Parts += $Arg
        }
    }
    return ($Parts -join " ")
}

function Invoke-DriverText {
    param(
        [string] $DriverExe,
        [string[]] $Arguments,
        [string] $LogPath
    )

    $ProcessArgs = @{
        FilePath     = $DriverExe
        ArgumentList = $Arguments
        NoNewWindow  = $true
        Wait         = $true
        PassThru     = $true
    }

    if ($LogPath) {
        $Parent = Split-Path -Parent $LogPath
        if ($Parent) {
            New-Item -ItemType Directory -Force -Path $Parent | Out-Null
        }
        $ProcessArgs.RedirectStandardOutput = $LogPath
        $ProcessArgs.RedirectStandardError = "$LogPath.stderr"
    }

    $Process = Start-Process @ProcessArgs
    if ($LogPath) {
        if (Test-Path -LiteralPath $LogPath) {
            Get-Content -LiteralPath $LogPath | ForEach-Object { Write-Host $_ }
        }
        $StdErrLog = "$LogPath.stderr"
        if (Test-Path -LiteralPath $StdErrLog) {
            Get-Content -LiteralPath $StdErrLog | ForEach-Object { Write-Host $_ }
        }
    }
    return $Process.ExitCode
}

if ($ExtraArgs -and $ExtraArgs.Count -gt 0) {
    $DriverArgs += @($ExtraArgs)
}

if ($Domain) {
    $DomainAscension = Get-AscensionFromDomain -Domain $Domain
    if ($Ascension -ne 0 -and $Ascension -ne $DomainAscension) {
        throw "-Domain $Domain implies -Ascension $DomainAscension, but -Ascension $Ascension was provided."
    }
    $Ascension = $DomainAscension
}

$DriverExe = Get-DriverExe -RepoRoot $RepoRoot -BuildProfile $BuildProfile
if ($Build -or (Test-DriverBuildNeeded -RepoRoot $RepoRoot -DriverExe $DriverExe)) {
    Invoke-DriverBuild -RepoRoot $RepoRoot -BuildProfile $BuildProfile
}
if (-not (Test-Path -LiteralPath $DriverExe)) {
    throw "branch_campaign_driver.exe was not found at $DriverExe"
}

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

$SourceInfo = $null
if ($ContinueRun -or $Inspect) {
    $SourceInfo = Resolve-SourceInfo -DriverExe $DriverExe -Selector $From
    if ($SourceInfo.run_config) {
        if ($Seed -le 0 -and $SourceInfo.run_config.seed -ne $null) {
            $Seed = [long] $SourceInfo.run_config.seed
        }
        if (-not $PSBoundParameters.ContainsKey("Ascension") -and $SourceInfo.run_config.ascension -ne $null) {
            $Ascension = [int] $SourceInfo.run_config.ascension
        }
        if (-not $PSBoundParameters.ContainsKey("Class") -and $SourceInfo.run_config.class) {
            $Class = ([string] $SourceInfo.run_config.class).ToLowerInvariant()
        }
        if (-not $PSBoundParameters.ContainsKey("Mode") -and $SourceInfo.run_config.mode) {
            $Mode = ([string] $SourceInfo.run_config.mode).ToLowerInvariant()
        }
    }
    if ($ContinueRun -and -not $PSBoundParameters.ContainsKey("Mode") -and (-not $SourceInfo.run_config -or -not $SourceInfo.run_config.mode)) {
        throw "Source artifact does not record a campaign mode; pass -Mode explicitly for continuation."
    }
}

if ($Seed -le 0) {
    $Seed = Get-Random -Minimum 1 -Maximum 2147483647
}

if ($Inspect) {
    if (-not $SourceInfo) {
        $SourceInfo = Resolve-SourceInfo -DriverExe $DriverExe -Selector $From
    }
    $InspectArgs = @(
        "campaign", "inspect",
        "--inspect-report", "$($SourceInfo.artifact.report_path)",
        "--inspect-checkpoint", "$($SourceInfo.artifact.checkpoint_path)",
        "--inspect-summary"
    )
    $InspectArgs += @($DriverArgs)
    Write-Host "source=$($SourceInfo.artifact.label)"
    Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $InspectArgs)
    if ($DryRun) {
        exit 0
    }
    exit (Invoke-DriverText -DriverExe $DriverExe -Arguments $InspectArgs -LogPath "")
}

$Label = $RunLabel
if (-not $Label) {
    if ($ContinueRun) {
        $Label = "continue-seed$Seed"
    } else {
        $Label = "campaign-seed$Seed"
    }
}

$OutputArtifact = New-OutputArtifact -DriverExe $DriverExe -Label $Label

$RunArgs = @("campaign")
if ($ContinueRun) {
    $RunArgs += @("continue")
} else {
    $RunArgs += @("run")
}
$RunArgs += @(
    "--preset", "$Mode",
    "--seed", "$Seed",
    "--ascension", "$Ascension",
    "--class", "$Class",
    "--out", "$($OutputArtifact.report_path)",
    "--checkpoint-out", "$($OutputArtifact.checkpoint_path)",
    "--experiment-wall-ms", "$ExperimentWallMs",
    "--search-wall-ms", "$SearchWallMs",
    "--search-max-nodes", "$SearchMaxNodes",
    "--branch-examples", "$BranchExamples"
)
if ($Domain) {
    $RunArgs += @("--ascension-domain", "$Domain")
}
if ($RetentionProfile) {
    $RunArgs += @("--retention-profile", "$RetentionProfile")
}
if ($ContinueRun) {
    if (-not $SourceInfo) {
        $SourceInfo = Resolve-SourceInfo -DriverExe $DriverExe -Selector $From
    }
    $RunArgs += @(
        "--resume", "$($SourceInfo.artifact.report_path)",
        "--resume-checkpoint", "$($SourceInfo.artifact.checkpoint_path)"
    )
}
if ($Rounds -gt 0) {
    $RunArgs += @("--rounds", "$Rounds")
}
if ($UntilRound -gt 0) {
    $RunArgs += @("--until-round", "$UntilRound")
}
if (-not $NoProgress) {
    $RunArgs += @("--progress")
    if ($VerboseProgress) {
        $RunArgs += @("--progress-detail", "verbose")
    }
}
$RunArgs += @($DriverArgs)

Write-Host "seed=$Seed"
Write-Host "mode=$Mode"
if ($ContinueRun) {
    Write-Host "source=$($SourceInfo.artifact.label)"
}
Write-Host "out=$($OutputArtifact.report_path)"
Write-Host "checkpoint=$($OutputArtifact.checkpoint_path)"
Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $RunArgs)

if ($DryRun) {
    exit 0
}

$LogPath = if ($Log) { "$($OutputArtifact.log_path)" } else { "" }
$ExitCode = Invoke-DriverText -DriverExe $DriverExe -Arguments $RunArgs -LogPath $LogPath
if ($ExitCode -eq 0) {
    Write-LatestPointer -DriverExe $DriverExe -ArtifactId "$($OutputArtifact.id)"
    Write-Host "latest=run:$($OutputArtifact.id)"
}
exit $ExitCode
