use super::enemy_mechanics_profile::{enemy_mechanics_profile, EnemyMechanicsProfileV1};
use super::enemy_phase_value::{enemy_phase_value, EnemyPhaseValueV1};
use super::pending_choice_fanout::pending_choice_fanout;
use super::pressure_value::{combat_pressure_value, CombatPressureValueV1};
use super::types::CombatSearchV2PhaseProfileReport;
use crate::runtime::combat::CombatState;
use crate::state::core::EngineState;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatSearchPhaseProfileV1 {
    pub(super) enemy_phase: EnemyPhaseValueV1,
    pub(super) enemy_mechanics: EnemyMechanicsProfileV1,
    pub(super) pressure: CombatPressureValueV1,
    pub(super) pending_choice: PendingChoicePhaseProfileV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PendingChoicePhaseProfileV1 {
    pub(super) present: bool,
    pub(super) kind: Option<PendingChoicePhaseKind>,
    pub(super) candidate_count: usize,
    pub(super) estimated_action_fanout: usize,
    pub(super) high_fanout: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PendingChoicePhaseKind {
    HandSelect,
    GridSelect,
    DiscoverySelect,
    ScrySelect,
    CardRewardSelect,
    ForeignInfluenceSelect,
    ChooseOneSelect,
    StanceChoice,
}

pub(super) fn combat_search_phase_profile(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatSearchPhaseProfileV1 {
    CombatSearchPhaseProfileV1 {
        enemy_phase: enemy_phase_value(combat),
        enemy_mechanics: enemy_mechanics_profile(combat),
        pressure: combat_pressure_value(combat),
        pending_choice: pending_choice_phase_profile(engine),
    }
}

pub(super) fn combat_search_phase_profile_report(
    profile: CombatSearchPhaseProfileV1,
) -> CombatSearchV2PhaseProfileReport {
    CombatSearchV2PhaseProfileReport {
        profiling_policy: "typed_phase_profile_for_value_and_rollout_no_prune_no_terminal_claim",
        special_enemy_phase_count: profile.special_enemy_phase_count(),
        split_pending_count: profile.enemy_phase.split_pending_count,
        split_debt_hp: profile.enemy_phase.split_debt_hp,
        guardian_mode_shift_pending_count: profile
            .enemy_mechanics
            .guardian_mode_shift_pending_count,
        guardian_defensive_count: profile.enemy_mechanics.guardian_defensive_count,
        lagavulin_sleeping_count: profile.enemy_mechanics.lagavulin_sleeping_count,
        lagavulin_waking_count: profile.enemy_mechanics.lagavulin_waking_count,
        pending_choice_present: profile.pending_choice.present,
        pending_choice_kind: profile
            .pending_choice
            .kind
            .map(PendingChoicePhaseKind::label),
        pending_choice_candidate_count: profile.pending_choice.candidate_count,
        pending_choice_estimated_action_fanout: profile.pending_choice.estimated_action_fanout,
        high_fanout_pending_choice: profile.pending_choice.high_fanout,
        notes: vec![
            "phase profile is a read-only state classifier used by value and rollout",
            "high-fanout pending choices stop rollout estimates but do not prune main search",
            "enemy phase counts are structural signals, not standalone proof of line quality",
        ],
    }
}

impl CombatSearchPhaseProfileV1 {
    pub(super) fn special_enemy_phase_count(self) -> usize {
        self.enemy_phase
            .split_pending_count
            .saturating_add(self.enemy_mechanics.guardian_mode_shift_pending_count)
            .saturating_add(self.enemy_mechanics.guardian_defensive_count)
            .saturating_add(self.enemy_mechanics.lagavulin_sleeping_count)
            .saturating_add(self.enemy_mechanics.lagavulin_waking_count)
    }
}

impl PendingChoicePhaseKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            PendingChoicePhaseKind::HandSelect => "hand_select",
            PendingChoicePhaseKind::GridSelect => "grid_select",
            PendingChoicePhaseKind::DiscoverySelect => "discovery_select",
            PendingChoicePhaseKind::ScrySelect => "scry_select",
            PendingChoicePhaseKind::CardRewardSelect => "card_reward_select",
            PendingChoicePhaseKind::ForeignInfluenceSelect => "foreign_influence_select",
            PendingChoicePhaseKind::ChooseOneSelect => "choose_one_select",
            PendingChoicePhaseKind::StanceChoice => "stance_choice",
        }
    }
}

fn pending_choice_phase_profile(engine: &EngineState) -> PendingChoicePhaseProfileV1 {
    let EngineState::PendingChoice(choice) = engine else {
        return PendingChoicePhaseProfileV1::default();
    };

    let kind = match choice {
        crate::state::core::PendingChoice::HandSelect { .. } => PendingChoicePhaseKind::HandSelect,
        crate::state::core::PendingChoice::GridSelect { .. } => PendingChoicePhaseKind::GridSelect,
        crate::state::core::PendingChoice::DiscoverySelect(_) => {
            PendingChoicePhaseKind::DiscoverySelect
        }
        crate::state::core::PendingChoice::ScrySelect { .. } => PendingChoicePhaseKind::ScrySelect,
        crate::state::core::PendingChoice::CardRewardSelect { .. } => {
            PendingChoicePhaseKind::CardRewardSelect
        }
        crate::state::core::PendingChoice::ForeignInfluenceSelect { .. } => {
            PendingChoicePhaseKind::ForeignInfluenceSelect
        }
        crate::state::core::PendingChoice::ChooseOneSelect { .. } => {
            PendingChoicePhaseKind::ChooseOneSelect
        }
        crate::state::core::PendingChoice::StanceChoice => PendingChoicePhaseKind::StanceChoice,
    };
    let fanout = pending_choice_fanout(choice);

    PendingChoicePhaseProfileV1 {
        present: true,
        kind: Some(kind),
        candidate_count: fanout.candidate_count,
        estimated_action_fanout: fanout.estimated_action_fanout,
        high_fanout: fanout.high_fanout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn phase_profile_reports_split_phase_and_pressure() {
        let mut combat = blank_test_combat();
        let mut slime = test_monster(EnemyId::AcidSlimeL);
        slime.id = 7;
        slime.current_hp = 30;
        slime.max_hp = 65;
        slime.set_planned_move_id(3);
        combat.entities.monsters = vec![slime];
        combat.entities.power_db.insert(
            7,
            vec![Power {
                power_type: crate::content::powers::PowerId::Split,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        let profile = combat_search_phase_profile(&EngineState::CombatPlayerTurn, &combat);

        assert_eq!(profile.enemy_phase.split_pending_count, 1);
        assert_eq!(profile.enemy_phase.split_debt_hp, 30);
        assert_eq!(profile.pressure.survival_margin, 80);
        assert_eq!(profile.special_enemy_phase_count(), 1);
    }

    #[test]
    fn phase_profile_marks_large_scry_as_high_fanout_rollout_boundary() {
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
            cards: vec![crate::content::cards::CardId::Strike; 7],
            card_uuids: (0..7).collect(),
        });
        let combat = blank_test_combat();

        let profile = combat_search_phase_profile(&engine, &combat);

        assert_eq!(
            profile.pending_choice.kind,
            Some(PendingChoicePhaseKind::ScrySelect)
        );
        assert_eq!(profile.pending_choice.candidate_count, 7);
        assert_eq!(profile.pending_choice.estimated_action_fanout, 128);
        assert!(profile.pending_choice.high_fanout);
    }

    #[test]
    fn phase_profile_keeps_small_discovery_rollout_eligible() {
        let engine =
            EngineState::PendingChoice(crate::state::core::PendingChoice::DiscoverySelect(
                crate::state::core::DiscoveryChoiceState {
                    cards: vec![
                        crate::content::cards::CardId::Strike,
                        crate::content::cards::CardId::Defend,
                        crate::content::cards::CardId::Bash,
                    ],
                    colorless: false,
                    card_type: None,
                    amount: 1,
                    can_skip: true,
                },
            ));
        let combat = blank_test_combat();

        let profile = combat_search_phase_profile(&engine, &combat);

        assert_eq!(profile.pending_choice.estimated_action_fanout, 4);
        assert!(!profile.pending_choice.high_fanout);
    }
}
