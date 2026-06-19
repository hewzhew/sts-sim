use crate::ai::strategic::format_compact_branch_signature;

use super::model::{BranchCampaignBranchSummaryV1, BranchCampaignBranchV1};
use super::selection_key::render_campaign_branch_selection_basis_v1;

pub(super) fn render_campaign_branch_state(branch: &BranchCampaignBranchV1) -> String {
    let state = branch
        .summary
        .as_ref()
        .map(|summary| {
            let deck_shape = render_campaign_branch_deck_shape_v1(summary)
                .map(|value| format!(" {value}"))
                .unwrap_or_default();
            let run_debt = render_campaign_branch_run_debt_v1(summary)
                .map(|value| format!(" debt=[{value}]"))
                .unwrap_or_default();
            format!(
                "A{}F{} HP {}/{} gold {} deck {}{}{}",
                summary.act,
                summary.floor,
                summary.hp,
                summary.max_hp,
                summary.gold,
                summary.deck_count,
                deck_shape,
                run_debt
            )
        })
        .unwrap_or_else(|| "start".to_string());
    let selection_basis = render_campaign_branch_selection_basis_v1(branch);
    let strategic_summary = format_compact_branch_signature(&branch.strategic_summary);
    if strategic_summary.is_empty() {
        format!("{state} {selection_basis}")
    } else {
        format!("{state} {selection_basis} strat=[{}]", strategic_summary)
    }
}

fn render_campaign_branch_run_debt_v1(summary: &BranchCampaignBranchSummaryV1) -> Option<String> {
    if summary.run_debt.is_empty() {
        return None;
    }
    let shown = summary
        .run_debt
        .iter()
        .take(2)
        .map(|label| truncate_campaign_run_debt_label_v1(label, 64))
        .collect::<Vec<_>>();
    let suffix = summary.run_debt.len().saturating_sub(shown.len());
    if suffix == 0 {
        Some(shown.join(" | "))
    } else {
        Some(format!("{} +{} more", shown.join(" | "), suffix))
    }
}

fn truncate_campaign_run_debt_label_v1(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    let prefix = label
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CampaignDeckKeyEntryV1 {
    card: String,
    upgrades: usize,
    count: usize,
}

fn render_campaign_branch_deck_shape_v1(summary: &BranchCampaignBranchSummaryV1) -> Option<String> {
    let entries = parse_campaign_deck_key_entries_v1(&summary.deck_key);
    if entries.is_empty() {
        return None;
    }
    let strike_count = entries
        .iter()
        .filter(|entry| campaign_deck_entry_is_strike_v1(&entry.card))
        .map(|entry| entry.count)
        .sum::<usize>();
    let defend_count = entries
        .iter()
        .filter(|entry| campaign_deck_entry_is_defend_v1(&entry.card))
        .map(|entry| entry.count)
        .sum::<usize>();
    let starter_count = entries
        .iter()
        .filter(|entry| campaign_deck_entry_is_starter_v1(&entry.card))
        .map(|entry| entry.count)
        .sum::<usize>();
    let upgraded_count = entries
        .iter()
        .filter(|entry| entry.upgrades > 0)
        .map(|entry| entry.count)
        .sum::<usize>();
    let additions = entries
        .iter()
        .filter(|entry| !campaign_deck_entry_is_starter_v1(&entry.card))
        .map(format_campaign_deck_key_entry_v1)
        .take(4)
        .collect::<Vec<_>>();
    let additions = if additions.is_empty() {
        "-".to_string()
    } else {
        additions.join(",")
    };
    Some(format!(
        "[S{strike_count} D{defend_count} starter{starter_count} add:{additions} upg{upgraded_count}]"
    ))
}

fn parse_campaign_deck_key_entries_v1(deck_key: &str) -> Vec<CampaignDeckKeyEntryV1> {
    deck_key
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            let (card_and_upgrade, count) = part.rsplit_once('x')?;
            let (card, upgrades) = card_and_upgrade.rsplit_once('+')?;
            Some(CampaignDeckKeyEntryV1 {
                card: card.trim().to_string(),
                upgrades: upgrades.trim().parse().ok()?,
                count: count.trim().parse().ok()?,
            })
        })
        .collect()
}

fn format_campaign_deck_key_entry_v1(entry: &CampaignDeckKeyEntryV1) -> String {
    let upgrade = match entry.upgrades {
        0 => String::new(),
        1 => "+".to_string(),
        value => format!("+{value}"),
    };
    let count = if entry.count > 1 {
        format!("x{}", entry.count)
    } else {
        String::new()
    };
    format!("{}{}{}", entry.card, upgrade, count)
}

fn campaign_deck_entry_is_starter_v1(card: &str) -> bool {
    campaign_deck_entry_is_strike_v1(card)
        || campaign_deck_entry_is_defend_v1(card)
        || matches!(
            card,
            "Bash" | "Neutralize" | "Survivor" | "Zap" | "Dualcast" | "Eruption" | "Vigilance"
        )
}

fn campaign_deck_entry_is_strike_v1(card: &str) -> bool {
    matches!(card, "Strike" | "StrikeG" | "StrikeB" | "StrikeP")
}

fn campaign_deck_entry_is_defend_v1(card: &str) -> bool {
    matches!(card, "Defend" | "DefendG" | "DefendB" | "DefendP")
}

pub(super) fn render_choice_path(labels: &[String]) -> String {
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels
            .iter()
            .map(|label| compact_campaign_choice_label_metadata_v1(label))
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

pub(super) fn render_compact_choice_path(labels: &[String]) -> String {
    const MAX_CHARS: usize = 140;
    if labels.is_empty() {
        return "-".to_string();
    }
    let compact = if labels.len() > 5 {
        let mut parts = Vec::new();
        parts.extend(labels.iter().take(2).cloned());
        parts.push("...".to_string());
        parts.extend(labels.iter().skip(labels.len().saturating_sub(3)).cloned());
        parts.join(" -> ")
    } else {
        render_choice_path(labels)
    };
    if compact.chars().count() <= MAX_CHARS {
        return compact;
    }
    let prefix = compact
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

pub(super) fn compact_campaign_choice_label_metadata_v1(label: &str) -> String {
    let parts = label
        .split(" | ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter(|part| {
            !part.starts_with("source=")
                && !part.starts_with("shop_legacy_estimate=")
                && !part.starts_with("deck mutation role=")
                && !part.starts_with("event_eval ")
                && !part.starts_with("total ")
                && *part != "auto leave shop"
        })
        .map(|part| part.replace(" gold then ", "g then ").replace(" gold", "g"))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        label.to_string()
    } else {
        parts.join(" ")
    }
}
