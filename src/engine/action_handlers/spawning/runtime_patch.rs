use crate::runtime::action::MonsterRuntimePatch;
use crate::runtime::combat::CombatState;
fn handle_update_hexaghost_state(
    monster_id: usize,
    activated: Option<bool>,
    orb_active_count: Option<u8>,
    burn_upgraded: Option<bool>,
    divider_damage: Option<i32>,
    clear_divider_damage: bool,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if activated.is_some()
            || orb_active_count.is_some()
            || burn_upgraded.is_some()
            || divider_damage.is_some()
            || clear_divider_damage
        {
            if let Some(value) = activated {
                monster.hexaghost.activated = value;
            }
            if let Some(value) = orb_active_count {
                monster.hexaghost.orb_active_count = value;
            }
            if let Some(value) = burn_upgraded {
                monster.hexaghost.burn_upgraded = value;
            }
            if clear_divider_damage {
                monster.hexaghost.divider_damage = None;
            }
            if let Some(value) = divider_damage {
                monster.hexaghost.divider_damage = Some(value);
            }
        }
    }
}

fn handle_update_lagavulin_state(
    monster_id: usize,
    idle_count: Option<u8>,
    debuff_turn_count: Option<u8>,
    is_out: Option<bool>,
    is_out_triggered: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = idle_count {
            monster.lagavulin.idle_count = value;
        }
        if let Some(value) = debuff_turn_count {
            monster.lagavulin.debuff_turn_count = value;
        }
        if let Some(value) = is_out {
            monster.lagavulin.is_out = value;
        }
        if let Some(value) = is_out_triggered {
            monster.lagavulin.is_out_triggered = value;
        }
    }
}

fn handle_update_jaw_worm_state(
    monster_id: usize,
    first_move: Option<bool>,
    hard_mode: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.jaw_worm.first_move = value;
        }
        if let Some(value) = hard_mode {
            monster.jaw_worm.hard_mode = value;
        }
        if let Some(value) = protocol_seeded {
            monster.jaw_worm.protocol_seeded = value;
        }
    }
}

fn handle_update_guardian_state(
    monster_id: usize,
    damage_threshold: Option<i32>,
    damage_taken: Option<i32>,
    is_open: Option<bool>,
    close_up_triggered: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = damage_threshold {
            monster.guardian.damage_threshold = value;
        }
        if let Some(value) = damage_taken {
            monster.guardian.damage_taken = value;
        }
        if let Some(value) = is_open {
            monster.guardian.is_open = value;
        }
        if let Some(value) = close_up_triggered {
            monster.guardian.close_up_triggered = value;
        }
    }
}

fn handle_update_byrd_state(
    monster_id: usize,
    first_move: Option<bool>,
    is_flying: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.byrd.first_move = value;
        }
        if let Some(value) = is_flying {
            monster.byrd.is_flying = value;
        }
        if let Some(value) = protocol_seeded {
            monster.byrd.protocol_seeded = value;
        }
    }
}

fn handle_update_chosen_state(
    monster_id: usize,
    first_turn: Option<bool>,
    used_hex: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.chosen.first_turn = value;
        }
        if let Some(value) = used_hex {
            monster.chosen.used_hex = value;
        }
        if let Some(value) = protocol_seeded {
            monster.chosen.protocol_seeded = value;
        }
    }
}

fn handle_update_snecko_state(
    monster_id: usize,
    first_turn: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.snecko.first_turn = value;
        }
        if let Some(value) = protocol_seeded {
            monster.snecko.protocol_seeded = value;
        }
    }
}

fn handle_update_shelled_parasite_state(
    monster_id: usize,
    first_move: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.shelled_parasite.first_move = value;
        }
        if let Some(value) = protocol_seeded {
            monster.shelled_parasite.protocol_seeded = value;
        }
    }
}

fn handle_update_bronze_automaton_state(
    monster_id: usize,
    first_turn: Option<bool>,
    num_turns: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.bronze_automaton.first_turn = value;
        }
        if let Some(value) = num_turns {
            monster.bronze_automaton.num_turns = value;
        }
        if let Some(value) = protocol_seeded {
            monster.bronze_automaton.protocol_seeded = value;
        }
    }
}

fn handle_update_bronze_orb_state(
    monster_id: usize,
    used_stasis: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = used_stasis {
            monster.bronze_orb.used_stasis = value;
        }
        if let Some(value) = protocol_seeded {
            monster.bronze_orb.protocol_seeded = value;
        }
    }
}

fn handle_update_collector_state(
    monster_id: usize,
    initial_spawn: Option<bool>,
    ult_used: Option<bool>,
    turns_taken: Option<u8>,
    enemy_slots: Option<[Option<usize>; 2]>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = initial_spawn {
            monster.collector.initial_spawn = value;
        }
        if let Some(value) = ult_used {
            monster.collector.ult_used = value;
        }
        if let Some(value) = turns_taken {
            monster.collector.turns_taken = value;
        }
        if let Some(value) = enemy_slots {
            monster.collector.enemy_slots = value;
        }
        if let Some(value) = protocol_seeded {
            monster.collector.protocol_seeded = value;
        }
    }
}

fn handle_update_champ_state(
    monster_id: usize,
    first_turn: Option<bool>,
    num_turns: Option<u8>,
    forge_times: Option<u8>,
    threshold_reached: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.champ.first_turn = value;
        }
        if let Some(value) = num_turns {
            monster.champ.num_turns = value;
        }
        if let Some(value) = forge_times {
            monster.champ.forge_times = value;
        }
        if let Some(value) = threshold_reached {
            monster.champ.threshold_reached = value;
        }
        if let Some(value) = protocol_seeded {
            monster.champ.protocol_seeded = value;
        }
    }
}

fn handle_update_book_of_stabbing_state(
    monster_id: usize,
    stab_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = stab_count {
            monster.book_of_stabbing.stab_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.book_of_stabbing.protocol_seeded = value;
        }
    }
}

fn handle_update_awakened_one_state(
    monster_id: usize,
    form1: Option<bool>,
    first_turn: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = form1 {
            monster.awakened_one.form1 = value;
        }
        if let Some(value) = first_turn {
            monster.awakened_one.first_turn = value;
        }
        if let Some(value) = protocol_seeded {
            monster.awakened_one.protocol_seeded = value;
        }
    }
}

fn handle_update_darkling_state(
    monster_id: usize,
    first_move: Option<bool>,
    nip_dmg: Option<i32>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.darkling.first_move = value;
        }
        if let Some(value) = nip_dmg {
            monster.darkling.nip_dmg = value;
        }
        if let Some(value) = protocol_seeded {
            monster.darkling.protocol_seeded = value;
        }
    }
}

fn handle_update_corrupt_heart_state(
    monster_id: usize,
    first_move: Option<bool>,
    move_count: Option<u8>,
    buff_count: Option<u8>,
    blood_hit_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.corrupt_heart.first_move = value;
        }
        if let Some(value) = move_count {
            monster.corrupt_heart.move_count = value;
        }
        if let Some(value) = buff_count {
            monster.corrupt_heart.buff_count = value;
        }
        if let Some(value) = blood_hit_count {
            monster.corrupt_heart.blood_hit_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.corrupt_heart.protocol_seeded = value;
        }
    }
}

fn handle_update_writhing_mass_state(
    monster_id: usize,
    first_move: Option<bool>,
    used_mega_debuff: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.writhing_mass.first_move = value;
        }
        if let Some(value) = used_mega_debuff {
            monster.writhing_mass.used_mega_debuff = value;
        }
        if let Some(value) = protocol_seeded {
            monster.writhing_mass.protocol_seeded = value;
        }
    }
}

fn handle_update_spiker_state(
    monster_id: usize,
    thorns_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = thorns_count {
            monster.spiker.thorns_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.spiker.protocol_seeded = value;
        }
    }
}

fn handle_update_spire_shield_state(
    monster_id: usize,
    move_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = move_count {
            monster.spire_shield.move_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.spire_shield.protocol_seeded = value;
        }
    }
}

fn handle_update_spire_spear_state(
    monster_id: usize,
    move_count: Option<u8>,
    skewer_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = move_count {
            monster.spire_spear.move_count = value;
        }
        if let Some(value) = skewer_count {
            monster.spire_spear.skewer_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.spire_spear.protocol_seeded = value;
        }
    }
}

fn handle_update_slaver_red_state(
    monster_id: usize,
    first_turn: Option<bool>,
    used_entangle: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.slaver_red.first_turn = value;
        }
        if let Some(value) = used_entangle {
            monster.slaver_red.used_entangle = value;
        }
        if let Some(value) = protocol_seeded {
            monster.slaver_red.protocol_seeded = value;
        }
    }
}

fn handle_update_gremlin_nob_state(
    monster_id: usize,
    used_bellow: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = used_bellow {
            monster.gremlin_nob.used_bellow = value;
        }
        if let Some(value) = protocol_seeded {
            monster.gremlin_nob.protocol_seeded = value;
        }
    }
}

fn handle_update_gremlin_leader_state(
    monster_id: usize,
    gremlin_slots: Option<[Option<usize>; 3]>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = gremlin_slots {
            monster.gremlin_leader.gremlin_slots = value;
        }
        if let Some(value) = protocol_seeded {
            monster.gremlin_leader.protocol_seeded = value;
        }
    }
}

fn handle_update_reptomancer_state(
    monster_id: usize,
    first_move: Option<bool>,
    dagger_slots: Option<[Option<usize>; 4]>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.reptomancer.first_move = value;
        }
        if let Some(value) = dagger_slots {
            monster.reptomancer.dagger_slots = value;
        }
        if let Some(value) = protocol_seeded {
            monster.reptomancer.protocol_seeded = value;
        }
    }
}

fn handle_update_nemesis_state(
    monster_id: usize,
    first_move: Option<bool>,
    scythe_cooldown: Option<i32>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.nemesis.first_move = value;
        }
        if let Some(value) = scythe_cooldown {
            monster.nemesis.scythe_cooldown = value;
        }
        if let Some(value) = protocol_seeded {
            monster.nemesis.protocol_seeded = value;
        }
    }
}

fn handle_update_giant_head_state(
    monster_id: usize,
    count: Option<i32>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = count {
            monster.giant_head.count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.giant_head.protocol_seeded = value;
        }
    }
}

fn handle_update_time_eater_state(
    monster_id: usize,
    used_haste: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = used_haste {
            monster.time_eater.used_haste = value;
        }
        if let Some(value) = protocol_seeded {
            monster.time_eater.protocol_seeded = value;
        }
    }
}

fn handle_update_donu_state(
    monster_id: usize,
    is_attacking: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = is_attacking {
            monster.donu.is_attacking = value;
        }
        if let Some(value) = protocol_seeded {
            monster.donu.protocol_seeded = value;
        }
    }
}

fn handle_update_deca_state(
    monster_id: usize,
    is_attacking: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = is_attacking {
            monster.deca.is_attacking = value;
        }
        if let Some(value) = protocol_seeded {
            monster.deca.protocol_seeded = value;
        }
    }
}

fn handle_update_transient_state(
    monster_id: usize,
    count: Option<i32>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = count {
            monster.transient.count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.transient.protocol_seeded = value;
        }
    }
}

fn handle_update_exploder_state(
    monster_id: usize,
    turn_count: Option<i32>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = turn_count {
            monster.exploder.turn_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.exploder.protocol_seeded = value;
        }
    }
}

fn handle_update_maw_state(
    monster_id: usize,
    roared: Option<bool>,
    turn_count: Option<i32>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = roared {
            monster.maw.roared = value;
        }
        if let Some(value) = turn_count {
            monster.maw.turn_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.maw.protocol_seeded = value;
        }
    }
}

fn handle_update_snake_dagger_state(
    monster_id: usize,
    first_move: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.snake_dagger.first_move = value;
        }
        if let Some(value) = protocol_seeded {
            monster.snake_dagger.protocol_seeded = value;
        }
    }
}

fn handle_update_gremlin_wizard_state(
    monster_id: usize,
    current_charge: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = current_charge {
            monster.gremlin_wizard.current_charge = value;
        }
        if let Some(value) = protocol_seeded {
            monster.gremlin_wizard.protocol_seeded = value;
        }
    }
}

fn handle_update_cultist_state(
    monster_id: usize,
    first_move: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.cultist.first_move = value;
        }
        if let Some(value) = protocol_seeded {
            monster.cultist.protocol_seeded = value;
        }
    }
}

fn handle_update_sentry_state(
    monster_id: usize,
    first_move: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.sentry.first_move = value;
        }
        if let Some(value) = protocol_seeded {
            monster.sentry.protocol_seeded = value;
        }
    }
}

fn handle_update_slime_boss_state(
    monster_id: usize,
    first_turn: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.slime_boss.first_turn = value;
        }
        if let Some(value) = protocol_seeded {
            monster.slime_boss.protocol_seeded = value;
        }
    }
}

fn handle_update_large_slime_state(
    monster_id: usize,
    split_triggered: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = split_triggered {
            monster.large_slime.split_triggered = value;
        }
        if let Some(value) = protocol_seeded {
            monster.large_slime.protocol_seeded = value;
        }
    }
}

fn handle_update_spheric_guardian_state(
    monster_id: usize,
    first_move: Option<bool>,
    second_move: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.spheric_guardian.first_move = value;
        }
        if let Some(value) = second_move {
            monster.spheric_guardian.second_move = value;
        }
        if let Some(value) = protocol_seeded {
            monster.spheric_guardian.protocol_seeded = value;
        }
    }
}

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
        } => handle_update_hexaghost_state(
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
        } => handle_update_lagavulin_state(
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
        } => {
            handle_update_jaw_worm_state(monster_id, first_move, hard_mode, protocol_seeded, state)
        }
        MonsterRuntimePatch::Guardian {
            damage_threshold,
            damage_taken,
            is_open,
            close_up_triggered,
        } => handle_update_guardian_state(
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
        } => handle_update_byrd_state(monster_id, first_move, is_flying, protocol_seeded, state),
        MonsterRuntimePatch::Chosen {
            first_turn,
            used_hex,
            protocol_seeded,
        } => handle_update_chosen_state(monster_id, first_turn, used_hex, protocol_seeded, state),
        MonsterRuntimePatch::Snecko {
            first_turn,
            protocol_seeded,
        } => handle_update_snecko_state(monster_id, first_turn, protocol_seeded, state),
        MonsterRuntimePatch::ShelledParasite {
            first_move,
            protocol_seeded,
        } => handle_update_shelled_parasite_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::BronzeAutomaton {
            first_turn,
            num_turns,
            protocol_seeded,
        } => handle_update_bronze_automaton_state(
            monster_id,
            first_turn,
            num_turns,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::BronzeOrb {
            used_stasis,
            protocol_seeded,
        } => handle_update_bronze_orb_state(monster_id, used_stasis, protocol_seeded, state),
        MonsterRuntimePatch::BookOfStabbing {
            stab_count,
            protocol_seeded,
        } => handle_update_book_of_stabbing_state(monster_id, stab_count, protocol_seeded, state),
        MonsterRuntimePatch::Collector {
            initial_spawn,
            ult_used,
            turns_taken,
            enemy_slots,
            protocol_seeded,
        } => handle_update_collector_state(
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
        } => handle_update_champ_state(
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
        } => {
            handle_update_awakened_one_state(monster_id, form1, first_turn, protocol_seeded, state)
        }
        MonsterRuntimePatch::Darkling {
            first_move,
            nip_dmg,
            protocol_seeded,
        } => handle_update_darkling_state(monster_id, first_move, nip_dmg, protocol_seeded, state),
        MonsterRuntimePatch::CorruptHeart {
            first_move,
            move_count,
            buff_count,
            blood_hit_count,
            protocol_seeded,
        } => handle_update_corrupt_heart_state(
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
        } => handle_update_writhing_mass_state(
            monster_id,
            first_move,
            used_mega_debuff,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Spiker {
            thorns_count,
            protocol_seeded,
        } => handle_update_spiker_state(monster_id, thorns_count, protocol_seeded, state),
        MonsterRuntimePatch::SpireShield {
            move_count,
            protocol_seeded,
        } => handle_update_spire_shield_state(monster_id, move_count, protocol_seeded, state),
        MonsterRuntimePatch::SpireSpear {
            move_count,
            skewer_count,
            protocol_seeded,
        } => handle_update_spire_spear_state(
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
        } => handle_update_slaver_red_state(
            monster_id,
            first_turn,
            used_entangle,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::GremlinNob {
            used_bellow,
            protocol_seeded,
        } => handle_update_gremlin_nob_state(monster_id, used_bellow, protocol_seeded, state),
        MonsterRuntimePatch::GremlinLeader {
            gremlin_slots,
            protocol_seeded,
        } => handle_update_gremlin_leader_state(monster_id, gremlin_slots, protocol_seeded, state),
        MonsterRuntimePatch::Reptomancer {
            first_move,
            dagger_slots,
            protocol_seeded,
        } => handle_update_reptomancer_state(
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
        } => handle_update_nemesis_state(
            monster_id,
            first_move,
            scythe_cooldown,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::GiantHead {
            count,
            protocol_seeded,
        } => handle_update_giant_head_state(monster_id, count, protocol_seeded, state),
        MonsterRuntimePatch::TimeEater {
            used_haste,
            protocol_seeded,
        } => handle_update_time_eater_state(monster_id, used_haste, protocol_seeded, state),
        MonsterRuntimePatch::Donu {
            is_attacking,
            protocol_seeded,
        } => handle_update_donu_state(monster_id, is_attacking, protocol_seeded, state),
        MonsterRuntimePatch::Deca {
            is_attacking,
            protocol_seeded,
        } => handle_update_deca_state(monster_id, is_attacking, protocol_seeded, state),
        MonsterRuntimePatch::Transient {
            count,
            protocol_seeded,
        } => handle_update_transient_state(monster_id, count, protocol_seeded, state),
        MonsterRuntimePatch::Exploder {
            turn_count,
            protocol_seeded,
        } => handle_update_exploder_state(monster_id, turn_count, protocol_seeded, state),
        MonsterRuntimePatch::Maw {
            roared,
            turn_count,
            protocol_seeded,
        } => handle_update_maw_state(monster_id, roared, turn_count, protocol_seeded, state),
        MonsterRuntimePatch::SnakeDagger {
            first_move,
            protocol_seeded,
        } => handle_update_snake_dagger_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::GremlinWizard {
            current_charge,
            protocol_seeded,
        } => handle_update_gremlin_wizard_state(monster_id, current_charge, protocol_seeded, state),
        MonsterRuntimePatch::Cultist {
            first_move,
            protocol_seeded,
        } => handle_update_cultist_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::Sentry {
            first_move,
            protocol_seeded,
        } => handle_update_sentry_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::SlimeBoss {
            first_turn,
            protocol_seeded,
        } => handle_update_slime_boss_state(monster_id, first_turn, protocol_seeded, state),
        MonsterRuntimePatch::LargeSlime {
            split_triggered,
            protocol_seeded,
        } => handle_update_large_slime_state(monster_id, split_triggered, protocol_seeded, state),
        MonsterRuntimePatch::SphericGuardian {
            first_move,
            second_move,
            protocol_seeded,
        } => handle_update_spheric_guardian_state(
            monster_id,
            first_move,
            second_move,
            protocol_seeded,
            state,
        ),
    }
}
