use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::protocol::java::{
    monster_id_from_java, power_id_from_java, power_instance_id_from_java,
};
use crate::runtime::combat::{CombatState, Power};

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
                let java_instance_id = if crate::content::powers::uses_distinct_instances(rust_pid)
                {
                    power_instance_id_from_java(java_id)
                } else {
                    None
                };
                if let Some(rust_p) = rust_powers.iter().find(|rp| {
                    rp.power_type == rust_pid
                        && (!crate::content::powers::uses_distinct_instances(rust_pid)
                            || rp.instance_id == java_instance_id)
                }) {
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
                    && (!crate::content::powers::uses_distinct_instances(rp.power_type)
                        || rp.instance_id == power_instance_id_from_java(jid))
            })
        });
        if !has_match {
            // GuardianThreshold is an internal Rust-only tracker, Java never exports it
            if rp.power_type == crate::runtime::combat::PowerId::GuardianThreshold {
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

fn java_monster_instance_id(monster: &Value) -> Option<u64> {
    monster.get("monster_instance_id").and_then(|v| v.as_u64())
}

fn java_monster_draw_x(monster: &Value) -> Option<i32> {
    monster
        .get("draw_x")
        .and_then(|v| v.as_i64().map(|value| value as i32))
        .or_else(|| {
            monster
                .get("draw_x")
                .and_then(|v| v.as_f64().map(|value| value.round() as i32))
        })
}

fn align_rust_monsters_to_java(cs: &CombatState, java_ms: &[Value]) -> Vec<Option<usize>> {
    let mut used = HashSet::new();
    let mut aligned = Vec::with_capacity(java_ms.len());

    for (java_index, java_monster) in java_ms.iter().enumerate() {
        let matched = {
            let java_type = monster_id_from_java(java_monster["id"].as_str().unwrap_or(""));
            let java_draw_x = java_monster_draw_x(java_monster);
            cs.entities
                .monsters
                .iter()
                .enumerate()
                .find(|(idx, monster)| {
                    !used.contains(idx)
                        && Some(monster.monster_type) == java_type.map(|id| id as usize)
                        && cs
                            .monster_protocol_identity(monster.id)
                            .and_then(|identity| identity.draw_x)
                            == java_draw_x
                        && monster.is_dying
                            == (java_monster["is_gone"].as_bool().unwrap_or(false)
                                && !java_monster["half_dead"].as_bool().unwrap_or(false))
                        && monster.half_dead == java_monster["half_dead"].as_bool().unwrap_or(false)
                })
                .map(|(idx, _)| idx)
        }
        .or_else(|| {
            java_monster_instance_id(java_monster).and_then(|instance_id| {
                cs.entities
                    .monsters
                    .iter()
                    .enumerate()
                    .find(|(idx, monster)| {
                        !used.contains(idx)
                            && cs
                                .monster_protocol_identity(monster.id)
                                .and_then(|identity| identity.instance_id)
                                == Some(instance_id)
                    })
                    .map(|(idx, _)| idx)
            })
        })
        .or_else(|| {
            (java_index < cs.entities.monsters.len() && !used.contains(&java_index))
                .then_some(java_index)
        })
        .or_else(|| {
            cs.entities
                .monsters
                .iter()
                .enumerate()
                .find(|(idx, _)| !used.contains(idx))
                .map(|(idx, _)| idx)
        });

        if let Some(idx) = matched {
            used.insert(idx);
        }
        aligned.push(matched);
    }

    aligned
}

pub fn compare_states(
    cs: &CombatState,
    java_snapshot: &Value,
    skip_piles: bool,
    context: &ActionContext,
) -> Vec<DiffResult> {
    compare_states_from_snapshots(cs, java_snapshot, java_snapshot, skip_piles, context)
}

pub fn compare_states_from_snapshots(
    cs: &CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
    skip_piles: bool,
    context: &ActionContext,
) -> Vec<DiffResult> {
    let mut diffs = Vec::new();
    let java_player = &truth_snapshot["player"];

    let java_hp = java_player["current_hp"]
        .as_i64()
        .unwrap_or(java_player["hp"].as_i64().unwrap_or(0)) as i32;
    if cs.entities.player.current_hp != java_hp {
        diffs.push(DiffResult {
            field: "player.hp".into(),
            rust_val: cs.entities.player.current_hp.to_string(),
            java_val: java_hp.to_string(),
            category: if context.was_end_turn {
                DiffCategory::ContentGap
            } else {
                DiffCategory::EngineBug
            },
        });
    }

    let java_block = java_player["block"].as_i64().unwrap_or(0) as i32;
    if cs.entities.player.block != java_block {
        diffs.push(DiffResult {
            field: "player.block".into(),
            rust_val: cs.entities.player.block.to_string(),
            java_val: java_block.to_string(),
            category: if context.was_end_turn {
                DiffCategory::ContentGap
            } else {
                DiffCategory::EngineBug
            },
        });
    }

    let java_energy = java_player["energy"].as_u64().unwrap_or(0) as u8;
    if cs.turn.energy != java_energy {
        diffs.push(DiffResult {
            field: "player.energy".into(),
            rust_val: cs.turn.energy.to_string(),
            java_val: java_energy.to_string(),
            category: DiffCategory::EngineBug,
        });
    }

    let truth_monsters = truth_snapshot["monsters"].as_array();
    let observation_monsters = observation_snapshot["monsters"].as_array();
    if let Some(java_ms) = truth_monsters {
        let alignment_monsters = observation_monsters.unwrap_or(java_ms);
        let aligned_indices = align_rust_monsters_to_java(cs, alignment_monsters);
        for (i, jm) in java_ms.iter().enumerate() {
            let Some(rust_idx) = aligned_indices.get(i).and_then(|idx| *idx) else {
                continue;
            };
            let rm = &cs.entities.monsters[rust_idx];
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
        let java_hand_size = truth_snapshot["hand_size"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                truth_snapshot["hand"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            });
        if cs.zones.hand.len() != java_hand_size {
            diffs.push(DiffResult {
                field: "hand_size".into(),
                rust_val: cs.zones.hand.len().to_string(),
                java_val: java_hand_size.to_string(),
                category: DiffCategory::EngineBug,
            });
        }

        let java_discard = truth_snapshot["discard_pile_size"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                truth_snapshot["discard_pile"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            });
        if cs.zones.discard_pile.len() != java_discard {
            diffs.push(DiffResult {
                field: "discard_pile_size".into(),
                rust_val: cs.zones.discard_pile.len().to_string(),
                java_val: java_discard.to_string(),
                category: DiffCategory::EngineBug,
            });
        }

        let java_exhaust = truth_snapshot["exhaust_pile_size"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or_else(|| {
                truth_snapshot["exhaust_pile"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            });
        if cs.zones.exhaust_pile.len() != java_exhaust {
            diffs.push(DiffResult {
                field: "exhaust_pile_size".into(),
                rust_val: cs.zones.exhaust_pile.len().to_string(),
                java_val: java_exhaust.to_string(),
                category: DiffCategory::EngineBug,
            });
        }
    }

    compare_powers(
        &mut diffs,
        "player",
        0,
        &cs.entities.power_db,
        &java_player["powers"],
        context,
    );

    if let Some(java_ms) = truth_monsters {
        let alignment_monsters = observation_monsters.unwrap_or(java_ms);
        let aligned_indices = align_rust_monsters_to_java(cs, alignment_monsters);
        for (i, jm) in java_ms.iter().enumerate() {
            let Some(rust_idx) = aligned_indices.get(i).and_then(|idx| *idx) else {
                continue;
            };

            // Skip power comparison for dead monsters (Java clears them asynchronously after death animations)
            let is_dead = jm["is_gone"].as_bool().unwrap_or(false)
                || jm["current_hp"].as_i64().unwrap_or(1) <= 0;
            if is_dead {
                continue;
            }

            let entity_id = cs.entities.monsters[rust_idx].id;
            compare_powers(
                &mut diffs,
                &format!("monster[{}]", i),
                entity_id,
                &cs.entities.power_db,
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
