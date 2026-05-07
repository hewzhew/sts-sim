// Mechanical split from turn_plan_probe.rs. Test module only.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::{CombatCard, Power};
    use crate::test_support::{blank_test_combat, planned_monster};

    fn card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
    }

    fn query<'a>(report: &'a CombatTurnPlanProbeReport, name: &str) -> &'a CombatPlanQueryReport {
        report
            .plan_queries
            .iter()
            .find(|query| query.query_name == name)
            .unwrap_or_else(|| panic!("missing query {name}"))
    }

    fn marginal_branch<'a>(
        report: &'a CombatDrawMarginalProbeReport,
        name: &str,
    ) -> &'a CombatDrawMarginalBranchReport {
        report
            .branches
            .iter()
            .find(|branch| branch.branch_name == name)
            .unwrap_or_else(|| panic!("missing branch {name}"))
    }

    #[test]
    fn draw_marginal_probe_reports_not_applicable_when_target_absent() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::Strike, 1));

        let report = probe_draw_marginal_value(
            &EngineState::CombatPlayerTurn,
            &combat,
            CardId::BattleTrance,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert_eq!(report.status, "not_applicable");
        assert_eq!(
            marginal_branch(&report, "forced_draw_best").status,
            "not_applicable"
        );
        assert!(report.marginal.is_none());
    }

    #[test]
    fn draw_marginal_probe_no_draw_branch_excludes_target_card() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::BattleTrance, 1));
        combat.zones.hand.push(card(CardId::Strike, 2));
        combat.zones.draw_pile.push(card(CardId::Strike, 10));

        let no_draw = probe_turn_plans_with_options(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
            &CombatTurnPlanExploreOptions {
                forbidden_target: Some(CombatDrawMarginalTarget::card(CardId::BattleTrance)),
                require_first_target: None,
            },
        );

        assert!(!no_draw.sequence_classes.iter().any(|sequence| sequence
            .action_keys
            .iter()
            .any(|key| key.contains("card:BattleTrance"))));
    }

    #[test]
    fn draw_marginal_probe_forced_battle_trance_can_show_setup_damage_gain() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 64;
        cultist.max_hp = 64;
        combat.entities.monsters.push(cultist);
        combat.turn.energy = 4;
        combat.zones.hand.push(card(CardId::BattleTrance, 1));
        combat.zones.hand.push(card(CardId::WildStrike, 2));
        combat.zones.draw_pile.push(card(CardId::Inflame, 10));
        combat.zones.draw_pile.push(card(CardId::Strike, 11));
        combat.zones.draw_pile.push(card(CardId::Strike, 12));

        let report = probe_draw_marginal_value(
            &EngineState::CombatPlayerTurn,
            &combat,
            CardId::BattleTrance,
            CombatTurnPlanProbeConfig {
                max_depth: 4,
                beam_width: 16,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert_eq!(report.status, "ok");
        assert!(!marginal_branch(&report, "forced_draw_best")
            .target_action_keys
            .is_empty());
        let marginal = report
            .marginal
            .expect("forced draw should produce marginal");
        assert!(
            marginal.damage_delta > 0 || marginal.setup_gain,
            "{marginal:?}"
        );
    }

    #[test]
    fn draw_marginal_probe_hand_instance_does_not_exclude_same_card_copy() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::PommelStrike, 10));
        combat.zones.hand.push(card(CardId::PommelStrike, 11));
        combat.zones.hand.push(card(CardId::Strike, 12));
        combat.zones.draw_pile.push(card(CardId::Strike, 20));

        let no_draw = probe_turn_plans_with_options(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
            &CombatTurnPlanExploreOptions {
                forbidden_target: Some(CombatDrawMarginalTarget::hand_instance(
                    CardId::PommelStrike,
                    0,
                    10,
                )),
                require_first_target: None,
            },
        );

        assert!(!no_draw
            .sequence_classes
            .iter()
            .flat_map(|sequence| sequence.action_keys.iter())
            .any(|key| key.contains("card:PommelStrike/hand:0")));
        assert!(no_draw
            .sequence_classes
            .iter()
            .flat_map(|sequence| sequence.action_keys.iter())
            .any(|key| key.contains("card:PommelStrike/hand:1")));
    }

    #[test]
    fn combat_turn_plan_probe_query_reports_lethal_partial() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 3);
        cultist.current_hp = 20;
        cultist.max_hp = 20;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Strike, 1));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let lethal = query(&report, "CanLethal");
        assert_eq!(lethal.status, "partial");
        let outcome = lethal.outcome.as_ref().expect("partial query has outcome");
        assert!(outcome.damage_done > 0);
        assert!(lethal
            .failed_constraints
            .iter()
            .any(|constraint| constraint.starts_with("missing_damage:")));
    }

    #[test]
    fn combat_turn_plan_probe_query_reports_full_block() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::Defend, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let full_block = query(&report, "CanFullBlock");
        assert_eq!(full_block.status, "feasible");
        let outcome = full_block
            .outcome
            .as_ref()
            .expect("feasible query has outcome");
        assert_eq!(outcome.projected_unblocked_damage, 0);
        assert!(outcome.remaining_energy >= 1);
    }

    #[test]
    fn combat_turn_plan_probe_query_reports_full_block_then_damage() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::Strike, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));
        combat.zones.hand.push(card(CardId::Defend, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 3,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let guarded_damage = query(&report, "CanFullBlockThenMaxDamage");
        assert_eq!(guarded_damage.status, "feasible");
        let outcome = guarded_damage
            .outcome
            .as_ref()
            .expect("feasible query has outcome");
        assert_eq!(outcome.projected_unblocked_damage, 0);
        assert!(outcome.damage_done > 0);
    }

    #[test]
    fn combat_turn_plan_probe_query_reports_setup_and_block() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::Inflame, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));
        combat.zones.hand.push(card(CardId::Defend, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 3,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let setup = query(&report, "CanPlaySetupAndStillBlock");
        assert_eq!(setup.status, "feasible");
        let outcome = setup.outcome.as_ref().expect("feasible query has outcome");
        assert!(outcome.played_setup_or_scaling);
        assert_eq!(outcome.projected_unblocked_damage, 0);
    }

    #[test]
    fn combat_turn_plan_probe_query_reports_kill_window_preservation() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 3);
        cultist.current_hp = 8;
        cultist.max_hp = 20;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Feed, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let kill_window = query(&report, "CanPreserveKillWindow");
        assert_eq!(kill_window.status, "feasible");
        let outcome = kill_window
            .outcome
            .as_ref()
            .expect("feasible query has outcome");
        assert!(!outcome.played_kill_window_card);
        assert!(outcome.kill_window_target_count > 0);
    }

    #[test]
    fn combat_turn_plan_probe_query_reports_kill_window_not_applicable() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 3));
        combat.zones.hand.push(card(CardId::Strike, 1));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let kill_window = query(&report, "CanPreserveKillWindow");
        assert_eq!(kill_window.status, "not_applicable");
    }

    #[test]
    fn combat_turn_plan_probe_marks_true_grit_random_key_card_risk() {
        let mut combat = blank_test_combat();
        combat.zones.hand.push(card(CardId::TrueGrit, 1));
        combat.zones.hand.push(card(CardId::Bash, 2));
        combat.zones.hand.push(card(CardId::Wound, 3));
        combat.zones.hand.push(card(CardId::Strike, 4));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let note = report
            .risk_notes
            .iter()
            .find(|note| note.kind == "true_grit_random_exhaust_overlay")
            .expect("unupgraded True Grit should emit static random exhaust overlay");
        assert_eq!(
            note.chance_model.as_deref(),
            Some("static_hand_distribution")
        );
        assert!(!note.exact_rng_branches);
        assert!(note.risk_is_overlay_only);
        assert!(note.bad_branch_probability_milli.unwrap_or_default() > 0);
        let affordance = report
            .first_action_affordances
            .iter()
            .find(|affordance| affordance.action_key.contains("card:TrueGrit"))
            .expect("True Grit should have a first-action affordance");
        assert!(affordance
            .risk_note_kinds
            .iter()
            .any(|kind| kind == "true_grit_random_exhaust_overlay"));
        assert!(affordance
            .major_tradeoffs
            .iter()
            .any(|tradeoff| tradeoff == "explicit_risk_note"));
    }

    #[test]
    fn combat_turn_plan_probe_marks_true_grit_bad_card_upside() {
        let mut combat = blank_test_combat();
        combat.zones.hand.push(card(CardId::TrueGrit, 1));
        combat.zones.hand.push(card(CardId::Wound, 2));
        combat.zones.hand.push(card(CardId::Slimed, 3));
        combat.zones.hand.push(card(CardId::Strike, 4));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );
        let note = report
            .risk_notes
            .iter()
            .find(|note| note.kind == "true_grit_random_exhaust_overlay")
            .expect("unupgraded True Grit should emit static random exhaust overlay");
        assert!(note.good_branch_probability_milli.unwrap_or_default() > 0);
    }

    #[test]
    fn combat_turn_plan_probe_marks_true_grit_plus_as_exact_selection() {
        let mut combat = blank_test_combat();
        let mut true_grit = card(CardId::TrueGrit, 1);
        true_grit.upgrades = 1;
        combat.zones.hand.push(true_grit);
        combat.zones.hand.push(card(CardId::Wound, 2));
        combat.zones.hand.push(card(CardId::Bash, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .risk_notes
            .iter()
            .any(|note| note.kind == "chosen_exhaust_pending" && note.exact_rng_branches));
        assert!(report.risk_notes.iter().any(|note| {
            note.kind == "exact_selection_resolution"
                && note
                    .affected_cards
                    .iter()
                    .any(|card| card.contains("Wound"))
        }));
    }

    #[test]
    fn combat_turn_plan_probe_records_vulnerable_order_sensitivity() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::Bash, 1));
        combat.zones.hand.push(card(CardId::Strike, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "debuff_before_damage_can_change_value")));
        let bash = report
            .first_action_affordances
            .iter()
            .find(|affordance| affordance.action_key.contains("card:Bash"))
            .expect("Bash should have first-action affordance rows");
        assert!(bash
            .supported_plans
            .iter()
            .any(|support| support.plan_name == "MaxDamage" && support.rank == 1));
        assert!(bash
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "debuff_before_damage_can_change_value"));
    }

    #[test]
    fn combat_turn_plan_probe_generation_canonical_prunes_pure_permutations() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Strike, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));
        combat.zones.hand.push(card(CardId::Defend, 4));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(
            report.probe_limits.pruned_by_generation_canonical_order > 0,
            "{:?}",
            report.probe_limits
        );
        assert!(
            report.probe_limits.pruned_by_generation_duplicate_card > 0,
            "{:?}",
            report.probe_limits
        );
        assert!(
            !report
                .probe_limits
                .generation_duplicate_prune_effects
                .is_empty(),
            "{:?}",
            report.probe_limits
        );
        assert!(
            report.probe_limits.pruned_by_generation_lane_order > 0,
            "{:?}",
            report.probe_limits
        );
        assert!(report.probe_limits.generation_canonical_candidates > 0);
        assert_eq!(
            report.probe_limits.abstract_equivalence_rejected_by_engine,
            0
        );
        assert!(report.probe_limits.abstract_equivalence_candidates > 0);
        assert!(report
            .sequence_classes
            .iter()
            .any(|sequence| sequence.compression_notes.iter().any(|note| note
                == "generation_canonical_candidate"
                || note == "abstract_candidate")));
    }

    #[test]
    fn combat_turn_plan_probe_allows_safe_relic_context_for_generation_prune() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::BurningBlood));
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Defend, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.probe_limits.abstract_equivalence_candidates > 0);
        assert!(
            report.probe_limits.pruned_by_generation_canonical_order
                + report.probe_limits.pruned_by_verified_abstract_equivalence
                > 0
        );
        assert_eq!(
            report.probe_limits.abstract_equivalence_rejected_by_engine,
            0
        );
    }

    #[test]
    fn combat_turn_plan_probe_does_not_block_irrelevant_exhaust_relic_for_pure_cards() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::CharonsAshes));
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Defend, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.probe_limits.abstract_equivalence_candidates > 0);
        assert!(
            report.probe_limits.pruned_by_generation_canonical_order
                + report.probe_limits.pruned_by_verified_abstract_equivalence
                > 0
        );
        assert!(!report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "abstract_context_blocker:relic_order_trigger:CharonsAshes")));
    }

    #[test]
    fn combat_turn_plan_probe_blocks_card_count_relic_context() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::LetterOpener));
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.entities.player.block = 99;
        combat.zones.hand.push(card(CardId::Defend, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.probe_limits.abstract_equivalence_blocked_by_context > 0);
        assert_eq!(
            report.probe_limits.pruned_by_verified_abstract_equivalence,
            0
        );
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "abstract_context_blocker:relic_order_trigger:LetterOpener")));
    }

    #[test]
    fn combat_turn_plan_probe_blocks_possible_kill_from_abstract_prune() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 5;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Strike, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(
            report
                .probe_limits
                .abstract_equivalence_blocked_by_action_semantics
                > 0
        );
    }

    #[test]
    fn combat_turn_plan_probe_rejects_abstract_match_when_engine_signature_differs() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        let mut override_defend = card(CardId::Defend, 1);
        override_defend.cost_for_turn = Some(1);
        combat.zones.hand.push(override_defend);
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.probe_limits.abstract_equivalence_candidates > 0);
        assert!(report.probe_limits.abstract_equivalence_rejected_by_engine > 0);
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "abstract_rejected_by_engine")));
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note.starts_with("abstract_reject_diff:"))));
    }

    #[test]
    fn combat_turn_plan_probe_blocks_abstract_after_future_draw_residue() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Headbutt, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));
        combat.zones.hand.push(card(CardId::Defend, 4));
        combat.zones.discard_pile.push(card(CardId::Strike, 20));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 3,
                beam_width: 16,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert_eq!(
            report.probe_limits.abstract_equivalence_rejected_by_engine,
            0
        );
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "sequence_residue:future_draw_order")));
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(
                |note| note == "generation_canonical_action_blocker:action_leaves_sequence_residue"
            )));
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "abstract_context_blocker:sequence_residue:future_draw_order")));
    }

    #[test]
    fn combat_turn_plan_probe_projects_inflame_before_attack() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Inflame, 1));
        combat.zones.hand.push(card(CardId::Strike, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let inflame_then_strike = report
            .sequence_classes
            .iter()
            .find(|sequence| {
                sequence.action_keys.len() >= 2
                    && sequence.action_keys[0].contains("card:Inflame")
                    && sequence.action_keys[1].contains("card:Strike")
            })
            .expect("Inflame -> Strike sequence should be explored");
        let strike_then_inflame = report
            .sequence_classes
            .iter()
            .find(|sequence| {
                sequence.action_keys.len() >= 2
                    && sequence.action_keys[0].contains("card:Strike")
                    && sequence.action_keys[1].contains("card:Inflame")
            })
            .expect("Strike -> Inflame sequence should be explored");
        assert!(inflame_then_strike.diagnostics.strength_projection > 0);
        assert!(
            inflame_then_strike.diagnostics.total_score
                > strike_then_inflame.diagnostics.total_score
        );
    }

    #[test]
    fn combat_turn_plan_probe_counts_multi_hit_strength_payoff() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.entities.power_db.insert(
            0,
            vec![Power {
                power_type: crate::runtime::combat::PowerId::Strength,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            }],
        );
        combat.zones.hand.push(card(CardId::Strike, 1));
        combat.zones.hand.push(card(CardId::TwinStrike, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let strike = report
            .sequence_classes
            .iter()
            .find(|sequence| sequence.action_keys[0].contains("card:Strike"))
            .expect("Strike sequence should be explored");
        let twin = report
            .sequence_classes
            .iter()
            .find(|sequence| sequence.action_keys[0].contains("card:TwinStrike"))
            .expect("Twin Strike sequence should be explored");
        assert!(twin.diagnostics.strength_projection > strike.diagnostics.strength_projection);
    }

    #[test]
    fn combat_turn_plan_probe_caps_strength_projection_by_available_hp() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 1;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Inflame, 1));
        combat.zones.hand.push(card(CardId::TwinStrike, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let inflame = report
            .sequence_classes
            .iter()
            .find(|sequence| sequence.action_keys[0].contains("card:Inflame"))
            .expect("Inflame sequence should be explored");
        assert!(inflame.diagnostics.strength_projection <= 1);
    }

    #[test]
    fn combat_turn_plan_probe_does_not_abstract_draw_sequences() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::PommelStrike, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let pommel = report
            .sequence_classes
            .iter()
            .find(|sequence| {
                sequence
                    .action_keys
                    .first()
                    .is_some_and(|key| key.contains("card:PommelStrike"))
            })
            .expect("Pommel Strike sequence should be explored");
        assert!(pommel
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "draw_changes_future_action_space"));
        assert!(!pommel
            .compression_notes
            .iter()
            .any(|note| note == "verified_abstract_equivalence"));
    }

    #[test]
    fn combat_turn_plan_probe_does_not_generation_prune_anger_copy_effect() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 60;
        cultist.max_hp = 60;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Anger, 1));
        combat.zones.hand.push(card(CardId::Anger, 2));
        combat.zones.hand.push(card(CardId::Defend, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.hand_cards.iter().any(|card| card.card_id == "Anger"
            && card
                .base_semantics
                .iter()
                .any(|tag| tag == "self_replicating")));
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "sequence_residue:generated_card_copy")));
        assert!(!report
            .probe_limits
            .generation_duplicate_prune_effects
            .keys()
            .any(|effect| effect.contains("Anger")));
    }

    #[test]
    fn combat_turn_plan_probe_does_not_treat_status_generation_as_pure_block() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::PowerThrough, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .hand_cards
            .iter()
            .any(|card| card.card_id == "PowerThrough"
                && card
                    .base_semantics
                    .iter()
                    .any(|tag| tag == "produces_status")));
        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .compression_notes
            .iter()
            .any(|note| note == "sequence_residue:generated_status_card")));
        assert!(!report
            .probe_limits
            .generation_duplicate_prune_effects
            .keys()
            .any(|effect| effect.contains("PowerThrough")));
    }

    #[test]
    fn combat_turn_plan_probe_gates_surplus_block_without_pressure() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.entities.player.block = 99;
        combat.zones.hand.push(card(CardId::Defend, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.probe_limits.pruned_by_plan_expansion_gate > 0);
        assert!(report
            .probe_limits
            .plan_expansion_gate_reasons
            .contains_key("surplus_block_without_pressure"));
        assert!(report
            .probe_limits
            .plan_expansion_gate_examples
            .iter()
            .any(|example| example.reason == "surplus_block_without_pressure"
                && example.pruned_action_key.contains("card:Defend")));
    }

    #[test]
    fn combat_turn_plan_probe_keeps_block_when_it_reduces_incoming() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.entities.player.block = 0;
        combat.zones.hand.push(card(CardId::Defend, 1));
        combat.zones.hand.push(card(CardId::Defend, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(!report
            .probe_limits
            .plan_expansion_gate_reasons
            .contains_key("surplus_block_without_pressure"));
        assert!(report
            .sequence_classes
            .iter()
            .any(|sequence| sequence.action_keys[0].contains("card:Defend")));
    }

    #[test]
    fn combat_turn_plan_probe_gates_repeated_action_space_changes() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 80;
        cultist.max_hp = 80;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::PommelStrike, 1));
        combat.zones.hand.push(card(CardId::ShrugItOff, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 8,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.probe_limits.pruned_by_plan_expansion_gate > 0);
        assert!(report
            .probe_limits
            .plan_expansion_gate_reasons
            .contains_key("secondary_action_space_change_budget"));
        assert!(report
            .probe_limits
            .plan_expansion_gate_examples
            .iter()
            .any(
                |example| example.reason == "secondary_action_space_change_budget"
                    && !example.partial_action_keys.is_empty()
            ));
    }

    #[test]
    fn combat_turn_plan_probe_does_not_gate_dropkick_without_vulnerable() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 80;
        cultist.max_hp = 80;
        combat.entities.monsters.push(cultist);
        combat.zones.hand.push(card(CardId::Dropkick, 1));
        combat.zones.hand.push(card(CardId::Dropkick, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 12,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(!report
            .probe_limits
            .plan_expansion_gate_reasons
            .contains_key("secondary_action_space_change_budget"));
        let dropkick = report
            .sequence_classes
            .iter()
            .find(|sequence| sequence.action_keys[0].contains("card:Dropkick"))
            .expect("Dropkick line should be explored");
        assert!(!dropkick
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "draw_changes_future_action_space"));
        assert!(!dropkick
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "energy_gain_changes_future_action_space"));
    }

    #[test]
    fn combat_turn_plan_probe_gates_repeated_dropkick_with_vulnerable() {
        let mut combat = blank_test_combat();
        let mut cultist = planned_monster(EnemyId::Cultist, 1);
        cultist.current_hp = 80;
        cultist.max_hp = 80;
        combat.entities.monsters.push(cultist);
        combat.entities.power_db.insert(
            1,
            vec![Power {
                power_type: crate::runtime::combat::PowerId::Vulnerable,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            }],
        );
        combat.zones.hand.push(card(CardId::Dropkick, 1));
        combat.zones.hand.push(card(CardId::Dropkick, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                beam_width: 12,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .probe_limits
            .plan_expansion_gate_reasons
            .contains_key("secondary_action_space_change_budget"));
        let dropkick = report
            .sequence_classes
            .iter()
            .find(|sequence| sequence.action_keys[0].contains("card:Dropkick"))
            .expect("Dropkick line should be explored");
        assert!(dropkick
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "draw_changes_future_action_space"));
        assert!(dropkick
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "energy_gain_changes_future_action_space"));
    }

    #[test]
    fn combat_turn_plan_probe_marks_exhaust_engine_block_setup() {
        let mut combat = blank_test_combat();
        combat.entities.power_db.insert(
            0,
            vec![Power {
                power_type: crate::runtime::combat::PowerId::FeelNoPain,
                instance_id: None,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );
        combat.zones.hand.push(card(CardId::SecondWind, 1));
        combat.zones.hand.push(card(CardId::Wound, 2));
        combat.zones.hand.push(card(CardId::Defend, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .risk_notes
            .iter()
            .any(|note| note.kind == "second_wind_multi_plan_semantics"));
        assert!(report.sequence_classes.iter().any(|sequence| {
            sequence.diagnostics.exhaust_value > 0 || sequence.diagnostics.block_score > 0
        }));
    }

    #[test]
    fn combat_turn_plan_probe_marks_fiend_fire_multi_plan_semantics() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::FiendFire, 1));
        combat.zones.hand.push(card(CardId::Bash, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .risk_notes
            .iter()
            .any(|note| note.kind == "fiend_fire_multi_plan_semantics"));
    }
}
