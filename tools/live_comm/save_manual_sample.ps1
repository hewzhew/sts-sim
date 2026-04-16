param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Slice,
    [string]$SamplesRoot = "D:\rust\sts_simulator\logs\manual_scenario_samples",
    [string]$CurrentRoot = "D:\rust\sts_simulator\logs\current",
    [string]$Note = "",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Normalize-SliceName {
    param([string]$Value)
    return ($Value.Trim().ToLowerInvariant() -replace '[^a-z0-9]+', '_').Trim('_')
}

$normalizedSlice = Normalize-SliceName $Slice
if ([string]::IsNullOrWhiteSpace($normalizedSlice)) {
    throw "Slice name must contain at least one alphanumeric character."
}

$framePath = Join-Path $CurrentRoot "manual_client_latest.json"
$rawPath = Join-Path $CurrentRoot "manual_client_raw.jsonl"
$bridgeLogPath = Join-Path $CurrentRoot "manual_client_bridge.log"

if (-not (Test-Path -LiteralPath $framePath)) {
    throw "Missing manual frame: $framePath"
}

$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$destDir = Join-Path $SamplesRoot "${normalizedSlice}_${timestamp}"

$files = @(
    @{ source = $framePath; target = "frame.json"; required = $true },
    @{ source = $rawPath; target = "manual_client_raw.jsonl"; required = $false },
    @{ source = $bridgeLogPath; target = "manual_client_bridge.log"; required = $false }
)

$summary = [ordered]@{
    slice = $Slice
    normalized_slice = $normalizedSlice
    saved_at = (Get-Date).ToString("o")
    source = $framePath
    note = $(if ([string]::IsNullOrWhiteSpace($Note)) { $null } else { $Note })
    files = @()
}

if ($DryRun) {
    $existingFiles = @()
    foreach ($file in $files) {
        if (Test-Path -LiteralPath $file.source) {
            $existingFiles += $file.target
        }
    }
    $summary.files = $existingFiles
    [ordered]@{
        destination = $destDir
        summary = $summary
    } | ConvertTo-Json -Depth 5
    exit 0
}

New-Item -ItemType Directory -Path $destDir -Force | Out-Null

foreach ($file in $files) {
    if (Test-Path -LiteralPath $file.source) {
        Copy-Item -LiteralPath $file.source -Destination (Join-Path $destDir $file.target) -Force
        $summary.files += $file.target
    } elseif ($file.required) {
        throw "Required source file missing: $($file.source)"
    }
}

$summary | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destDir "summary.json") -Encoding UTF8

Write-Output $destDir
