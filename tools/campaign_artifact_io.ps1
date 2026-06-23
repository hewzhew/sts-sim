function Read-CampaignArtifactText {
    param(
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    $Bytes = [System.IO.File]::ReadAllBytes($Path)
    $IsGzip = $Path.EndsWith(".gz", [System.StringComparison]::OrdinalIgnoreCase)
    if (-not $IsGzip -and $Bytes.Length -ge 2) {
        $IsGzip = ($Bytes[0] -eq 0x1f -and $Bytes[1] -eq 0x8b)
    }

    if ($IsGzip) {
        $InputStream = [System.IO.MemoryStream]::new($Bytes)
        $GzipStream = $null
        $Reader = $null
        try {
            $GzipStream = [System.IO.Compression.GzipStream]::new(
                $InputStream,
                [System.IO.Compression.CompressionMode]::Decompress
            )
            $Reader = [System.IO.StreamReader]::new($GzipStream)
            return $Reader.ReadToEnd()
        } finally {
            if ($Reader) {
                $Reader.Dispose()
            } elseif ($GzipStream) {
                $GzipStream.Dispose()
            }
            $InputStream.Dispose()
        }
    }

    return [System.Text.Encoding]::UTF8.GetString($Bytes)
}

function Read-CampaignJsonArtifact {
    param(
        [string] $Path
    )

    try {
        $Text = Read-CampaignArtifactText -Path $Path
        if ($null -eq $Text) {
            return $null
        }
        return $Text | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Read-CampaignJsonArtifactOrThrow {
    param(
        [string] $Path,
        [string] $Role = "campaign artifact"
    )

    $Text = Read-CampaignArtifactText -Path $Path
    if ($null -eq $Text) {
        throw "Missing $Role at $Path"
    }
    try {
        return $Text | ConvertFrom-Json
    } catch {
        throw "Failed to parse $Role at $Path as JSON: $($_.Exception.Message)"
    }
}
