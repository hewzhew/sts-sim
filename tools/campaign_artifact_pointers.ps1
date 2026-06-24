function Read-CampaignLatestPointer {
    $PointerPath = Get-CampaignLatestPointerPath
    if (-not (Test-Path -LiteralPath $PointerPath)) {
        return $null
    }
    try {
        $Pointer = Get-Content -LiteralPath $PointerPath -Raw | ConvertFrom-Json
        if ($Pointer.schema_name -ne "CampaignLatestPointerV1") {
            return $null
        }
        if (-not $Pointer.artifact_id) {
            return $null
        }
        return $Pointer
    } catch {
        return $null
    }
}

function Read-CampaignScratchLatestPointer {
    $PointerPath = Get-CampaignScratchLatestPointerPath
    if (-not (Test-Path -LiteralPath $PointerPath)) {
        return $null
    }
    try {
        $Pointer = Get-Content -LiteralPath $PointerPath -Raw | ConvertFrom-Json
        if ($Pointer.schema_name -ne "CampaignScratchLatestPointerV1") {
            return $null
        }
        if (-not $Pointer.artifact_id) {
            return $null
        }
        return $Pointer
    } catch {
        return $null
    }
}
