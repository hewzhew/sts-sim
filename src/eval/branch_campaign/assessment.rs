use super::model::{BranchCampaignBranchSummaryV1, BranchCampaignBranchV1};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BranchCampaignGateStatusV1 {
    Unknown = 0,
    Unsolved = 1,
    AtRisk = 2,
    Passable = 3,
    Solved = 4,
}

impl BranchCampaignGateStatusV1 {
    fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Unsolved => "unsolved",
            Self::AtRisk => "at_risk",
            Self::Passable => "passable",
            Self::Solved => "solved",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BranchCampaignFormationStageV1 {
    Unknown = 0,
    StarterSurvival = 1,
    EarlyStableIncomplete = 2,
    DirectionSeeded = 3,
    PackageForming = 4,
    ScalingOnline = 5,
    EngineOnline = 6,
}

impl BranchCampaignFormationStageV1 {
    fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::StarterSurvival => "starter_survival",
            Self::EarlyStableIncomplete => "early_stable_incomplete",
            Self::DirectionSeeded => "direction_seeded",
            Self::PackageForming => "package_forming",
            Self::ScalingOnline => "scaling_online",
            Self::EngineOnline => "engine_online",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BranchCampaignResourceConversionV1 {
    None = 0,
    BufferOnly = 1,
    PendingConversion = 2,
    StrongConversionWindow = 3,
}

impl BranchCampaignResourceConversionV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BufferOnly => "buffer_only",
            Self::PendingConversion => "pending_conversion",
            Self::StrongConversionWindow => "strong_conversion_window",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BranchCampaignTrajectoryJobsV1 {
    frontload: i32,
    transition_frontload: i32,
    scaling: i32,
    defense: i32,
    engine_generators: i32,
    engine_payoffs: i32,
    draw_energy: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchCampaignAssessmentV1 {
    pub formation_stage: BranchCampaignFormationStageV1,
    pub immediate_gate: BranchCampaignGateStatusV1,
    pub act_boss_gate: BranchCampaignGateStatusV1,
    pub transition_gate: BranchCampaignGateStatusV1,
    pub resource_conversion: BranchCampaignResourceConversionV1,
    pub missing_critical_jobs: Vec<&'static str>,
}

impl BranchCampaignAssessmentV1 {
    pub fn priority_key(&self) -> BranchCampaignAssessmentPriorityKeyV1 {
        (
            immediate_gate_viability_v1(self.immediate_gate),
            self.act_boss_gate,
            self.transition_gate,
            self.formation_stage,
            -(self.missing_critical_jobs.len() as i32),
            self.resource_conversion,
            self.immediate_gate,
        )
    }
}

pub type BranchCampaignAssessmentPriorityKeyV1 = (
    i32,
    BranchCampaignGateStatusV1,
    BranchCampaignGateStatusV1,
    BranchCampaignFormationStageV1,
    i32,
    BranchCampaignResourceConversionV1,
    BranchCampaignGateStatusV1,
);

pub fn campaign_branch_assessment_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignAssessmentV1 {
    let Some(summary) = branch.summary.as_ref() else {
        return BranchCampaignAssessmentV1 {
            formation_stage: BranchCampaignFormationStageV1::Unknown,
            immediate_gate: BranchCampaignGateStatusV1::Unknown,
            act_boss_gate: BranchCampaignGateStatusV1::Unknown,
            transition_gate: BranchCampaignGateStatusV1::Unknown,
            resource_conversion: BranchCampaignResourceConversionV1::None,
            missing_critical_jobs: Vec::new(),
        };
    };
    let jobs = parse_campaign_trajectory_jobs_v1(&summary.trajectory_key);
    let formation_stage = campaign_formation_stage_v1(summary, &jobs);
    let immediate_gate = campaign_immediate_gate_v1(summary, &jobs);
    let act_boss_gate = campaign_act_boss_gate_v1(branch, summary, &jobs);
    let transition_gate = campaign_transition_gate_v1(summary, &jobs);
    let resource_conversion = campaign_resource_conversion_v1(branch, summary);
    let missing_critical_jobs = campaign_missing_critical_jobs_v1(summary, &jobs);
    BranchCampaignAssessmentV1 {
        formation_stage,
        immediate_gate,
        act_boss_gate,
        transition_gate,
        resource_conversion,
        missing_critical_jobs,
    }
}

pub fn render_campaign_branch_assessment_v1(branch: &BranchCampaignBranchV1) -> String {
    let assessment = campaign_branch_assessment_v1(branch);
    let missing = if assessment.missing_critical_jobs.is_empty() {
        "-".to_string()
    } else {
        assessment.missing_critical_jobs.join(",")
    };
    format!(
        "assess=[stage={} gate={} boss={} trans={} resource={} missing={}]",
        assessment.formation_stage.label(),
        assessment.immediate_gate.label(),
        assessment.act_boss_gate.label(),
        assessment.transition_gate.label(),
        assessment.resource_conversion.label(),
        missing
    )
}

fn campaign_formation_stage_v1(
    summary: &BranchCampaignBranchSummaryV1,
    jobs: &BranchCampaignTrajectoryJobsV1,
) -> BranchCampaignFormationStageV1 {
    if jobs.engine_generators > 0 && jobs.engine_payoffs > 0 {
        return BranchCampaignFormationStageV1::EngineOnline;
    }
    if jobs.scaling > 0 && (jobs.draw_energy > 0 || jobs.defense > 0) {
        return BranchCampaignFormationStageV1::ScalingOnline;
    }
    if jobs.scaling > 0 && jobs.defense > 0 {
        return BranchCampaignFormationStageV1::PackageForming;
    }
    match summary.formation_stage.as_str() {
        "Mature" | "PlanCommitted" | "PlanSeeded" => {
            BranchCampaignFormationStageV1::DirectionSeeded
        }
        "Transitional" => BranchCampaignFormationStageV1::EarlyStableIncomplete,
        "StarterShell" => BranchCampaignFormationStageV1::StarterSurvival,
        _ => {
            if jobs.frontload + jobs.transition_frontload > 0 || jobs.defense > 0 {
                BranchCampaignFormationStageV1::EarlyStableIncomplete
            } else {
                BranchCampaignFormationStageV1::StarterSurvival
            }
        }
    }
}

fn campaign_immediate_gate_v1(
    summary: &BranchCampaignBranchSummaryV1,
    jobs: &BranchCampaignTrajectoryJobsV1,
) -> BranchCampaignGateStatusV1 {
    if hp_percent_v1(summary) < 20 {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    let frontload = jobs.frontload + jobs.transition_frontload;
    if summary.act == 1 && summary.floor <= 5 {
        if frontload > 0 {
            return BranchCampaignGateStatusV1::Solved;
        }
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if frontload <= 0 {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if jobs.defense > 0 || jobs.draw_energy > 0 || hp_percent_v1(summary) >= 55 {
        BranchCampaignGateStatusV1::Passable
    } else {
        BranchCampaignGateStatusV1::AtRisk
    }
}

fn campaign_act_boss_gate_v1(
    branch: &BranchCampaignBranchV1,
    summary: &BranchCampaignBranchSummaryV1,
    jobs: &BranchCampaignTrajectoryJobsV1,
) -> BranchCampaignGateStatusV1 {
    if branch.strategic_summary.boss_readiness_milli >= 700 {
        return BranchCampaignGateStatusV1::Solved;
    }
    if branch.strategic_summary.boss_readiness_milli >= 300 {
        return BranchCampaignGateStatusV1::Passable;
    }
    if summary.act == 1 && summary.floor < 7 {
        return BranchCampaignGateStatusV1::Unknown;
    }
    if jobs.scaling > 0 || jobs.engine_generators + jobs.engine_payoffs > 1 {
        return BranchCampaignGateStatusV1::Passable;
    }
    if jobs.frontload + jobs.transition_frontload > 1 && jobs.defense > 0 {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    BranchCampaignGateStatusV1::Unsolved
}

fn campaign_transition_gate_v1(
    summary: &BranchCampaignBranchSummaryV1,
    jobs: &BranchCampaignTrajectoryJobsV1,
) -> BranchCampaignGateStatusV1 {
    if summary.act >= 2 {
        if jobs.scaling > 0 || jobs.engine_generators + jobs.engine_payoffs > 1 {
            return BranchCampaignGateStatusV1::Passable;
        }
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if summary.floor < 7 {
        return BranchCampaignGateStatusV1::Unknown;
    }
    if jobs.scaling > 0 || jobs.engine_generators + jobs.engine_payoffs > 1 {
        BranchCampaignGateStatusV1::Passable
    } else if jobs.draw_energy > 0 && jobs.defense > 0 {
        BranchCampaignGateStatusV1::AtRisk
    } else {
        BranchCampaignGateStatusV1::Unsolved
    }
}

fn campaign_resource_conversion_v1(
    branch: &BranchCampaignBranchV1,
    summary: &BranchCampaignBranchSummaryV1,
) -> BranchCampaignResourceConversionV1 {
    let frontier = branch.frontier_title.to_ascii_lowercase();
    if frontier.contains("shop") && summary.gold >= 150 {
        return BranchCampaignResourceConversionV1::StrongConversionWindow;
    }
    if frontier.contains("shop") && summary.gold >= 75 {
        return BranchCampaignResourceConversionV1::PendingConversion;
    }
    if frontier.contains("campfire") && hp_percent_v1(summary) >= 55 {
        return BranchCampaignResourceConversionV1::PendingConversion;
    }
    if hp_percent_v1(summary) >= 60 {
        BranchCampaignResourceConversionV1::BufferOnly
    } else {
        BranchCampaignResourceConversionV1::None
    }
}

fn campaign_missing_critical_jobs_v1(
    summary: &BranchCampaignBranchSummaryV1,
    jobs: &BranchCampaignTrajectoryJobsV1,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if jobs.frontload + jobs.transition_frontload <= 0 {
        missing.push("frontload");
    }
    if summary.act >= 2 || summary.floor >= 7 {
        if jobs.scaling <= 0 && jobs.engine_generators + jobs.engine_payoffs <= 1 {
            missing.push("scaling_or_engine");
        }
    }
    if summary.act >= 2 && jobs.defense <= 0 {
        missing.push("act2_mitigation");
    }
    if summary
        .formation_needs
        .iter()
        .any(|need| need == "DrawEnergy")
        && jobs.draw_energy <= 0
    {
        missing.push("draw_energy");
    }
    missing
}

fn parse_campaign_trajectory_jobs_v1(key: &str) -> BranchCampaignTrajectoryJobsV1 {
    let mut jobs = BranchCampaignTrajectoryJobsV1::default();
    for part in key.split('|') {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        let value = value.parse::<i32>().unwrap_or_default();
        match name {
            "frontload" => jobs.frontload = value,
            "transition" => jobs.transition_frontload = value,
            "scaling" => jobs.scaling = value,
            "defense" => jobs.defense = value,
            "engine_gen" => jobs.engine_generators = value,
            "engine_payoff" => jobs.engine_payoffs = value,
            "draw_energy" => jobs.draw_energy = value,
            _ => {}
        }
    }
    jobs
}

fn hp_percent_v1(summary: &BranchCampaignBranchSummaryV1) -> i32 {
    if summary.max_hp <= 0 {
        return 0;
    }
    summary.hp.max(0).saturating_mul(100) / summary.max_hp
}

fn immediate_gate_viability_v1(status: BranchCampaignGateStatusV1) -> i32 {
    match status {
        BranchCampaignGateStatusV1::Solved | BranchCampaignGateStatusV1::Passable => 2,
        BranchCampaignGateStatusV1::Unknown => 1,
        BranchCampaignGateStatusV1::AtRisk => 0,
        BranchCampaignGateStatusV1::Unsolved => -1,
    }
}
