use crate::content::monsters::EnemyId;
use crate::content::relics::RelicId;
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};

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
    RuleFilteredDistribution,
    FallbackUnknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonsterObservation {
    pub entity_id: usize,
    pub monster_type: usize,
    pub enemy_id: Option<EnemyId>,
    pub name: &'static str,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub slot: u8,
    pub turn_count: u32,
    pub next_move_byte: u8,
    pub current_intent_visible: bool,
    pub move_history: Vec<u8>,
    pub visible_powers: Vec<(PowerId, i32)>,
    pub lagavulin_idle_count: u8,
    pub lagavulin_is_out_triggered: bool,
    pub chosen_first_turn: bool,
    pub chosen_used_hex: bool,
    pub chosen_protocol_seeded: bool,
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
                && matches!(monster.current_intent, Intent::Unknown)
                && !monster.move_history.is_empty()
        })
}

pub fn build_monster_observation(
    combat: &CombatState,
    monster: &MonsterEntity,
) -> MonsterObservation {
    let enemy_id = EnemyId::from_id(monster.monster_type);
    MonsterObservation {
        entity_id: monster.id,
        monster_type: monster.monster_type,
        enemy_id,
        name: enemy_id.map(|id| id.get_name()).unwrap_or("Unknown"),
        current_hp: monster.current_hp,
        max_hp: monster.max_hp,
        block: monster.block,
        slot: monster.slot,
        turn_count: combat.turn.turn_count,
        next_move_byte: monster.next_move_byte,
        current_intent_visible: !matches!(monster.current_intent, Intent::Unknown),
        move_history: monster.move_history.iter().copied().collect(),
        visible_powers: visible_powers(combat, monster.id),
        lagavulin_idle_count: monster.lagavulin.idle_count,
        lagavulin_is_out_triggered: monster.lagavulin.is_out_triggered,
        chosen_first_turn: monster.chosen.first_turn,
        chosen_used_hex: monster.chosen.used_hex,
        chosen_protocol_seeded: monster.chosen.protocol_seeded,
    }
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
        .map(|monster| {
            let observation = build_monster_observation(combat, monster);
            if !hidden && observation.current_intent_visible {
                exact_visible_belief(monster)
            } else {
                predict_monster_belief(&observation, combat.meta.ascension_level)
            }
        })
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

fn visible_powers(combat: &CombatState, entity_id: usize) -> Vec<(PowerId, i32)> {
    combat
        .entities
        .power_db
        .get(&entity_id)
        .map(|powers| {
            powers
                .iter()
                .map(|power| (power.power_type, power.amount))
                .collect()
        })
        .unwrap_or_default()
}

fn exact_visible_belief(monster: &MonsterEntity) -> MonsterBeliefState {
    let (base_damage, hits) = intent_damage_and_hits(&monster.current_intent);
    MonsterBeliefState {
        entity_id: monster.id,
        monster_name: EnemyId::from_id(monster.monster_type)
            .map(|id| id.get_name())
            .unwrap_or("Unknown"),
        certainty: MonsterBeliefCertainty::Exact,
        predicted_moves: vec![PredictedMove {
            move_id: monster.next_move_byte,
            intent: monster.current_intent.clone(),
            base_damage,
            hits,
            probability: 1.0,
            rationale_key: Some("visible_intent"),
        }],
        public_state_complete: true,
        inference_source: MonsterInferenceSource::VisibleIntent,
        expected_incoming_damage: total_damage_for_intent(&monster.current_intent) as f32,
        max_incoming_damage: total_damage_for_intent(&monster.current_intent),
        attack_probability: if is_attack_intent(&monster.current_intent) {
            1.0
        } else {
            0.0
        },
        rationale_key: Some("visible_intent"),
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

fn build_belief(
    observation: &MonsterObservation,
    certainty: MonsterBeliefCertainty,
    inference_source: MonsterInferenceSource,
    predicted_moves: Vec<PredictedMove>,
    public_state_complete: bool,
    rationale_key: Option<&'static str>,
) -> MonsterBeliefState {
    let predicted_moves = collapse_predicted_moves(predicted_moves);
    let expected_incoming_damage = predicted_moves
        .iter()
        .map(|predicted| predicted.probability * total_damage_for_intent(&predicted.intent) as f32)
        .sum::<f32>();
    let max_incoming_damage = predicted_moves
        .iter()
        .map(|predicted| total_damage_for_intent(&predicted.intent))
        .max()
        .unwrap_or(0);
    let attack_probability = predicted_moves
        .iter()
        .filter(|predicted| is_attack_intent(&predicted.intent))
        .map(|predicted| predicted.probability)
        .sum::<f32>()
        .clamp(0.0, 1.0);
    MonsterBeliefState {
        entity_id: observation.entity_id,
        monster_name: observation.name,
        certainty,
        predicted_moves,
        public_state_complete,
        inference_source,
        expected_incoming_damage,
        max_incoming_damage,
        attack_probability,
        rationale_key,
    }
}

fn exact_belief(
    observation: &MonsterObservation,
    inference_source: MonsterInferenceSource,
    predicted_moves: Vec<PredictedMove>,
    rationale_key: Option<&'static str>,
) -> MonsterBeliefState {
    build_belief(
        observation,
        MonsterBeliefCertainty::Exact,
        inference_source,
        predicted_moves,
        true,
        rationale_key,
    )
}

fn distribution_belief(
    observation: &MonsterObservation,
    predicted_moves: Vec<PredictedMove>,
    rationale_key: Option<&'static str>,
) -> MonsterBeliefState {
    build_belief(
        observation,
        MonsterBeliefCertainty::Distribution,
        MonsterInferenceSource::RuleFilteredDistribution,
        predicted_moves,
        true,
        rationale_key,
    )
}

fn unknown_belief(
    observation: &MonsterObservation,
    rationale_key: Option<&'static str>,
) -> MonsterBeliefState {
    MonsterBeliefState {
        entity_id: observation.entity_id,
        monster_name: observation.name,
        certainty: MonsterBeliefCertainty::Unknown,
        predicted_moves: Vec::new(),
        public_state_complete: false,
        inference_source: MonsterInferenceSource::FallbackUnknown,
        expected_incoming_damage: 0.0,
        max_incoming_damage: 0,
        attack_probability: 0.0,
        rationale_key,
    }
}

fn collapse_predicted_moves(predicted_moves: Vec<PredictedMove>) -> Vec<PredictedMove> {
    let mut collapsed = Vec::<PredictedMove>::new();
    for predicted in predicted_moves
        .into_iter()
        .filter(|predicted| predicted.probability > 0.0)
    {
        if let Some(existing) = collapsed.iter_mut().find(|existing| {
            existing.move_id == predicted.move_id && existing.intent == predicted.intent
        }) {
            existing.probability += predicted.probability;
            if existing.rationale_key.is_none() {
                existing.rationale_key = predicted.rationale_key;
            }
        } else {
            collapsed.push(predicted);
        }
    }
    let total_probability = collapsed
        .iter()
        .map(|predicted| predicted.probability)
        .sum::<f32>();
    if total_probability > 0.0 {
        for predicted in &mut collapsed {
            predicted.probability /= total_probability;
        }
    }
    collapsed
}

fn predicted_move(
    move_id: u8,
    intent: Intent,
    probability: f32,
    rationale_key: Option<&'static str>,
) -> PredictedMove {
    let (base_damage, hits) = intent_damage_and_hits(&intent);
    PredictedMove {
        move_id,
        intent,
        base_damage,
        hits,
        probability,
        rationale_key,
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

pub fn predict_monster_belief(
    observation: &MonsterObservation,
    ascension_level: u8,
) -> MonsterBeliefState {
    use EnemyId::*;

    let Some(enemy_id) = observation.enemy_id else {
        return unknown_belief(observation, Some("unsupported_monster"));
    };

    match enemy_id {
        Cultist => predict_cultist(observation),
        Sentry => predict_sentry(observation, ascension_level),
        Lagavulin => predict_lagavulin(observation, ascension_level),
        Chosen => predict_chosen(observation, ascension_level),
        JawWorm => predict_jaw_worm(observation, ascension_level),
        LouseNormal => predict_louse(observation, ascension_level, false),
        LouseDefensive => predict_louse(observation, ascension_level, true),
        Byrd => predict_byrd(observation, ascension_level),
        BookOfStabbing => predict_book_of_stabbing(observation, ascension_level),
        TimeEater => predict_time_eater(observation, ascension_level),
        AwakenedOne => predict_awakened_one(observation),
        _ => unknown_belief(observation, Some("unsupported_monster")),
    }
}

fn predict_cultist(observation: &MonsterObservation) -> MonsterBeliefState {
    exact_belief(
        observation,
        MonsterInferenceSource::ExactRule,
        vec![if observation.move_history.is_empty() {
            predicted_move(3, Intent::Buff, 1.0, Some("cultist_incantation"))
        } else {
            predicted_move(
                1,
                Intent::Attack { damage: 6, hits: 1 },
                1.0,
                Some("cultist_dark_strike"),
            )
        }],
        Some("cultist_pattern"),
    )
}

fn predict_sentry(observation: &MonsterObservation, ascension_level: u8) -> MonsterBeliefState {
    exact_belief(
        observation,
        MonsterInferenceSource::ExactRule,
        vec![if observation.move_history.is_empty() {
            if observation.slot % 2 == 0 {
                predicted_move(3, Intent::Debuff, 1.0, Some("sentry_open_bolt"))
            } else {
                predicted_move(
                    4,
                    Intent::Attack {
                        damage: if ascension_level >= 3 { 10 } else { 9 },
                        hits: 1,
                    },
                    1.0,
                    Some("sentry_open_beam"),
                )
            }
        } else if observation.move_history.last() == Some(&4) {
            predicted_move(3, Intent::Debuff, 1.0, Some("sentry_alternate_bolt"))
        } else {
            predicted_move(
                4,
                Intent::Attack {
                    damage: if ascension_level >= 3 { 10 } else { 9 },
                    hits: 1,
                },
                1.0,
                Some("sentry_alternate_beam"),
            )
        }],
        Some("sentry_alternation"),
    )
}

fn predict_lagavulin(observation: &MonsterObservation, ascension_level: u8) -> MonsterBeliefState {
    let damage = if ascension_level >= 3 { 20 } else { 18 };
    if !observation.lagavulin_is_out_triggered && observation.lagavulin_idle_count < 3 {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                5,
                Intent::Sleep,
                1.0,
                Some("lagavulin_sleep_cycle"),
            )],
            Some("lagavulin_public_state"),
        );
    }

    let mut attack_count = 0;
    for &move_id in observation.move_history.iter().rev() {
        if matches!(move_id, 1 | 4 | 5) {
            break;
        }
        if move_id == 3 {
            attack_count += 1;
        }
    }
    exact_belief(
        observation,
        MonsterInferenceSource::ExactRule,
        vec![if attack_count >= 2 {
            predicted_move(1, Intent::StrongDebuff, 1.0, Some("lagavulin_debuff_cycle"))
        } else {
            predicted_move(
                3,
                Intent::Attack { damage, hits: 1 },
                1.0,
                Some("lagavulin_attack_cycle"),
            )
        }],
        Some("lagavulin_public_state"),
    )
}

fn predict_chosen(observation: &MonsterObservation, ascension_level: u8) -> MonsterBeliefState {
    let zap_dmg = if ascension_level >= 2 { 21 } else { 18 };
    let debilitate_dmg = if ascension_level >= 2 { 12 } else { 10 };
    let poke_dmg = if ascension_level >= 2 { 6 } else { 5 };
    let (first_turn, used_hex) = if observation.chosen_protocol_seeded {
        (observation.chosen_first_turn, observation.chosen_used_hex)
    } else {
        (
            observation.move_history.is_empty(),
            observation.move_history.contains(&4),
        )
    };

    if ascension_level >= 17 && !used_hex {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                4,
                Intent::StrongDebuff,
                1.0,
                Some("chosen_forced_hex"),
            )],
            Some("chosen_hex_gate"),
        );
    }
    if ascension_level < 17 {
        if first_turn {
            return exact_belief(
                observation,
                MonsterInferenceSource::ExactRule,
                vec![predicted_move(
                    5,
                    Intent::Attack {
                        damage: poke_dmg,
                        hits: 2,
                    },
                    1.0,
                    Some("chosen_first_turn_poke"),
                )],
                Some("chosen_first_turn"),
            );
        }
        if !used_hex {
            return exact_belief(
                observation,
                MonsterInferenceSource::ExactRule,
                vec![predicted_move(
                    4,
                    Intent::StrongDebuff,
                    1.0,
                    Some("chosen_forced_hex"),
                )],
                Some("chosen_hex_gate"),
            );
        }
    }

    let last_move = observation.move_history.last().copied().unwrap_or(0);
    if last_move != 3 && last_move != 2 {
        distribution_belief(
            observation,
            vec![
                predicted_move(
                    3,
                    Intent::AttackDebuff {
                        damage: debilitate_dmg,
                        hits: 1,
                    },
                    0.50,
                    Some("chosen_split_debilitate"),
                ),
                predicted_move(2, Intent::Debuff, 0.50, Some("chosen_split_drain")),
            ],
            Some("chosen_branch_standard"),
        )
    } else {
        distribution_belief(
            observation,
            vec![
                predicted_move(
                    1,
                    Intent::Attack {
                        damage: zap_dmg,
                        hits: 1,
                    },
                    0.40,
                    Some("chosen_split_zap"),
                ),
                predicted_move(
                    5,
                    Intent::Attack {
                        damage: poke_dmg,
                        hits: 2,
                    },
                    0.60,
                    Some("chosen_split_poke"),
                ),
            ],
            Some("chosen_branch_repeat_filter"),
        )
    }
}

fn predict_louse(
    observation: &MonsterObservation,
    ascension_level: u8,
    defensive: bool,
) -> MonsterBeliefState {
    let (bite_min, bite_max) = if ascension_level >= 17 {
        (9, 12)
    } else if ascension_level >= 7 {
        (4, 8)
    } else {
        (3, 7)
    };
    let utility = predicted_move(
        4,
        if defensive {
            Intent::Debuff
        } else {
            Intent::Buff
        },
        1.0,
        Some(if defensive {
            "louse_weaken"
        } else {
            "louse_strengthen"
        }),
    );
    let last = observation.move_history.last().copied();
    let prev = if observation.move_history.len() >= 2 {
        Some(observation.move_history[observation.move_history.len() - 2])
    } else {
        None
    };
    let last_two = |byte: u8| last == Some(byte) && prev == Some(byte);

    if (ascension_level >= 17 && last == Some(4)) || (ascension_level < 17 && last_two(4)) {
        return distribution_belief(
            observation,
            bite_damage_distribution(bite_min, bite_max, 1.0),
            Some("louse_attack_locked"),
        );
    }
    if last_two(3) {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![utility],
            Some("louse_repeat_break"),
        );
    }
    distribution_belief(
        observation,
        {
            let mut moves = vec![PredictedMove {
                probability: 0.25,
                ..utility
            }];
            moves.extend(bite_damage_distribution(bite_min, bite_max, 0.75));
            moves
        },
        Some("louse_default_split"),
    )
}

fn bite_damage_distribution(
    min_damage: i32,
    max_damage: i32,
    total_probability: f32,
) -> Vec<PredictedMove> {
    let outcomes = (max_damage - min_damage + 1).max(1) as f32;
    (min_damage..=max_damage)
        .map(|damage| {
            predicted_move(
                3,
                Intent::Attack { damage, hits: 1 },
                total_probability / outcomes,
                Some("louse_bite"),
            )
        })
        .collect()
}

fn predict_jaw_worm(observation: &MonsterObservation, ascension_level: u8) -> MonsterBeliefState {
    let chomp_dmg = if ascension_level >= 2 { 12 } else { 11 };
    let thrash_dmg = 7;
    if observation.move_history.is_empty() {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                1,
                Intent::Attack {
                    damage: chomp_dmg,
                    hits: 1,
                },
                1.0,
                Some("jaw_worm_first_chomp"),
            )],
            Some("jaw_worm_first_turn"),
        );
    }

    let last_move = observation.move_history.last().copied();
    let last_move_before = if observation.move_history.len() >= 2 {
        Some(observation.move_history[observation.move_history.len() - 2])
    } else {
        None
    };
    let last_two_moves = last_move.is_some() && last_move == last_move_before;
    let attack = predicted_move(
        1,
        Intent::Attack {
            damage: chomp_dmg,
            hits: 1,
        },
        1.0,
        Some("jaw_worm_chomp"),
    );
    let defend_buff = predicted_move(2, Intent::DefendBuff, 1.0, Some("jaw_worm_bellow"));
    let attack_defend = predicted_move(
        3,
        Intent::AttackDefend {
            damage: thrash_dmg,
            hits: 1,
        },
        1.0,
        Some("jaw_worm_thrash"),
    );

    let moves = if last_move == Some(1) {
        vec![
            PredictedMove {
                probability: 0.25 * 0.5625,
                ..defend_buff.clone()
            },
            PredictedMove {
                probability: 0.25 * (1.0 - 0.5625),
                ..attack_defend.clone()
            },
            PredictedMove {
                probability: 0.30,
                ..attack_defend
            },
            PredictedMove {
                probability: 0.45,
                ..defend_buff
            },
        ]
    } else if last_two_moves && last_move == Some(3) {
        vec![
            PredictedMove {
                probability: 0.25,
                ..attack.clone()
            },
            PredictedMove {
                probability: 0.30 * 0.357,
                ..attack.clone()
            },
            PredictedMove {
                probability: 0.30 * (1.0 - 0.357),
                ..predicted_move(2, Intent::DefendBuff, 1.0, Some("jaw_worm_bellow"))
            },
            PredictedMove {
                probability: 0.45,
                ..predicted_move(2, Intent::DefendBuff, 1.0, Some("jaw_worm_bellow"))
            },
        ]
    } else if last_move == Some(2) {
        vec![
            PredictedMove {
                probability: 0.25,
                ..attack
            },
            PredictedMove {
                probability: 0.30,
                ..attack_defend.clone()
            },
            PredictedMove {
                probability: 0.45 * 0.416,
                ..predicted_move(
                    1,
                    Intent::Attack {
                        damage: chomp_dmg,
                        hits: 1,
                    },
                    1.0,
                    Some("jaw_worm_chomp"),
                )
            },
            PredictedMove {
                probability: 0.45 * (1.0 - 0.416),
                ..attack_defend
            },
        ]
    } else {
        vec![
            PredictedMove {
                probability: 0.25,
                ..attack
            },
            PredictedMove {
                probability: 0.30,
                ..attack_defend
            },
            PredictedMove {
                probability: 0.45,
                ..predicted_move(2, Intent::DefendBuff, 1.0, Some("jaw_worm_bellow"))
            },
        ]
    };
    distribution_belief(observation, moves, Some("jaw_worm_rule_distribution"))
}

fn predict_byrd(observation: &MonsterObservation, ascension_level: u8) -> MonsterBeliefState {
    let peck_count = if ascension_level >= 2 { 6 } else { 5 };
    let swoop_dmg = if ascension_level >= 2 { 14 } else { 12 };
    let peck = predicted_move(
        1,
        Intent::Attack {
            damage: 1,
            hits: peck_count as u8,
        },
        1.0,
        Some("byrd_peck"),
    );
    let swoop = predicted_move(
        3,
        Intent::Attack {
            damage: swoop_dmg,
            hits: 1,
        },
        1.0,
        Some("byrd_swoop"),
    );
    let headbutt = predicted_move(
        5,
        Intent::Attack { damage: 3, hits: 1 },
        1.0,
        Some("byrd_headbutt"),
    );
    let caw = predicted_move(6, Intent::Buff, 1.0, Some("byrd_caw"));

    if observation.move_history.is_empty() {
        return distribution_belief(
            observation,
            vec![
                PredictedMove {
                    probability: 0.375,
                    ..caw
                },
                PredictedMove {
                    probability: 0.625,
                    ..peck
                },
            ],
            Some("byrd_open_split"),
        );
    }

    let is_flying = observation.move_history.last() != Some(&4);
    if !is_flying {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![headbutt],
            Some("byrd_grounded_headbutt"),
        );
    }

    let last_move = observation.move_history.last().copied();
    let last_two_pecks = observation
        .move_history
        .iter()
        .rev()
        .take(2)
        .copied()
        .collect::<Vec<_>>()
        == vec![1, 1];

    let moves = if last_two_pecks {
        vec![
            PredictedMove {
                probability: 0.50 * 0.40,
                ..swoop.clone()
            },
            PredictedMove {
                probability: 0.50 * 0.60,
                ..caw.clone()
            },
            PredictedMove {
                probability: 0.20,
                ..swoop.clone()
            },
            PredictedMove {
                probability: 0.30,
                ..caw
            },
        ]
    } else if last_move == Some(3) {
        vec![
            PredictedMove {
                probability: 0.50,
                ..peck.clone()
            },
            PredictedMove {
                probability: 0.20 * 0.375,
                ..predicted_move(6, Intent::Buff, 1.0, Some("byrd_caw"))
            },
            PredictedMove {
                probability: 0.20 * 0.625,
                ..predicted_move(
                    1,
                    Intent::Attack {
                        damage: 1,
                        hits: peck_count as u8,
                    },
                    1.0,
                    Some("byrd_peck"),
                )
            },
            PredictedMove {
                probability: 0.30,
                ..predicted_move(6, Intent::Buff, 1.0, Some("byrd_caw"))
            },
        ]
    } else if last_move == Some(6) {
        vec![
            PredictedMove {
                probability: 0.50,
                ..peck.clone()
            },
            PredictedMove {
                probability: 0.20,
                ..swoop.clone()
            },
            PredictedMove {
                probability: 0.30 * 0.2857,
                ..predicted_move(
                    3,
                    Intent::Attack {
                        damage: swoop_dmg,
                        hits: 1,
                    },
                    1.0,
                    Some("byrd_swoop"),
                )
            },
            PredictedMove {
                probability: 0.30 * (1.0 - 0.2857),
                ..predicted_move(
                    1,
                    Intent::Attack {
                        damage: 1,
                        hits: peck_count as u8,
                    },
                    1.0,
                    Some("byrd_peck"),
                )
            },
        ]
    } else {
        vec![
            PredictedMove {
                probability: 0.50,
                ..peck
            },
            PredictedMove {
                probability: 0.20,
                ..swoop
            },
            PredictedMove {
                probability: 0.30,
                ..predicted_move(6, Intent::Buff, 1.0, Some("byrd_caw"))
            },
        ]
    };
    distribution_belief(observation, moves, Some("byrd_rule_distribution"))
}

fn predict_book_of_stabbing(
    observation: &MonsterObservation,
    ascension_level: u8,
) -> MonsterBeliefState {
    let stab_dmg = if ascension_level >= 3 { 7 } else { 6 };
    let big_stab_dmg = if ascension_level >= 3 { 24 } else { 21 };
    let stab_hits = 1
        + observation
            .move_history
            .iter()
            .filter(|&&move_id| move_id == 1)
            .count() as u8
        + if ascension_level >= 18 {
            observation
                .move_history
                .iter()
                .filter(|&&move_id| move_id == 2)
                .count() as u8
        } else {
            0
        }
        + 1;
    let stab = predicted_move(
        1,
        Intent::Attack {
            damage: stab_dmg,
            hits: stab_hits,
        },
        1.0,
        Some("book_stab"),
    );
    let big = predicted_move(
        2,
        Intent::Attack {
            damage: big_stab_dmg,
            hits: 1,
        },
        1.0,
        Some("book_big_stab"),
    );

    let last = observation.move_history.last().copied().unwrap_or(0);
    let second_last = if observation.move_history.len() >= 2 {
        observation.move_history[observation.move_history.len() - 2]
    } else {
        0
    };
    let last_two_stabs = observation.move_history.len() >= 2 && last == 1 && second_last == 1;
    if last_two_stabs {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![big],
            Some("book_repeat_break"),
        );
    }
    if last == 2 {
        return distribution_belief(
            observation,
            vec![
                PredictedMove {
                    probability: 0.15,
                    ..stab
                },
                PredictedMove {
                    probability: 0.85,
                    ..big
                },
            ],
            Some("book_big_to_split"),
        );
    }
    distribution_belief(
        observation,
        vec![
            PredictedMove {
                probability: 0.15,
                ..big
            },
            PredictedMove {
                probability: 0.85,
                ..stab
            },
        ],
        Some("book_default_split"),
    )
}

fn predict_time_eater(observation: &MonsterObservation, ascension_level: u8) -> MonsterBeliefState {
    let is_half_hp = observation.current_hp < observation.max_hp / 2;
    let used_haste = observation.move_history.contains(&5);
    let reverb_dmg = if ascension_level >= 4 { 8 } else { 7 };
    let head_slam_dmg = if ascension_level >= 4 { 32 } else { 26 };
    let reverb = predicted_move(
        2,
        Intent::Attack {
            damage: reverb_dmg,
            hits: 3,
        },
        1.0,
        Some("time_eater_reverb"),
    );
    let ripple = predicted_move(3, Intent::DefendDebuff, 1.0, Some("time_eater_ripple"));
    let head_slam = predicted_move(
        4,
        Intent::AttackDebuff {
            damage: head_slam_dmg,
            hits: 1,
        },
        1.0,
        Some("time_eater_head_slam"),
    );
    if is_half_hp && !used_haste {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                5,
                Intent::Buff,
                1.0,
                Some("time_eater_haste"),
            )],
            Some("time_eater_haste_gate"),
        );
    }

    let last_move = observation.move_history.last().copied().unwrap_or(0);
    let last_two = |byte: u8| {
        observation.move_history.len() >= 2
            && observation.move_history[observation.move_history.len() - 1] == byte
            && observation.move_history[observation.move_history.len() - 2] == byte
    };

    if last_two(2) {
        return distribution_belief(
            observation,
            vec![
                PredictedMove {
                    probability: 0.45,
                    ..head_slam.clone()
                },
                PredictedMove {
                    probability: 0.35,
                    ..head_slam.clone()
                },
                PredictedMove {
                    probability: 0.20,
                    ..if last_move != 3 { ripple } else { head_slam }
                },
            ],
            Some("time_eater_reverb_repeat_filter"),
        );
    }
    if last_move == 4 {
        return distribution_belief(
            observation,
            vec![
                PredictedMove {
                    probability: 0.45,
                    ..reverb.clone()
                },
                PredictedMove {
                    probability: 0.35 * 0.66,
                    ..reverb.clone()
                },
                PredictedMove {
                    probability: 0.35 * (1.0 - 0.66),
                    ..ripple.clone()
                },
                PredictedMove {
                    probability: 0.20,
                    ..ripple
                },
            ],
            Some("time_eater_after_head_slam"),
        );
    }
    if last_move == 3 {
        return distribution_belief(
            observation,
            vec![
                PredictedMove {
                    probability: 0.45,
                    ..reverb.clone()
                },
                PredictedMove {
                    probability: 0.35,
                    ..head_slam.clone()
                },
                PredictedMove {
                    probability: 0.20,
                    ..head_slam
                },
            ],
            Some("time_eater_after_ripple"),
        );
    }
    distribution_belief(
        observation,
        vec![
            PredictedMove {
                probability: 0.45,
                ..reverb
            },
            PredictedMove {
                probability: 0.35,
                ..head_slam
            },
            PredictedMove {
                probability: 0.20,
                ..predicted_move(3, Intent::DefendDebuff, 1.0, Some("time_eater_ripple"))
            },
        ],
        Some("time_eater_default_split"),
    )
}

fn predict_awakened_one(observation: &MonsterObservation) -> MonsterBeliefState {
    let is_phase_one = !observation.move_history.contains(&3);
    if observation.current_hp <= 0 && is_phase_one {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                3,
                Intent::Unknown,
                1.0,
                Some("awakened_one_rebirth"),
            )],
            Some("awakened_one_rebirth"),
        );
    }

    if is_phase_one {
        if observation.move_history.is_empty() {
            return exact_belief(
                observation,
                MonsterInferenceSource::ExactRule,
                vec![predicted_move(
                    1,
                    Intent::Attack {
                        damage: 20,
                        hits: 1,
                    },
                    1.0,
                    Some("awakened_one_open_slash"),
                )],
                Some("awakened_one_phase1_open"),
            );
        }

        let last_move = observation.move_history.last().copied().unwrap_or(0);
        let last_two_slash = observation
            .move_history
            .iter()
            .rev()
            .take(2)
            .copied()
            .collect::<Vec<_>>()
            == vec![1, 1];
        if last_move == 2 {
            return exact_belief(
                observation,
                MonsterInferenceSource::ExactRule,
                vec![predicted_move(
                    1,
                    Intent::Attack {
                        damage: 20,
                        hits: 1,
                    },
                    1.0,
                    Some("awakened_one_after_soul_strike"),
                )],
                Some("awakened_one_phase1_filter"),
            );
        }
        if last_two_slash {
            return exact_belief(
                observation,
                MonsterInferenceSource::ExactRule,
                vec![predicted_move(
                    2,
                    Intent::Attack { damage: 6, hits: 4 },
                    1.0,
                    Some("awakened_one_repeat_break"),
                )],
                Some("awakened_one_phase1_filter"),
            );
        }
        return distribution_belief(
            observation,
            vec![
                predicted_move(
                    2,
                    Intent::Attack { damage: 6, hits: 4 },
                    0.25,
                    Some("awakened_one_soul_strike"),
                ),
                predicted_move(
                    1,
                    Intent::Attack {
                        damage: 20,
                        hits: 1,
                    },
                    0.75,
                    Some("awakened_one_slash"),
                ),
            ],
            Some("awakened_one_phase1_split"),
        );
    }

    let last_move = observation.move_history.last().copied().unwrap_or(0);
    if last_move == 3 {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                5,
                Intent::Attack {
                    damage: 40,
                    hits: 1,
                },
                1.0,
                Some("awakened_one_dark_echo"),
            )],
            Some("awakened_one_phase2_open"),
        );
    }

    let last_two = |byte: u8| {
        observation.move_history.len() >= 2
            && observation.move_history[observation.move_history.len() - 1] == byte
            && observation.move_history[observation.move_history.len() - 2] == byte
    };
    if last_two(6) {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                8,
                Intent::Attack {
                    damage: 10,
                    hits: 3,
                },
                1.0,
                Some("awakened_one_phase2_tackle"),
            )],
            Some("awakened_one_phase2_repeat_break"),
        );
    }
    if last_two(8) {
        return exact_belief(
            observation,
            MonsterInferenceSource::ExactRule,
            vec![predicted_move(
                6,
                Intent::AttackDebuff {
                    damage: 18,
                    hits: 1,
                },
                1.0,
                Some("awakened_one_phase2_sludge"),
            )],
            Some("awakened_one_phase2_repeat_break"),
        );
    }
    distribution_belief(
        observation,
        vec![
            predicted_move(
                6,
                Intent::AttackDebuff {
                    damage: 18,
                    hits: 1,
                },
                0.50,
                Some("awakened_one_phase2_sludge"),
            ),
            predicted_move(
                8,
                Intent::Attack {
                    damage: 10,
                    hits: 3,
                },
                0.50,
                Some("awakened_one_phase2_tackle"),
            ),
        ],
        Some("awakened_one_phase2_split"),
    )
}
