use crate::content::monsters::{resolve_monster_turn_plan, EnemyId};
use crate::content::relics::RelicId;
use crate::projection::combat::MonsterMovePreview;
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};
use crate::semantics::combat::{EffectStrength, MonsterMoveSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonsterBeliefCertainty {
    Exact,
    Distribution,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonsterInferenceSource {
    VisibleIntent,
    ExactRule,
    FallbackUnknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictedMove {
    pub move_id: u8,
    pub intent: Intent,
    pub base_damage: i32,
    pub hits: u8,
    pub probability: f32,
    pub rationale_key: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonsterBeliefState {
    pub entity_id: usize,
    pub monster_name: &'static str,
    pub certainty: MonsterBeliefCertainty,
    pub predicted_moves: Vec<PredictedMove>,
    pub public_state_complete: bool,
    pub inference_source: MonsterInferenceSource,
    pub expected_incoming_damage: f32,
    pub max_incoming_damage: i32,
    pub attack_probability: f32,
    pub rationale_key: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatBeliefState {
    pub hidden_intent_active: bool,
    pub monsters: Vec<MonsterBeliefState>,
    pub expected_incoming_damage: f32,
    pub max_incoming_damage: i32,
    pub attack_probability: f32,
    pub lethal_probability: f32,
    pub urgent_probability: f32,
    pub public_state_complete: bool,
}

pub fn hidden_intent_active(combat: &CombatState) -> bool {
    combat.entities.player.has_relic(RelicId::RunicDome)
        || combat.entities.monsters.iter().any(|monster| {
            !monster.is_dying
                && !monster.is_escaped
                && !monster.half_dead
                && monster.current_hp > 0
                && !combat.monster_has_protocol_visible_intent(monster.id)
        })
}

pub fn build_combat_belief_state(combat: &CombatState) -> CombatBeliefState {
    let hidden = hidden_intent_active(combat);
    let monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .map(|monster| build_monster_belief_state(combat, monster))
        .collect::<Vec<_>>();

    let expected_incoming_damage = monsters
        .iter()
        .map(|belief| belief.expected_incoming_damage)
        .sum::<f32>();
    let max_incoming_damage = monsters
        .iter()
        .map(|belief| belief.max_incoming_damage)
        .sum::<i32>();
    let attack_probability = 1.0
        - monsters.iter().fold(1.0f32, |acc, belief| {
            acc * (1.0 - belief.attack_probability)
        });
    let damage_distribution = convolve_damage_distribution(&monsters);
    let player_block = combat.entities.player.block.max(0);
    let player_hp = combat.entities.player.current_hp.max(1);
    let lethal_probability = damage_distribution
        .iter()
        .filter(|(damage, _)| (*damage - player_block).max(0) >= player_hp)
        .map(|(_, probability)| *probability)
        .sum::<f32>()
        .clamp(0.0, 1.0);
    let urgent_probability = damage_distribution
        .iter()
        .filter(|(damage, _)| (*damage - player_block).max(0) >= 8)
        .map(|(_, probability)| *probability)
        .sum::<f32>()
        .clamp(0.0, 1.0);

    CombatBeliefState {
        hidden_intent_active: hidden,
        public_state_complete: monsters.iter().all(|belief| belief.public_state_complete),
        monsters,
        expected_incoming_damage,
        max_incoming_damage,
        attack_probability,
        lethal_probability,
        urgent_probability,
    }
}

pub fn total_damage_for_intent(intent: &Intent) -> i32 {
    match intent {
        Intent::Attack { damage, hits }
        | Intent::AttackBuff { damage, hits }
        | Intent::AttackDebuff { damage, hits }
        | Intent::AttackDefend { damage, hits } => (*damage * *hits as i32).max(0),
        _ => 0,
    }
}

pub fn is_attack_intent(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::Attack { .. }
            | Intent::AttackBuff { .. }
            | Intent::AttackDebuff { .. }
            | Intent::AttackDefend { .. }
    )
}

fn build_monster_belief_state(combat: &CombatState, monster: &MonsterEntity) -> MonsterBeliefState {
    let plan = resolve_monster_turn_plan(combat, monster);
    let spec = plan.summary_spec();
    let preview = preview_for_plan(combat, monster, &plan);
    let is_public = is_public_plan(combat, monster);
    let monster_name = EnemyId::from_id(monster.monster_type)
        .map(|id| id.get_name())
        .unwrap_or("Unknown");

    if matches!(spec, MonsterMoveSpec::Unknown) {
        return MonsterBeliefState {
            entity_id: monster.id,
            monster_name,
            certainty: MonsterBeliefCertainty::Unknown,
            predicted_moves: Vec::new(),
            public_state_complete: false,
            inference_source: MonsterInferenceSource::FallbackUnknown,
            expected_incoming_damage: 0.0,
            max_incoming_damage: 0,
            attack_probability: 0.0,
            rationale_key: Some("unknown_plan"),
        };
    }

    let intent = if is_public {
        combat.monster_protocol_visible_intent(monster.id).clone()
    } else {
        intent_from_spec(&spec)
    };
    let (base_damage, hits) = intent_damage_and_hits(&intent);
    let total_damage = preview
        .total_damage
        .unwrap_or_else(|| total_damage_for_intent(&intent));
    let attack_probability = if preview.damage_per_hit.is_some() || is_attack_intent(&intent) {
        1.0
    } else {
        0.0
    };

    MonsterBeliefState {
        entity_id: monster.id,
        monster_name,
        certainty: if is_public {
            MonsterBeliefCertainty::Exact
        } else {
            MonsterBeliefCertainty::Distribution
        },
        predicted_moves: vec![PredictedMove {
            move_id: plan.move_id,
            intent,
            base_damage,
            hits,
            probability: 1.0,
            rationale_key: Some(if is_public {
                "visible_intent"
            } else {
                "semantic_plan"
            }),
        }],
        public_state_complete: is_public,
        inference_source: if is_public {
            MonsterInferenceSource::VisibleIntent
        } else {
            MonsterInferenceSource::ExactRule
        },
        expected_incoming_damage: total_damage as f32,
        max_incoming_damage: total_damage,
        attack_probability,
        rationale_key: Some(if is_public {
            "visible_intent"
        } else {
            "semantic_plan"
        }),
    }
}

fn is_public_plan(combat: &CombatState, monster: &MonsterEntity) -> bool {
    !combat.entities.player.has_relic(RelicId::RunicDome)
        && combat.monster_has_protocol_visible_intent(monster.id)
}

fn preview_for_plan(
    combat: &CombatState,
    monster: &MonsterEntity,
    plan: &crate::semantics::combat::MonsterTurnPlan,
) -> MonsterMovePreview {
    let visible_damage = if combat.monster_has_protocol_visible_intent(monster.id) {
        let observed = combat
            .monster_protocol_preview_damage_per_hit(monster.id)
            .max(0);
        (observed > 0).then_some(observed)
    } else {
        None
    };
    let semantic_damage = plan.attack().map(|attack| attack.base_damage.max(0));
    MonsterMovePreview::from_plan(plan, visible_damage.or(semantic_damage))
}

fn intent_from_spec(spec: &MonsterMoveSpec) -> Intent {
    match spec {
        MonsterMoveSpec::Attack(attack) => Intent::Attack {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AttackAddCard(attack, _) => Intent::AttackDebuff {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AttackUpgradeCards(attack, _) => Intent::AttackDebuff {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AttackBuff(attack, _) => Intent::AttackBuff {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AttackSustain(attack) => Intent::AttackBuff {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AttackDebuff(attack, _) => Intent::AttackDebuff {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AttackDefend(attack, _) => Intent::AttackDefend {
            damage: attack.base_damage,
            hits: attack.hits,
        },
        MonsterMoveSpec::AddCard(add_card) => match add_card.visible_strength {
            EffectStrength::Strong => Intent::StrongDebuff,
            EffectStrength::Normal => Intent::Debuff,
        },
        MonsterMoveSpec::Buff(_) => Intent::Buff,
        MonsterMoveSpec::Heal(_) => Intent::Buff,
        MonsterMoveSpec::Debuff(_) => Intent::Debuff,
        MonsterMoveSpec::StrongDebuff(_) => Intent::StrongDebuff,
        MonsterMoveSpec::Defend(_) => Intent::Defend,
        MonsterMoveSpec::DefendDebuff(_, _) => Intent::DefendDebuff,
        MonsterMoveSpec::DefendBuff(_, _) => Intent::DefendBuff,
        MonsterMoveSpec::Escape => Intent::Escape,
        MonsterMoveSpec::Magic => Intent::Magic,
        MonsterMoveSpec::Sleep => Intent::Sleep,
        MonsterMoveSpec::Stun => Intent::Stun,
        MonsterMoveSpec::Debug => Intent::Debug,
        MonsterMoveSpec::None => Intent::None,
        MonsterMoveSpec::Unknown => Intent::Unknown,
    }
}

fn intent_damage_and_hits(intent: &Intent) -> (i32, u8) {
    match intent {
        Intent::Attack { damage, hits }
        | Intent::AttackBuff { damage, hits }
        | Intent::AttackDebuff { damage, hits }
        | Intent::AttackDefend { damage, hits } => (*damage, *hits),
        _ => (0, 0),
    }
}

fn convolve_damage_distribution(monster_beliefs: &[MonsterBeliefState]) -> Vec<(i32, f32)> {
    let mut distribution = vec![(0, 1.0f32)];
    for belief in monster_beliefs {
        if belief.predicted_moves.is_empty() {
            continue;
        }
        let mut next = std::collections::BTreeMap::<i32, f32>::new();
        for (base_damage, base_probability) in &distribution {
            for predicted in &belief.predicted_moves {
                let total_damage = base_damage + total_damage_for_intent(&predicted.intent);
                *next.entry(total_damage).or_insert(0.0) +=
                    base_probability * predicted.probability;
            }
        }
        distribution = next.into_iter().collect();
    }
    distribution
}
