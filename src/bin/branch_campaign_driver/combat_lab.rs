use std::collections::BTreeMap;

use sts_simulator::content::cards::{get_card_definition, CardRarity, CardTag, CardType};
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::branch_campaign::BranchCampaignBranchV1;
use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlCommand, RunControlHpLossLimit,
    RunControlSearchCombatOptions, RunControlSession,
};
use sts_simulator::sim::combat_start::build_natural_combat_start;
use sts_simulator::state::core::{ActiveCombat, CombatStartRequest, EngineState};
use sts_simulator::state::map::node::RoomType;

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
    let Some(boss) = session
        .run_state
        .boss_key
        .or_else(|| session.run_state.boss_list.first().copied())
    else {
        return vec!["Boss preview: unavailable reason=no current act boss".to_string()];
    };
    let initial_hp = session.run_state.current_hp;
    let initial_potions = session.run_state.potions.clone();
    let mut preview = session.clone();
    let request = CombatStartRequest::room(boss, RoomType::MonsterRoomBoss);
    let (engine_state, combat_state) =
        match build_natural_combat_start(&mut preview.run_state, boss, RoomType::MonsterRoomBoss) {
            Ok(start) => start,
            Err(err) => {
                return vec![format!(
                    "Boss preview: unavailable boss={boss:?} reason=combat_start_failed: {err}"
                )]
            }
        };
    preview.engine_state = engine_state.clone();
    preview.active_combat = Some(ActiveCombat::new(
        engine_state,
        combat_state,
        request.context,
    ));
    let previous_trajectory_signature = preview
        .last_combat_automation_trajectory()
        .map(trajectory_signature_v1);

    let mut options = search_options.clone();
    if options.max_hp_loss.is_none() {
        options.max_hp_loss = Some(RunControlHpLossLimit::Unlimited);
    }
    let mut lines = Vec::new();
    lines.push(format!(
        "Boss preview: boss={boss:?} search={}",
        render_search_options_v1(&options)
    ));
    match preview.apply_command(RunControlCommand::SearchCombat(options)) {
        Ok(outcome) => {
            let hp_loss = initial_hp.saturating_sub(preview.run_state.current_hp);
            let boundary = build_decision_surface(&preview).view.header.title;
            let record = preview
                .last_combat_automation_trajectory()
                .filter(|record| {
                    Some(trajectory_signature_v1(record)) != previous_trajectory_signature
                });
            if let Some(record) = record {
                let result =
                    boss_preview_result_label_v1(preview.active_combat.is_none(), &record.source);
                lines.push(format!(
                    "  result={} hp_loss={} final_hp={}/{} actions={} next_boundary={} potions_used={}",
                    result,
                    hp_loss,
                    preview.run_state.current_hp,
                    preview.run_state.max_hp,
                    record.action_count,
                    boundary,
                    potion_slots_changed_v1(&initial_potions, &preview.run_state.potions)
                ));
            } else {
                lines.push(format!(
                    "  result=unresolved_no_trajectory hp_loss={} final_hp={}/{} next_boundary={}",
                    hp_loss, preview.run_state.current_hp, preview.run_state.max_hp, boundary
                ));
            }
            lines.extend(search_message_digest_v1(&outcome.message));
        }
        Err(err) => {
            lines.push(format!("  result=error reason={err}"));
        }
    }
    lines
}

fn trajectory_signature_v1(
    record: &sts_simulator::eval::run_control::CombatAutomationTrajectoryRecordV1,
) -> (String, usize, String, String) {
    (
        record.source.clone(),
        record.action_count,
        record
            .actions
            .first()
            .map(|action| action.action_key.clone())
            .unwrap_or_default(),
        record
            .actions
            .last()
            .map(|action| action.action_key.clone())
            .unwrap_or_default(),
    )
}

fn boss_preview_result_label_v1(combat_finished: bool, trajectory_source: &str) -> &'static str {
    if combat_finished && trajectory_source == "search_combat" {
        "complete_win_applied"
    } else if trajectory_source.contains("turn_segment") {
        "turn_segment_applied"
    } else {
        "partial_search_applied"
    }
}

fn potion_slots_changed_v1<T: std::fmt::Debug>(before: &[Option<T>], after: &[Option<T>]) -> usize {
    let max_len = before.len().max(after.len());
    (0..max_len)
        .filter(|idx| format!("{:?}", before.get(*idx)) != format!("{:?}", after.get(*idx)))
        .count()
}

fn search_message_digest_v1(message: &str) -> Vec<String> {
    let interesting_prefixes = [
        "  result=",
        "  detail=",
        "  best_complete_candidate",
        "  coverage_status=",
        "  terminal_wins=",
        "  nodes_expanded=",
        "  nodes_generated=",
        "  reason=",
    ];
    let mut lines = Vec::new();
    for line in message.lines() {
        let trimmed = line.trim_start();
        if interesting_prefixes
            .iter()
            .any(|prefix| line.starts_with(prefix) || trimmed.starts_with(prefix.trim_start()))
        {
            lines.push(format!("  search: {}", trimmed));
        }
        if lines.len() >= 8 {
            break;
        }
    }
    if lines.is_empty() {
        if let Some(first) = message.lines().find(|line| !line.trim().is_empty()) {
            lines.push(format!(
                "  search: {}",
                render_truncated_text_v1(first.trim(), 180)
            ));
        }
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

        assert!(rendered.contains("Boss preview: unavailable reason=no current act boss"));
    }

    #[test]
    fn boss_preview_result_distinguishes_segments_from_complete_wins() {
        assert_eq!(
            boss_preview_result_label_v1(true, "search_combat"),
            "complete_win_applied"
        );
        assert_eq!(
            boss_preview_result_label_v1(false, "search_combat_turn_segment"),
            "turn_segment_applied"
        );
    }
}
