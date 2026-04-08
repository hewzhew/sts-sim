use serde_json::Value;
use std::collections::HashMap;

use super::mapper::power_id_from_java;
use crate::combat::{CombatState, Power};

// ============================================================================
// Diff Classification
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiffCategory {
    /// Core engine math error: HP, block, energy, hand/pile sizes.
    /// These MUST be fixed — they indicate broken game logic.
    EngineBug,
    /// A power/effect exists in Java but not in Rust, and it's traceable
    /// to an unimplemented monster move, power hook, or relic.
    ContentGap,
    /// Harmless timing difference: dead monster powers, animation lag, etc.
    Timing,
}

impl std::fmt::Display for DiffCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffCategory::EngineBug => write!(f, "ENGINE_BUG"),
            DiffCategory::ContentGap => write!(f, "CONTENT_GAP"),
            DiffCategory::Timing => write!(f, "TIMING"),
        }
    }
}

// ============================================================================
// State Comparison
// ============================================================================

pub struct DiffResult {
    pub field: String,
    pub rust_val: String,
    pub java_val: String,
    pub category: DiffCategory,
}

/// Context about what happened last frame, used to classify diffs.
#[derive(Clone, Default)]
pub struct ActionContext {
    /// What command was sent last frame (e.g. "END", "PLAY 2 0")
    pub last_command: String,
    /// Was the last action an end-turn?
    pub was_end_turn: bool,
    /// Monster intents from the last frame (index → intent string)
    pub monster_intents: Vec<String>,
    /// Monster names from the last frame  
    pub monster_names: Vec<String>,
    /// Whether the Java snapshot includes rng_state for deterministic validation
    pub has_rng_state: bool,
}

impl ActionContext {
    pub fn describe(&self) -> String {
        if self.last_command.is_empty() {
            return "unknown".into();
        }
        let mut s = self.last_command.clone();
        if self.was_end_turn && !self.monster_intents.is_empty() {
            let intents: Vec<String> = self
                .monster_intents
                .iter()
                .enumerate()
                .map(|(i, intent)| {
                    let name = self.monster_names.get(i).map(|s| s.as_str()).unwrap_or("?");
                    format!("M[{}]({})={}", i, name, intent)
                })
                .collect();
            s.push_str(&format!(" → monsters: [{}]", intents.join(", ")));
        }
        s
    }
}

pub fn compare_powers(
    diffs: &mut Vec<DiffResult>,
    prefix: &str,
    entity_id: usize,
    power_db: &HashMap<usize, Vec<Power>>,
    java_powers: &Value,
    context: &ActionContext,
) {
    let rust_powers = power_db.get(&entity_id).cloned().unwrap_or_default();
    let java_arr = java_powers.as_array();

    if let Some(arr) = java_arr {
        for p in arr {
            let java_id = p["id"].as_str().unwrap_or("");
            let java_amount = p["amount"].as_i64().unwrap_or(0) as i32;

            if let Some(rust_pid) = power_id_from_java(java_id) {
                if let Some(rust_p) = rust_powers.iter().find(|rp| rp.power_type == rust_pid) {
                    if rust_p.amount != java_amount {
                        diffs.push(DiffResult {
                            field: format!("{}.power[{}].amount", prefix, java_id),
                            rust_val: rust_p.amount.to_string(),
                            java_val: java_amount.to_string(),
                            // Amount mismatch on a power both sides have → engine bug
                            category: DiffCategory::EngineBug,
                        });
                    }
                } else {
                    // Java has this power, Rust doesn't → classify
                    let category = classify_missing_power(prefix, java_id, context);
                    diffs.push(DiffResult {
                        field: format!("{}.power[{}]", prefix, java_id),
                        rust_val: "MISSING".into(),
                        java_val: format!("amount={}", java_amount),
                        category,
                    });
                }
            }
            // If power_id_from_java returns None, the power type itself is unmapped.
            // This is logged by validate_parse separately, not here.
        }
    }

    for rp in &rust_powers {
        let has_match = java_arr.map_or(false, |arr| {
            arr.iter().any(|jp| {
                let jid = jp["id"].as_str().unwrap_or("");
                power_id_from_java(jid) == Some(rp.power_type)
            })
        });
        if !has_match {
            // GuardianThreshold is an internal Rust-only tracker, Java never exports it
            if rp.power_type == crate::combat::PowerId::GuardianThreshold {
                continue;
            }
            // Rust has a power that Java doesn't → always an engine bug
            // (Rust computed something that shouldn't exist)
            diffs.push(DiffResult {
                field: format!("{}.power[{:?}]", prefix, rp.power_type),
                rust_val: format!("amount={}", rp.amount),
                java_val: "MISSING".into(),
                category: DiffCategory::EngineBug,
            });
        }
    }
}

/// Classify why a power exists in Java but not in Rust.
/// Uses the action context (what happened last frame) to infer
/// whether this is a content gap or an engine bug.
fn classify_missing_power(
    prefix: &str,
    java_power_id: &str,
    context: &ActionContext,
) -> DiffCategory {
    // If this diff is on the player and the last action was EndTurn,
    // check if any monster had a debuff-producing intent.
    // If so, it's likely a content gap (monster move not implemented).
    if prefix == "player" && context.was_end_turn {
        // Powers typically applied by monster debuff moves
        let debuff_from_monster = matches!(
            java_power_id,
            "Weakened"
                | "Vulnerable"
                | "Frail"
                | "Entangled"
                | "Dexterity"
                | "Strength"
                | "No Block"
                | "No Draw"
                | "Draw Reduction"
                | "Constricted"
                | "Hex"
                | "Bias"
        );

        if debuff_from_monster {
            // Check if any monster had a debuff intent
            let monster_was_debuffing = context.monster_intents.iter().any(|intent| {
                matches!(
                    intent.as_str(),
                    "StrongDebuff" | "Debuff" | "AttackDebuff" | "DefendDebuff"
                )
            });
            if monster_was_debuffing {
                return DiffCategory::ContentGap;
            }
        }
    }

    // If this diff is on a monster and the last action was EndTurn,
    // monster self-buffs during their turn that we missed
    if prefix.starts_with("monster") && context.was_end_turn {
        let self_buff = matches!(
            java_power_id,
            "Metallicize"
                | "Ritual"
                | "Thorns"
                | "Plated Armor"
                | "Regenerate"
                | "Angry"
                | "Curl Up"
                | "Sharp Hide"
                | "Spore Cloud"
                | "Malleable"
                | "Mode Shift"
                | "Fading"
                | "Invincible"
                | "Curiosity"
                | "Time Warp"
                | "Stasis"
        );
        if self_buff {
            return DiffCategory::ContentGap;
        }
    }

    // Default: if we can't explain why the power is missing, it's an engine bug.
    DiffCategory::EngineBug
}

pub fn compare_states(
    cs: &CombatState,
    java_snapshot: &Value,
    skip_piles: bool,
    context: &ActionContext,
) -> Vec<DiffResult> {
    let mut diffs = Vec::new();
    let java_player = &java_snapshot["player"];

    let java_hp = java_player["current_hp"]
        .as_i64()
        .unwrap_or(java_player["hp"].as_i64().unwrap_or(0)) as i32;
    if cs.player.current_hp != java_hp {
        diffs.push(DiffResult {
            field: "player.hp".into(),
            rust_val: cs.player.current_hp.to_string(),
            java_val: java_hp.to_string(),
            category: if context.was_end_turn {
                DiffCategory::ContentGap
            } else {
                DiffCategory::EngineBug
            },
        });
    }

    let java_block = java_player["block"].as_i64().unwrap_or(0) as i32;
    if cs.player.block != java_block {
        diffs.push(DiffResult {
            field: "player.block".into(),
            rust_val: cs.player.block.to_string(),
            java_val: java_block.to_string(),
            category: if context.was_end_turn {
                DiffCategory::ContentGap
            } else {
                DiffCategory::EngineBug
            },
        });
    }

    let java_energy = java_player["energy"].as_u64().unwrap_or(0) as u8;
    if cs.energy != java_energy {
        diffs.push(DiffResult {
            field: "player.energy".into(),
            rust_val: cs.energy.to_string(),
            java_val: java_energy.to_string(),
            category: DiffCategory::EngineBug,
        });
    }

    let java_monsters = java_snapshot["monsters"].as_array();
    if let Some(java_ms) = java_monsters {
        for (i, jm) in java_ms.iter().enumerate() {
            if i >= cs.monsters.len() {
                continue;
            }
            let rm = &cs.monsters[i];
            let jm_hp = jm["current_hp"]
                .as_i64()
                .unwrap_or(jm["hp"].as_i64().unwrap_or(0)) as i32;
            let jm_block = jm["block"].as_i64().unwrap_or(0) as i32;

            if rm.current_hp != jm_hp {
                diffs.push(DiffResult {
                    field: format!("monster[{}].hp", i),
                    rust_val: rm.current_hp.to_string(),
                    java_val: jm_hp.to_string(),
                    category: DiffCategory::EngineBug,
                });
            }
            if rm.block != jm_block {
                diffs.push(DiffResult {
                    field: format!("monster[{}].block", i),
                    rust_val: rm.block.to_string(),
                    java_val: jm_block.to_string(),
                    category: if context.was_end_turn {
                        DiffCategory::ContentGap
                    } else {
                        DiffCategory::EngineBug
                    },
                });
            }
        }
    }

    if !skip_piles {
        let java_hand_size = java_snapshot["hand_size"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                java_snapshot["hand"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            });
        if cs.hand.len() != java_hand_size {
            diffs.push(DiffResult {
                field: "hand_size".into(),
                rust_val: cs.hand.len().to_string(),
                java_val: java_hand_size.to_string(),
                category: DiffCategory::EngineBug,
            });
        }

        let java_discard = java_snapshot["discard_pile_size"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                java_snapshot["discard_pile"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            });
        if cs.discard_pile.len() != java_discard {
            diffs.push(DiffResult {
                field: "discard_pile_size".into(),
                rust_val: cs.discard_pile.len().to_string(),
                java_val: java_discard.to_string(),
                category: DiffCategory::EngineBug,
            });
        }

        let java_exhaust = java_snapshot["exhaust_pile_size"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                java_snapshot["exhaust_pile"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            });
        if cs.exhaust_pile.len() != java_exhaust {
            diffs.push(DiffResult {
                field: "exhaust_pile_size".into(),
                rust_val: cs.exhaust_pile.len().to_string(),
                java_val: java_exhaust.to_string(),
                category: DiffCategory::EngineBug,
            });
        }
    }

    compare_powers(
        &mut diffs,
        "player",
        0,
        &cs.power_db,
        &java_player["powers"],
        context,
    );

    if let Some(java_ms) = java_monsters {
        for (i, jm) in java_ms.iter().enumerate() {
            if i >= cs.monsters.len() {
                continue;
            }

            // Skip power comparison for dead monsters (Java clears them asynchronously after death animations)
            let is_dead = jm["is_gone"].as_bool().unwrap_or(false)
                || jm["current_hp"].as_i64().unwrap_or(1) <= 0;
            if is_dead {
                continue;
            }

            let entity_id = cs.monsters[i].id;
            compare_powers(
                &mut diffs,
                &format!("monster[{}]", i),
                entity_id,
                &cs.power_db,
                &jm["powers"],
                context,
            );
        }
    }

    filter_nondiagnostic_random_target_diffs(diffs, context)
}

fn filter_nondiagnostic_random_target_diffs(
    diffs: Vec<DiffResult>,
    context: &ActionContext,
) -> Vec<DiffResult> {
    if context.has_rng_state || !context.was_end_turn {
        return diffs;
    }

    // replay_short has no ai_rng state, so Shield Gremlin's protect target is
    // not deterministically replayable. If the only divergence is a pure block
    // redistribution between two monsters after an end-turn with Shield Gremlin
    // present, treat it as non-diagnostic instead of a hard failure.
    let has_shield_gremlin = context
        .monster_names
        .iter()
        .any(|name| name == "GremlinTsundere" || name == "Shield Gremlin");
    if !has_shield_gremlin {
        return diffs;
    }

    let block_diffs: Vec<_> = diffs
        .iter()
        .filter(|d| d.field.starts_with("monster[") && d.field.ends_with("].block"))
        .collect();

    if diffs.len() == 2 && block_diffs.len() == 2 {
        let parsed: Option<Vec<(i32, i32)>> = block_diffs
            .iter()
            .map(|d| Some((d.rust_val.parse().ok()?, d.java_val.parse().ok()?)))
            .collect();

        if let Some(values) = parsed {
            let (r0, j0) = values[0];
            let (r1, j1) = values[1];
            if r0 == j1 && r1 == j0 && r0 + r1 == j0 + j1 {
                return Vec::new();
            }
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::parser::parse_replay;
    use crate::diff::replay_support::{continue_deferred_pending_choice, tick_until_stable};
    use crate::diff::state_sync::{build_combat_state, sync_state};
    use crate::state::core::{ClientInput, EngineState, PendingChoice};

    #[test]
    fn replay_short_combat2_bash_keeps_vulnerable_in_java_snapshot() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replay_short.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 2)
            .expect("combat 2 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(5) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| t + 1),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let bash_result = &combat.actions[4].result;
        let java_powers = bash_result["monsters"][0]["powers"]
            .as_array()
            .expect("monster powers array");
        assert!(
            java_powers
                .iter()
                .any(|p| p["id"].as_str() == Some("Vulnerable")),
            "expected Java snapshot to contain Vulnerable, got {:?}",
            java_powers
        );

        let diffs = compare_states(&cs, bash_result, false, &ActionContext::default());
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[0].power[Vulnerable]"),
            "unexpected vulnerable diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn replay_short_combat12_speed_potion_clears_temp_dexterity_on_end_turn() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replay_short.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 12)
            .expect("combat 12 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(7) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| t + 1),
                },
                "potion" => {
                    let cmd = action.command.as_deref().expect("potion command");
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    ClientInput::UsePotion {
                        potion_index: parts[2].parse::<usize>().expect("potion slot"),
                        target: parts
                            .get(3)
                            .and_then(|s| s.parse::<usize>().ok())
                            .map(|t| t + 1),
                    }
                }
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };

            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let end_turn_result = &combat.actions[6].result;
        let java_player_powers = end_turn_result["player"]["powers"]
            .as_array()
            .expect("player powers array");
        assert!(
            java_player_powers.is_empty(),
            "expected Java snapshot to clear temporary dexterity, got {:?}",
            java_player_powers
        );

        let diffs = compare_states(&cs, end_turn_result, true, &ActionContext::default());
        assert!(
            diffs.iter().all(|d| {
                d.field != "player.power[Dexterity]" && d.field != "player.power[DexterityDown]"
            }),
            "unexpected dexterity diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn replay_short_combat10_twin_strike_kills_louse_before_curl_up_block_resolves() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replay_short.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 10)
            .expect("combat 10 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(2) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| t + 1),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let twin_strike_result = &combat.actions[1].result;
        let java_target = &twin_strike_result["monsters"][2];
        assert_eq!(java_target["hp"].as_i64(), Some(0));
        assert_eq!(java_target["block"].as_i64(), Some(0));

        let diffs = compare_states(&cs, twin_strike_result, false, &ActionContext::default());
        assert!(
            diffs
                .iter()
                .all(|d| { d.field != "monster[2].hp" && d.field != "monster[2].block" }),
            "unexpected twin strike/curl up diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn replay_short_combat5_shield_gremlin_random_block_swap_is_ignored_without_rng_state() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replay_short.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 5)
            .expect("combat 5 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(4) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| t + 1),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let end_turn_result = &combat.actions[3].result;
        let context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            monster_intents: end_turn_result["monsters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["intent"].as_str().unwrap_or("?").to_string())
                .collect(),
            monster_names: end_turn_result["monsters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["id"].as_str().unwrap_or("?").to_string())
                .collect(),
            has_rng_state: false,
        };

        let diffs = compare_states(&cs, end_turn_result, true, &context);
        assert!(
            diffs.is_empty(),
            "expected Shield Gremlin random protect target diff to be ignored, got {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_battle_trance_uses_java_no_draw_sentinel_amount() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 7)
            .expect("combat 7 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(6) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let battle_trance_result = &combat.actions[5].result;
        let local_no_draw = cs
            .power_db
            .get(&0)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == crate::content::powers::PowerId::NoDraw)
            })
            .map(|p| p.amount);
        assert_eq!(
            local_no_draw,
            Some(-1),
            "expected local state to contain No Draw(-1), got {:?}",
            cs.power_db.get(&0)
        );
        let context = ActionContext {
            last_command: "Battle Trance".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, battle_trance_result, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "player.power[No Draw]"
                    && d.field != "player.power[No Draw].amount"),
            "unexpected No Draw diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_gremlin_nob_enrage_grants_strength_when_skill_is_played() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 8)
            .expect("combat 8 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(5) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let defend_result = &combat.actions[4].result;
        let nob_id = cs.monsters[0].id;
        let local_strength = cs
            .power_db
            .get(&nob_id)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == crate::content::powers::PowerId::Strength)
            })
            .map(|p| p.amount);
        assert_eq!(
            local_strength,
            Some(2),
            "expected Gremlin Nob to gain 2 Strength after player used a skill, got {:?}",
            cs.power_db.get(&nob_id)
        );

        let context = ActionContext {
            last_command: "Defend".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, defend_result, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[0].power[Strength]"),
            "unexpected Gremlin Nob Enrage diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_dropkick_into_sharp_hide_triggers_centennial_puzzle_draw() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 10)
            .expect("combat 10 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(8) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "potion" => ClientInput::UsePotion {
                    potion_index: action
                        .command
                        .as_deref()
                        .and_then(|cmd| cmd.split_whitespace().nth(2))
                        .and_then(|slot| slot.parse::<usize>().ok())
                        .expect("expected replay potion command to include slot index"),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let dropkick_result = &combat.actions[7].result;
        assert_eq!(
            cs.player.current_hp, 77,
            "expected Sharp Hide to deal 3 damage before Centennial Puzzle draw, got hp={} with relics {:?}",
            cs.player.current_hp, cs.player.relics
        );
        assert_eq!(
            cs.hand.len(),
            8,
            "expected Dropkick + Centennial Puzzle to leave 8 cards in hand, got {:?}",
            cs.hand.iter().map(|c| c.id).collect::<Vec<_>>()
        );

        let puzzle_used = cs
            .player
            .relics
            .iter()
            .find(|r| r.id == crate::content::relics::RelicId::CentennialPuzzle)
            .map(|r| r.used_up);
        assert_eq!(
            puzzle_used,
            Some(true),
            "expected Centennial Puzzle to mark itself used after first HP loss, got {:?}",
            cs.player.relics
        );

        let context = ActionContext {
            last_command: "Dropkick".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, dropkick_result, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "hand_size" && d.field != "player.hp"),
            "unexpected Dropkick/Sharp Hide/Centennial Puzzle diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_floor18_end_turn_keeps_fusion_hammer_energy_master() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 11)
            .expect("combat 11 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(6) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;

            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            tick_until_stable(&mut es, &mut cs, input);
            prev_snapshot = action.result.clone();
        }

        let end_turn_result = &combat.actions[5].result;
        let has_fusion_hammer = cs
            .player
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::FusionHammer);
        assert!(
            has_fusion_hammer,
            "expected Fusion Hammer relic in local state"
        );
        assert_eq!(
            cs.player.energy_master, 4,
            "expected Fusion Hammer to keep energy_master at 4, got relics {:?}",
            cs.player.relics
        );
        assert_eq!(
            cs.energy, 4,
            "expected new turn energy to recharge to 4, got energy={} energy_master={} relics {:?}",
            cs.energy, cs.player.energy_master, cs.player.relics
        );

        let context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, end_turn_result, true, &context);
        assert!(
            diffs.iter().all(|d| d.field != "player.energy"),
            "unexpected floor 18 energy diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_thunderclap_hits_byrd_for_two_through_flight() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 14)
            .expect("combat 14 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        let action = &combat.actions[0];
        sync_state(&mut cs, &prev_snapshot);
        let mut es = EngineState::CombatPlayerTurn;
        let input = match action.action_type.as_str() {
            "play" => ClientInput::PlayCard {
                card_index: action.card_index.unwrap(),
                target: action.target.map(|t| cs.monsters[t as usize].id),
            },
            other => panic!("unexpected action type {other}"),
        };
        tick_until_stable(&mut es, &mut cs, input);
        prev_snapshot = action.result.clone();

        assert_eq!(
            cs.monsters[0].current_hp,
            29,
            "expected Thunderclap to deal 2 damage to Byrd through Flight, got monsters {:?}",
            cs.monsters
                .iter()
                .map(|m| (m.id, m.current_hp, m.block))
                .collect::<Vec<_>>()
        );
        let byrd_flight = cs
            .power_db
            .get(&cs.monsters[0].id)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == crate::content::powers::PowerId::Flight)
            })
            .map(|p| p.amount);
        assert_eq!(
            byrd_flight,
            Some(2),
            "expected Byrd Flight to drop from 3 to 2 after Thunderclap, got {:?}",
            cs.power_db.get(&cs.monsters[0].id)
        );

        let context = ActionContext {
            last_command: "Thunderclap".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[0].hp" && d.field != "monster[0].power[Flight].amount"),
            "unexpected Thunderclap/Flight diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_battle_trance_does_not_trigger_mad_gremlin_angry() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 15)
            .expect("combat 15 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        let action = &combat.actions[0];
        sync_state(&mut cs, &prev_snapshot);
        let mut es = EngineState::CombatPlayerTurn;
        let input = match action.action_type.as_str() {
            "play" => ClientInput::PlayCard {
                card_index: action.card_index.unwrap(),
                target: action.target.map(|t| cs.monsters[t as usize].id),
            },
            other => panic!("unexpected action type {other}"),
        };
        tick_until_stable(&mut es, &mut cs, input);
        prev_snapshot = action.result.clone();

        let mad_gremlin_id = cs.monsters[0].id;
        let local_strength = cs
            .power_db
            .get(&mad_gremlin_id)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == crate::content::powers::PowerId::Strength)
            })
            .map(|p| p.amount);
        assert_eq!(
            local_strength,
            None,
            "expected Battle Trance not to grant Mad Gremlin Strength, got {:?}",
            cs.power_db.get(&mad_gremlin_id)
        );

        let context = ActionContext {
            last_command: "Battle Trance".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[0].power[Strength]"),
            "unexpected Mad Gremlin Angry diff after Battle Trance: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_collector_gamblers_brew_replay_continues_before_fear_potion() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 16)
            .expect("combat 16 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending: Option<PendingChoice> = None;

        for action in combat.actions.iter().take(2) {
            sync_state(&mut cs, &prev_snapshot);

            if let Some(pending) = carried_pending.take() {
                let alive = continue_deferred_pending_choice(&pending, &mut cs, &action.result)
                    .expect("expected deferred GamblingChip continuation to be inferred");
                assert!(
                    alive,
                    "expected deferred GamblingChip continuation to keep combat alive"
                );
            }

            let mut es = EngineState::CombatPlayerTurn;
            let cmd = action.command.as_deref().expect("potion command");
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            let input = ClientInput::UsePotion {
                potion_index: parts[2].parse::<usize>().expect("potion slot"),
                target: parts
                    .get(3)
                    .and_then(|s| s.parse::<usize>().ok())
                    .map(|t| cs.monsters[t as usize].id),
            };
            tick_until_stable(&mut es, &mut cs, input);
            carried_pending = match &es {
                EngineState::PendingChoice(choice) => Some(choice.clone()),
                _ => None,
            };
            prev_snapshot = action.result.clone();
        }

        assert_eq!(
            cs.discard_pile.len(),
            3,
            "expected deferred GamblingChip replay continuation to discard 3 cards, got {:?}",
            cs.discard_pile
                .iter()
                .map(|c| (c.id, c.uuid))
                .collect::<Vec<_>>()
        );

        let context = ActionContext {
            last_command: "POTION(potion use 1 0)".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "discard_pile_size" && d.field != "draw_pile_size"),
            "unexpected Collector deferred Gamblers Brew diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_collector_spawn_order_keeps_torch_heads_before_collector() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 16)
            .expect("combat 16 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut es = EngineState::CombatPlayerTurn;

        sync_state(&mut cs, &prev_snapshot);
        let alive = tick_until_stable(&mut es, &mut cs, ClientInput::EndTurn);
        assert!(alive, "expected Collector summon turn to keep combat alive");
        prev_snapshot = combat.actions[6].result.clone();

        let local_order: Vec<_> = cs
            .monsters
            .iter()
            .map(|m| crate::content::monsters::EnemyId::from_id(m.monster_type))
            .collect();
        assert_eq!(
            local_order,
            vec![
                Some(crate::content::monsters::EnemyId::TorchHead),
                Some(crate::content::monsters::EnemyId::TorchHead),
                Some(crate::content::monsters::EnemyId::TheCollector),
            ],
            "expected Collector summon order [TorchHead, TorchHead, TheCollector], got {:?}",
            cs.monsters
                .iter()
                .map(|m| (m.id, m.slot, m.logical_position, m.monster_type))
                .collect::<Vec<_>>()
        );

        let context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            has_rng_state: true,
            monster_intents: prev_snapshot["monsters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["intent"].as_str().unwrap_or("?").to_string())
                .collect(),
            monster_names: prev_snapshot["monsters"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["id"].as_str().unwrap_or("?").to_string())
                .collect(),
        };
        let diffs = compare_states(&cs, &prev_snapshot, true, &context);
        assert!(
            diffs.iter().all(|d| {
                d.field != "monster[0].hp"
                    && d.field != "monster[1].hp"
                    && d.field != "monster[0].power[Minion]"
                    && d.field != "monster[1].power[Minion]"
            }),
            "unexpected Collector spawn-order diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_awakened_one_gamblers_brew_defers_toy_ornithopter_heal_until_after_choice() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 26)
            .expect("combat 26 not found");
        let action = combat
            .actions
            .first()
            .expect("expected first potion action");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut es = EngineState::CombatPlayerTurn;
        let cmd = action.command.as_deref().expect("potion command");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let input = ClientInput::UsePotion {
            potion_index: parts[2].parse::<usize>().expect("potion slot"),
            target: parts
                .get(3)
                .and_then(|s| s.parse::<usize>().ok())
                .map(|t| cs.monsters[t as usize].id),
        };

        let alive = tick_until_stable(&mut es, &mut cs, input);
        assert!(alive, "expected Awakened One combat to stay alive");
        assert!(
            matches!(
                es,
                EngineState::PendingChoice(PendingChoice::HandSelect {
                    reason: crate::state::core::HandSelectReason::GamblingChip,
                    ..
                })
            ),
            "expected Gamblers Brew to suspend for hand select, got {:?}",
            es
        );
        assert_eq!(
            cs.player.current_hp, 64,
            "expected Toy Ornithopter heal to remain queued until after Gambling Chip selection"
        );

        let context = ActionContext {
            last_command: "POTION(potion use 0)".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &action.result, false, &context);
        assert!(
            diffs.iter().all(|d| d.field != "player.hp"),
            "unexpected Awakened One Gamblers Brew hp diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_awakened_one_gamblers_brew_continuation_preserves_deferred_toy_ornithopter_heal()
    {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 26)
            .expect("combat 26 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        let first = &combat.actions[0];
        let second = &combat.actions[1];

        sync_state(&mut cs, &prev_snapshot);
        let mut es = EngineState::CombatPlayerTurn;
        let first_cmd = first.command.as_deref().expect("first potion command");
        let first_parts: Vec<&str> = first_cmd.split_whitespace().collect();
        let first_input = ClientInput::UsePotion {
            potion_index: first_parts[2].parse::<usize>().expect("potion slot"),
            target: first_parts
                .get(3)
                .and_then(|s| s.parse::<usize>().ok())
                .map(|t| cs.monsters[t as usize].id),
        };
        assert!(tick_until_stable(&mut es, &mut cs, first_input));
        let pending = match &es {
            EngineState::PendingChoice(choice) => choice.clone(),
            other => panic!(
                "expected pending choice after Gamblers Brew, got {:?}",
                other
            ),
        };
        prev_snapshot = first.result.clone();

        sync_state(&mut cs, &prev_snapshot);
        let alive = continue_deferred_pending_choice(&pending, &mut cs, &second.result)
            .expect("expected deferred Gambling Chip continuation to succeed");
        assert!(alive, "expected deferred continuation to keep combat alive");

        let second_cmd = second.command.as_deref().expect("second potion command");
        let second_parts: Vec<&str> = second_cmd.split_whitespace().collect();
        let second_input = ClientInput::UsePotion {
            potion_index: second_parts[2].parse::<usize>().expect("potion slot"),
            target: second_parts
                .get(3)
                .and_then(|s| s.parse::<usize>().ok())
                .map(|t| cs.monsters[t as usize].id),
        };
        let mut second_es = EngineState::CombatPlayerTurn;
        assert!(tick_until_stable(&mut second_es, &mut cs, second_input));
        assert_eq!(
            cs.player.current_hp, 74,
            "expected deferred Toy Ornithopter heal from Gamblers Brew plus Fire Potion heal"
        );
    }

    #[test]
    fn rng_replay_awakened_one_regenerate_does_not_tick_down_on_end_turn() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 26)
            .expect("combat 26 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending: Option<PendingChoice> = None;

        for action in combat.actions.iter().take(8) {
            sync_state(&mut cs, &prev_snapshot);

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    let alive = continue_deferred_pending_choice(&pending, &mut cs, &action.result)
                        .expect("expected deferred GamblingChip continuation");
                    assert!(alive, "expected deferred continuation to keep combat alive");
                }
            }

            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "potion" => {
                    let cmd = action.command.as_deref().expect("potion command");
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    ClientInput::UsePotion {
                        potion_index: parts[2].parse::<usize>().expect("potion slot"),
                        target: parts
                            .get(3)
                            .and_then(|s| s.parse::<usize>().ok())
                            .map(|t| cs.monsters[t as usize].id),
                    }
                }
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };

            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Awakened One combat to stay alive");
            carried_pending = match &es {
                EngineState::PendingChoice(choice) => Some(choice.clone()),
                _ => None,
            };
            prev_snapshot = action.result.clone();
        }

        let awakened = &cs.monsters[2];
        let regen = cs.power_db.get(&awakened.id).and_then(|powers| {
            powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Regen)
                .map(|p| p.amount)
        });
        assert_eq!(
            regen,
            Some(10),
            "expected Awakened One RegenerateMonsterPower to remain at 10 after end turn"
        );

        let context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, true, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[2].power[Regenerate]"),
            "unexpected Awakened One Regenerate diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_awakened_one_phase_one_death_waits_until_end_turn_to_rebirth() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 26)
            .expect("combat 26 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending: Option<PendingChoice> = None;

        for action in combat.actions.iter().take(35) {
            sync_state(&mut cs, &prev_snapshot);

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    let alive = continue_deferred_pending_choice(&pending, &mut cs, &action.result)
                        .expect("expected deferred GamblingChip continuation");
                    assert!(alive, "expected deferred continuation to keep combat alive");
                }
            }

            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "potion" => {
                    let cmd = action.command.as_deref().expect("potion command");
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    ClientInput::UsePotion {
                        potion_index: parts[2].parse::<usize>().expect("potion slot"),
                        target: parts
                            .get(3)
                            .and_then(|s| s.parse::<usize>().ok())
                            .map(|t| cs.monsters[t as usize].id),
                    }
                }
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };

            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Awakened One combat to stay alive");
            carried_pending = match &es {
                EngineState::PendingChoice(choice) => Some(choice.clone()),
                _ => None,
            };
            prev_snapshot = action.result.clone();
        }

        let awakened = &cs.monsters[2];
        assert_eq!(awakened.current_hp, 0);
        assert!(awakened.half_dead, "expected Awakened One to be half-dead");
        assert_eq!(awakened.next_move_byte, 3);
        let regen = cs.power_db.get(&awakened.id).and_then(|powers| {
            powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Regen)
                .map(|p| p.amount)
        });
        assert_eq!(regen, Some(10));
        assert_eq!(
            cs.discard_pile.len(),
            12,
            "expected current Perfected Strike to reach discard before rebirth"
        );

        let kill_context = ActionContext {
            last_command: "PLAY".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let kill_diffs = compare_states(&cs, &prev_snapshot, false, &kill_context);
        assert!(
            kill_diffs.iter().all(|d| {
                d.field != "monster[2].hp"
                    && d.field != "monster[2].power[Regenerate]"
                    && d.field != "discard_pile_size"
            }),
            "unexpected Awakened One phase-1 death diff: {:?}",
            kill_diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );

        let rebirth_action = &combat.actions[35];
        sync_state(&mut cs, &prev_snapshot);
        let mut es = EngineState::CombatPlayerTurn;
        let alive = tick_until_stable(&mut es, &mut cs, ClientInput::EndTurn);
        assert!(alive, "expected rebirth end turn to keep combat alive");
        let reborn = &cs.monsters[2];
        assert_eq!(reborn.current_hp, 300);
        assert!(!reborn.half_dead);
        assert_eq!(reborn.next_move_byte, 5);

        let rebirth_context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            has_rng_state: true,
            ..Default::default()
        };
        let rebirth_diffs = compare_states(&cs, &rebirth_action.result, true, &rebirth_context);
        assert!(
            rebirth_diffs.iter().all(|d| {
                d.field != "monster[2].hp"
                    && d.field != "monster[2].power[Regenerate]"
                    && d.field != "monster[2].move_id"
            }),
            "unexpected Awakened One rebirth diff: {:?}",
            rebirth_diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_collector_dropkick_keeps_java_float_damage_pipeline() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 16)
            .expect("combat 16 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending: Option<PendingChoice> = None;

        for action in combat.actions.iter().take(26) {
            sync_state(&mut cs, &prev_snapshot);

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    let alive = continue_deferred_pending_choice(&pending, &mut cs, &action.result)
                        .expect("expected deferred GamblingChip continuation to be inferred");
                    assert!(
                        alive,
                        "expected deferred pending choice to keep combat alive"
                    );
                }
            }

            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "potion" => {
                    let cmd = action.command.as_deref().expect("potion command");
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    ClientInput::UsePotion {
                        potion_index: parts[2].parse::<usize>().expect("potion slot"),
                        target: parts
                            .get(3)
                            .and_then(|s| s.parse::<usize>().ok())
                            .map(|t| cs.monsters[t as usize].id),
                    }
                }
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(
                alive,
                "expected combat to stay alive through Dropkick replay"
            );
            carried_pending = match &es {
                EngineState::PendingChoice(choice) => Some(choice.clone()),
                _ => None,
            };
            prev_snapshot = action.result.clone();
        }

        assert_eq!(
            cs.monsters[2].current_hp, 85,
            "expected Dropkick under Weak + Vulnerable to deal 5 damage (5 * 0.75 * 1.5 floored once), got monsters {:?}",
            cs.monsters
                .iter()
                .map(|m| (m.id, m.current_hp, m.block))
                .collect::<Vec<_>>()
        );

        let context = ActionContext {
            last_command: "Dropkick".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs.iter().all(|d| d.field != "monster[2].hp"),
            "unexpected Dropkick float-damage diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_orb_walkers_gain_strength_from_generic_strength_up_at_end_of_round() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 18)
            .expect("combat 18 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(6) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Orb Walker combat to stay alive");
            prev_snapshot = action.result.clone();
        }

        let strengths: Vec<_> = cs
            .monsters
            .iter()
            .map(|m| {
                cs.power_db.get(&m.id).and_then(|powers| {
                    powers
                        .iter()
                        .find(|p| p.power_type == crate::content::powers::PowerId::Strength)
                        .map(|p| p.amount)
                })
            })
            .collect();
        assert_eq!(
            strengths,
            vec![Some(3), Some(3)],
            "expected both Orb Walkers to gain 3 Strength from GenericStrengthUp after end turn, got {:?}",
            cs.power_db
        );

        let context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, true, &context);
        assert!(
            diffs.iter().all(|d| d.field != "monster[0].power[Strength]"
                && d.field != "monster[1].power[Strength]"),
            "unexpected Orb Walker GenericStrengthUp diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_battle_trance_stops_drawing_when_hand_is_full_instead_of_discarding_overflow() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 18)
            .expect("combat 18 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(8) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Orb Walker combat to stay alive");
            prev_snapshot = action.result.clone();
        }

        assert_eq!(
            cs.hand.len(),
            10,
            "expected Battle Trance to stop at full hand size 10, got {:?}",
            cs.hand.iter().map(|c| c.id).collect::<Vec<_>>()
        );
        assert_eq!(
            cs.discard_pile.len(),
            2,
            "expected overflow draws to stop instead of discarding extra cards, got discard {:?}",
            cs.discard_pile
                .iter()
                .map(|c| (c.id, c.uuid))
                .collect::<Vec<_>>()
        );

        let context = ActionContext {
            last_command: "Battle Trance".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "discard_pile_size" && d.field != "hand_size"),
            "unexpected Battle Trance full-hand draw diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_writhing_mass_keeps_malleable_base_power_after_end_turn() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 21)
            .expect("combat 21 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(5) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Writhing Mass combat to stay alive");
            prev_snapshot = action.result.clone();
        }

        let malleable = cs
            .power_db
            .get(&cs.monsters[0].id)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
            })
            .map(|p| (p.amount, p.extra_data));
        assert_eq!(
            malleable,
            Some((3, 3)),
            "expected Writhing Mass Malleable to reset to basePower 3 and remain present, got {:?}",
            cs.power_db.get(&cs.monsters[0].id)
        );

        let context = ActionContext {
            last_command: "END_TURN".into(),
            was_end_turn: true,
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, true, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[0].power[Malleable]"),
            "unexpected Writhing Mass Malleable diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_writhing_mass_malleable_triggers_on_big_nonlethal_hit_using_pre_hit_hp() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 21)
            .expect("combat 21 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(9) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Writhing Mass combat to stay alive");
            prev_snapshot = action.result.clone();
        }

        let writhing = &cs.monsters[0];
        assert_eq!(
            writhing.block, 3,
            "expected Writhing Mass to gain 3 block from Malleable after Perfected Strike, got hp/block {:?}",
            (writhing.current_hp, writhing.block)
        );
        let malleable_amount = cs.power_db.get(&writhing.id).and_then(|powers| {
            powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Malleable)
                .map(|p| p.amount)
        });
        assert_eq!(
            malleable_amount,
            Some(4),
            "expected Malleable to increment from 3 to 4 after the hit, got {:?}",
            cs.power_db.get(&writhing.id)
        );

        let context = ActionContext {
            last_command: "Perfected Strike".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs.iter().all(|d| d.field != "monster[0].block"),
            "unexpected Writhing Mass Malleable block diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_transient_shifting_adds_shackled_to_restore_lost_strength() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 22)
            .expect("combat 22 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(2) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Transient combat to stay alive");
            prev_snapshot = action.result.clone();
        }

        let transient = &cs.monsters[0];
        let shackled = cs.power_db.get(&transient.id).and_then(|powers| {
            powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Shackled)
                .map(|p| p.amount)
        });
        assert_eq!(
            shackled,
            Some(7),
            "expected Transient Shifting to add Shackled(7) after Dropkick, got {:?}",
            cs.power_db.get(&transient.id)
        );

        let context = ActionContext {
            last_command: "Dropkick".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs
                .iter()
                .all(|d| d.field != "monster[0].power[Shackled]"),
            "unexpected Transient Shifting/Shackled diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rng_replay_transient_letter_opener_thorns_also_stacks_shifting() {
        let replay = parse_replay(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
        ));
        let combat = replay
            .combats
            .iter()
            .find(|c| c.combat_idx == 22)
            .expect("combat 22 not found");

        let mut cs = build_combat_state(&combat.start_snapshot, &Value::Null);
        let mut prev_snapshot = combat.start_snapshot.clone();

        for action in combat.actions.iter().take(4) {
            sync_state(&mut cs, &prev_snapshot);
            let mut es = EngineState::CombatPlayerTurn;
            let input = match action.action_type.as_str() {
                "play" => ClientInput::PlayCard {
                    card_index: action.card_index.unwrap(),
                    target: action.target.map(|t| cs.monsters[t as usize].id),
                },
                "end_turn" => ClientInput::EndTurn,
                other => panic!("unexpected action type {other}"),
            };
            let alive = tick_until_stable(&mut es, &mut cs, input);
            assert!(alive, "expected Transient combat to stay alive");
            prev_snapshot = action.result.clone();
        }

        let transient = &cs.monsters[0];
        let powers = cs
            .power_db
            .get(&transient.id)
            .expect("expected Transient powers after Shrug It Off");
        let shackled = powers
            .iter()
            .find(|p| p.power_type == crate::content::powers::PowerId::Shackled)
            .map(|p| p.amount);
        let strength = powers
            .iter()
            .find(|p| p.power_type == crate::content::powers::PowerId::Strength)
            .map(|p| p.amount);
        assert_eq!(
            shackled,
            Some(12),
            "expected Letter Opener THORNS to stack Shifting up to Shackled(12), got {:?}",
            powers
        );
        assert_eq!(
            strength,
            Some(-12),
            "expected Letter Opener THORNS to stack Shifting up to Strength(-12), got {:?}",
            powers
        );

        let context = ActionContext {
            last_command: "Shrug It Off".into(),
            has_rng_state: true,
            ..Default::default()
        };
        let diffs = compare_states(&cs, &prev_snapshot, false, &context);
        assert!(
            diffs.iter().all(|d| {
                d.field != "monster[0].power[Shackled]" && d.field != "monster[0].power[Strength]"
            }),
            "unexpected Transient cumulative Shifting diff: {:?}",
            diffs
                .iter()
                .map(|d| (&d.field, &d.rust_val, &d.java_val))
                .collect::<Vec<_>>()
        );
    }
}
