use crate::content::cards::java_id;
use crate::runtime::combat::Intent;
use crate::sim::combat::{combat_terminal, stable_boundary};
use crate::sim::combat_action::combat_action_key;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::EngineState;
use crate::state::run::RunState;

use super::session::RunControlSession;

pub fn render_run_control_state(session: &RunControlSession) -> String {
    super::panels::render_run_control_main(session)
}

pub fn render_run_control_details(session: &RunControlSession) -> String {
    let mut out = String::new();
    let (player_hp, player_max_hp) = session.visible_player_hp();
    push_line(
        &mut out,
        format!(
            "engine={:?} seed={} act={} floor={} hp={}/{} gold={} deck={} relics={} potions={}",
            session.engine_state,
            session.run_state.seed,
            session.run_state.act_num,
            session.run_state.floor_num,
            player_hp,
            player_max_hp,
            session.run_state.gold,
            session.run_state.master_deck.len(),
            session.run_state.relics.len(),
            session
                .visible_potions()
                .iter()
                .filter(|slot| slot.is_some())
                .count()
        ),
    );
    if let Some(outcome) = session.last_combat_baseline() {
        push_line(
            &mut out,
            format!(
                "last_combat case={} terminal={:?} final_hp={} hp_loss={} turns={} potions_used={} cards_played={}",
                outcome.case_id,
                outcome.terminal,
                outcome.final_hp,
                outcome.hp_loss,
                outcome.turns,
                outcome.potions_used,
                outcome.cards_played
            ),
        );
    }
    if let Some(case) = session.last_capture_case() {
        push_line(
            &mut out,
            format!(
                "last_capture_case root={} case_id={} combat_sequence={} baseline_ready={}",
                case.root.display(),
                case.case_id,
                case.combat_sequence,
                session.last_completed_manual_combat_matches_capture_case()
            ),
        );
    }
    render_candidate_resolution_details(session, &mut out);

    match &session.engine_state {
        EngineState::MapNavigation => render_map_state(session, &mut out),
        EngineState::EventRoom => render_event_state(session, &mut out),
        EngineState::RewardScreen(reward) => render_reward_state(reward, &mut out),
        EngineState::TreasureRoom(chest) => {
            push_line(&mut out, format!("treasure={chest:?} command=open"));
        }
        EngineState::Campfire => {
            push_line(
                &mut out,
                "campfire commands=rest|smith <deck_idx>|dig|lift|toke <deck_idx>|recall",
            );
        }
        EngineState::Shop(shop) => {
            push_line(
                &mut out,
                format!(
                    "shop cards={} relics={} potions={} purge_cost={} purge_available={}",
                    shop.cards.len(),
                    shop.relics.len(),
                    shop.potions.len(),
                    shop.purge_cost,
                    shop.purge_available
                ),
            );
        }
        EngineState::RunPendingChoice(choice) => {
            push_line(
                &mut out,
                format!(
                    "run_choice reason={:?} min={} max={} command=select <deck_idx...>",
                    choice.reason, choice.min_choices, choice.max_choices
                ),
            );
            render_master_deck(&session.run_state, &mut out);
        }
        EngineState::CombatStart(request) => {
            push_line(
                &mut out,
                format!(
                    "combat_start encounter={:?} room={:?}",
                    request.encounter_id, request.room_type
                ),
            );
        }
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => render_combat_state(session, &mut out),
        EngineState::BossRelicSelect(choice) => {
            for (idx, relic) in choice.relics.iter().enumerate() {
                push_line(&mut out, format!("boss_relic[{idx}]={relic:?}"));
            }
        }
        EngineState::GameOver(result) => push_line(&mut out, format!("game_over={result:?}")),
    }

    out
}

fn render_candidate_resolution_details(session: &RunControlSession, out: &mut String) {
    let view = super::view_model::build_run_control_view_model(session);
    if view.candidates.is_empty() {
        return;
    }
    push_line(out, "candidates:");
    for candidate in view.candidates {
        push_line(
            out,
            format!(
                "candidate[{}] command={} label={}",
                candidate.id,
                candidate.action.command_hint(),
                candidate.label
            ),
        );
        if let Some(resolution) = candidate.resolution.as_ref() {
            for line in resolution.detail_lines() {
                push_line(out, format!("  {line}"));
            }
        }
    }
}

pub fn render_run_control_raw(session: &RunControlSession) -> String {
    format!("{session:#?}")
}

pub fn render_combat_actions(session: &RunControlSession) -> Result<String, String> {
    let position = session.current_combat_position_for_actions()?;
    let actions = get_legal_moves(&position.engine, &position.combat);
    if actions.is_empty() {
        return Ok("no legal combat actions".to_string());
    }

    let mut out = String::new();
    for (idx, action) in actions.iter().enumerate() {
        push_line(
            &mut out,
            format!(
                "action[{idx}] {} {:?}",
                combat_action_key(&position.combat, action),
                action
            ),
        );
    }
    Ok(out)
}

fn render_map_state(session: &RunControlSession, out: &mut String) {
    let target_y = if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    };
    if target_y == 15 {
        push_line(out, "map target: go 0 -> boss");
        return;
    }
    if target_y < 0 || target_y as usize >= session.run_state.map.graph.len() {
        push_line(out, "map has no next row");
        return;
    }
    for node in &session.run_state.map.graph[target_y as usize] {
        if session.run_state.map.can_travel_to(node.x, node.y, false) {
            push_line(
                out,
                format!("map target: go {} -> y={} {:?}", node.x, node.y, node.class),
            );
        }
    }
}

fn render_event_state(session: &RunControlSession, out: &mut String) {
    let Some(event) = session.run_state.event_state.as_ref() else {
        push_line(out, "event state missing");
        return;
    };
    push_line(
        out,
        format!(
            "event={:?} screen={} completed={}",
            event.id, event.current_screen, event.completed
        ),
    );
    for (idx, option) in crate::engine::event_handler::get_event_options(&session.run_state)
        .iter()
        .enumerate()
    {
        push_line(
            out,
            format!(
                "event[{idx}] disabled={} label={}",
                option.ui.disabled, option.ui.text
            ),
        );
    }
}

fn render_reward_state(reward: &crate::state::rewards::RewardState, out: &mut String) {
    if let Some(cards) = reward.pending_card_choice.as_ref() {
        for (idx, card) in cards.iter().enumerate() {
            push_line(out, format!("card[{idx}] {:?}+{}", card.id, card.upgrades));
        }
        return;
    }
    for (idx, item) in reward.items.iter().enumerate() {
        push_line(out, format!("reward[{idx}] {item:?}"));
    }
}

fn render_master_deck(run_state: &RunState, out: &mut String) {
    for (idx, card) in run_state.master_deck.iter().enumerate() {
        push_line(
            out,
            format!(
                "deck[{idx}] {}#{}+{}",
                java_id(card.id),
                card.uuid,
                card.upgrades
            ),
        );
    }
}

fn render_combat_state(session: &RunControlSession, out: &mut String) {
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        push_line(out, "combat state missing");
        return;
    };
    let capture_state = session.current_active_combat_position().ok();
    let stable = capture_state
        .as_ref()
        .is_some_and(|position| stable_boundary(&position.engine, &position.combat));
    let terminal = capture_state
        .as_ref()
        .map(|position| combat_terminal(&position.engine, &position.combat));
    push_line(
        out,
        format!(
            "combat stable_capture={} terminal={:?} hp={}/{} block={} energy={} turn={}",
            stable,
            terminal,
            combat.entities.player.current_hp,
            combat.entities.player.max_hp,
            combat.entities.player.block,
            combat.turn.energy,
            combat.turn.turn_count
        ),
    );
    for (idx, card) in combat.zones.hand.iter().enumerate() {
        push_line(
            out,
            format!(
                "hand[{idx}] {}#{}+{} cost={}",
                java_id(card.id),
                card.uuid,
                card.upgrades,
                card.cost_for_turn_java()
            ),
        );
    }
    for monster in &combat.entities.monsters {
        let observation = combat
            .runtime
            .monster_protocol
            .get(&monster.id)
            .map(|protocol| &protocol.observation);
        let turn_plan = monster.turn_plan();
        let intent = observation
            .filter(|obs| obs.visible_intent != Intent::Unknown)
            .map(|obs| format!("{:?}", obs.visible_intent))
            .unwrap_or_else(|| format!("{:?}", turn_plan.summary_spec()));
        let damage = observation
            .filter(|obs| obs.preview_damage_per_hit > 0)
            .map(|obs| obs.preview_damage_per_hit)
            .or_else(|| turn_plan.attack().map(|attack| attack.base_damage))
            .unwrap_or(0);
        push_line(
            out,
            format!(
                "monster[slot={}] id={} type={} hp={}/{} block={} alive={} intent={} dmg={}",
                monster.slot,
                monster.id,
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.block,
                monster.is_alive_for_action(),
                intent,
                damage,
            ),
        );
    }
}

fn push_line(out: &mut String, line: impl AsRef<str>) {
    out.push_str(line.as_ref());
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicId;
    use crate::eval::run_control::commands::RunControlCommand;
    use crate::eval::run_control::session::{RunControlConfig, RunControlSession};
    use crate::state::core::EngineState;
    use crate::state::rewards::BossRelicChoiceState;

    #[test]
    fn main_view_is_default_and_keeps_debug_fields_out_of_startup_panel() {
        let session = RunControlSession::new(RunControlConfig {
            seed: 521,
            ..RunControlConfig::default()
        });
        let rendered = render_run_control_state(&session);

        assert!(rendered.contains("Act 1 Floor 0"));
        assert!(rendered.contains("Neow Intro"));
        assert!(rendered.contains("Neow greets you."));
        assert!(rendered.contains("0 | Proceed"));
        assert!(rendered.contains("Inspect: deck | map | relics"));
        assert!(
            !rendered.contains("Route note:"),
            "startup main panel should not present route preview as part of the current screen"
        );
        assert!(
            !rendered.contains("capture-case <benchmark_dir>"),
            "startup main panel should not dump full command help"
        );
        assert!(
            !rendered.contains("screen=0"),
            "startup main panel should not show internal event screen ids"
        );
        assert!(
            !rendered.contains("attacks=6"),
            "startup main panel should not show deck stats by default"
        );
    }

    #[test]
    fn details_view_preserves_low_level_engine_state_output() {
        let session = RunControlSession::new(RunControlConfig::default());
        let rendered = render_run_control_details(&session);

        assert!(rendered.contains("engine=EventRoom"));
        assert!(rendered.contains("event=Neow"));
    }

    #[test]
    fn neow_bonus_main_panel_does_not_present_map_as_current_action() {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 521,
            ..RunControlConfig::default()
        });
        session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");
        let rendered = render_run_control_state(&session);

        assert!(rendered.contains("Neow Bonus"));
        assert!(!rendered.contains("Route note:"));
        assert!(!rendered.contains("go <x>"));
        assert!(!rendered.contains("known:"));
        assert!(!rendered.contains("partial:"));
        assert!(rendered.contains("gain 100 gold"));
        assert!(rendered.contains("3 random uncommon colorless card choices"));
        assert!(rendered.contains("random rare relic outcome"));
        assert!(rendered.contains("lose starter relic Burning Blood"));
        assert!(rendered.contains("random boss relic outcome"));
    }

    #[test]
    fn details_view_exposes_structured_candidate_resolution() {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 521,
            ..RunControlConfig::default()
        });
        session
            .apply_command(RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");
        let rendered = render_run_control_details(&session);

        assert!(rendered.contains("candidates:"));
        assert!(rendered.contains("resolution: Partial"));
        assert!(rendered.contains("known_effects:"));
        assert!(rendered.contains("unresolved_effects:"));
        assert!(rendered.contains("distribution known, result hidden"));
    }

    #[test]
    fn reward_screen_details_expose_visible_resolution_boundaries() {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 521,
            ..RunControlConfig::default()
        });
        session
            .apply_command(RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");
        session
            .apply_command(RunControlCommand::Candidate("0".to_string()))
            .expect("Neow colorless option should open a card reward item");

        let reward_item_details = render_run_control_details(&session);
        assert!(reward_item_details.contains("visible card reward choices"));
        assert!(reward_item_details.contains("Dramatic Entrance"));

        session
            .apply_command(RunControlCommand::Candidate("0".to_string()))
            .expect("claiming card reward item should open pending card choice");
        let card_choice_details = render_run_control_details(&session);
        assert!(card_choice_details.contains("resolution: Known"));
        assert!(card_choice_details.contains("obtain 1 specific card Dramatic Entrance"));
    }

    #[test]
    fn boss_relic_details_expose_specific_relic_resolution() {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 521,
            ..RunControlConfig::default()
        });
        session.engine_state =
            EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![RelicId::CoffeeDripper]));

        let rendered = render_run_control_details(&session);

        assert!(rendered.contains("resolution: Known"));
        assert!(rendered.contains("obtain 1 specific relic Coffee Dripper"));
    }
}
