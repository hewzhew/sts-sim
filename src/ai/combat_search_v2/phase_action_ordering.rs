use super::enemy_phase_transition::EnemyPhaseTransitionHint;
use super::phase_profile::CombatSearchPhaseProfileV1;
use crate::content::cards::CardType;

// Kept smaller than the main role gaps in action_priority; phase facts nudge nearby
// ordering decisions without turning this module into an alternate policy.
const PHASE_ROLE_ADJUSTMENT: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PhaseActionOrderingFacts {
    pub(super) card_type: CardType,
    pub(super) block: i32,
    pub(super) mitigation: i32,
    pub(super) phase_transition: EnemyPhaseTransitionHint,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PhaseActionOrderingHint {
    pub(super) role_rank_adjustment: i32,
    pub(super) phase_setup: i32,
    pub(super) phase_survival: i32,
    pub(super) phase_transition_safety: i32,
}

pub(super) fn phase_action_ordering_hint(
    profile: CombatSearchPhaseProfileV1,
    facts: PhaseActionOrderingFacts,
) -> PhaseActionOrderingHint {
    let mut hint = PhaseActionOrderingHint {
        phase_transition_safety: -facts.phase_transition.ordering_risk_score(),
        ..PhaseActionOrderingHint::default()
    };

    if profile.enemy_mechanics.lagavulin_sleeping_count > 0 {
        apply_lagavulin_sleep_hint(&mut hint, facts);
    }
    if profile.enemy_mechanics.guardian_defensive_count > 0
        || profile.enemy_mechanics.guardian_mode_shift_pending_count > 0
    {
        hint.phase_survival = facts.block.saturating_add(facts.mitigation);
    }

    hint
}

impl PhaseActionOrderingHint {
    pub(super) fn has_signal(self) -> bool {
        self.role_rank_adjustment != 0
            || self.phase_setup != 0
            || self.phase_survival != 0
            || self.phase_transition_safety != 0
    }
}

fn apply_lagavulin_sleep_hint(hint: &mut PhaseActionOrderingHint, facts: PhaseActionOrderingFacts) {
    if facts.phase_transition.lagavulin_wake_risk_count > 0 {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_sub(PHASE_ROLE_ADJUSTMENT);
        hint.phase_setup -= 1;
    } else if facts.card_type == CardType::Power {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_add(PHASE_ROLE_ADJUSTMENT);
        hint.phase_setup += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::super::phase_profile::combat_search_phase_profile;
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn lagavulin_sleep_phase_penalizes_wake_damage_and_rewards_power_setup() {
        let mut combat = blank_test_combat();
        let mut lagavulin = test_monster(EnemyId::Lagavulin);
        lagavulin.id = 1;
        lagavulin.lagavulin.is_out = false;
        combat.entities.monsters = vec![lagavulin];
        combat.zones.hand = vec![
            CombatCard::new(crate::content::cards::CardId::Strike, 10),
            CombatCard::new(crate::content::cards::CardId::Inflame, 11),
        ];
        let profile = combat_search_phase_profile(&EngineState::CombatPlayerTurn, &combat);

        let wake_hint = phase_action_ordering_hint(
            profile,
            PhaseActionOrderingFacts {
                card_type: CardType::Attack,
                block: 0,
                mitigation: 0,
                phase_transition:
                    super::super::enemy_phase_transition::enemy_phase_transition_hint_for_input(
                        &combat,
                        &ClientInput::PlayCard {
                            card_index: 0,
                            target: Some(1),
                        },
                    ),
            },
        );
        let setup_hint = phase_action_ordering_hint(
            profile,
            PhaseActionOrderingFacts {
                card_type: CardType::Power,
                block: 0,
                mitigation: 0,
                phase_transition:
                    super::super::enemy_phase_transition::enemy_phase_transition_hint_for_input(
                        &combat,
                        &ClientInput::PlayCard {
                            card_index: 1,
                            target: None,
                        },
                    ),
            },
        );

        assert!(wake_hint.role_rank_adjustment < 0);
        assert!(wake_hint.phase_transition_safety < 0);
        assert!(setup_hint.role_rank_adjustment > 0);
        assert!(setup_hint.phase_setup > wake_hint.phase_setup);
    }

    #[test]
    fn guardian_defensive_phase_records_survival_tiebreak() {
        let mut combat = blank_test_combat();
        let mut guardian = test_monster(EnemyId::TheGuardian);
        guardian.guardian.is_open = false;
        combat.entities.monsters = vec![guardian];
        let profile = combat_search_phase_profile(&EngineState::CombatPlayerTurn, &combat);

        let hint = phase_action_ordering_hint(
            profile,
            PhaseActionOrderingFacts {
                card_type: CardType::Skill,
                block: 8,
                mitigation: 3,
                phase_transition: EnemyPhaseTransitionHint::default(),
            },
        );

        assert_eq!(hint.phase_survival, 11);
        assert!(hint.has_signal());
    }
}
