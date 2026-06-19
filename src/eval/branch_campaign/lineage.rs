use std::collections::BTreeMap;

use super::model::BranchCampaignBranchV1;

const BOSS_RELIC_CHOICE_LABELS_V1: &[&str] = &[
    "Astrolabe",
    "BlackBlood",
    "BlackStar",
    "BustedCrown",
    "CallingBell",
    "CoffeeDripper",
    "CursedKey",
    "Ectoplasm",
    "EmptyCage",
    "FrozenCore",
    "FusionHammer",
    "HolyWater",
    "HoveringKite",
    "Inserter",
    "MarkOfPain",
    "NuclearBattery",
    "PandorasBox",
    "PhilosopherStone",
    "RingOfTheSerpent",
    "RunicCube",
    "RunicDome",
    "RunicPyramid",
    "SacredBark",
    "SlaversCollar",
    "SneckoEye",
    "Sozu",
    "TinyHouse",
    "VelvetChoker",
    "VioletLotus",
    "WristBlade",
];

pub(super) fn campaign_branch_first_lineage_key_v1(branch: &BranchCampaignBranchV1) -> String {
    branch
        .choice_labels
        .first()
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| branch.branch_id.clone())
}

pub(super) fn campaign_branch_path_lineage_key_v1(branch: &BranchCampaignBranchV1) -> String {
    let first = branch
        .choice_labels
        .first()
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| branch.branch_id.clone());
    let Some(latest) = branch.choice_labels.last() else {
        return first;
    };
    if latest == &first || latest.trim().is_empty() {
        return first;
    }
    format!("{first} | latest={latest}")
}

pub(super) fn campaign_branch_boss_relic_lineage_key_v1(
    branch: &BranchCampaignBranchV1,
) -> Option<String> {
    let relics = branch
        .choice_labels
        .iter()
        .filter_map(|label| campaign_boss_relic_label_v1(label))
        .collect::<Vec<_>>();
    (!relics.is_empty()).then(|| relics.join(">"))
}

fn campaign_boss_relic_label_v1(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if BOSS_RELIC_CHOICE_LABELS_V1.contains(&trimmed) {
        Some(trimmed.to_string())
    } else if let Some(first_token) = trimmed.split_whitespace().next() {
        BOSS_RELIC_CHOICE_LABELS_V1
            .contains(&first_token)
            .then(|| first_token.to_string())
    } else {
        None
    }
}

pub(super) fn campaign_boss_relic_lineage_counts_v1(
    branches: &[BranchCampaignBranchV1],
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in branches {
        let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
            continue;
        };
        *counts.entry(lineage).or_insert(0) += 1;
    }
    counts
}

pub(super) fn campaign_boss_relic_lineage_counts_for_pool_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in active.iter().chain(frozen.iter()) {
        let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
            continue;
        };
        *counts.entry(lineage).or_insert(0) += 1;
    }
    counts
}

pub(super) fn render_string_counts_v1(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}
