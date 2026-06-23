$script:CampaignPathContext = $null
$script:CampaignDir = ""
$script:ScratchCampaignDir = ""
$script:LegacyLatestModePath = ""
$script:LegacyLatestCommandPath = ""
$script:LegacyLatestManifestPath = ""
$script:LegacyLatestLogPath = ""
$script:LegacyLatestCampaignPath = ""
$script:LegacyLatestCheckpointPath = ""

. (Join-Path $PSScriptRoot "campaign_artifact_paths.ps1")
. (Join-Path $PSScriptRoot "campaign_artifact_refs.ps1")
. (Join-Path $PSScriptRoot "campaign_artifact_legacy.ps1")
. (Join-Path $PSScriptRoot "campaign_artifact_pointers.ps1")
. (Join-Path $PSScriptRoot "campaign_artifact_source.ps1")
