use serde::Serialize;
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::potions::PotionId;
use sts_simulator::content::powers::{store::power_amount, PowerId};
use sts_simulator::runtime::combat::{CombatState, MonsterEntity};
use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_simulator::state::core::ClientInput;

use super::focus::{focus_witness_line, CombatReviewFocus};

#[derive(Serialize)]
pub(super) struct ChampPhaseAudit {
    schema: &'static str,
    contract: &'static str,
    basis_line: &'static str,
    witness_action_count: Option<usize>,
    replayed_actions: usize,
    truncated_by_preview: bool,
    truncated: bool,
    timed_out: bool,
    initial_snapshot: ChampPhaseSnapshot,
    first_below_half_hp: Option<ChampHpCrossing>,
    split_trigger: Option<ChampSplitTrigger>,
    post_split_snapshot: Option<ChampPhaseSnapshot>,
    resources_before_split: ChampResourceTiming,
    flags: Vec<ChampPhaseAuditFlag>,
    verdict: ChampPhaseAuditVerdict,
}

#[derive(Clone, Serialize)]
struct ChampPhaseSnapshot {
    step_index: usize,
    player_hp: i32,
    player_max_hp: i32,
    player_block: i32,
    champ_hp: i32,
    champ_max_hp: i32,
    champ_block: i32,
    champ_strength: i32,
    champ_weak: i32,
    champ_vulnerable: i32,
    champ_threshold_reached: bool,
    champ_move_id: u8,
    total_enemy_hp: i32,
    living_enemy_count: usize,
}

#[derive(Serialize)]
struct ChampSplitTrigger {
    step_index: usize,
    action_key: String,
    input: ClientInput,
    before: ChampPhaseSnapshot,
    after: ChampPhaseSnapshot,
}

#[derive(Serialize)]
struct ChampHpCrossing {
    step_index: usize,
    action_key: String,
    input: ClientInput,
    before_champ_hp: i32,
    after_champ_hp: i32,
}

#[derive(Default, Serialize)]
struct ChampResourceTiming {
    disarm_used_before_split: bool,
    disarm_step: Option<usize>,
    fear_potion_used_before_split: bool,
    fear_potion_step: Option<usize>,
    strength_potion_used_before_split: bool,
    strength_potion_step: Option<usize>,
    steroid_potion_used_before_split: bool,
    steroid_potion_step: Option<usize>,
    forge_potion_used_before_split: bool,
    forge_potion_step: Option<usize>,
    potions_used_before_split: u32,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChampPhaseAuditFlag {
    NoSplitReached,
    SplitObserved,
    DisarmSpentBeforeSplit,
    FearPotionSpentBeforeSplit,
    BurstPotionSpentBeforeSplit,
    SplitWithLowHp,
    SplitWithChampHpStillHigh,
    ReplayTruncated,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChampPhaseAuditVerdict {
    NoSplitReached,
    SplitObserved,
    ResourceSpentBeforeSplit,
    SplitWithLowHp,
    Unclear,
}

pub(super) fn champ_phase_audit(
    root: &CombatPosition,
    focus: &CombatReviewFocus,
) -> Option<ChampPhaseAudit> {
    let initial_snapshot = champ_phase_snapshot(0, &root.combat)?;
    let witness = focus_witness_line(focus);
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut resources_before_split = ChampResourceTiming::default();
    let mut first_below_half_hp = None;
    let mut split_trigger = None;
    let mut post_split_snapshot = None;
    let mut replayed_actions = 0usize;
    let mut truncated = false;
    let mut timed_out = false;

    for (index, action) in witness.actions.iter().cloned().enumerate() {
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
        let step_index = index + 1;
        let before = champ_phase_snapshot(step_index - 1, &position.combat)?;
        if !before.champ_threshold_reached {
            note_champ_resource_before_split(
                &position,
                &action.input,
                step_index,
                &mut resources_before_split,
            );
        }
        let step = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: 250,
                deadline: None,
            },
        );
        replayed_actions = replayed_actions.saturating_add(1);
        truncated |= step.truncated;
        timed_out |= step.timed_out;
        if let Some(after) = champ_phase_snapshot(step_index, &step.position.combat) {
            if first_below_half_hp.is_none() && crossed_below_champ_half_hp(&before, &after) {
                first_below_half_hp = Some(ChampHpCrossing {
                    step_index,
                    action_key: action.action_key.clone(),
                    input: action.input.clone(),
                    before_champ_hp: before.champ_hp,
                    after_champ_hp: after.champ_hp,
                });
            }
            if split_trigger.is_none()
                && !before.champ_threshold_reached
                && after.champ_threshold_reached
            {
                post_split_snapshot = Some(after.clone());
                split_trigger = Some(ChampSplitTrigger {
                    step_index,
                    action_key: action.action_key,
                    input: action.input,
                    before,
                    after,
                });
            }
        }
        position = step.position;
        if truncated || timed_out || step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }

    let truncated_by_preview = witness
        .action_count
        .is_some_and(|count| count > witness.actions.len());
    let flags = champ_phase_flags(
        split_trigger.as_ref(),
        post_split_snapshot.as_ref(),
        &resources_before_split,
        truncated || timed_out || truncated_by_preview,
    );
    let verdict = champ_phase_verdict(
        &flags,
        split_trigger.is_some(),
        truncated || timed_out || truncated_by_preview,
    );

    Some(ChampPhaseAudit {
        schema: "champ_phase_audit_v0",
        contract: "exact_replay_timing_snapshot_only_no_search_policy_change_no_strategy_verdict",
        basis_line: focus.selected_review,
        witness_action_count: witness.action_count,
        replayed_actions,
        truncated_by_preview,
        truncated,
        timed_out,
        initial_snapshot,
        first_below_half_hp,
        split_trigger,
        post_split_snapshot,
        resources_before_split,
        flags,
        verdict,
    })
}

fn crossed_below_champ_half_hp(before: &ChampPhaseSnapshot, after: &ChampPhaseSnapshot) -> bool {
    before.champ_hp * 2 >= before.champ_max_hp && after.champ_hp * 2 < after.champ_max_hp
}

fn champ_phase_snapshot(step_index: usize, combat: &CombatState) -> Option<ChampPhaseSnapshot> {
    let champ = champ_entity(combat)?;
    Some(ChampPhaseSnapshot {
        step_index,
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        player_block: combat.entities.player.block,
        champ_hp: champ.current_hp,
        champ_max_hp: champ.max_hp,
        champ_block: champ.block,
        champ_strength: power_amount(combat, champ.id, PowerId::Strength),
        champ_weak: power_amount(combat, champ.id, PowerId::Weak),
        champ_vulnerable: power_amount(combat, champ.id, PowerId::Vulnerable),
        champ_threshold_reached: champ.champ.threshold_reached,
        champ_move_id: champ.planned_move_id(),
        total_enemy_hp: audit_total_enemy_hp(combat),
        living_enemy_count: audit_living_enemy_count(combat),
    })
}

fn champ_entity(combat: &CombatState) -> Option<&MonsterEntity> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::Champ))
}

fn note_champ_resource_before_split(
    position: &CombatPosition,
    input: &ClientInput,
    step_index: usize,
    resources: &mut ChampResourceTiming,
) {
    match input {
        ClientInput::PlayCard { card_index, .. } => {
            if position
                .combat
                .zones
                .hand
                .get(*card_index)
                .is_some_and(|card| card.id == CardId::Disarm)
            {
                resources.disarm_used_before_split = true;
                resources.disarm_step.get_or_insert(step_index);
            }
        }
        ClientInput::UsePotion { potion_index, .. } => {
            resources.potions_used_before_split =
                resources.potions_used_before_split.saturating_add(1);
            match position
                .combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|potion| potion.as_ref())
                .map(|potion| potion.id)
            {
                Some(PotionId::FearPotion) => {
                    resources.fear_potion_used_before_split = true;
                    resources.fear_potion_step.get_or_insert(step_index);
                }
                Some(PotionId::StrengthPotion) => {
                    resources.strength_potion_used_before_split = true;
                    resources.strength_potion_step.get_or_insert(step_index);
                }
                Some(PotionId::SteroidPotion) => {
                    resources.steroid_potion_used_before_split = true;
                    resources.steroid_potion_step.get_or_insert(step_index);
                }
                Some(PotionId::BlessingOfTheForge) => {
                    resources.forge_potion_used_before_split = true;
                    resources.forge_potion_step.get_or_insert(step_index);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn champ_phase_flags(
    split_trigger: Option<&ChampSplitTrigger>,
    post_split: Option<&ChampPhaseSnapshot>,
    resources: &ChampResourceTiming,
    replay_truncated: bool,
) -> Vec<ChampPhaseAuditFlag> {
    let mut flags = Vec::new();
    if split_trigger.is_some() {
        flags.push(ChampPhaseAuditFlag::SplitObserved);
    } else {
        flags.push(ChampPhaseAuditFlag::NoSplitReached);
    }
    if resources.disarm_used_before_split {
        flags.push(ChampPhaseAuditFlag::DisarmSpentBeforeSplit);
    }
    if resources.fear_potion_used_before_split {
        flags.push(ChampPhaseAuditFlag::FearPotionSpentBeforeSplit);
    }
    if resources.strength_potion_used_before_split || resources.steroid_potion_used_before_split {
        flags.push(ChampPhaseAuditFlag::BurstPotionSpentBeforeSplit);
    }
    if let Some(snapshot) = post_split {
        if snapshot.player_hp * 4 <= snapshot.player_max_hp.max(1) {
            flags.push(ChampPhaseAuditFlag::SplitWithLowHp);
        }
        if snapshot.champ_hp * 5 > snapshot.champ_max_hp * 2 {
            flags.push(ChampPhaseAuditFlag::SplitWithChampHpStillHigh);
        }
    }
    if replay_truncated {
        flags.push(ChampPhaseAuditFlag::ReplayTruncated);
    }
    flags
}

fn champ_phase_verdict(
    flags: &[ChampPhaseAuditFlag],
    split_observed: bool,
    replay_engine_limited: bool,
) -> ChampPhaseAuditVerdict {
    if replay_engine_limited {
        return ChampPhaseAuditVerdict::Unclear;
    }
    if !split_observed {
        return ChampPhaseAuditVerdict::NoSplitReached;
    }
    if flags
        .iter()
        .any(|flag| matches!(flag, ChampPhaseAuditFlag::SplitWithLowHp))
    {
        return ChampPhaseAuditVerdict::SplitWithLowHp;
    }
    if flags.iter().any(|flag| {
        matches!(
            flag,
            ChampPhaseAuditFlag::DisarmSpentBeforeSplit
                | ChampPhaseAuditFlag::FearPotionSpentBeforeSplit
                | ChampPhaseAuditFlag::BurstPotionSpentBeforeSplit
        )
    }) {
        return ChampPhaseAuditVerdict::ResourceSpentBeforeSplit;
    }
    ChampPhaseAuditVerdict::SplitObserved
}

fn audit_total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

fn audit_living_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
}
