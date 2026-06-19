use crate::ai::strategic::{format_compact_branch_signature, BranchSignatureCompact};
use serde::{Deserialize, Serialize};

use super::{BranchCampaignBranchV1, BranchCampaignReportV1};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignStrategicSignalsV1 {
    pub groups: Vec<BranchCampaignStrategicSignalGroupV1>,
}

impl BranchCampaignStrategicSignalsV1 {
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignStrategicSignalGroupV1 {
    pub label: String,
    pub count: usize,
    pub average: BranchSignatureCompact,
}

pub(super) fn campaign_strategic_signals_for_render_v1(
    report: &BranchCampaignReportV1,
) -> BranchCampaignStrategicSignalsV1 {
    if !report.strategic_signals.is_empty() {
        return report.strategic_signals.clone();
    }
    campaign_strategic_signals_from_report_v1(report)
}

fn campaign_strategic_signals_from_report_v1(
    report: &BranchCampaignReportV1,
) -> BranchCampaignStrategicSignalsV1 {
    campaign_strategic_signals_from_groups_v1(
        &report.active,
        &report.frozen,
        &report.victories,
        &report.abandoned,
        &report.stuck,
    )
}

pub(super) fn campaign_strategic_signals_from_groups_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
    victories: &[BranchCampaignBranchV1],
    abandoned: &[BranchCampaignBranchV1],
    stuck: &[BranchCampaignBranchV1],
) -> BranchCampaignStrategicSignalsV1 {
    let mut groups = Vec::new();
    push_campaign_strategic_group_v1(&mut groups, "active", active);
    push_campaign_strategic_group_v1(&mut groups, "frozen", frozen);
    push_campaign_strategic_group_v1(&mut groups, "victory", victories);
    push_campaign_strategic_group_v1(&mut groups, "abandoned", abandoned);
    push_campaign_strategic_group_v1(&mut groups, "stuck", stuck);
    BranchCampaignStrategicSignalsV1 { groups }
}

pub(super) fn render_campaign_strategic_signals_v1(
    signals: &BranchCampaignStrategicSignalsV1,
) -> Option<String> {
    if signals.groups.is_empty() {
        return None;
    }
    let groups = signals
        .groups
        .iter()
        .map(|group| {
            format!(
                "{} n={} avg=[{}]",
                group.label,
                group.count,
                format_compact_branch_signature(&group.average)
            )
        })
        .collect::<Vec<_>>();
    Some(format!("Strategic signals: {}", groups.join(" ")))
}

pub(super) fn render_campaign_strategic_concern_v1(
    signals: &BranchCampaignStrategicSignalsV1,
) -> Option<String> {
    let active = campaign_strategic_group_by_label_v1(signals, "active")?;
    let frozen = campaign_strategic_group_by_label_v1(signals, "frozen")?;
    let engine_delta = frozen
        .average
        .engine_score_milli
        .saturating_sub(active.average.engine_score_milli);
    let package_delta = frozen
        .average
        .package_coherence_milli
        .saturating_sub(active.average.package_coherence_milli);
    if engine_delta < 200 && package_delta < 200 {
        return None;
    }
    let mut concerns = Vec::new();
    if engine_delta >= 200 {
        concerns.push(format!(
            "frozen_engine_above_active={}",
            format_signal_delta_1dp_v1(engine_delta)
        ));
    }
    if package_delta >= 200 {
        concerns.push(format!(
            "frozen_package_above_active={}",
            format_signal_delta_1dp_v1(package_delta)
        ));
    }
    Some(format!("Strategic concern: {}", concerns.join(" ")))
}

fn campaign_strategic_group_by_label_v1<'a>(
    signals: &'a BranchCampaignStrategicSignalsV1,
    label: &str,
) -> Option<&'a BranchCampaignStrategicSignalGroupV1> {
    signals.groups.iter().find(|group| group.label == label)
}

fn format_signal_delta_1dp_v1(value_milli: i32) -> String {
    let value_milli = value_milli.clamp(0, 1000);
    let tenths = (value_milli + 50) / 100;
    format!("{}.{}", tenths / 10, tenths % 10)
}

fn push_campaign_strategic_group_v1(
    groups: &mut Vec<BranchCampaignStrategicSignalGroupV1>,
    label: &str,
    branches: &[BranchCampaignBranchV1],
) {
    let Some((count, average)) = average_campaign_strategic_signature_v1(branches) else {
        return;
    };
    groups.push(BranchCampaignStrategicSignalGroupV1 {
        label: label.to_string(),
        count,
        average,
    });
}

fn average_campaign_strategic_signature_v1(
    branches: &[BranchCampaignBranchV1],
) -> Option<(usize, BranchSignatureCompact)> {
    let mut count = 0usize;
    let mut boss = 0i64;
    let mut clean = 0i64;
    let mut engine = 0i64;
    let mut cycle_debt = 0i64;
    let mut setup_debt = 0i64;
    let mut economy = 0i64;
    let mut package = 0i64;
    for branch in branches
        .iter()
        .filter(|branch| !branch.strategic_summary.is_empty())
    {
        count = count.saturating_add(1);
        boss += i64::from(branch.strategic_summary.boss_readiness_milli);
        clean += i64::from(branch.strategic_summary.clean_score_milli);
        engine += i64::from(branch.strategic_summary.engine_score_milli);
        cycle_debt += i64::from(branch.strategic_summary.cycle_debt_milli);
        setup_debt += i64::from(branch.strategic_summary.setup_debt_milli);
        economy += i64::from(branch.strategic_summary.economy_conversion_milli);
        package += i64::from(branch.strategic_summary.package_coherence_milli);
    }
    if count == 0 {
        return None;
    }
    Some((
        count,
        BranchSignatureCompact {
            present: true,
            boss_readiness_milli: average_campaign_signal_milli_v1(boss, count),
            clean_score_milli: average_campaign_signal_milli_v1(clean, count),
            engine_score_milli: average_campaign_signal_milli_v1(engine, count),
            cycle_debt_milli: average_campaign_signal_milli_v1(cycle_debt, count),
            setup_debt_milli: average_campaign_signal_milli_v1(setup_debt, count),
            economy_conversion_milli: average_campaign_signal_milli_v1(economy, count),
            package_coherence_milli: average_campaign_signal_milli_v1(package, count),
            bucket_mask: 0,
        },
    ))
}

fn average_campaign_signal_milli_v1(total: i64, count: usize) -> i32 {
    ((total + count as i64 / 2) / count as i64) as i32
}
