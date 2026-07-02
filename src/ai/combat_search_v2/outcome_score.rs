use super::*;
use crate::content::monsters::EnemyId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CombatOutcomeScore {
    terminal_rank: i32,
    run_hygiene: i32,
    final_hp: i32,
    potion_conservation: i32,
    faster_turns: i32,
    fewer_cards_played: i32,
    enemy_progress: i32,
    shorter_line: i32,
}

impl CombatOutcomeScore {
    pub(super) fn from_node(node: &SearchNode) -> Self {
        Self {
            terminal_rank: terminal_rank(terminal_label(&node.engine, &node.combat)),
            run_hygiene: -external_burden_count(&node.combat),
            final_hp: node.combat.entities.player.current_hp,
            potion_conservation: -((node.potions_used + node.potions_discarded) as i32),
            faster_turns: -(node.combat.turn.turn_count as i32),
            fewer_cards_played: -(node.cards_played as i32),
            enemy_progress: -total_living_enemy_hp(&node.combat),
            shorter_line: -(node.actions.len() as i32),
        }
    }
}

impl Ord for CombatOutcomeScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.terminal_rank
            .cmp(&other.terminal_rank)
            .then_with(|| self.run_hygiene.cmp(&other.run_hygiene))
            .then_with(|| self.final_hp.cmp(&other.final_hp))
            .then_with(|| self.potion_conservation.cmp(&other.potion_conservation))
            .then_with(|| self.faster_turns.cmp(&other.faster_turns))
            .then_with(|| self.fewer_cards_played.cmp(&other.fewer_cards_played))
            .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
            .then_with(|| self.shorter_line.cmp(&other.shorter_line))
    }
}

impl PartialOrd for CombatOutcomeScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn external_burden_count(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            monster.monster_type == EnemyId::WrithingMass as usize
                && monster.writhing_mass.used_mega_debuff
        })
        .count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::blank_test_combat;

    #[test]
    fn outcome_score_prefers_higher_final_hp_for_same_terminal() {
        let low = SearchNode::test_node_with_hp(10);
        let high = SearchNode::test_node_with_hp(20);

        assert!(CombatOutcomeScore::from_node(&high) > CombatOutcomeScore::from_node(&low));
    }

    impl SearchNode {
        fn test_node_with_hp(hp: i32) -> Self {
            let mut combat = blank_test_combat();
            combat.entities.player.current_hp = hp;
            SearchNode {
                engine: EngineState::CombatPlayerTurn,
                combat,
                actions: Vec::new(),
                turn_prefix: TurnPrefixState::default(),
                initial_hp: 80,
                potions_used: 0,
                potions_discarded: 0,
                cards_played: 0,
                potion_tactical_priority: 0,
                last_turn_branch_priority: 0,
                action_prior_score: None,
                rollout_estimate: RolloutNodeEstimate::unevaluated(),
            }
        }
    }
}
