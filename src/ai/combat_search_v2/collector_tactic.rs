use crate::content::monsters::{get_hp_range, EnemyId};
use crate::runtime::combat::{CombatState, MonsterEntity};

use super::CombatSearchActionPriorPluginId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct CollectorTacticValueV0 {
    applicable: i32,
    formation: i32,
    primary_progress: i32,
    secondary_progress: i32,
}

impl CollectorTacticValueV0 {
    pub(super) fn is_applicable(self) -> bool {
        self.applicable > 0
    }
}

pub(super) fn collector_tactic_value(
    combat: &CombatState,
    plugin: CombatSearchActionPriorPluginId,
) -> CollectorTacticValueV0 {
    let Some(collector) = living_enemy(combat, EnemyId::TheCollector) else {
        return CollectorTacticValueV0::default();
    };
    let torches = living_enemies(combat, EnemyId::TorchHead);

    match plugin {
        CombatSearchActionPriorPluginId::CollectorBossRace => CollectorTacticValueV0 {
            applicable: 1,
            formation: 0,
            primary_progress: -collector.current_hp,
            secondary_progress: 0,
        },
        CombatSearchActionPriorPluginId::CollectorSingleHeadControl => {
            let initial_spawn_window = torches.is_empty() && collector.collector.initial_spawn;
            let formation = if initial_spawn_window {
                2
            } else {
                match torches.len() {
                    1 => 3,
                    2.. => 2,
                    0 => 1,
                }
            };
            let (primary_progress, secondary_progress) = if initial_spawn_window {
                let expected_head_hp =
                    get_hp_range(EnemyId::TorchHead, combat.meta.ascension_level).0;
                (-expected_head_hp, -collector.current_hp)
            } else if torches.len() >= 2 {
                let focused_head_hp = torches
                    .iter()
                    .map(|torch| torch.current_hp)
                    .min()
                    .unwrap_or_default();
                (-focused_head_hp, -collector.current_hp)
            } else if let Some(torch) = torches.first() {
                (-collector.current_hp, -torch.current_hp)
            } else {
                (-collector.current_hp, 0)
            };
            CollectorTacticValueV0 {
                applicable: 1,
                formation,
                primary_progress,
                secondary_progress,
            }
        }
        CombatSearchActionPriorPluginId::Default
        | CombatSearchActionPriorPluginId::KeyCardOnline => CollectorTacticValueV0::default(),
    }
}

pub(super) fn collector_tactic_target_rank(
    combat: &CombatState,
    target: Option<usize>,
    plugin: CombatSearchActionPriorPluginId,
) -> i32 {
    let Some(target) = target else {
        return 0;
    };
    let Some(collector) = living_enemy(combat, EnemyId::TheCollector) else {
        return 0;
    };
    let torches = living_enemies(combat, EnemyId::TorchHead);

    match plugin {
        CombatSearchActionPriorPluginId::CollectorBossRace => {
            if target == collector.id {
                2
            } else {
                0
            }
        }
        CombatSearchActionPriorPluginId::CollectorSingleHeadControl => match torches.len() {
            2.. => {
                let focused_head = torches
                    .iter()
                    .min_by_key(|torch| (torch.current_hp, torch.id));
                if focused_head.is_some_and(|torch| target == torch.id) {
                    2
                } else if torches.iter().any(|torch| target == torch.id) {
                    1
                } else {
                    0
                }
            }
            1 => {
                if target == collector.id {
                    2
                } else if target == torches[0].id {
                    -2
                } else {
                    0
                }
            }
            0 => i32::from(target == collector.id) * 2,
        },
        CombatSearchActionPriorPluginId::Default
        | CombatSearchActionPriorPluginId::KeyCardOnline => 0,
    }
}

fn living_enemy(combat: &CombatState, enemy_id: EnemyId) -> Option<&MonsterEntity> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| is_living_enemy(monster, enemy_id))
}

fn living_enemies(combat: &CombatState, enemy_id: EnemyId) -> Vec<&MonsterEntity> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| is_living_enemy(monster, enemy_id))
        .collect()
}

fn is_living_enemy(monster: &MonsterEntity, enemy_id: EnemyId) -> bool {
    monster.is_alive_for_action() && EnemyId::from_id(monster.monster_type) == Some(enemy_id)
}
