use super::super::*;

pub(super) fn accepted_complete_win(
    node: &SearchNode,
    config: &CombatSearchV2Config,
    initial_external_burden_count: i32,
) -> bool {
    if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Win {
        return false;
    }
    let hp_loss = (node.initial_hp - node.combat.entities.player.current_hp).max(0) as u32;
    let has_new_external_burden = super::super::outcome_score::external_burden_count(&node.combat)
        > initial_external_burden_count;
    match config.satisfaction {
        CombatSearchV2Satisfaction::BudgetOrExhaustion => false,
        CombatSearchV2Satisfaction::ZeroLossOrBudget => {
            hp_loss == 0
                && !super::super::external_payoff::has_external_payoff_opportunity(&node.combat)
        }
        CombatSearchV2Satisfaction::FirstCompleteWin => true,
        CombatSearchV2Satisfaction::HpLossAtMost(limit) => hp_loss <= limit,
        CombatSearchV2Satisfaction::FirstCompleteWinWithoutNewExternalBurden => {
            !has_new_external_burden
        }
        CombatSearchV2Satisfaction::HpLossAtMostWithoutNewExternalBurden(limit) => {
            hp_loss <= limit && !has_new_external_burden
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::MetaChange;
    use crate::state::core::RunResult;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn clean_satisfaction_does_not_close_session_on_burdened_win() {
        let mut combat = blank_test_combat();
        let mut mass = test_monster(EnemyId::WrithingMass);
        mass.writhing_mass.used_mega_debuff = true;
        combat.entities.monsters = vec![mass];
        combat
            .meta
            .meta_changes
            .push(MetaChange::AddCardToMasterDeck(CardId::Parasite));
        let node = SearchNode::root(EngineState::GameOver(RunResult::Victory), combat);
        let mut config = CombatSearchV2Config {
            satisfaction: CombatSearchV2Satisfaction::FirstCompleteWinWithoutNewExternalBurden,
            ..CombatSearchV2Config::default()
        };

        assert!(!accepted_complete_win(&node, &config, 0));

        config.satisfaction = CombatSearchV2Satisfaction::FirstCompleteWin;
        assert!(accepted_complete_win(&node, &config, 0));
    }

    #[test]
    fn clean_satisfaction_uses_new_unblocked_burden_not_enemy_history() {
        let mut combat = blank_test_combat();
        let mut mass = test_monster(EnemyId::WrithingMass);
        mass.writhing_mass.used_mega_debuff = true;
        combat.entities.monsters = vec![mass];
        combat
            .meta
            .meta_changes
            .push(MetaChange::AddCardToMasterDeck(CardId::Parasite));
        combat
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::Omamori));
        let node = SearchNode::root(EngineState::GameOver(RunResult::Victory), combat);
        let config = CombatSearchV2Config {
            satisfaction: CombatSearchV2Satisfaction::FirstCompleteWinWithoutNewExternalBurden,
            ..CombatSearchV2Config::default()
        };

        assert!(accepted_complete_win(&node, &config, 0));
        assert!(accepted_complete_win(&node, &config, 1));
    }
}
