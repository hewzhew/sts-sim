function Invoke-CampaignArtifactPrune {
    param(
        [Parameter(Mandatory = $true)]
        [string] $DriverExe,
        [Parameter(Mandatory = $true)]
        [string] $CampaignDir,
        [int] $KeepRuns = 5,
        [int] $KeepScratch = 1,
        [bool] $Apply = $false,
        [bool] $Json = $false
    )

    $Args = @(
        "artifact",
        "prune",
        "--campaign-dir",
        $CampaignDir,
        "--keep-runs",
        "$KeepRuns",
        "--keep-scratch",
        "$KeepScratch"
    )
    if ($Apply) {
        $Args += "--apply"
    }
    if ($Json) {
        $Args += "--json"
    }

    & $DriverExe @Args | ForEach-Object { Write-Host $_ }
    return $LASTEXITCODE
}
