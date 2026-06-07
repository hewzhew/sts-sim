use crate::ai::route_planner_v1::{route_targets, summarize_route_from, RoutePlannerConfigV1};
use crate::content::relics::RelicId;
use crate::state::events::{EventCardKind, EventEffect, EventOption, EventRelicKind};
use crate::state::map::node::RoomType;
use crate::state::run::RunState;

#[derive(Clone, Debug, PartialEq)]
pub struct NeowDecisionInputV1 {
    pub player_class: String,
    pub map: NeowMapFeaturesV1,
    pub choices: Vec<NeowChoiceInputV1>,
    pub config: NeowGuidanceConfigV1,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NeowMapFeaturesV1 {
    pub early_shop_available: bool,
    pub shop_before_first_elite: bool,
    pub early_elite_available: bool,
    pub lament_elite_snipe_candidate: bool,
    pub path_flexibility: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowGuidanceConfigV1 {
    pub early_shop_gold_bonus: f32,
    pub shop_before_elite_gold_bonus: f32,
    pub lament_elite_snipe_bonus: f32,
    pub early_elite_potion_bonus: f32,
    pub early_elite_immediate_bonus: f32,
    pub ironclad_boss_swap_penalty: f32,
    pub low_flex_boss_swap_penalty: f32,
    pub boss_swap_variance_penalty: f32,
}

impl Default for NeowGuidanceConfigV1 {
    fn default() -> Self {
        Self {
            early_shop_gold_bonus: 2.5,
            shop_before_elite_gold_bonus: 1.0,
            lament_elite_snipe_bonus: 5.0,
            early_elite_potion_bonus: 1.7,
            early_elite_immediate_bonus: 0.7,
            ironclad_boss_swap_penalty: 1.6,
            low_flex_boss_swap_penalty: 1.2,
            boss_swap_variance_penalty: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowChoiceInputV1 {
    pub index: usize,
    pub label: String,
    pub effects: Vec<EventEffect>,
    pub class: NeowChoiceClassV1,
}

impl NeowChoiceInputV1 {
    pub fn from_effects(index: usize, label: impl Into<String>, effects: Vec<EventEffect>) -> Self {
        let class = classify_neow_choice(&effects);
        Self {
            index,
            label: label.into(),
            effects,
            class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeowChoiceClassV1 {
    Lament,
    MaxHp,
    Gold,
    Potions,
    CardReward,
    RareCardReward,
    ColorlessCardReward,
    CommonRelic,
    RareRelic,
    BossSwap,
    Remove,
    Upgrade,
    Transform,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowDecisionTraceV1 {
    pub label_role: &'static str,
    pub map: NeowMapFeaturesV1,
    pub candidates: Vec<NeowCandidateTraceV1>,
    pub selected_index: Option<usize>,
}

impl NeowDecisionTraceV1 {
    pub fn selected(&self) -> Option<&NeowCandidateTraceV1> {
        self.selected_index
            .and_then(|index| self.candidates.get(index))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowCandidateTraceV1 {
    pub index: usize,
    pub label: String,
    pub class: NeowChoiceClassV1,
    pub terms: NeowScoreTermsV1,
    pub total: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeowScoreTermsV1 {
    pub immediate_power: f32,
    pub first_elite_security: f32,
    pub boss_matchup_help: f32,
    pub shop_convertibility: f32,
    pub path_flexibility: f32,
    pub character_synergy: f32,
    pub downside_cost: f32,
    pub variance_penalty: f32,
}

impl NeowScoreTermsV1 {
    pub fn total(self) -> f32 {
        self.immediate_power
            + self.first_elite_security
            + self.boss_matchup_help
            + self.shop_convertibility
            + self.path_flexibility
            + self.character_synergy
            + self.downside_cost
            + self.variance_penalty
    }
}

pub fn choices_from_event_options_v1(options: &[EventOption]) -> Vec<NeowChoiceInputV1> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            NeowChoiceInputV1::from_effects(
                index,
                option.ui.text.clone(),
                option.semantics.effects.clone(),
            )
        })
        .collect()
}

pub fn rank_neow_choices_v1(input: NeowDecisionInputV1) -> NeowDecisionTraceV1 {
    let mut candidates = input
        .choices
        .iter()
        .map(|choice| {
            let terms = score_neow_choice(choice, &input.player_class, &input.map, &input.config);
            NeowCandidateTraceV1 {
                index: choice.index,
                label: choice.label.clone(),
                class: choice.class,
                terms,
                total: terms.total(),
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .total
            .partial_cmp(&left.total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.index.cmp(&right.index))
    });
    NeowDecisionTraceV1 {
        label_role: "behavior_policy_not_teacher",
        map: input.map,
        selected_index: (!candidates.is_empty()).then_some(0),
        candidates,
    }
}

pub fn neow_map_features_from_run_state_v1(run_state: &RunState) -> NeowMapFeaturesV1 {
    let config = RoutePlannerConfigV1::default();
    let targets = route_targets(run_state);
    let summaries = targets
        .iter()
        .map(|target| summarize_route_from(run_state, target.x, target.y, &config))
        .collect::<Vec<_>>();
    let early_shop_available = summaries
        .iter()
        .filter_map(|summary| summary.first_shop_floor)
        .any(|floor| floor <= 4);
    let early_elite_available = has_elite_by_floor(run_state, 6);
    let shop_before_first_elite = has_shop_before_first_elite(run_state);
    let lament_elite_snipe_candidate = has_lament_elite_snipe_candidate(run_state);
    let path_count = summaries
        .iter()
        .map(|summary| summary.path_count)
        .sum::<usize>();
    let path_flexibility =
        ((targets.len() as f32) / 4.0 + (path_count as f32).log10() / 3.0).clamp(0.0, 1.0);

    NeowMapFeaturesV1 {
        early_shop_available,
        shop_before_first_elite,
        early_elite_available,
        lament_elite_snipe_candidate,
        path_flexibility,
    }
}

fn classify_neow_choice(effects: &[EventEffect]) -> NeowChoiceClassV1 {
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::Specific(RelicId::NeowsLament),
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::Lament;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::LoseStarterRelic { .. }))
        && effects.iter().any(|effect| {
            matches!(
                effect,
                EventEffect::ObtainRelic {
                    kind: EventRelicKind::RandomBossRelic,
                    ..
                }
            )
        })
    {
        return NeowChoiceClassV1::BossSwap;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::RandomCommonRelic,
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::CommonRelic;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::RandomRareRelic,
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::RareRelic;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::GainGold(amount) if *amount > 0))
    {
        return NeowChoiceClassV1::Gold;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::ObtainPotion { .. }))
    {
        return NeowChoiceClassV1::Potions;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainCard {
                kind: EventCardKind::RandomClassRare,
                ..
            } | EventEffect::OfferCards {
                kind: EventCardKind::RandomClassRare,
                ..
            }
        )
    }) {
        return NeowChoiceClassV1::RareCardReward;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::OfferCards {
                kind: EventCardKind::RandomColorless
                    | EventCardKind::RandomColorlessUncommon
                    | EventCardKind::RandomColorlessRare,
                ..
            } | EventEffect::ObtainColorlessCard { .. }
        )
    }) {
        return NeowChoiceClassV1::ColorlessCardReward;
    }
    if effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainCard { .. } | EventEffect::OfferCards { .. }
        )
    }) {
        return NeowChoiceClassV1::CardReward;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::RemoveCard { .. }))
    {
        return NeowChoiceClassV1::Remove;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::UpgradeCard { .. }))
    {
        return NeowChoiceClassV1::Upgrade;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::TransformCard { .. }))
    {
        return NeowChoiceClassV1::Transform;
    }
    if effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::GainMaxHp(amount) if *amount > 0))
    {
        return NeowChoiceClassV1::MaxHp;
    }
    NeowChoiceClassV1::Unknown
}

fn score_neow_choice(
    choice: &NeowChoiceInputV1,
    player_class: &str,
    map: &NeowMapFeaturesV1,
    config: &NeowGuidanceConfigV1,
) -> NeowScoreTermsV1 {
    let mut terms = NeowScoreTermsV1 {
        immediate_power: base_immediate_power(choice.class),
        downside_cost: downside_cost(&choice.effects),
        variance_penalty: variance_penalty(choice.class),
        ..NeowScoreTermsV1::default()
    };

    match choice.class {
        NeowChoiceClassV1::Gold => {
            if map.early_shop_available {
                terms.shop_convertibility += config.early_shop_gold_bonus;
            }
            if map.shop_before_first_elite {
                terms.shop_convertibility += config.shop_before_elite_gold_bonus;
            }
        }
        NeowChoiceClassV1::Lament => {
            if map.lament_elite_snipe_candidate {
                terms.first_elite_security += config.lament_elite_snipe_bonus;
            } else if map.early_elite_available {
                terms.first_elite_security += 1.0;
            }
        }
        NeowChoiceClassV1::Potions => {
            if map.early_elite_available {
                terms.first_elite_security += config.early_elite_potion_bonus;
            }
        }
        NeowChoiceClassV1::CardReward
        | NeowChoiceClassV1::RareCardReward
        | NeowChoiceClassV1::ColorlessCardReward
        | NeowChoiceClassV1::CommonRelic
        | NeowChoiceClassV1::RareRelic
        | NeowChoiceClassV1::Transform => {
            if map.early_elite_available {
                terms.first_elite_security += config.early_elite_immediate_bonus;
            }
        }
        NeowChoiceClassV1::BossSwap => {
            terms.path_flexibility += map.path_flexibility;
            if map.path_flexibility < 0.4 {
                terms.downside_cost -= config.low_flex_boss_swap_penalty;
            }
            terms.variance_penalty -= config.boss_swap_variance_penalty;
        }
        _ => {}
    }

    terms.character_synergy += character_synergy(choice.class, player_class, config);
    terms
}

fn base_immediate_power(class: NeowChoiceClassV1) -> f32 {
    match class {
        NeowChoiceClassV1::Lament => 1.0,
        NeowChoiceClassV1::MaxHp => 1.2,
        NeowChoiceClassV1::Gold => 1.4,
        NeowChoiceClassV1::Potions => 2.4,
        NeowChoiceClassV1::CardReward => 2.6,
        NeowChoiceClassV1::RareCardReward => 2.9,
        NeowChoiceClassV1::ColorlessCardReward => 2.2,
        NeowChoiceClassV1::CommonRelic => 2.8,
        NeowChoiceClassV1::RareRelic => 2.7,
        NeowChoiceClassV1::BossSwap => 2.2,
        NeowChoiceClassV1::Remove => 1.2,
        NeowChoiceClassV1::Upgrade => 1.1,
        NeowChoiceClassV1::Transform => 2.3,
        NeowChoiceClassV1::Unknown => 0.0,
    }
}

fn variance_penalty(class: NeowChoiceClassV1) -> f32 {
    match class {
        NeowChoiceClassV1::BossSwap => -0.4,
        NeowChoiceClassV1::RareRelic | NeowChoiceClassV1::Transform => -0.3,
        NeowChoiceClassV1::ColorlessCardReward => -0.2,
        _ => 0.0,
    }
}

fn character_synergy(
    class: NeowChoiceClassV1,
    player_class: &str,
    config: &NeowGuidanceConfigV1,
) -> f32 {
    match player_class.to_ascii_lowercase().as_str() {
        "ironclad" | "red" => match class {
            NeowChoiceClassV1::BossSwap => -config.ironclad_boss_swap_penalty,
            NeowChoiceClassV1::RareCardReward => 0.4,
            NeowChoiceClassV1::CommonRelic => 0.1,
            _ => 0.0,
        },
        "silent" | "green" => match class {
            NeowChoiceClassV1::Potions
            | NeowChoiceClassV1::CardReward
            | NeowChoiceClassV1::Transform
            | NeowChoiceClassV1::Gold => 0.4,
            NeowChoiceClassV1::Remove => -0.2,
            _ => 0.0,
        },
        "defect" | "blue" => match class {
            NeowChoiceClassV1::BossSwap => 0.2,
            NeowChoiceClassV1::CommonRelic | NeowChoiceClassV1::CardReward => 0.2,
            _ => 0.0,
        },
        "watcher" | "purple" => match class {
            NeowChoiceClassV1::Remove | NeowChoiceClassV1::Transform => 0.8,
            NeowChoiceClassV1::Upgrade => 0.5,
            NeowChoiceClassV1::BossSwap => -0.4,
            _ => 0.0,
        },
        _ => 0.0,
    }
}

fn downside_cost(effects: &[EventEffect]) -> f32 {
    effects.iter().map(effect_downside_cost).sum()
}

fn effect_downside_cost(effect: &EventEffect) -> f32 {
    match effect {
        EventEffect::LoseHp(amount) => -((*amount as f32) / 10.0).min(2.5),
        EventEffect::LoseMaxHp(amount) => -((*amount as f32) / 5.0).min(2.2),
        EventEffect::LoseGold(amount) => -((*amount as f32) / 100.0).min(2.5),
        EventEffect::ObtainCurse { .. } => -1.8,
        EventEffect::LoseStarterRelic { .. } => -0.8,
        _ => 0.0,
    }
}

fn has_elite_by_floor(run_state: &RunState, max_visible_floor: i32) -> bool {
    run_state.map.graph.iter().any(|row| {
        row.iter().any(|node| {
            node.y + 1 <= max_visible_floor && node.class == Some(RoomType::MonsterRoomElite)
        })
    })
}

fn has_shop_before_first_elite(run_state: &RunState) -> bool {
    route_targets(run_state).iter().any(|target| {
        let mut stack = vec![(target.x, target.y, None::<i32>, None::<i32>)];
        while let Some((x, y, first_shop, first_elite)) = stack.pop() {
            let Some(node) = run_state
                .map
                .graph
                .get(y.max(0) as usize)
                .and_then(|row| row.get(x.max(0) as usize))
            else {
                continue;
            };
            let first_shop = match (first_shop, node.class) {
                (None, Some(RoomType::ShopRoom)) => Some(node.y + 1),
                _ => first_shop,
            };
            let first_elite = match (first_elite, node.class) {
                (None, Some(RoomType::MonsterRoomElite)) => Some(node.y + 1),
                _ => first_elite,
            };
            if let Some(shop_floor) = first_shop {
                if shop_floor <= 4 && first_elite.is_none_or(|elite_floor| shop_floor < elite_floor)
                {
                    return true;
                }
            }
            for edge in &node.edges {
                stack.push((edge.dst_x, edge.dst_y, first_shop, first_elite));
            }
        }
        false
    })
}

fn has_lament_elite_snipe_candidate(run_state: &RunState) -> bool {
    route_targets(run_state)
        .iter()
        .any(|target| elite_within_three_combats(run_state, target.x, target.y, 0))
}

fn elite_within_three_combats(run_state: &RunState, x: i32, y: i32, combats_so_far: usize) -> bool {
    let Some(node) = run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))
    else {
        return false;
    };
    let combats = combats_so_far
        + usize::from(matches!(
            node.class,
            Some(RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss)
        ));
    if node.class == Some(RoomType::MonsterRoomElite) {
        return combats <= 3;
    }
    if combats >= 3 {
        return false;
    }
    node.edges
        .iter()
        .any(|edge| elite_within_three_combats(run_state, edge.dst_x, edge.dst_y, combats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicId;
    use crate::state::events::{EventEffect, EventRelicKind};

    fn choice(index: usize, label: &str, effects: Vec<EventEffect>) -> NeowChoiceInputV1 {
        NeowChoiceInputV1::from_effects(index, label, effects)
    }

    #[test]
    fn early_shop_makes_gold_convertible() {
        let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
            player_class: "Ironclad".to_string(),
            map: NeowMapFeaturesV1 {
                early_shop_available: true,
                shop_before_first_elite: true,
                ..NeowMapFeaturesV1::default()
            },
            choices: vec![
                choice(0, "Obtain 100 Gold.", vec![EventEffect::GainGold(100)]),
                choice(
                    1,
                    "Obtain a random common relic.",
                    vec![EventEffect::ObtainRelic {
                        count: 1,
                        kind: EventRelicKind::RandomCommonRelic,
                    }],
                ),
            ],
            config: NeowGuidanceConfigV1::default(),
        });

        assert_eq!(trace.selected().map(|choice| choice.index), Some(0));
        assert!(trace.selected().unwrap().terms.shop_convertibility > 0.0);
    }

    #[test]
    fn lament_is_prioritized_when_it_can_snipe_an_elite() {
        let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
            player_class: "Ironclad".to_string(),
            map: NeowMapFeaturesV1 {
                lament_elite_snipe_candidate: true,
                early_elite_available: true,
                ..NeowMapFeaturesV1::default()
            },
            choices: vec![
                choice(
                    0,
                    "Enemies in your next three combats have 1 HP.",
                    vec![EventEffect::ObtainRelic {
                        count: 1,
                        kind: EventRelicKind::Specific(RelicId::NeowsLament),
                    }],
                ),
                choice(1, "Max HP +8", vec![EventEffect::GainMaxHp(8)]),
            ],
            config: NeowGuidanceConfigV1::default(),
        });

        assert_eq!(trace.selected().map(|choice| choice.index), Some(0));
        assert!(trace.selected().unwrap().terms.first_elite_security > 0.0);
    }

    #[test]
    fn ironclad_does_not_default_to_boss_swap_when_stable_options_exist() {
        let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
            player_class: "Ironclad".to_string(),
            map: NeowMapFeaturesV1 {
                path_flexibility: 0.25,
                ..NeowMapFeaturesV1::default()
            },
            choices: vec![
                choice(
                    0,
                    "Obtain a random common relic.",
                    vec![EventEffect::ObtainRelic {
                        count: 1,
                        kind: EventRelicKind::RandomCommonRelic,
                    }],
                ),
                choice(
                    1,
                    "Obtain a random Boss Relic. Lose your starter Relic.",
                    vec![
                        EventEffect::LoseStarterRelic { specific: None },
                        EventEffect::ObtainRelic {
                            count: 1,
                            kind: EventRelicKind::RandomBossRelic,
                        },
                    ],
                ),
            ],
            config: NeowGuidanceConfigV1::default(),
        });

        assert_eq!(trace.selected().map(|choice| choice.index), Some(0));
        assert!(trace.candidates[1].terms.downside_cost < 0.0);
    }
}
