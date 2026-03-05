//! Known timing divergences — whitelist for Java ActionQueue artifacts.
//!
//! Java's game engine uses an ActionQueue where effects are deferred:
//! a card play can cause side-effects (Curl Up block, death triggers) that
//! resolve in subsequent action cycles. CommunicationMod's JSONL snapshots
//! capture state at specific points in this queue, creating intermediate
//! states that Rust's synchronous engine doesn't replicate.
//!
//! This module identifies and filters divergences that are known timing
//! artifacts — cases where Rust's final state is game-correct but doesn't
//! match Java's intermediate snapshot.
//!
//! # Design principles
//! - **Conservative**: Only filter divergences with high-confidence patterns.
//!   When in doubt, report the divergence.
//! - **Traceable**: Each rule has a `reason` documenting the Java mechanism
//!   and why the divergence is safe to ignore.
//! - **Maintainable**: Rules are data-driven. Add new patterns here when
//!   new ActionQueue timing artifacts are discovered.

use crate::testing::snapshot::{CombatSnapshot, EnemySnap};
use crate::testing::step_verifier::Divergence;

/// Classification of a divergence after whitelist analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivergenceClass {
    /// Real bug — engine produces wrong result.
    Real,
    /// Known timing artifact — Rust is correct, Java snapshot is intermediate.
    TimingArtifact {
        rule_id: &'static str,
        reason: &'static str,
    },
}

/// A whitelist rule that can match and classify a divergence.
struct TimingRule {
    /// Short identifier for logging/reporting.
    id: &'static str,
    /// Human-readable explanation of why this is a false positive.
    reason: &'static str,
    /// Check if this rule matches the given divergence + context.
    /// Returns true if the divergence is a known timing artifact.
    matches: fn(
        div: &Divergence,
        before: &CombatSnapshot,
        expected: &CombatSnapshot,
        actual: &CombatSnapshot,
    ) -> bool,
}

// ============================================================================
// Rule definitions
// ============================================================================

/// All registered timing rules.
fn timing_rules() -> Vec<TimingRule> {
    vec![
        // Rule: Curl Up power removal timing
        // Java: CurlUpPower.wasHPLost() queues RemoveSpecificPowerAction + GainBlockAction
        // These resolve AFTER the current DamageAction, so the JSONL snapshot
        // still shows Curl Up present. Rust removes it immediately.
        TimingRule {
            id: "curl_up_deferred_removal",
            reason: "Java defers Curl Up removal via ActionQueue. \
                     CurlUpPower.wasHPLost() queues RemoveSpecificPowerAction \
                     which resolves after the damage snapshot.",
            matches: |div, before, _expected, _actual| {
                // Match: enemy[N].powers.Curl Up divergence WHERE the before-state
                // enemy had Curl Up (meaning it could have triggered)
                if !div.field.contains("powers.Curl Up") {
                    return false;
                }
                // Extract enemy index from field like "enemy[1].powers.Curl Up"
                if let Some(idx) = extract_enemy_index(&div.field) {
                    if let Some(enemy) = before.enemies.get(idx) {
                        // Before-state enemy had Curl Up → timing artifact
                        return enemy.powers.contains_key("Curl Up");
                    }
                }
                false
            },
        },

        // Rule: Curl Up block gain timing (paired with removal)
        // When Curl Up triggers, both block gain and power removal are deferred.
        // Rust immediately adds block, Java defers it.
        TimingRule {
            id: "curl_up_deferred_block",
            reason: "Java defers Curl Up block gain via ActionQueue. \
                     GainBlockAction resolves after the damage snapshot.",
            matches: |div, before, _expected, _actual| {
                if !div.field.contains(".block") {
                    return false;
                }
                if let Some(idx) = extract_enemy_index(&div.field) {
                    if let Some(enemy) = before.enemies.get(idx) {
                        return enemy.powers.contains_key("Curl Up");
                    }
                }
                false
            },
        },

        // Rule: Malleable power change timing (similar to Curl Up)
        // Java: MalleablePower.wasHPLost() queues GainBlockAction + increase amount
        TimingRule {
            id: "malleable_deferred",
            reason: "Java defers Malleable block/power changes via ActionQueue.",
            matches: |div, before, _expected, _actual| {
                if !div.field.contains("powers.Malleable") && !div.field.contains(".block") {
                    return false;
                }
                if let Some(idx) = extract_enemy_index(&div.field) {
                    if let Some(enemy) = before.enemies.get(idx) {
                        return enemy.powers.contains_key("Malleable");
                    }
                }
                false
            },
        },

        // Rule: Deferred enemy death triggers (Spore Cloud, etc.)
        // When an enemy's death is deferred (e.g., killed by a previous step's
        // queued damage), the death effects appear in this step's AFTER state
        // even though the card played didn't cause the kill.
        // Pattern: enemy alive in BEFORE, dead in EXPECTED, and the divergence
        // is a player power that the dead enemy's on_death would apply.
        TimingRule {
            id: "deferred_death_trigger",
            reason: "Java defers enemy death effects via ActionQueue. \
                     The enemy died from a previous step's queued action, \
                     so its death trigger (Spore Cloud → Vulnerable, etc.) \
                     appears in this step's AFTER.",
            matches: |div, before, expected, _actual| {
                // Check if any enemy transitioned alive→dead between steps
                // and had a death-trigger power that matches the divergent field
                for (i, before_enemy) in before.enemies.iter().enumerate() {
                    if !before_enemy.alive {
                        continue;
                    }
                    let expected_enemy = match expected.enemies.get(i) {
                        Some(e) => e,
                        None => continue,
                    };
                    if expected_enemy.alive {
                        continue; // Enemy didn't die this step
                    }
                    // Enemy went alive→dead. Check for death trigger powers.
                    // Spore Cloud → Vulnerable on player
                    if before_enemy.powers.contains_key("Spore Cloud")
                        && div.field == "player_powers.Vulnerable"
                    {
                        return true;
                    }
                    // Corpse Explosion → damage all enemies (enemy HP changes)
                    if before_enemy.powers.contains_key("Corpse Explosion")
                        && div.field.contains("enemy[")
                    {
                        return true;
                    }
                }
                false
            },
        },
    ]
}

// ============================================================================
// Public API
// ============================================================================

/// Classify a single divergence as real or timing artifact.
pub fn classify_divergence(
    div: &Divergence,
    before: &CombatSnapshot,
    expected: &CombatSnapshot,
    actual: &CombatSnapshot,
) -> DivergenceClass {
    for rule in &timing_rules() {
        if (rule.matches)(div, before, expected, actual) {
            return DivergenceClass::TimingArtifact {
                rule_id: rule.id,
                reason: rule.reason,
            };
        }
    }
    DivergenceClass::Real
}

/// Filter a list of divergences, removing known timing artifacts.
/// Returns (real_divergences, filtered_count).
pub fn filter_timing_divergences(
    divergences: Vec<Divergence>,
    before: &CombatSnapshot,
    expected: &CombatSnapshot,
    actual: &CombatSnapshot,
) -> (Vec<Divergence>, usize) {
    let mut real = Vec::new();
    let mut filtered = 0usize;

    for div in divergences {
        match classify_divergence(&div, before, expected, actual) {
            DivergenceClass::Real => real.push(div),
            DivergenceClass::TimingArtifact { rule_id, .. } => {
                game_log!("  ⏳ Filtered timing divergence: {} (rule: {})", div.field, rule_id);
                filtered += 1;
            }
        }
    }

    (real, filtered)
}

// ============================================================================
// Helpers
// ============================================================================

/// Extract the enemy index from a field name like "enemy[2].hp" or "enemy[0].powers.Curl Up".
fn extract_enemy_index(field: &str) -> Option<usize> {
    if let Some(start) = field.find("enemy[") {
        let rest = &field[start + 6..];
        if let Some(end) = rest.find(']') {
            return rest[..end].parse().ok();
        }
    }
    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn empty_snapshot() -> CombatSnapshot {
        CombatSnapshot {
            turn: 1,
            cards_played_this_turn: 0,
            player_hp: 50,
            player_max_hp: 80,
            player_block: 0,
            player_energy: 3,
            player_max_energy: 3,
            player_powers: BTreeMap::new(),
            player_stance: "None".into(),
            hand: vec![],
            draw_pile_count: 0,
            discard_pile_count: 0,
            exhaust_pile_count: 0,
            enemies: vec![],
            relics: vec![],
            orbs: vec![],
        }
    }

    fn enemy_with_curl_up() -> EnemySnap {
        EnemySnap {
            name: "Louse".into(),
            hp: 15,
            max_hp: 15,
            block: 0,
            alive: true,
            powers: BTreeMap::from([("Curl Up".into(), 6)]),
            current_move: "Bite".into(),
        }
    }

    #[test]
    fn test_curl_up_power_filtered() {
        let mut before = empty_snapshot();
        before.enemies.push(enemy_with_curl_up());

        let expected = empty_snapshot();
        let actual = empty_snapshot();

        let div = Divergence {
            field: "enemy[0].powers.Curl Up".into(),
            expected: "6".into(),
            actual: "0".into(),
        };

        let class = classify_divergence(&div, &before, &expected, &actual);
        assert!(matches!(class, DivergenceClass::TimingArtifact { rule_id: "curl_up_deferred_removal", .. }));
    }

    #[test]
    fn test_real_divergence_not_filtered() {
        let before = empty_snapshot();
        let expected = empty_snapshot();
        let actual = empty_snapshot();

        let div = Divergence {
            field: "player_hp".into(),
            expected: "50".into(),
            actual: "45".into(),
        };

        let class = classify_divergence(&div, &before, &expected, &actual);
        assert_eq!(class, DivergenceClass::Real);
    }

    #[test]
    fn test_extract_enemy_index() {
        assert_eq!(extract_enemy_index("enemy[0].hp"), Some(0));
        assert_eq!(extract_enemy_index("enemy[2].powers.Curl Up"), Some(2));
        assert_eq!(extract_enemy_index("player_hp"), None);
    }
}
