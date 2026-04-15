use crate::combat::CombatState;
use crate::state::core::ClientInput;
use std::collections::HashSet;

mod apply;
mod attack;
mod debuff;
mod draw;
mod exhaust;
mod posture;
mod power;
mod scoring;
mod sim;
mod support;

use apply::apply_play;
use scoring::{evaluate, play_priority};
use sim::{active_monsters, build_sim_state, fast_hash, get_plays, Play, SimState, MAX_STATES};

#[derive(Clone, Debug)]
pub struct HeuristicMoveStat {
    pub input: ClientInput,
    pub score: i64,
    pub priority: i32,
}

#[derive(Clone, Debug)]
pub struct HeuristicDiagnostics {
    pub chosen_move: ClientInput,
    pub baseline_score: i64,
    pub top_moves: Vec<HeuristicMoveStat>,
}

pub fn decide_heuristic(combat: &CombatState) -> ClientInput {
    diagnose_decision(combat).chosen_move
}

pub fn evaluate_combat_state(combat: &CombatState) -> i64 {
    evaluate(&build_sim_state(combat))
}

pub fn diagnose_decision(combat: &CombatState) -> HeuristicDiagnostics {
    let init = build_sim_state(combat);

    if active_monsters(&init).next().is_none() {
        return HeuristicDiagnostics {
            chosen_move: ClientInput::EndTurn,
            baseline_score: evaluate(&init),
            top_moves: Vec::new(),
        };
    }

    if let Some(potion_input) = super::potions::choose_immediate_potion(combat) {
        return HeuristicDiagnostics {
            chosen_move: potion_input,
            baseline_score: evaluate(&init),
            top_moves: Vec::new(),
        };
    }

    let mut best_score = evaluate(&init);
    let mut best_first: Option<Play> = None;
    let mut seen = HashSet::with_capacity(8192);
    seen.insert(fast_hash(&init));

    dfs(&init, None, &mut best_score, &mut best_first, &mut seen);

    if best_first.is_none() {
        let mut fallback: Option<(Play, i64, i32)> = None;
        for (card_idx, target) in get_plays(&init) {
            let mut next = init.clone();
            apply_play(&mut next, card_idx, target);
            let score = evaluate(&next);
            let priority = play_priority(&init, card_idx);
            if score > best_score || (score == best_score && priority > 0) {
                match fallback {
                    Some((_, best_fallback_score, best_fallback_priority))
                        if score < best_fallback_score
                            || (score == best_fallback_score
                                && priority <= best_fallback_priority) => {}
                    _ => fallback = Some(((card_idx, target), score, priority)),
                }
            }
        }
        if let Some((play, _, _)) = fallback {
            best_first = Some(play);
        }
    }

    let chosen_move = match best_first {
        Some((idx, target)) => ClientInput::PlayCard {
            card_index: idx,
            target,
        },
        None => ClientInput::EndTurn,
    };

    let mut top_moves = get_plays(&init)
        .into_iter()
        .map(|(card_idx, target)| {
            let mut next = init.clone();
            apply_play(&mut next, card_idx, target);
            HeuristicMoveStat {
                input: ClientInput::PlayCard {
                    card_index: card_idx,
                    target,
                },
                score: evaluate(&next),
                priority: play_priority(&init, card_idx),
            }
        })
        .collect::<Vec<_>>();
    top_moves.push(HeuristicMoveStat {
        input: ClientInput::EndTurn,
        score: evaluate(&init),
        priority: 0,
    });
    top_moves.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.priority.cmp(&a.priority))
    });
    top_moves.truncate(8);

    HeuristicDiagnostics {
        chosen_move,
        baseline_score: best_score,
        top_moves,
    }
}

pub fn describe_end_turn_options(combat: &CombatState) -> Vec<String> {
    let state = build_sim_state(combat);
    let end_score = evaluate(&state);
    let plays = get_plays(&state);
    if plays.is_empty() {
        return vec![format!("END score={} no_legal_plays", end_score)];
    }

    let mut lines = vec![format!(
        "END score={} legal_plays={}",
        end_score,
        plays.len()
    )];
    let mut scored: Vec<(i64, i32, usize, Option<usize>)> = plays
        .into_iter()
        .map(|(card_idx, target)| {
            let mut next = state.clone();
            apply_play(&mut next, card_idx, target);
            (
                evaluate(&next),
                play_priority(&state, card_idx),
                card_idx,
                target,
            )
        })
        .collect();
    scored.sort_by(|a, b| b.cmp(a));
    for (score, priority, card_idx, target) in scored.into_iter().take(8) {
        let card = &combat.zones.hand[card_idx];
        let target_label = target
            .map(|t| t.to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "play idx={} card={} target={} score={} priority={}",
            card_idx, card.id as u16, target_label, score, priority
        ));
    }
    lines
}

fn dfs(
    state: &SimState,
    first_play: Option<Play>,
    best_score: &mut i64,
    best_first: &mut Option<Play>,
    seen: &mut HashSet<u64>,
) {
    if seen.len() >= MAX_STATES {
        return;
    }

    let mut plays = get_plays(state);
    plays.sort_unstable_by(|a, b| play_priority(state, b.0).cmp(&play_priority(state, a.0)));

    for &(card_idx, target) in &plays {
        let mut ns = state.clone();
        apply_play(&mut ns, card_idx, target);

        let h = fast_hash(&ns);
        if !seen.insert(h) {
            continue;
        }

        let real_first = first_play.unwrap_or((card_idx, target));
        let score = evaluate(&ns);
        if score > *best_score || (score == *best_score && best_first.is_none()) {
            *best_score = score;
            *best_first = Some(real_first);
        }

        dfs(&ns, Some(real_first), best_score, best_first, seen);
    }
}
