use std::collections::BTreeMap;

use sts_simulator::content::cards::{get_card_definition, CardRarity, CardTag, CardType};
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::branch_campaign::BranchCampaignBranchV1;
use sts_simulator::eval::combat_lab_probe_v1::{
    current_act_boss_preview_probe_v1, CombatLabProbePacketV1,
};
use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlSearchCombatOptions, RunControlSession,
};
use sts_simulator::state::core::EngineState;

pub(super) fn render_checkpoint_combat_lab_v1(
    seed: u64,
    match_index: usize,
    match_count: usize,
    session: &RunControlSession,
    commands: &[String],
    branch: Option<&BranchCampaignBranchV1>,
    search_options: &RunControlSearchCombatOptions,
    probe_boss: bool,
) -> String {
    let surface = build_decision_surface(session);
    let direct_search = current_combat_search_available_v1(&session.engine_state);
    let mut lines = Vec::new();
    lines.push(format!(
        "CombatLabProbeV1 seed={} match={}/{} boundary={} direct_search={}",
        seed,
        match_index + 1,
        match_count,
        surface.view.header.title,
        if direct_search {
            "available"
        } else {
            "unavailable"
        }
    ));
    lines.push(format!(
        "Branch: A{}F{} HP {}/{} gold {} deck {} status={} rank={}",
        session.run_state.act_num,
        session.run_state.floor_num,
        visible_hp_v1(session).0,
        visible_hp_v1(session).1,
        session.run_state.gold,
        session.run_state.master_deck.len(),
        branch
            .map(|branch| format!("{:?}", branch.status))
            .unwrap_or_else(|| "unknown".to_string()),
        branch
            .map(|branch| branch.rank_key.to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    lines.push(format!(
        "Deck duplicates: {}",
        render_card_counts_v1(&nonstarter_duplicate_cards_v1(session))
    ));
    lines.push(format!(
        "Deck shape: curses={} unupgraded_nonstarter={} powers={} skills={} attacks={}",
        deck_card_type_count_v1(session, CardType::Curse),
        unupgraded_nonstarter_count_v1(session),
        deck_card_type_count_v1(session, CardType::Power),
        deck_card_type_count_v1(session, CardType::Skill),
        deck_card_type_count_v1(session, CardType::Attack)
    ));

    let relic_debts = relic_debt_labels_v1(session, branch);
    lines.push(format!(
        "Relic debts: {}",
        if relic_debts.is_empty() {
            "-".to_string()
        } else {
            relic_debts.join(" ")
        }
    ));

    let boss_pressure = branch
        .and_then(|branch| branch.summary.as_ref())
        .map(|summary| summary.boss_pressure.clone())
        .unwrap_or_default();
    lines.push(format!(
        "Boss pressure: {}",
        if boss_pressure.is_empty() {
            "-".to_string()
        } else {
            boss_pressure.join(" ")
        }
    ));

    lines.push("Probe targets:".to_string());
    if direct_search {
        lines.push(format!(
            "  current_combat: runnable via --inspect-search {}",
            render_search_options_v1(search_options)
        ));
    } else {
        lines.push(format!(
            "  current_combat: unavailable boundary={}",
            surface.view.header.title
        ));
    }
    if !boss_pressure.is_empty() {
        lines.push(format!(
            "  boss_pressure: report-only labels=[{}]",
            boss_pressure.join(" ")
        ));
    }
    lines.extend(render_boss_preview_v1(session, search_options, probe_boss));

    let upstream = upstream_probe_notes_v1(session, branch, &relic_debts);
    lines.push("Upstream probes:".to_string());
    if upstream.is_empty() {
        lines.push("  -".to_string());
    } else {
        for note in upstream {
            lines.push(format!("  {note}"));
        }
    }
    if !commands.is_empty() {
        lines.push(format!(
            "Recent choices: {}",
            render_truncated_text_v1(&commands.join(" -> "), 360)
        ));
    }
    lines.join("\n")
}

fn current_combat_search_available_v1(engine_state: &EngineState) -> bool {
    matches!(
        engine_state,
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing
    )
}

fn render_boss_preview_v1(
    session: &RunControlSession,
    search_options: &RunControlSearchCombatOptions,
    probe_boss: bool,
) -> Vec<String> {
    if !probe_boss {
        return vec!["Boss preview: disabled".to_string()];
    }
    let packet = current_act_boss_preview_probe_v1(session, search_options, "checkpoint_inspect");
    render_boss_preview_packet_v1(&packet, search_options)
}

fn render_boss_preview_packet_v1(
    packet: &CombatLabProbePacketV1,
    search_options: &RunControlSearchCombatOptions,
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Boss preview: boss={} source={} search={}",
        packet.boss.as_deref().unwrap_or("unknown"),
        packet.source,
        render_search_options_v1(search_options)
    ));
    lines.push(format!(
        "  result={} hp_loss={} final_hp={} actions={} boundary={}",
        packet.result,
        packet
            .hp_loss
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        match (packet.final_hp, packet.max_hp) {
            (Some(hp), Some(max_hp)) => format!("{hp}/{max_hp}"),
            _ => "-".to_string(),
        },
        packet
            .actions
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        packet.boundary,
    ));
    for digest in &packet.search_digest {
        lines.push(format!("  search: {digest}"));
    }
    lines
}

fn visible_hp_v1(session: &RunControlSession) -> (i32, i32) {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp))
}

fn nonstarter_duplicate_cards_v1(session: &RunControlSession) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for card in &session.run_state.master_deck {
        let def = get_card_definition(card.id);
        if card_is_starter_or_noise_v1(&def) {
            continue;
        }
        *counts.entry(def.name.to_string()).or_insert(0) += 1;
    }
    counts.retain(|_, count| *count > 1);
    counts
}

fn card_is_starter_or_noise_v1(def: &sts_simulator::content::cards::CardDefinition) -> bool {
    def.rarity == CardRarity::Basic
        || def.card_type == CardType::Curse
        || def.card_type == CardType::Status
        || def.tags.contains(&CardTag::StarterStrike)
        || def.tags.contains(&CardTag::StarterDefend)
}

fn render_card_counts_v1(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .iter()
        .map(|(card, count)| format!("{card}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn deck_card_type_count_v1(session: &RunControlSession, card_type: CardType) -> usize {
    session
        .run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == card_type)
        .count()
}

fn unupgraded_nonstarter_count_v1(session: &RunControlSession) -> usize {
    session
        .run_state
        .master_deck
        .iter()
        .filter(|card| {
            let def = get_card_definition(card.id);
            !card_is_starter_or_noise_v1(&def) && card.upgrades == 0
        })
        .count()
}

fn relic_debt_labels_v1(
    session: &RunControlSession,
    branch: Option<&BranchCampaignBranchV1>,
) -> Vec<String> {
    let mut relics = Vec::<RelicId>::new();
    for relic in &session.run_state.relics {
        if !relics.contains(&relic.id) {
            relics.push(relic.id);
        }
    }
    if let Some(branch) = branch {
        for label in &branch.choice_labels {
            if let Some(relic) = boss_relic_from_label_v1(label) {
                if !relics.contains(&relic) {
                    relics.push(relic);
                }
            }
        }
    }
    let mut labels = relics
        .into_iter()
        .filter_map(relic_debt_label_v1)
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

fn relic_debt_label_v1(relic: RelicId) -> Option<String> {
    let debt = match relic {
        RelicId::CallingBell => "curse_debt",
        RelicId::CursedKey => "chest_curse_or_relic_skip_debt",
        RelicId::FusionHammer => "smith_lock",
        RelicId::BustedCrown => "reward_width_debt",
        RelicId::CoffeeDripper => "rest_lock",
        RelicId::Ectoplasm => "gold_income_lock",
        RelicId::MarkOfPain => "wound_deck_debt",
        RelicId::PhilosopherStone => "enemy_strength_debt",
        RelicId::RunicDome => "intent_visibility_debt",
        RelicId::SneckoEye => "random_cost_deck_shape_debt",
        RelicId::Sozu => "potion_lock",
        RelicId::VelvetChoker => "card_play_cap_debt",
        _ => return None,
    };
    Some(format!("{relic:?}={debt}"))
}

fn boss_relic_from_label_v1(label: &str) -> Option<RelicId> {
    match label.trim() {
        "BustedCrown" => Some(RelicId::BustedCrown),
        "CallingBell" => Some(RelicId::CallingBell),
        "CoffeeDripper" => Some(RelicId::CoffeeDripper),
        "CursedKey" => Some(RelicId::CursedKey),
        "Ectoplasm" => Some(RelicId::Ectoplasm),
        "FusionHammer" => Some(RelicId::FusionHammer),
        "MarkOfPain" => Some(RelicId::MarkOfPain),
        "PhilosopherStone" => Some(RelicId::PhilosopherStone),
        "RunicDome" => Some(RelicId::RunicDome),
        "SneckoEye" => Some(RelicId::SneckoEye),
        "Sozu" => Some(RelicId::Sozu),
        "VelvetChoker" => Some(RelicId::VelvetChoker),
        _ => None,
    }
}

fn upstream_probe_notes_v1(
    session: &RunControlSession,
    branch: Option<&BranchCampaignBranchV1>,
    relic_debts: &[String],
) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(branch) = branch {
        for (card, count) in nonstarter_duplicate_cards_v1(session) {
            let mentions = branch
                .choice_labels
                .iter()
                .filter(|label| normalized_text_v1(label).contains(&normalized_text_v1(&card)))
                .count();
            if mentions > 1 {
                notes.push(format!(
                    "duplicate_acquisition: {card} deck_count={count} path_mentions={mentions}"
                ));
            }
        }
    }
    for debt in relic_debts {
        notes.push(format!("relic_debt_contract: {debt}"));
    }
    notes
}

fn normalized_text_v1(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn render_search_options_v1(options: &RunControlSearchCombatOptions) -> String {
    let mut parts = Vec::new();
    if let Some(max_nodes) = options.max_nodes {
        parts.push(format!("max_nodes={max_nodes}"));
    }
    if let Some(wall_ms) = options.wall_ms {
        parts.push(format!("wall_ms={wall_ms}"));
    }
    if let Some(max_hp_loss) = &options.max_hp_loss {
        parts.push(format!("max_hp_loss={max_hp_loss:?}"));
    }
    if parts.is_empty() {
        "(default search budget)".to_string()
    } else {
        parts.join(" ")
    }
}

fn render_truncated_text_v1(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let prefix = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::relics::RelicState;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1,
    };
    use sts_simulator::eval::run_control::RunControlConfig;
    use sts_simulator::runtime::combat::CombatCard;

    #[test]
    fn combat_lab_packet_reports_duplicate_debts_and_boss_pressure() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 24;
        session.run_state.current_hp = 54;
        session.run_state.max_hp = 80;
        session.run_state.gold = 194;
        session
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::Clothesline, 101));
        session
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::Clothesline, 102));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::FusionHammer));
        let branch = BranchCampaignBranchV1 {
            branch_id: "branch".to_string(),
            commands: vec!["pick Clothesline".to_string()],
            choice_labels: vec![
                "Clothesline".to_string(),
                "CallingBell".to_string(),
                "Clothesline".to_string(),
            ],
            summary: Some(BranchCampaignBranchSummaryV1 {
                act: 2,
                floor: 24,
                hp: 54,
                max_hp: 80,
                gold: 194,
                deck_count: session.run_state.master_deck.len(),
                deck_key: String::new(),
                formation_stage: "PlanSeeded".to_string(),
                formation_strengths: Vec::new(),
                formation_needs: Vec::new(),
                trajectory_key: String::new(),
                boss: "TheChamp".to_string(),
                boss_pressure: vec![
                    "missing:champ_transition_burst".to_string(),
                    "red:no_execute_block_plan".to_string(),
                ],
            }),
            strategic_summary: Default::default(),
            frontier_title: "Shop".to_string(),
            status: BranchCampaignBranchStatusV1::Active,
            stop_reason: String::new(),
            lineage_decision_signal_rank_adjustment: 0,
            rank_key: 22_100,
            final_boss_combat_record: None,
            combat_lab_probes: Vec::new(),
        };

        let rendered = render_checkpoint_combat_lab_v1(
            521,
            0,
            1,
            &session,
            &branch.commands,
            Some(&branch),
            &RunControlSearchCombatOptions {
                wall_ms: Some(1_000),
                ..RunControlSearchCombatOptions::default()
            },
            false,
        );

        assert!(rendered.contains("CombatLabProbeV1 seed=521"));
        assert!(rendered.contains("direct_search=unavailable"));
        assert!(rendered.contains("Deck duplicates: Clothesline=2"));
        assert!(rendered.contains("Relic debts: CallingBell=curse_debt FusionHammer=smith_lock"));
        assert!(rendered
            .contains("Boss pressure: missing:champ_transition_burst red:no_execute_block_plan"));
        assert!(
            rendered.contains("duplicate_acquisition: Clothesline deck_count=2 path_mentions=2")
        );
        assert!(rendered.contains("Boss preview: disabled"));
    }

    #[test]
    fn combat_lab_boss_preview_reports_missing_boss_without_search() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.boss_key = None;
        session.run_state.boss_list.clear();

        let rendered = render_checkpoint_combat_lab_v1(
            521,
            0,
            1,
            &session,
            &[],
            None,
            &RunControlSearchCombatOptions::default(),
            true,
        );

        assert!(rendered.contains("Boss preview: boss=unknown source=checkpoint_inspect"));
        assert!(rendered.contains("result=unavailable_no_current_act_boss"));
    }
}
