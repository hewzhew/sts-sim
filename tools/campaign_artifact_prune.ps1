function ConvertTo-CampaignArtifactPruneSet {
    param(
        [string[]] $Paths
    )

    $Set = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($Path in @($Paths | Where-Object { $_ })) {
        if (Test-Path -LiteralPath $Path) {
            $Resolved = (Resolve-Path -LiteralPath $Path).Path
            [void] $Set.Add($Resolved)
        }
    }
    return ,$Set
}

function Add-CampaignArtifactProtectedPath {
    param(
        [System.Collections.Generic.HashSet[string]] $Set,
        [string] $Path
    )

    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
        return
    }
    [void] $Set.Add((Resolve-Path -LiteralPath $Path).Path)
}

function Add-CampaignArtifactProtectedDirectory {
    param(
        [System.Collections.Generic.HashSet[string]] $Set,
        [string] $Path
    )

    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
        return
    }
    Get-ChildItem -LiteralPath $Path -Recurse -File | ForEach-Object {
        Add-CampaignArtifactProtectedPath -Set $Set -Path $_.FullName
    }
}

function Add-CampaignArtifactProtectedPointerFiles {
    param(
        [System.Collections.Generic.HashSet[string]] $Set
    )

    Add-CampaignArtifactProtectedPath -Set $Set -Path (Get-CampaignLatestPointerPath)
    Add-CampaignArtifactProtectedPath -Set $Set -Path (Get-CampaignScratchLatestPointerPath)

    $Latest = Read-CampaignLatestPointer
    if ($Latest) {
        foreach ($Name in @("report", "state", "journal", "checkpoint", "manifest", "command", "log")) {
            Add-CampaignArtifactProtectedPath -Set $Set -Path ([string] $Latest.$Name)
        }
    }

    $ScratchLatest = Read-CampaignScratchLatestPointer
    if ($ScratchLatest) {
        foreach ($Name in @("report", "state", "journal", "checkpoint", "manifest", "command", "log")) {
            Add-CampaignArtifactProtectedPath -Set $Set -Path ([string] $ScratchLatest.$Name)
        }
    }

    foreach ($Path in @(
        $script:LegacyLatestModePath,
        $script:LegacyLatestCommandPath,
        $script:LegacyLatestManifestPath,
        $script:LegacyLatestLogPath,
        $script:LegacyLatestCampaignPath,
        $script:LegacyLatestCheckpointPath
    )) {
        Add-CampaignArtifactProtectedPath -Set $Set -Path $Path
    }
}

function Get-CampaignScratchArtifactGroupId {
    param(
        [string] $FileName
    )

    $Id = $FileName
    foreach ($Suffix in @(
        ".decision_outcomes.after.jsonl",
        ".campaign.state.json.gz",
        ".campaign.state.json",
        ".campaign.journal.json.gz",
        ".campaign.journal.json",
        ".campaign.json.gz",
        ".campaign.json",
        ".checkpoint.json.gz",
        ".checkpoint.json",
        ".manifest.json",
        ".command.txt",
        ".log"
    )) {
        if ($Id.EndsWith($Suffix, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $Id.Substring(0, $Id.Length - $Suffix.Length)
        }
    }
    return $Id
}

function Get-CampaignArtifactPruneCandidates {
    param(
        [int] $KeepRuns = 5,
        [int] $KeepScratch = 1
    )

    Assert-CampaignArtifactPathsInitialized
    $Root = (Resolve-Path -LiteralPath $script:CampaignDir).Path
    $Protected = ConvertTo-CampaignArtifactPruneSet -Paths @()
    Add-CampaignArtifactProtectedPointerFiles -Set $Protected

    $RunsDir = Get-CampaignRunsDir
    if (Test-Path -LiteralPath $RunsDir) {
        $RunDirs = @(Get-ChildItem -LiteralPath $RunsDir -Directory | Sort-Object LastWriteTimeUtc -Descending)
        foreach ($Dir in @($RunDirs | Select-Object -First $KeepRuns)) {
            Add-CampaignArtifactProtectedDirectory -Set $Protected -Path $Dir.FullName
        }
    }

    if (Test-Path -LiteralPath $script:ScratchCampaignDir) {
        $ScratchFiles = @(Get-ChildItem -LiteralPath $script:ScratchCampaignDir -File)
        $ScratchGroups = @(
            $ScratchFiles |
                Where-Object { $_.Name -ne "latest.json" } |
                Group-Object { Get-CampaignScratchArtifactGroupId -FileName $_.Name } |
                ForEach-Object {
                    $Newest = ($_.Group | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1)
                    [pscustomobject]@{
                        Id = $_.Name
                        NewestUtc = $Newest.LastWriteTimeUtc
                        Files = @($_.Group)
                    }
                } |
                Sort-Object NewestUtc -Descending
        )
        foreach ($Group in @($ScratchGroups | Select-Object -First $KeepScratch)) {
            foreach ($File in $Group.Files) {
                Add-CampaignArtifactProtectedPath -Set $Protected -Path $File.FullName
            }
        }
    }

    $Candidates = New-Object System.Collections.Generic.List[object]
    $AllFiles = @(Get-ChildItem -LiteralPath $Root -Recurse -File)
    foreach ($File in $AllFiles) {
        $Resolved = (Resolve-Path -LiteralPath $File.FullName).Path
        if (-not $Resolved.StartsWith($Root, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to inspect path outside campaign artifact root: $Resolved"
        }
        if ($Protected.Contains($Resolved)) {
            continue
        }

        $Relative = $Resolved.Substring($Root.Length).TrimStart("\")
        $Top = ($Relative -split "\\")[0]
        $Class = if ($Top -eq "runs") {
            "old_run"
        } elseif ($Top -eq "scratch") {
            "old_scratch"
        } elseif ($Top -eq "perf") {
            "perf"
        } elseif ($Top -eq "diagnostics") {
            "diagnostic"
        } elseif ($Top -like "samples-*") {
            "sample"
        } elseif ($Relative -notmatch "\\") {
            "loose_root"
        } else {
            "other"
        }

        $Candidates.Add([pscustomobject]@{
            Class = $Class
            Path = $Resolved
            RelativePath = $Relative
            Bytes = [long] $File.Length
            LastWriteTimeUtc = $File.LastWriteTimeUtc
        })
    }

    return @($Candidates | Sort-Object Class, RelativePath)
}

function Remove-CampaignArtifactEmptyDirectories {
    Assert-CampaignArtifactPathsInitialized
    $Root = (Resolve-Path -LiteralPath $script:CampaignDir).Path
    Get-ChildItem -LiteralPath $Root -Recurse -Directory |
        Sort-Object FullName -Descending |
        ForEach-Object {
            $Resolved = (Resolve-Path -LiteralPath $_.FullName).Path
            if (-not $Resolved.StartsWith($Root, [System.StringComparison]::OrdinalIgnoreCase)) {
                throw "Refusing to remove directory outside campaign artifact root: $Resolved"
            }
            if (-not (Get-ChildItem -LiteralPath $Resolved -Force | Select-Object -First 1)) {
                Remove-Item -LiteralPath $Resolved -Force
            }
        }
}

function Invoke-CampaignArtifactPrune {
    param(
        [int] $KeepRuns = 5,
        [int] $KeepScratch = 1,
        [bool] $Apply = $false
    )

    $Candidates = @(Get-CampaignArtifactPruneCandidates -KeepRuns $KeepRuns -KeepScratch $KeepScratch)
    $TotalBytes = [long] (($Candidates | Measure-Object Bytes -Sum).Sum)
    $Mode = if ($Apply) { "apply" } else { "dry-run" }

    Write-Host "CampaignArtifactPruneV1 mode=$Mode candidates=$($Candidates.Count) reclaim=$(Format-CampaignArtifactSize -Bytes $TotalBytes) keep_runs=$KeepRuns keep_scratch=$KeepScratch"
    $Candidates |
        Group-Object Class |
        Sort-Object { ($_.Group | Measure-Object Bytes -Sum).Sum } -Descending |
        ForEach-Object {
            $Bytes = [long] (($_.Group | Measure-Object Bytes -Sum).Sum)
            Write-Host ("  {0,-12} files={1,4} bytes={2,10}" -f $_.Name, $_.Count, (Format-CampaignArtifactSize -Bytes $Bytes))
        }

    Write-Host "Largest candidates:"
    foreach ($Candidate in @($Candidates | Sort-Object Bytes -Descending | Select-Object -First 12)) {
        Write-Host ("  {0,10} | {1,-12} | {2}" -f (Format-CampaignArtifactSize -Bytes $Candidate.Bytes), $Candidate.Class, $Candidate.RelativePath)
    }

    if (-not $Apply) {
        Write-Host "No files deleted. Re-run with -PruneApply to remove these candidates."
        return 0
    }

    $Root = (Resolve-Path -LiteralPath $script:CampaignDir).Path
    foreach ($Candidate in $Candidates) {
        $Resolved = (Resolve-Path -LiteralPath $Candidate.Path).Path
        if (-not $Resolved.StartsWith($Root, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to delete path outside campaign artifact root: $Resolved"
        }
        Remove-Item -LiteralPath $Resolved -Force
    }
    Remove-CampaignArtifactEmptyDirectories
    Write-Host "Deleted $($Candidates.Count) campaign artifact file(s)."
    return 0
}
