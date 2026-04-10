param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [switch]$DryRun
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$profilesDir = Join-Path $scriptDir "profiles"
$targetPath = Join-Path $scriptDir "profile.json"

$candidates = @(
    (Join-Path $profilesDir $Name),
    (Join-Path $profilesDir "$Name.json")
)

$sourcePath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $sourcePath) {
    $available = Get-ChildItem $profilesDir -Filter *.json -File | Select-Object -ExpandProperty BaseName
    throw "Profile '$Name' not found. Available profiles: $($available -join ', ')"
}

$sourceContent = Get-Content $sourcePath -Raw
if ($DryRun) {
    [pscustomobject]@{
        source = $sourcePath
        target = $targetPath
        content = ($sourceContent | ConvertFrom-Json)
    } | ConvertTo-Json -Depth 12
    exit 0
}

Set-Content -Path $targetPath -Value $sourceContent -NoNewline
[pscustomobject]@{
    source = $sourcePath
    target = $targetPath
    activated_profile = (Split-Path $sourcePath -Leaf)
} | ConvertTo-Json -Depth 4
