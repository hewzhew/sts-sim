use crate::content::monsters::EnemyId;
use crate::runtime::combat::CollectorRuntimeState;
use crate::runtime::combat::{
    AwakenedOneRuntimeState, BookOfStabbingRuntimeState, BronzeAutomatonRuntimeState,
    BronzeOrbRuntimeState, ByrdRuntimeState, ChampRuntimeState, ChosenRuntimeState,
    CorruptHeartRuntimeState, DarklingRuntimeState, GuardianRuntimeState, HexaghostRuntimeState,
    JawWormRuntimeState, LagavulinRuntimeState, LouseRuntimeState, MonsterEntity, MonsterMoveState,
    ShelledParasiteRuntimeState, SneckoRuntimeState, ThiefRuntimeState,
};
use crate::semantics::combat::{
    AddCardStep, ApplyPowerStep, AttackSpec, AttackStep, BlockStep, DebuffSpec, HealStep,
    MonsterMoveSpec, MonsterTurnSteps, MoveStep, RandomBlockStep, RemovePowerStep, SpawnHpValue,
    SpawnMonsterStep, StealGoldStep, UpgradeCardsStep, UtilityStep,
};

pub(super) fn stable_monster_signature(monster: &MonsterEntity) -> String {
    format!(
        "{}:hp{}:{}:blk{}:dy{}:esc{}:half{}:move={}:rt={}",
        stable_monster_kind_signature(monster.monster_type),
        monster.current_hp,
        monster.max_hp,
        monster.block,
        monster.is_dying,
        monster.is_escaped,
        monster.half_dead,
        stable_move_state_signature(&monster.move_state),
        stable_monster_runtime_signature(monster),
    )
}

fn stable_monster_kind_signature(monster_type: usize) -> String {
    EnemyId::from_id(monster_type)
        .map(|enemy| format!("{enemy:?}"))
        .unwrap_or_else(|| format!("id{monster_type}"))
}

fn stable_monster_runtime_signature(monster: &MonsterEntity) -> String {
    match EnemyId::from_id(monster.monster_type) {
        Some(EnemyId::Hexaghost) => {
            format!("hex:{}", stable_hexaghost_signature(&monster.hexaghost))
        }
        Some(EnemyId::LouseNormal) | Some(EnemyId::LouseDefensive) => {
            format!("louse:{}", stable_louse_signature(&monster.louse))
        }
        Some(EnemyId::JawWorm) => format!("jaw:{}", stable_jaw_worm_signature(&monster.jaw_worm)),
        Some(EnemyId::Looter) | Some(EnemyId::Mugger) => {
            format!("thief:{}", stable_thief_signature(&monster.thief))
        }
        Some(EnemyId::Byrd) => format!("byrd:{}", stable_byrd_signature(&monster.byrd)),
        Some(EnemyId::Chosen) => format!("chosen:{}", stable_chosen_signature(&monster.chosen)),
        Some(EnemyId::Snecko) => format!("snecko:{}", stable_snecko_signature(&monster.snecko)),
        Some(EnemyId::ShelledParasite) => format!(
            "parasite:{}",
            stable_shelled_parasite_signature(&monster.shelled_parasite)
        ),
        Some(EnemyId::BronzeAutomaton) => format!(
            "bronze_auto:{}",
            stable_bronze_automaton_signature(&monster.bronze_automaton)
        ),
        Some(EnemyId::BronzeOrb) => {
            format!(
                "bronze_orb:{}",
                stable_bronze_orb_signature(&monster.bronze_orb)
            )
        }
        Some(EnemyId::BookOfStabbing) => {
            format!(
                "book:{}",
                stable_book_of_stabbing_signature(&monster.book_of_stabbing)
            )
        }
        Some(EnemyId::TheCollector) => {
            format!(
                "collector:{}",
                stable_collector_signature(&monster.collector)
            )
        }
        Some(EnemyId::Champ) => format!("champ:{}", stable_champ_signature(&monster.champ)),
        Some(EnemyId::AwakenedOne) => {
            format!(
                "awakened:{}",
                stable_awakened_one_signature(&monster.awakened_one)
            )
        }
        Some(EnemyId::CorruptHeart) => {
            format!(
                "heart:{}",
                stable_corrupt_heart_signature(&monster.corrupt_heart)
            )
        }
        Some(EnemyId::Darkling) => {
            format!("darkling:{}", stable_darkling_signature(&monster.darkling))
        }
        Some(EnemyId::Lagavulin) => {
            format!(
                "lagavulin:{}",
                stable_lagavulin_signature(&monster.lagavulin)
            )
        }
        Some(EnemyId::TheGuardian) => {
            format!("guardian:{}", stable_guardian_signature(&monster.guardian))
        }
        Some(_) => "_".to_string(),
        None => stable_all_monster_runtime_signature(monster),
    }
}

fn stable_all_monster_runtime_signature(monster: &MonsterEntity) -> String {
    format!(
        concat!(
            "hex={}:louse={}:jaw={}:thief={}:byrd={}:chosen={}:",
            "snecko={}:parasite={}:bronze_auto={}:bronze_orb={}:book={}:",
            "collector={}:champ={}:awakened={}:heart={}:darkling={}:lagavulin={}:guardian={}"
        ),
        stable_hexaghost_signature(&monster.hexaghost),
        stable_louse_signature(&monster.louse),
        stable_jaw_worm_signature(&monster.jaw_worm),
        stable_thief_signature(&monster.thief),
        stable_byrd_signature(&monster.byrd),
        stable_chosen_signature(&monster.chosen),
        stable_snecko_signature(&monster.snecko),
        stable_shelled_parasite_signature(&monster.shelled_parasite),
        stable_bronze_automaton_signature(&monster.bronze_automaton),
        stable_bronze_orb_signature(&monster.bronze_orb),
        stable_book_of_stabbing_signature(&monster.book_of_stabbing),
        stable_collector_signature(&monster.collector),
        stable_champ_signature(&monster.champ),
        stable_awakened_one_signature(&monster.awakened_one),
        stable_corrupt_heart_signature(&monster.corrupt_heart),
        stable_darkling_signature(&monster.darkling),
        stable_lagavulin_signature(&monster.lagavulin),
        stable_guardian_signature(&monster.guardian),
    )
}

fn stable_move_state_signature(move_state: &MonsterMoveState) -> String {
    format!(
        "id{}:hist{}:steps{}:visible{}",
        move_state.planned_move_id,
        move_state
            .history
            .iter()
            .map(|move_id| move_id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        stable_turn_steps_signature(move_state.planned_steps.as_ref()),
        stable_move_spec_signature(move_state.planned_visible_spec.as_ref()),
    )
}

fn stable_turn_steps_signature(steps: Option<&MonsterTurnSteps>) -> String {
    match steps {
        Some(steps) => steps
            .iter()
            .map(stable_move_step_signature)
            .collect::<Vec<_>>()
            .join("|"),
        None => "_".to_string(),
    }
}

fn stable_move_step_signature(step: &MoveStep) -> String {
    match step {
        MoveStep::Attack(step) => format!("atk:{}", stable_attack_step_signature(step)),
        MoveStep::ApplyPower(step) => format!("pow:{}", stable_apply_power_step_signature(step)),
        MoveStep::Heal(step) => format!("heal:{}", stable_heal_step_signature(step)),
        MoveStep::GainBlock(step) => format!("blk:{}", stable_block_step_signature(step)),
        MoveStep::GainBlockRandomMonster(step) => {
            format!("blk_rand:{}", stable_random_block_step_signature(step))
        }
        MoveStep::AddCard(step) => format!("add:{}", stable_add_card_step_signature(step)),
        MoveStep::StealGold(step) => format!("steal:{}", stable_steal_gold_step_signature(step)),
        MoveStep::UpgradeCards(step) => {
            format!("upgrade:{}", stable_upgrade_cards_step_signature(step))
        }
        MoveStep::RemovePower(step) => {
            format!("rm_pow:{}", stable_remove_power_step_signature(step))
        }
        MoveStep::Utility(step) => format!("util:{}", stable_utility_step_signature(step)),
        MoveStep::SpawnMonster(step) => {
            format!("spawn:{}", stable_spawn_monster_step_signature(step))
        }
        MoveStep::Suicide => "suicide".to_string(),
        MoveStep::Charge => "charge".to_string(),
        MoveStep::Escape => "escape".to_string(),
        MoveStep::Magic => "magic".to_string(),
        MoveStep::Sleep => "sleep".to_string(),
        MoveStep::Stun => "stun".to_string(),
        MoveStep::Debug => "debug".to_string(),
        MoveStep::None => "none".to_string(),
    }
}

fn stable_move_spec_signature(spec: Option<&MonsterMoveSpec>) -> String {
    match spec {
        Some(spec) => stable_visible_move_spec_signature(spec),
        None => "_".to_string(),
    }
}

fn stable_visible_move_spec_signature(spec: &MonsterMoveSpec) -> String {
    match spec {
        MonsterMoveSpec::Attack(spec) => format!("attack:{}", stable_attack_spec_signature(spec)),
        MonsterMoveSpec::AttackAddCard(attack, add) => format!(
            "attack_add:{}:{}",
            stable_attack_spec_signature(attack),
            stable_add_card_step_signature(add)
        ),
        MonsterMoveSpec::AttackUpgradeCards(attack, upgrade) => format!(
            "attack_upgrade:{}:{}",
            stable_attack_spec_signature(attack),
            stable_upgrade_cards_step_signature(upgrade)
        ),
        MonsterMoveSpec::AttackBuff(attack, buff) => format!(
            "attack_buff:{}:{:?}:{}",
            stable_attack_spec_signature(attack),
            buff.power_id,
            buff.amount
        ),
        MonsterMoveSpec::AttackSustain(attack) => {
            format!("attack_sustain:{}", stable_attack_spec_signature(attack))
        }
        MonsterMoveSpec::AttackDebuff(attack, debuff) => format!(
            "attack_debuff:{}:{}",
            stable_attack_spec_signature(attack),
            stable_debuff_spec_signature(debuff)
        ),
        MonsterMoveSpec::AttackDefend(attack, defend) => format!(
            "attack_defend:{}:blk{}",
            stable_attack_spec_signature(attack),
            defend.block
        ),
        MonsterMoveSpec::AddCard(add) => format!("add:{}", stable_add_card_step_signature(add)),
        MonsterMoveSpec::Buff(buff) => format!("buff:{:?}:{}", buff.power_id, buff.amount),
        MonsterMoveSpec::Debuff(debuff) => {
            format!("debuff:{}", stable_debuff_spec_signature(debuff))
        }
        MonsterMoveSpec::StrongDebuff(debuff) => {
            format!("strong_debuff:{}", stable_debuff_spec_signature(debuff))
        }
        MonsterMoveSpec::Defend(defend) => format!("defend:blk{}", defend.block),
        MonsterMoveSpec::DefendDebuff(defend, debuff) => format!(
            "defend_debuff:blk{}:{}",
            defend.block,
            stable_debuff_spec_signature(debuff)
        ),
        MonsterMoveSpec::DefendBuff(defend, buff) => {
            format!(
                "defend_buff:blk{}:{:?}:{}",
                defend.block, buff.power_id, buff.amount
            )
        }
        MonsterMoveSpec::Heal(heal) => format!("heal:{:?}:{}", heal.target, heal.amount),
        MonsterMoveSpec::Escape => "escape".to_string(),
        MonsterMoveSpec::Magic => "magic".to_string(),
        MonsterMoveSpec::Sleep => "sleep".to_string(),
        MonsterMoveSpec::Stun => "stun".to_string(),
        MonsterMoveSpec::Debug => "debug".to_string(),
        MonsterMoveSpec::None => "none".to_string(),
        MonsterMoveSpec::Unknown => "unknown".to_string(),
    }
}

fn stable_attack_spec_signature(spec: &AttackSpec) -> String {
    format!(
        "d{}:h{}:{:?}",
        spec.base_damage, spec.hits, spec.damage_kind
    )
}

fn stable_attack_step_signature(step: &AttackStep) -> String {
    format!(
        "{:?}:{}",
        step.target,
        stable_attack_spec_signature(&step.attack)
    )
}

fn stable_apply_power_step_signature(step: &ApplyPowerStep) -> String {
    format!(
        "{:?}:{:?}:{}:{:?}:{:?}",
        step.target, step.power_id, step.amount, step.effect, step.visible_strength
    )
}

fn stable_heal_step_signature(step: &HealStep) -> String {
    format!("{:?}:{}", step.target, step.amount)
}

fn stable_block_step_signature(step: &BlockStep) -> String {
    format!("{:?}:{}", step.target, step.amount)
}

fn stable_random_block_step_signature(step: &RandomBlockStep) -> String {
    step.amount.to_string()
}

fn stable_add_card_step_signature(step: &AddCardStep) -> String {
    format!(
        "{:?}:{}:{}:{:?}:{:?}",
        step.card_id, step.amount, step.upgraded, step.destination, step.visible_strength
    )
}

fn stable_steal_gold_step_signature(step: &StealGoldStep) -> String {
    step.amount.to_string()
}

fn stable_upgrade_cards_step_signature(step: &UpgradeCardsStep) -> String {
    format!("{:?}", step.card_id)
}

fn stable_remove_power_step_signature(step: &RemovePowerStep) -> String {
    format!("{:?}:{:?}", step.target, step.power_id)
}

fn stable_utility_step_signature(step: &UtilityStep) -> String {
    match step {
        UtilityStep::RemoveAllDebuffs { target } => format!("rm_all_debuffs:{target:?}"),
    }
}

fn stable_spawn_monster_step_signature(step: &SpawnMonsterStep) -> String {
    format!(
        "{:?}:off{}:drawx{:?}:hp{}:{}:minion{}",
        step.monster_id,
        step.logical_position_offset,
        step.protocol_draw_x_offset,
        stable_spawn_hp_value_signature(step.hp.current),
        stable_spawn_hp_value_signature(step.hp.max),
        step.is_minion,
    )
}

fn stable_spawn_hp_value_signature(value: SpawnHpValue) -> String {
    match value {
        SpawnHpValue::Rolled => "rolled".to_string(),
        SpawnHpValue::Fixed(value) => format!("fixed{value}"),
        SpawnHpValue::SourceCurrentHp => "source_cur".to_string(),
        SpawnHpValue::SourceMaxHp => "source_max".to_string(),
    }
}

fn stable_debuff_spec_signature(spec: &DebuffSpec) -> String {
    format!("{:?}:{}:{:?}", spec.power_id, spec.amount, spec.strength)
}

fn stable_hexaghost_signature(state: &HexaghostRuntimeState) -> String {
    format!(
        "act{}:orbs{}:burn_upg{}:divider{:?}",
        state.activated, state.orb_active_count, state.burn_upgraded, state.divider_damage
    )
}

fn stable_louse_signature(state: &LouseRuntimeState) -> String {
    format!("bite{:?}", state.bite_damage)
}

fn stable_jaw_worm_signature(state: &JawWormRuntimeState) -> String {
    format!("hard{}", state.hard_mode)
}

fn stable_thief_signature(state: &ThiefRuntimeState) -> String {
    format!(
        "seed{}:slash{}:gold{}",
        state.protocol_seeded, state.slash_count, state.stolen_gold
    )
}

fn stable_byrd_signature(state: &ByrdRuntimeState) -> String {
    format!(
        "seed{}:first{}:flying{}",
        state.protocol_seeded, state.first_move, state.is_flying
    )
}

fn stable_chosen_signature(state: &ChosenRuntimeState) -> String {
    format!(
        "seed{}:first{}:hex{}",
        state.protocol_seeded, state.first_turn, state.used_hex
    )
}

fn stable_snecko_signature(state: &SneckoRuntimeState) -> String {
    format!("seed{}:first{}", state.protocol_seeded, state.first_turn)
}

fn stable_shelled_parasite_signature(state: &ShelledParasiteRuntimeState) -> String {
    format!("seed{}:first{}", state.protocol_seeded, state.first_move)
}

fn stable_bronze_automaton_signature(state: &BronzeAutomatonRuntimeState) -> String {
    format!(
        "seed{}:first{}:turns{}",
        state.protocol_seeded, state.first_turn, state.num_turns
    )
}

fn stable_bronze_orb_signature(state: &BronzeOrbRuntimeState) -> String {
    format!("seed{}:stasis{}", state.protocol_seeded, state.used_stasis)
}

fn stable_book_of_stabbing_signature(state: &BookOfStabbingRuntimeState) -> String {
    format!("seed{}:stab{}", state.protocol_seeded, state.stab_count)
}

fn stable_collector_signature(state: &CollectorRuntimeState) -> String {
    format!(
        "seed{}:spawn{}:ult{}:turns{}",
        state.protocol_seeded, state.initial_spawn, state.ult_used, state.turns_taken
    )
}

fn stable_champ_signature(state: &ChampRuntimeState) -> String {
    format!(
        "seed{}:first{}:turns{}:forge{}:threshold{}",
        state.protocol_seeded,
        state.first_turn,
        state.num_turns,
        state.forge_times,
        state.threshold_reached
    )
}

fn stable_awakened_one_signature(state: &AwakenedOneRuntimeState) -> String {
    format!(
        "seed{}:form1{}:first{}",
        state.protocol_seeded, state.form1, state.first_turn
    )
}

fn stable_corrupt_heart_signature(state: &CorruptHeartRuntimeState) -> String {
    format!(
        "seed{}:first{}:moves{}:buff{}",
        state.protocol_seeded, state.first_move, state.move_count, state.buff_count
    )
}

fn stable_darkling_signature(state: &DarklingRuntimeState) -> String {
    format!("first{}:nip{}", state.first_move, state.nip_dmg)
}

fn stable_lagavulin_signature(state: &LagavulinRuntimeState) -> String {
    format!(
        "out{}:idle{}:debuff{}:trigger{}",
        state.is_out, state.idle_count, state.debuff_turn_count, state.is_out_triggered
    )
}

fn stable_guardian_signature(state: &GuardianRuntimeState) -> String {
    format!(
        "threshold{}:taken{}:open{}:close{}",
        state.damage_threshold, state.damage_taken, state.is_open, state.close_up_triggered
    )
}
