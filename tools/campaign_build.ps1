function Resolve-CampaignBuildContext {
    param(
        [string] $RepoRoot,
        [string] $BuildProfile,
        [bool] $DebugBuild,
        [bool] $BuildProfileBound
    )

    $ResolvedBuildProfile = $BuildProfile
    if ($DebugBuild) {
        if ($BuildProfileBound -and $BuildProfile -ne "debug") {
            throw "-DebugBuild conflicts with -BuildProfile $BuildProfile. Use only one build profile selector."
        }
        $ResolvedBuildProfile = "debug"
    }

    $DriverExe = Join-Path $RepoRoot "target\$ResolvedBuildProfile\branch_campaign_driver.exe"
    $BuildArgs = @("build", "--quiet", "--bin", "branch_campaign_driver")
    switch ($ResolvedBuildProfile) {
        "debug" {
            # Default cargo dev profile.
        }
        "release" {
            $BuildArgs += "--release"
        }
        default {
            $BuildArgs += @("--profile", "$ResolvedBuildProfile")
        }
    }

    return [pscustomobject]@{
        BuildProfile = $ResolvedBuildProfile
        DriverExe = $DriverExe
        BuildArgs = $BuildArgs
    }
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

function Write-CampaignBuildCommandPreview {
    param(
        [string[]] $BuildArgs
    )

    $RenderedBuildArgs = $BuildArgs | ForEach-Object {
        if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
    }
    Write-Host ("cargo " + ($RenderedBuildArgs -join " "))
}
