use sts_simulator::ai::combat_search_v2::CombatSearchV2DecisionCandidateReport;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::sim::combat::{CombatPosition, CombatStepResult};
use sts_simulator::state::core::ClientInput;

use super::types::RootActionRoleDuelTransition;

pub(super) fn root_transition(
    position: &CombatPosition,
    step: &CombatStepResult,
    candidate: &CombatSearchV2DecisionCandidateReport,
) -> RootActionRoleDuelTransition {
    RootActionRoleDuelTransition {
        status: step_status(step),
        terminal: step.terminal,
        engine_steps: step.engine_steps,
        player_hp: position.combat.entities.player.current_hp,
        player_block: position.combat.entities.player.block,
        energy: position.combat.turn.energy,
        living_enemy_count: position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| !monster.is_dead_or_escaped())
            .count(),
        cultists_alive: position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| {
                !monster.is_dead_or_escaped()
                    && EnemyId::from_id(monster.monster_type) == Some(EnemyId::Cultist)
            })
            .count(),
        total_enemy_hp: candidate.one_step.total_enemy_hp,
        visible_incoming_damage: candidate.one_step.visible_incoming_damage,
        survival_margin: candidate.one_step.survival_margin,
    }
}

pub(super) fn root_potions_used(input: &ClientInput) -> u32 {
    if matches!(input, ClientInput::UsePotion { .. }) {
        1
    } else {
        0
    }
}

fn step_status(step: &CombatStepResult) -> &'static str {
    if step.timed_out {
        "timed_out"
    } else if step.truncated {
        "engine_step_limit"
    } else if !step.alive {
        "player_dead"
    } else {
        "stable"
    }
}
