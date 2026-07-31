use super::*;

/// Projects Bronze Automaton's opening Artifact barrier into a typed
/// mitigation-setup plan.
///
/// Weak and enemy-Strength-down resources have unusually high value against
/// Hyper Beam, but Artifact consumes those debuffs before they take effect.
/// This plan therefore exposes exact progress toward opening the boss without
/// prescribing a particular strip card, potion, or action sequence. Stasis
/// timing remains a separate future milestone rather than being guessed here.
pub fn bronze_automaton_combat_plan_v1(
    position: &CombatPosition,
) -> Option<CombatPlanProjectionV1> {
    if combat_terminal(&position.engine, &position.combat) != CombatTerminal::Unresolved {
        return None;
    }
    let combat = &position.combat;
    let automaton = bronze_automaton(combat)?;
    if !automaton.is_alive_for_action() {
        return None;
    }

    let mut resources = combat_plan_resources_v1(combat);
    resources.remaining_artifact_sensitive_mitigation = count_live_cards(combat, |card| {
        card_supplies_artifact_sensitive_mitigation(card)
    });
    let mut envelope = combat_plan_state_envelope_v1(combat);
    envelope.priority_target_hp_with_block = Some(
        automaton
            .current_hp
            .max(0)
            .saturating_add(automaton.block.max(0)),
    );
    envelope.priority_target_artifact =
        Some(combat.get_power(automaton.id, PowerId::Artifact).max(0));

    let artifact = envelope.priority_target_artifact.unwrap_or_default();
    let stage = if artifact > 0 && resources.remaining_artifact_sensitive_mitigation > 0 {
        CombatPlanStageV1::ExposeAttackMitigationTarget
    } else {
        CombatPlanStageV1::ConvertToLethal
    };
    let (next_milestone, primary) = match stage {
        CombatPlanStageV1::ExposeAttackMitigationTarget => (
            CombatPlanMilestoneV1::AttackMitigationTargetExposed,
            CombatPlanObligationV1::ExposeAttackMitigationTarget {
                protected_attackers: 1,
            },
        ),
        CombatPlanStageV1::ConvertToLethal => (
            CombatPlanMilestoneV1::EncounterDefeated,
            CombatPlanObligationV1::ConvertPreparedEngineToLethal,
        ),
        _ => unreachable!("Bronze Automaton plan uses only mitigation setup and conversion"),
    };

    Some(CombatPlanProjectionV1 {
        schema: COMBAT_PLAN_SCHEMA_V1.to_owned(),
        plan: CombatPlanIdV1::BronzeAutomatonControl,
        stage,
        next_milestone,
        primary,
        supporting: Vec::new(),
        resources,
        envelope,
    })
}

pub fn bronze_automaton_plan_transition_v1(
    before: &CombatPosition,
    after: &CombatPosition,
) -> Option<CombatPlanTransitionV1> {
    let before_plan = bronze_automaton_combat_plan_v1(before)?;
    if combat_terminal(&after.engine, &after.combat) == CombatTerminal::Win {
        return Some(CombatPlanTransitionV1 {
            schema: COMBAT_PLAN_SCHEMA_V1.to_owned(),
            plan: before_plan.plan,
            before_stage: before_plan.stage,
            after_stage: None,
            completed_milestones: vec![CombatPlanMilestoneV1::EncounterDefeated],
            events: Vec::new(),
            resources_before: before_plan.resources,
            resources_after: None,
            envelope_before: before_plan.envelope,
            envelope_after: None,
        });
    }

    let after_plan = bronze_automaton_combat_plan_v1(after);
    let mut completed_milestones = Vec::new();
    if before_plan
        .envelope
        .priority_target_artifact
        .is_some_and(|artifact| artifact > 0)
        && after_plan.as_ref().is_some_and(|plan| {
            plan.envelope
                .priority_target_artifact
                .is_some_and(|artifact| artifact <= 0)
        })
    {
        completed_milestones.push(CombatPlanMilestoneV1::AttackMitigationTargetExposed);
    }
    let events = after_plan
        .as_ref()
        .map(|after_plan| combat_plan_transition_events_v1(&before_plan, after_plan))
        .unwrap_or_default();
    Some(CombatPlanTransitionV1 {
        schema: COMBAT_PLAN_SCHEMA_V1.to_owned(),
        plan: before_plan.plan,
        before_stage: before_plan.stage,
        after_stage: after_plan.as_ref().map(|plan| plan.stage),
        completed_milestones,
        events,
        resources_before: before_plan.resources,
        resources_after: after_plan.as_ref().map(|plan| plan.resources),
        envelope_before: before_plan.envelope,
        envelope_after: after_plan.as_ref().map(|plan| plan.envelope),
    })
}

pub(super) fn bronze_automaton_state_guide_rank_v1(
    _position: &CombatPosition,
    plan: &CombatPlanProjectionV1,
) -> Option<CombatPlanStateGuideRankV1> {
    if plan.plan != CombatPlanIdV1::BronzeAutomatonControl {
        return None;
    }
    let artifact = plan.envelope.priority_target_artifact.unwrap_or_default();
    let mitigation_supply = plan.resources.remaining_artifact_sensitive_mitigation as i32;
    let hp_with_block = plan
        .envelope
        .player_hp
        .saturating_add(plan.envelope.player_block);
    let boss_hp_with_block = plan
        .envelope
        .priority_target_hp_with_block
        .unwrap_or_default();

    // This lane is deliberately categorical. A fully exposed boss outranks an
    // opening in progress; while still protected, preserving the payoff
    // supply precedes removing another Artifact stack. If the supply has been
    // lost, this guide becomes a weak fallback and ordinary survival/progress
    // lanes remain independent.
    let components = if artifact <= 0 {
        vec![
            2,
            0,
            0,
            i32::from(plan.envelope.visible_damage_margin >= 0),
            plan.envelope.visible_damage_margin,
            hp_with_block,
            -boss_hp_with_block,
        ]
    } else if mitigation_supply > 0 {
        vec![
            1,
            mitigation_supply,
            -artifact,
            i32::from(plan.envelope.visible_damage_margin >= 0),
            plan.envelope.visible_damage_margin,
            hp_with_block,
            -boss_hp_with_block,
        ]
    } else {
        vec![
            0,
            0,
            -artifact,
            i32::from(plan.envelope.visible_damage_margin >= 0),
            plan.envelope.visible_damage_margin,
            hp_with_block,
            -boss_hp_with_block,
        ]
    };
    Some(CombatPlanStateGuideRankV1 { components })
}

pub(super) fn bronze_automaton_action_timing_v1(
    before_position: &CombatPosition,
    after_position: &CombatPosition,
) -> CombatPlanActionTimingV1 {
    let Some(before) = bronze_automaton_combat_plan_v1(before_position) else {
        return CombatPlanActionTimingV1::Neutral;
    };
    if before.stage != CombatPlanStageV1::ExposeAttackMitigationTarget {
        return CombatPlanActionTimingV1::Neutral;
    }
    let Some(after) = bronze_automaton_combat_plan_v1(after_position) else {
        return CombatPlanActionTimingV1::Neutral;
    };
    let before_artifact = before.envelope.priority_target_artifact.unwrap_or_default();
    let after_artifact = after.envelope.priority_target_artifact.unwrap_or_default();
    if after_artifact >= before_artifact {
        return CombatPlanActionTimingV1::Neutral;
    }
    if after.resources.remaining_artifact_sensitive_mitigation
        < before.resources.remaining_artifact_sensitive_mitigation
    {
        CombatPlanActionTimingV1::Defer(
            CombatPlanActionDeferralV1::PreserveArtifactSensitiveMitigationUntilTargetExposed,
        )
    } else {
        CombatPlanActionTimingV1::PreferNow
    }
}

fn bronze_automaton(combat: &CombatState) -> Option<&MonsterEntity> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| enemy_id(monster) == Some(EnemyId::BronzeAutomaton) && !monster.is_escaped)
}

fn card_supplies_artifact_sensitive_mitigation(card: &CombatCard) -> bool {
    card_definition_with_upgrades(card.id, card.upgrades)
        .play_effects
        .iter()
        .any(|effect| {
            matches!(
                effect,
                PlayEffect::Provide(
                    Mechanic::Weak
                        | Mechanic::EnemyStrengthDown
                        | Mechanic::TemporaryEnemyStrengthDown
                )
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_core::content::powers::store;
    use sts_core::runtime::combat::{Power, PowerPayload};
    use sts_core::state::core::EngineState;
    use sts_core::test_support::{blank_test_combat, test_monster};

    fn bronze_position(artifact: i32) -> CombatPosition {
        let mut combat = blank_test_combat();
        let mut automaton = test_monster(EnemyId::BronzeAutomaton);
        automaton.id = 10;
        automaton.current_hp = 300;
        automaton.max_hp = 300;
        combat.entities.monsters.push(automaton);
        store::set_powers_for(
            &mut combat,
            10,
            vec![Power {
                power_type: PowerId::Artifact,
                instance_id: None,
                amount: artifact,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat.zones.hand = vec![
            CombatCard::new(CardId::Disarm, 1),
            CombatCard::new(CardId::Shockwave, 2),
            CombatCard::new(CardId::ThunderClap, 3),
        ];
        CombatPosition::new(EngineState::CombatPlayerTurn, combat)
    }

    #[test]
    fn projection_exposes_artifact_barrier_only_with_mitigation_payoff() {
        let position = bronze_position(3);
        let plan = bronze_automaton_combat_plan_v1(&position).expect("Bronze plan");

        assert_eq!(plan.plan, CombatPlanIdV1::BronzeAutomatonControl);
        assert_eq!(plan.stage, CombatPlanStageV1::ExposeAttackMitigationTarget);
        assert_eq!(
            plan.next_milestone,
            CombatPlanMilestoneV1::AttackMitigationTargetExposed
        );
        assert_eq!(plan.resources.remaining_artifact_sensitive_mitigation, 2);
        assert_eq!(plan.envelope.priority_target_artifact, Some(3));
    }

    #[test]
    fn guide_prefers_non_payoff_artifact_progress_without_losing_supply() {
        let protected = bronze_position(3);
        let stripped_once = bronze_position(2);
        let mut spent_payoff = bronze_position(2);
        spent_payoff
            .combat
            .zones
            .hand
            .retain(|card| card.id == CardId::ThunderClap);

        let protected_rank = combat_plan_state_guide_rank_v1(&protected).expect("protected rank");
        let stripped_rank = combat_plan_state_guide_rank_v1(&stripped_once).expect("stripped rank");
        let spent_rank = combat_plan_state_guide_rank_v1(&spent_payoff).expect("spent-payoff rank");

        assert!(stripped_rank.components() > protected_rank.components());
        assert!(protected_rank.components() > spent_rank.components());
    }

    #[test]
    fn removing_final_artifact_completes_exposure_milestone() {
        let before = bronze_position(1);
        let after = bronze_position(0);

        let transition =
            bronze_automaton_plan_transition_v1(&before, &after).expect("Bronze transition");

        assert_eq!(
            transition.completed_milestones,
            vec![CombatPlanMilestoneV1::AttackMitigationTargetExposed]
        );
        assert!(transition.events.contains(
            &CombatPlanTransitionEventV1::PriorityTargetArtifactChanged {
                before: Some(1),
                after: Some(0),
            }
        ));
    }

    #[test]
    fn action_timing_prefers_cheap_strips_and_defers_spending_the_payoff() {
        let before = bronze_position(3);
        let cheap_strip = bronze_position(2);
        assert_eq!(
            bronze_automaton_action_timing_v1(&before, &cheap_strip),
            CombatPlanActionTimingV1::PreferNow
        );

        let mut spent_payoff = bronze_position(2);
        spent_payoff
            .combat
            .zones
            .hand
            .retain(|card| card.id != CardId::Disarm);
        assert_eq!(
            bronze_automaton_action_timing_v1(&before, &spent_payoff),
            CombatPlanActionTimingV1::Defer(
                CombatPlanActionDeferralV1::PreserveArtifactSensitiveMitigationUntilTargetExposed
            )
        );
        assert!(combat_plan_has_timed_action_preference_v1(&before));
        assert!(combat_plan_supports_initial_policy_prefix_v1(&before));
    }
}
