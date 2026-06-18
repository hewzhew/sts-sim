use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRunDomainV1 {
    pub ascension_level: u8,
    pub player_class: String,
    pub label: String,
    pub role: String,
}

impl Default for BranchCampaignRunDomainV1 {
    fn default() -> Self {
        branch_campaign_run_domain_v1(0, "Ironclad")
    }
}

pub fn branch_campaign_ascension_domain_label_v1(ascension_level: u8) -> &'static str {
    match ascension_level {
        0 => "debug_a0",
        10 => "bane_a10",
        15 => "event_a15",
        17 => "monster_ai_a17",
        20 => "target_a20",
        _ => "custom",
    }
}

pub fn branch_campaign_ascension_domain_role_v1(ascension_level: u8) -> &'static str {
    match ascension_level {
        0 => "debug_domain",
        20 => "target_domain",
        _ => "curriculum_domain",
    }
}

pub fn branch_campaign_run_domain_v1(
    ascension_level: u8,
    player_class: &str,
) -> BranchCampaignRunDomainV1 {
    BranchCampaignRunDomainV1 {
        ascension_level,
        player_class: player_class.to_string(),
        label: branch_campaign_ascension_domain_label_v1(ascension_level).to_string(),
        role: branch_campaign_ascension_domain_role_v1(ascension_level).to_string(),
    }
}
