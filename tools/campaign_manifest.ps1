function Write-CampaignWrapperManifest {
    param(
        [string] $Path,
        [object] $Manifest
    )

    if (-not $Path) {
        return
    }
    $Parent = Split-Path -Parent $Path
    if ($Parent) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $Manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Path
}

function Convert-CampaignRequestForManifest {
    param(
        [object] $Request
    )

    if (-not $Request) {
        return [ordered]@{
            schema_name = ""
            kind = ""
            source_intent = ""
            output_intent = ""
            plan_coverage_gaps = $false
            continue_coverage_gaps = $false
            reads_campaign_source = $false
            is_continuation_family = $false
            uses_coverage_gap = $false
        }
    }

    return [ordered]@{
        schema_name = $Request.SchemaName
        kind = $Request.Kind
        source_intent = $Request.SourceIntent
        output_intent = $Request.OutputIntent
        plan_coverage_gaps = [bool] $Request.PlanCoverageGaps
        continue_coverage_gaps = [bool] $Request.ContinueCoverageGaps
        reads_campaign_source = [bool] $Request.ReadsCampaignSource
        is_continuation_family = [bool] $Request.IsContinuationFamily
        uses_coverage_gap = [bool] $Request.UsesCoverageGap
    }
}

function Write-CampaignPrimaryDriverCommandRecord {
    param(
        [string] $PrimaryDriverCommandLine,
        [object] $Context
    )

    if (-not $Context.OutputArtifact) {
        throw "Primary driver command recording requires an output artifact. Plan-only commands should not call this writer."
    }

    Set-Content -LiteralPath $Context.RunCommandPath -Value $PrimaryDriverCommandLine
    if ($Context.OutputArtifact.Kind -eq "run") {
        Write-CampaignLatestPointer -Artifact $Context.OutputArtifact
        Write-Host "latest-pointer=$(Get-CampaignLatestPointerPath)"
    } elseif ($Context.OutputArtifact.Kind -eq "scratch") {
        Write-CampaignScratchLatestPointer -Artifact $Context.OutputArtifact
        Write-Host "scratch-latest-pointer=$(Get-CampaignScratchLatestPointerPath)"
    }
    Write-Host "primary-driver-command=$($Context.RunCommandPath)"
    Write-Host "manifest=$($Context.RunManifestPath)"
}

function New-CampaignWrapperManifestBase {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string] $CommandKind,
        [string[]] $PrimaryDriverArgs,
        [string] $PrimaryDriverCommand,
        [object] $Context
    )

    return [ordered]@{
        schema_name = "CampaignWrapperManifestV1"
        schema_version = 1
        created_at = (Get-Date).ToString("o")
        stage = $Stage
        exit_code = $ExitCode
        wrapper_script = $Context.WrapperScript
        command_kind = $CommandKind
        request = Convert-CampaignRequestForManifest -Request $Context.CampaignRequest
        mode = $Context.Mode
        seed = $Context.Seed
        ascension = $Context.Ascension
        class = $Context.Class
        build_profile = $Context.BuildProfile
        driver_exe = "$($Context.DriverExe)"
        scratch = [bool] $Context.Scratch
        scratch_label = $Context.ScratchLabel
        output_artifact = if ($Context.OutputArtifact) { "$($Context.OutputArtifact.Label)" } else { "" }
        output_report = "$($Context.RunOutputCampaignPath)"
        output_checkpoint = "$($Context.RunOutputCheckpointPath)"
        command_file_semantics = "primary_driver_command"
        command_file = "$($Context.RunCommandPath)"
        manifest_file = "$($Context.RunManifestPath)"
        wrapper_invocation = [ordered]@{
            line = $Context.WrapperInvocationLine
            bound_parameters = $Context.WrapperBoundParameters
        }
        primary_driver = [ordered]@{
            args = @($PrimaryDriverArgs)
            command = $PrimaryDriverCommand
            command_file = "$($Context.RunCommandPath)"
        }
    }
}

function New-CampaignRunWrapperManifest {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext,
        [object] $Context
    )

    $Manifest = New-CampaignWrapperManifestBase `
        -ExitCode $ExitCode `
        -Stage $Stage `
        -CommandKind "campaign_run" `
        -PrimaryDriverArgs $Context.DriverArgs `
        -PrimaryDriverCommand $Context.RenderedCommand `
        -Context $Context
    $Manifest["resume_report"] = if ($Context.ResumeCampaignPath) { "$($Context.ResumeCampaignPath)" } else { "" }
    $Manifest["resume_checkpoint"] = if ($Context.ResumeCheckpointPath) { "$($Context.ResumeCheckpointPath)" } else { "" }
    $Manifest["log_file"] = if ($Context.Log) { "$($Context.RunLogPath)" } else { "" }
    $Manifest["round_budget"] = [ordered]@{
        source = $Context.RoundBudgetSource
        target_rounds = $Context.TargetRounds
        additional_rounds = $Context.RoundBudgetAdditionalRounds
    }

    if ($Context.UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs -RunIdentityArgs $RunIdentityArgs -StepRounds $Context.MilestoneStepRounds -OptionContext $OptionContext
        $Manifest["milestone"] = [ordered]@{
            target = $Context.UntilMilestone
            stop = $Context.ResolvedMilestoneStop
            step_rounds = $Context.MilestoneStepRounds
            max_additional_rounds = $Context.MilestoneMaxRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $Context.DriverExe -Arguments $MilestoneResumeArgs)
        }
    }

    return $Manifest
}
