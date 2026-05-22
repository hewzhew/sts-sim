mod beyond;
mod city;
mod ending;
mod exordium;

use crate::runtime::action::MonsterRuntimePatch;
use crate::runtime::combat::CombatState;

pub fn handle_update_monster_runtime(
    monster_id: usize,
    patch: MonsterRuntimePatch,
    state: &mut CombatState,
) {
    match patch {
        MonsterRuntimePatch::Hexaghost {
            activated,
            orb_active_count,
            burn_upgraded,
            divider_damage,
            clear_divider_damage,
        } => exordium::handle_update_hexaghost_state(
            monster_id,
            activated,
            orb_active_count,
            burn_upgraded,
            divider_damage,
            clear_divider_damage,
            state,
        ),
        MonsterRuntimePatch::Lagavulin {
            idle_count,
            debuff_turn_count,
            is_out,
            is_out_triggered,
        } => exordium::handle_update_lagavulin_state(
            monster_id,
            idle_count,
            debuff_turn_count,
            is_out,
            is_out_triggered,
            state,
        ),
        MonsterRuntimePatch::JawWorm {
            first_move,
            hard_mode,
            protocol_seeded,
        } => exordium::handle_update_jaw_worm_state(
            monster_id,
            first_move,
            hard_mode,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Guardian {
            damage_threshold,
            damage_taken,
            is_open,
            close_up_triggered,
        } => exordium::handle_update_guardian_state(
            monster_id,
            damage_threshold,
            damage_taken,
            is_open,
            close_up_triggered,
            state,
        ),
        MonsterRuntimePatch::Byrd {
            first_move,
            is_flying,
            protocol_seeded,
        } => city::handle_update_byrd_state(
            monster_id,
            first_move,
            is_flying,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Chosen {
            first_turn,
            used_hex,
            protocol_seeded,
        } => city::handle_update_chosen_state(
            monster_id,
            first_turn,
            used_hex,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Snecko {
            first_turn,
            protocol_seeded,
        } => city::handle_update_snecko_state(monster_id, first_turn, protocol_seeded, state),
        MonsterRuntimePatch::ShelledParasite {
            first_move,
            protocol_seeded,
        } => city::handle_update_shelled_parasite_state(
            monster_id,
            first_move,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::BronzeAutomaton {
            first_turn,
            num_turns,
            protocol_seeded,
        } => city::handle_update_bronze_automaton_state(
            monster_id,
            first_turn,
            num_turns,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::BronzeOrb {
            used_stasis,
            protocol_seeded,
        } => city::handle_update_bronze_orb_state(monster_id, used_stasis, protocol_seeded, state),
        MonsterRuntimePatch::BookOfStabbing {
            stab_count,
            protocol_seeded,
        } => city::handle_update_book_of_stabbing_state(
            monster_id,
            stab_count,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Collector {
            initial_spawn,
            ult_used,
            turns_taken,
            enemy_slots,
            protocol_seeded,
        } => city::handle_update_collector_state(
            monster_id,
            initial_spawn,
            ult_used,
            turns_taken,
            enemy_slots,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Champ {
            first_turn,
            num_turns,
            forge_times,
            threshold_reached,
            protocol_seeded,
        } => city::handle_update_champ_state(
            monster_id,
            first_turn,
            num_turns,
            forge_times,
            threshold_reached,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::AwakenedOne {
            form1,
            first_turn,
            protocol_seeded,
        } => beyond::handle_update_awakened_one_state(
            monster_id,
            form1,
            first_turn,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Darkling {
            first_move,
            nip_dmg,
            protocol_seeded,
        } => beyond::handle_update_darkling_state(
            monster_id,
            first_move,
            nip_dmg,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::CorruptHeart {
            first_move,
            move_count,
            buff_count,
            blood_hit_count,
            protocol_seeded,
        } => ending::handle_update_corrupt_heart_state(
            monster_id,
            first_move,
            move_count,
            buff_count,
            blood_hit_count,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::WrithingMass {
            first_move,
            used_mega_debuff,
            protocol_seeded,
        } => beyond::handle_update_writhing_mass_state(
            monster_id,
            first_move,
            used_mega_debuff,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Spiker {
            thorns_count,
            protocol_seeded,
        } => beyond::handle_update_spiker_state(monster_id, thorns_count, protocol_seeded, state),
        MonsterRuntimePatch::SpireShield {
            move_count,
            protocol_seeded,
        } => {
            ending::handle_update_spire_shield_state(monster_id, move_count, protocol_seeded, state)
        }
        MonsterRuntimePatch::SpireSpear {
            move_count,
            skewer_count,
            protocol_seeded,
        } => ending::handle_update_spire_spear_state(
            monster_id,
            move_count,
            skewer_count,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::SlaverRed {
            first_turn,
            used_entangle,
            protocol_seeded,
        } => city::handle_update_slaver_red_state(
            monster_id,
            first_turn,
            used_entangle,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::GremlinNob {
            used_bellow,
            protocol_seeded,
        } => exordium::handle_update_gremlin_nob_state(
            monster_id,
            used_bellow,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::GremlinLeader {
            gremlin_slots,
            protocol_seeded,
        } => city::handle_update_gremlin_leader_state(
            monster_id,
            gremlin_slots,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Reptomancer {
            first_move,
            dagger_slots,
            protocol_seeded,
        } => beyond::handle_update_reptomancer_state(
            monster_id,
            first_move,
            dagger_slots,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Nemesis {
            first_move,
            scythe_cooldown,
            protocol_seeded,
        } => beyond::handle_update_nemesis_state(
            monster_id,
            first_move,
            scythe_cooldown,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::GiantHead {
            count,
            protocol_seeded,
        } => beyond::handle_update_giant_head_state(monster_id, count, protocol_seeded, state),
        MonsterRuntimePatch::TimeEater {
            used_haste,
            protocol_seeded,
        } => beyond::handle_update_time_eater_state(monster_id, used_haste, protocol_seeded, state),
        MonsterRuntimePatch::Donu {
            is_attacking,
            protocol_seeded,
        } => beyond::handle_update_donu_state(monster_id, is_attacking, protocol_seeded, state),
        MonsterRuntimePatch::Deca {
            is_attacking,
            protocol_seeded,
        } => beyond::handle_update_deca_state(monster_id, is_attacking, protocol_seeded, state),
        MonsterRuntimePatch::Transient {
            count,
            protocol_seeded,
        } => beyond::handle_update_transient_state(monster_id, count, protocol_seeded, state),
        MonsterRuntimePatch::Exploder {
            turn_count,
            protocol_seeded,
        } => beyond::handle_update_exploder_state(monster_id, turn_count, protocol_seeded, state),
        MonsterRuntimePatch::Maw {
            roared,
            turn_count,
            protocol_seeded,
        } => {
            beyond::handle_update_maw_state(monster_id, roared, turn_count, protocol_seeded, state)
        }
        MonsterRuntimePatch::SnakeDagger {
            first_move,
            protocol_seeded,
        } => {
            beyond::handle_update_snake_dagger_state(monster_id, first_move, protocol_seeded, state)
        }
        MonsterRuntimePatch::GremlinWizard {
            current_charge,
            protocol_seeded,
        } => exordium::handle_update_gremlin_wizard_state(
            monster_id,
            current_charge,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Cultist {
            first_move,
            protocol_seeded,
        } => exordium::handle_update_cultist_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::Sentry {
            first_move,
            protocol_seeded,
        } => exordium::handle_update_sentry_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::SlimeBoss {
            first_turn,
            protocol_seeded,
        } => {
            exordium::handle_update_slime_boss_state(monster_id, first_turn, protocol_seeded, state)
        }
        MonsterRuntimePatch::LargeSlime {
            split_triggered,
            protocol_seeded,
        } => exordium::handle_update_large_slime_state(
            monster_id,
            split_triggered,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::SphericGuardian {
            first_move,
            second_move,
            protocol_seeded,
        } => city::handle_update_spheric_guardian_state(
            monster_id,
            first_move,
            second_move,
            protocol_seeded,
            state,
        ),
    }
}
