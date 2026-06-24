function Write-CampaignWrapperManifest {
    param(
        [string] $Path,
        [Alias("Manifest")]
        [object] $Payload,
        [object] $Context
    )

    if (-not $Path) {
        return
    }
    if (-not $Context -or -not $Context.DriverExe) {
        throw "Rust campaign manifest writer requires DriverExe."
    }

    $PayloadJson = $Payload | ConvertTo-Json -Depth 24 -Compress
    $ManifestArgs = @(
        "artifact",
        "write-manifest",
        "--manifest-path", "$Path",
        "--payload-schema-name", "CampaignWrapperManifestPayloadV1",
        "--created-at", (Get-Date).ToString("o"),
        "--json"
    )
    $Output = $PayloadJson | & $Context.DriverExe @ManifestArgs 2>&1
    $ExitCode = $LASTEXITCODE
    if ($ExitCode -ne 0) {
        $RenderedOutput = ($Output | Out-String).Trim()
        throw "Rust campaign manifest writer failed with exit code $ExitCode. $RenderedOutput"
    }
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

function Convert-CampaignDriverPassthroughForManifest {
    param(
        [object] $Context
    )

    $OptionContext = $null
    if ($Context) {
        if ($Context.PSObject.Properties.Name -contains "CampaignSharedDriverOptionContext") {
            $OptionContext = $Context.CampaignSharedDriverOptionContext
        } elseif ($Context.PSObject.Properties.Name -contains "DriverPassthroughContext") {
            $OptionContext = $Context.DriverPassthroughContext
        }
    }

    if ($null -eq $OptionContext) {
        return [ordered]@{
            explicit_driver_args = @()
            compatibility_extra_args = @()
            effective_args = @()
            compatibility_capture_used = $false
        }
    }

    return [ordered]@{
        explicit_driver_args = @($OptionContext.ExplicitDriverArgs)
        compatibility_extra_args = @($OptionContext.CompatibilityExtraArgs)
        effective_args = @($OptionContext.DriverPassthroughArgs)
        compatibility_capture_used = [bool] $OptionContext.HasCompatibilityExtraArgs
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

    Write-CampaignArtifactText -Path $Context.RunCommandPath -Text "$PrimaryDriverCommandLine`n"
    if ($Context.OutputArtifact.Kind -eq "run") {
        Write-CampaignLatestPointerViaDriver -Context $Context -Kind "run"
        Write-Host "latest-pointer=$(Get-CampaignLatestPointerPath)"
    } elseif ($Context.OutputArtifact.Kind -eq "scratch") {
        Write-CampaignLatestPointerViaDriver -Context $Context -Kind "scratch"
        Write-Host "scratch-latest-pointer=$(Get-CampaignScratchLatestPointerPath)"
    }
    Write-Host "primary-driver-command=$($Context.RunCommandPath)"
    Write-Host "manifest=$($Context.RunManifestPath)"
}

function Write-CampaignLatestPointerViaDriver {
    param(
        [object] $Context,
        [ValidateSet("run", "scratch")]
        [string] $Kind
    )

    if (-not $Context.DriverExe) {
        throw "Internal error: Rust artifact pointer writer requires DriverExe."
    }
    if (-not $Context.OutputArtifact -or -not $Context.OutputArtifact.Id) {
        throw "Internal error: Rust artifact pointer writer requires an output artifact id."
    }

    $PointerArgs = @(
        "artifact",
        "write-latest",
        "--kind", $Kind,
        "$($Context.OutputArtifact.Id)",
        "--updated-at", (Get-Date).ToString("o"),
        "--campaign-dir", "$script:CampaignDir",
        "--json"
    )
    & $Context.DriverExe @PointerArgs | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Rust artifact pointer writer failed with exit code $LASTEXITCODE."
    }
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
        output_state = if ($Context.OutputArtifact -and $Context.OutputArtifact.StatePath) { "$($Context.OutputArtifact.StatePath)" } elseif ($Context.RunOutputCampaignPath) { "$(Get-CampaignStateSidecarPath -ReportPath $Context.RunOutputCampaignPath)" } else { "" }
        output_journal = if ($Context.OutputArtifact -and $Context.OutputArtifact.JournalPath) { "$($Context.OutputArtifact.JournalPath)" } elseif ($Context.RunOutputCampaignPath) { "$(Get-CampaignJournalSidecarPath -ReportPath $Context.RunOutputCampaignPath)" } else { "" }
        output_checkpoint = "$($Context.RunOutputCheckpointPath)"
        command_file_semantics = "primary_driver_command"
        command_file = "$($Context.RunCommandPath)"
        manifest_file = "$($Context.RunManifestPath)"
        wrapper_invocation = [ordered]@{
            line = $Context.WrapperInvocationLine
            bound_parameters = $Context.WrapperBoundParameters
        }
        driver_passthrough = Convert-CampaignDriverPassthroughForManifest -Context $Context
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
    $Manifest["resume_state"] = if ($Context.ResumeCampaignPath) { "$(Get-CampaignStateSidecarPath -ReportPath $Context.ResumeCampaignPath)" } else { "" }
    $Manifest["resume_journal"] = if ($Context.ResumeCampaignPath) { "$(Get-CampaignJournalSidecarPath -ReportPath $Context.ResumeCampaignPath)" } else { "" }
    $Manifest["resume_checkpoint"] = if ($Context.ResumeCheckpointPath) { "$($Context.ResumeCheckpointPath)" } else { "" }
    $Manifest["log_file"] = if ($Context.Log) { "$($Context.RunLogPath)" } else { "" }
    $Manifest["round_budget"] = [ordered]@{
        source = $Context.RoundBudgetSource
        target_rounds = $Context.TargetRounds
        additional_rounds = $Context.RoundBudgetAdditionalRounds
    }

    if ($Context.UntilMilestoneBound) {
        $MilestoneContext = New-CampaignMilestoneContext `
            -ReportPath $Context.RunOutputCampaignPath `
            -CheckpointPath $Context.RunOutputCheckpointPath `
            -DriverExe $Context.DriverExe `
            -UntilMilestone $Context.UntilMilestone `
            -ResolvedMilestoneStop $Context.ResolvedMilestoneStop `
            -MilestoneStepRounds $Context.MilestoneStepRounds `
            -MilestoneMaxRounds $Context.MilestoneMaxRounds `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext
        $MilestoneResumeArgs = New-CampaignMilestoneResumeDriverArgs `
            -MilestoneContext $MilestoneContext `
            -StepRounds $Context.MilestoneStepRounds
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
