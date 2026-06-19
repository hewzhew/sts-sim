use std::collections::{BTreeMap, BTreeSet};

use sts_simulator::ai::deck_startup_profile_v1::deck_startup_profile_v1;
use sts_simulator::ai::noncombat_strategy_v1::build_run_strategy_snapshot_from_run_state_v2;
use sts_simulator::content::cards::{get_card_definition, CardTag, CardType};
use sts_simulator::eval::branch_campaign::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchV1, BranchCampaignReportV1,
};
use sts_simulator::eval::event_boundary_packet_v1::{
    event_boundary_packet_from_session_v1, EventBoundaryPacketV1,
};
use sts_simulator::eval::reward_boundary_packet_v1::{
    reward_boundary_packet_from_session_v1, RewardBoundaryPacketV1,
};
use sts_simulator::eval::run_control::{build_decision_surface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;

pub(crate) fn render_checkpoint_inspect_summary_v1(
    seed: u64,
    matches: &[(Vec<String>, RunControlSession)],
    report: Option<&BranchCampaignReportV1>,
    branch_examples: usize,
) -> String {
    let limit = branch_examples.max(1);
    let mut lines = Vec::new();
    lines.push(format!(
        "Checkpoint summary: seed={seed} sessions={}{}",
        matches.len(),
        report
            .map(|report| format!(
                " report_rounds={} stop={}",
                report.rounds_completed, report.stop_reason
            ))
            .unwrap_or_default()
    ));

    let sessions_by_commands = matches
        .iter()
        .map(|(commands, session)| (commands.clone(), session))
        .collect::<BTreeMap<_, _>>();

    if let Some(report) = report {
        render_report_section(
            &mut lines,
            "Active",
            BranchCampaignBranchStatusV1::Active,
            &report.active,
            &sessions_by_commands,
            limit,
        );
        render_report_section(
            &mut lines,
            "Frozen",
            BranchCampaignBranchStatusV1::Frozen,
            &report.frozen,
            &sessions_by_commands,
            limit,
        );
        render_report_section(
            &mut lines,
            "Abandoned",
            BranchCampaignBranchStatusV1::Abandoned,
            &report.abandoned,
            &sessions_by_commands,
            limit,
        );
        render_report_section(
            &mut lines,
            "Stuck",
            BranchCampaignBranchStatusV1::Stuck,
            &report.stuck,
            &sessions_by_commands,
            limit,
        );
        render_report_section(
            &mut lines,
            "Victories",
            BranchCampaignBranchStatusV1::TerminalVictory,
            &report.victories,
            &sessions_by_commands,
            limit,
        );
    } else {
        lines.push("No --inspect-report supplied; showing checkpoint sessions only.".to_string());
        for (index, (commands, session)) in matches.iter().take(limit).enumerate() {
            lines.push(String::new());
            lines.push(format!(
                "{}. {}",
                index + 1,
                render_session_headline(session, None, None)
            ));
            lines.extend(render_session_details(session, commands, None));
        }
    }

    lines.join("\n")
}

fn render_report_section(
    lines: &mut Vec<String>,
    label: &str,
    status: BranchCampaignBranchStatusV1,
    branches: &[BranchCampaignBranchV1],
    sessions_by_commands: &BTreeMap<Vec<String>, &RunControlSession>,
    limit: usize,
) {
    if branches.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push(format!("{label}: {} branch(es)", branches.len()));
    let mut shown = 0usize;
    for branch in branches {
        if shown >= limit {
            break;
        }
        let Some(session) = sessions_by_commands.get(&branch.commands).copied() else {
            continue;
        };
        shown += 1;
        lines.push(format!(
            "  {}. {}",
            shown,
            render_session_headline(session, Some(status.clone()), Some(branch))
        ));
        lines.extend(
            render_session_details(session, &branch.commands, Some(branch))
                .into_iter()
                .map(|line| format!("     {line}")),
        );
    }
    if shown == 0 {
        lines.push("  no matching checkpoint session after filters".to_string());
    }
}

fn render_session_headline(
    session: &RunControlSession,
    status: Option<BranchCampaignBranchStatusV1>,
    branch: Option<&BranchCampaignBranchV1>,
) -> String {
    let surface = build_decision_surface(session);
    let status = status
        .map(|status| format!("{status:?} "))
        .unwrap_or_default();
    let rank = branch
        .map(|branch| {
            let lineage = if branch.lineage_decision_signal_rank_adjustment == 0 {
                String::new()
            } else {
                format!(
                    " lineage_signal={}",
                    branch.lineage_decision_signal_rank_adjustment
                )
            };
            format!(" rank={}{}", branch.rank_key, lineage)
        })
        .unwrap_or_default();
    let (hp, max_hp) = visible_session_hp(session);
    format!(
        "{}A{}F{} HP {}/{} gold {} deck {} | {}{}",
        status,
        session.run_state.act_num,
        session.run_state.floor_num,
        hp,
        max_hp,
        session.run_state.gold,
        session.run_state.master_deck.len(),
        surface.view.header.title,
        rank
    )
}

fn visible_session_hp(session: &RunControlSession) -> (i32, i32) {
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

fn render_session_details(
    session: &RunControlSession,
    commands: &[String],
    branch: Option<&BranchCampaignBranchV1>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    let formation = strategy.formation_summary();
    let startup = deck_startup_profile_v1(&session.run_state);
    let deck = deck_health_summary(session);

    lines.push(format!(
        "deck: {} | starters={} strikes={} defends={} curses={} attacks={} skills={} powers={} upgraded={}",
        render_grouped_deck(&session.run_state.master_deck),
        strategy.resources.starter_cards,
        deck.starter_strikes,
        deck.starter_defends,
        deck.curses,
        deck.attacks,
        deck.skills,
        deck.powers,
        deck.upgraded,
    ));
    lines.push(format!(
        "formation: stage={:?} needs=[{}] strengths=[{}]",
        formation.stage,
        join_debug(&formation.needs),
        join_debug(&formation.strengths)
    ));
    lines.push(format!(
        "strength: stable={} temp_burst={} converters={} convertible={} payoffs={} diagnosis=[{}]",
        startup.persistent_strength_source_count,
        startup.temporary_strength_burst_count,
        startup.strength_converter_count,
        startup.convertible_strength_source_count,
        startup.strength_payoff_count,
        render_strength_diagnosis(&startup)
    ));
    let liabilities = startup_liability_labels(&startup);
    lines.push(format!(
        "startup: setup_debt={} payment={}->{} strong_draw={}->{} exhaust={} strength_src={} strength_payoff={} snecko_low/high={}/{} liabilities=[{}]",
        startup.setup_debt,
        startup.setup_payment,
        startup.effective_setup_payment,
        startup.strong_draw_count,
        startup.effective_strong_draw_count,
        startup.exhaust_engine_count,
        startup.persistent_strength_source_count,
        startup.strength_payoff_count,
        startup.low_cost_card_count,
        startup.high_cost_card_count,
        if liabilities.is_empty() { "-".to_string() } else { liabilities.join(",") }
    ));
    lines.push(format!(
        "relics: {} | potions: {}",
        render_relics(session),
        render_potions(session)
    ));
    if let Some(outcome) = session.last_combat_baseline() {
        lines.push(format!(
            "last_combat: terminal={:?} final_hp={} hp_loss={} turns={} potions_used={} cards_played={}",
            outcome.terminal,
            outcome.final_hp,
            outcome.hp_loss,
            outcome.turns,
            outcome.potions_used,
            outcome.cards_played
        ));
    }
    let boss_pressure = boss_pressure_labels(session);
    if !boss_pressure.is_empty() {
        lines.push(format!("boss_pressure: {}", boss_pressure.join(", ")));
    }
    if let Some(packet) = event_boundary_packet_from_session_v1(session) {
        lines.push(render_event_boundary_summary_v1(&packet));
    }
    if let Some(packet) = reward_boundary_packet_from_session_v1(session) {
        lines.push(render_reward_boundary_summary_v1(&packet));
    }
    if let Some(branch) = branch {
        if !branch.stop_reason.trim().is_empty() {
            lines.push(format!("stop: {}", first_line(&branch.stop_reason)));
        }
        lines.push(format!(
            "recent choices: {}",
            render_recent_path(&branch.choice_labels)
        ));
    } else {
        lines.push(format!("commands: {}", render_recent_path(commands)));
    }
    lines
}

fn render_event_boundary_summary_v1(packet: &EventBoundaryPacketV1) -> String {
    let candidates = packet
        .candidates
        .iter()
        .take(4)
        .map(|candidate| {
            let effects = candidate
                .effects
                .iter()
                .take(3)
                .map(render_event_effect_short_v1)
                .collect::<Vec<_>>()
                .join("+");
            if effects.is_empty() {
                format!(
                    "{}:{}:{}",
                    candidate.command, candidate.action_kind, candidate.role
                )
            } else {
                format!(
                    "{}:{}:{}:{}",
                    candidate.command, candidate.action_kind, candidate.role, effects
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let suffix = if packet.candidates.len() > 4 {
        format!(" | ... {} more", packet.candidates.len() - 4)
    } else {
        String::new()
    };
    format!(
        "event_boundary: {} screen={} class={} candidates=[{}{}]",
        packet.event_id, packet.current_screen, packet.boundary_class, candidates, suffix
    )
}

fn render_event_effect_short_v1(
    effect: &sts_simulator::eval::event_boundary_packet_v1::EventEffectSnapshotV1,
) -> String {
    if effect.params.is_empty() {
        return effect.kind.clone();
    }
    let params = effect
        .params
        .iter()
        .take(3)
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    let suffix = if effect.params.len() > 3 { ",..." } else { "" };
    format!("{}({}{})", effect.kind, params, suffix)
}

fn render_reward_boundary_summary_v1(packet: &RewardBoundaryPacketV1) -> String {
    let candidates = packet
        .candidates
        .iter()
        .take(4)
        .map(|candidate| {
            let cards = if candidate.cards.is_empty() {
                String::new()
            } else {
                format!(
                    ":{}",
                    candidate
                        .cards
                        .iter()
                        .take(3)
                        .map(|card| card.display_label.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            format!(
                "{}:{}:{}{}",
                candidate.command, candidate.reward_kind, candidate.role, cards
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let suffix = if packet.candidates.len() > 4 {
        format!(" | ... {} more", packet.candidates.len() - 4)
    } else {
        String::new()
    };
    format!(
        "reward_boundary: {} context={} class={} candidates=[{}{}]",
        packet.surface, packet.screen_context, packet.boundary_class, candidates, suffix
    )
}

#[derive(Default)]
struct DeckHealthSummary {
    attacks: usize,
    skills: usize,
    powers: usize,
    curses: usize,
    starter_strikes: usize,
    starter_defends: usize,
    upgraded: usize,
}

fn deck_health_summary(session: &RunControlSession) -> DeckHealthSummary {
    let mut summary = DeckHealthSummary::default();
    for card in &session.run_state.master_deck {
        let def = get_card_definition(card.id);
        match def.card_type {
            CardType::Attack => summary.attacks += 1,
            CardType::Skill => summary.skills += 1,
            CardType::Power => summary.powers += 1,
            CardType::Curse => summary.curses += 1,
            CardType::Status => {}
        }
        if def.tags.contains(&CardTag::StarterStrike) {
            summary.starter_strikes += 1;
        }
        if def.tags.contains(&CardTag::StarterDefend) {
            summary.starter_defends += 1;
        }
        if card.upgrades > 0 {
            summary.upgraded += 1;
        }
    }
    summary
}

fn render_grouped_deck(cards: &[CombatCard]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for card in cards {
        *counts.entry(card_label(card)).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(label, count)| {
            if count > 1 {
                format!("{label}x{count}")
            } else {
                label
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn card_label(card: &CombatCard) -> String {
    let name = get_card_definition(card.id).name;
    match card.upgrades {
        0 => name.to_string(),
        1 => format!("{name}+"),
        upgrades => format!("{name}+{upgrades}"),
    }
}

fn startup_liability_labels(
    startup: &sts_simulator::ai::deck_startup_profile_v1::DeckStartupProfileV1,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    if startup.has_setup_debt_high_payment_low {
        labels.push("setup_debt_high_payment_low");
    }
    if startup.has_fnp_duplicate_without_exhaust_engine {
        labels.push("fnp_duplicate_without_exhaust");
    }
    if startup.has_dual_wield_without_target {
        labels.push("dual_wield_without_target");
    }
    if startup.has_anger_duplicate_without_digest {
        labels.push("anger_duplicate_without_digest");
    }
    if startup.has_strength_payoff_without_strength {
        labels.push("strength_payoff_without_strength");
    }
    if startup.has_rupture_without_self_damage {
        labels.push("rupture_without_self_damage");
    }
    if startup.has_armaments_unupgraded_duplicate {
        labels.push("armaments_unupgraded_duplicate");
    }
    if startup.has_pyramid_unupgraded_apparition {
        labels.push("pyramid_unupgraded_apparition");
    }
    if startup.has_snecko_low_cost_volatility {
        labels.push("snecko_low_cost_volatility");
    }
    if startup.has_snecko_offering_reliability_debt {
        labels.push("snecko_offering_reliability");
    }
    labels
}

fn render_strength_diagnosis(
    startup: &sts_simulator::ai::deck_startup_profile_v1::DeckStartupProfileV1,
) -> String {
    let mut labels = Vec::new();
    if startup.strength_payoff_count > 0 && startup.persistent_strength_source_count == 0 {
        if startup.convertible_strength_source_count > 0 {
            labels.push("payoff_has_convertible_burst_not_stable_scaling");
        } else if startup.temporary_strength_burst_count > 0 {
            labels.push("payoff_has_temporary_burst_not_stable_scaling");
        } else {
            labels.push("payoff_without_strength_source");
        }
    }
    if startup.temporary_strength_burst_count > 0 && startup.strength_converter_count == 0 {
        labels.push("temporary_strength_has_no_converter");
    }
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(",")
    }
}

fn render_relics(session: &RunControlSession) -> String {
    if session.run_state.relics.is_empty() {
        return "-".to_string();
    }
    session
        .run_state
        .relics
        .iter()
        .map(|relic| format!("{:?}", relic.id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_potions(session: &RunControlSession) -> String {
    let potions = session
        .run_state
        .potions
        .iter()
        .filter_map(|slot| slot.as_ref().map(|potion| format!("{:?}", potion.id)))
        .collect::<Vec<_>>();
    if potions.is_empty() {
        "-".to_string()
    } else {
        potions.join(", ")
    }
}

fn boss_pressure_labels(session: &RunControlSession) -> Vec<String> {
    let Some(boss) = session
        .run_state
        .boss_key
        .or_else(|| session.run_state.boss_list.first().copied())
    else {
        return Vec::new();
    };
    sts_simulator::ai::boss_mechanics_v1::boss_mechanic_pressure_profile_v1(
        &session.run_state,
        boss,
    )
    .summary_labels()
}

#[cfg(test)]
mod tests {
    use super::render_session_headline;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};
    use sts_simulator::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use sts_simulator::state::map::node::RoomType;

    #[test]
    fn inspect_headline_uses_visible_active_combat_hp() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.current_hp = 71;
        session.run_state.max_hp = 80;
        let mut combat = sts_simulator::test_support::blank_test_combat();
        combat.entities.player.current_hp = 14;
        combat.entities.player.max_hp = 80;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        let headline = render_session_headline(&session, None, None);

        assert!(
            headline.contains("HP 14/80"),
            "headline should use active combat HP, got {headline}"
        );
        assert!(
            !headline.contains("HP 71/80"),
            "stale run HP should not appear while active combat is present"
        );
    }
}

fn join_debug<T: std::fmt::Debug>(items: &[T]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items
            .iter()
            .map(|item| format!("{item:?}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn render_recent_path(values: &[String]) -> String {
    const TAIL: usize = 8;
    if values.is_empty() {
        return "-".to_string();
    }
    let mut deduped = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        if seen.insert(value) {
            deduped.push(value.as_str());
        }
    }
    let start = deduped.len().saturating_sub(TAIL);
    let mut rendered = deduped[start..].join(" -> ");
    if start > 0 {
        rendered = format!("... -> {rendered}");
    }
    rendered
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or(value).to_string()
}
