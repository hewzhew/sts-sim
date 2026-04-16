use crate::bot::agent::Agent;
use crate::map::node::RoomType;
use crate::state::run::RunState;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RoutePressureSummary {
    pub upcoming_normal_pressure: i32,
    pub upcoming_elite_pressure: i32,
    pub upcoming_boss_pressure: i32,
    pub nearby_recovery_windows: i32,
    pub nearby_shop_windows: i32,
    pub recall_window_pressure: i32,
    pub remaining_recall_windows: i32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UpgradeAssessment {
    pub best_upgrade_index: Option<usize>,
    pub best_upgrade_value: i32,
    pub upgrade_rationale_key: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PurgeAssessment {
    pub best_purge_index: usize,
    pub purge_value: i32,
    pub purge_rationale_key: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct NoncombatNeedSnapshot {
    pub survival_pressure: i32,
    pub best_upgrade_value: i32,
    pub purge_value: i32,
    pub key_urgency: i32,
    pub long_term_meta_value: i32,
    pub route: RoutePressureSummary,
    pub best_upgrade_index: Option<usize>,
    pub best_purge_index: usize,
    pub upgrade: UpgradeAssessment,
    pub purge: PurgeAssessment,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ShopNeedProfile {
    pub damage_gap: i32,
    pub block_gap: i32,
    pub control_gap: i32,
    pub upgrade_hunger: i32,
    pub purge_hunger: i32,
    pub shell_incomplete: bool,
}

pub(crate) fn build_noncombat_need_snapshot_for_run(rs: &RunState) -> NoncombatNeedSnapshot {
    crate::bot::Agent::new_policy_model().build_noncombat_need_snapshot(rs)
}

pub(crate) fn build_shop_need_profile_for_run(rs: &RunState) -> ShopNeedProfile {
    crate::bot::Agent::new_policy_model().build_shop_need_profile(rs)
}

impl Agent {
    pub(crate) fn build_noncombat_need_snapshot(&self, rs: &RunState) -> NoncombatNeedSnapshot {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let route = build_route_pressure_summary(rs);
        let upgrade = self.assess_best_upgrade(rs, &profile);
        let purge = self.assess_best_purge(rs);
        let survival_pressure = self.noncombat_survival_pressure(rs, &profile, &route);
        let key_urgency =
            self.noncombat_key_urgency(rs, &route, survival_pressure, upgrade.best_upgrade_value);
        let long_term_meta_value = self.noncombat_long_term_meta_value(
            rs,
            &profile,
            &route,
            survival_pressure,
            upgrade.best_upgrade_value,
            purge.purge_value,
        );

        NoncombatNeedSnapshot {
            survival_pressure,
            best_upgrade_value: upgrade.best_upgrade_value,
            purge_value: purge.purge_value,
            key_urgency,
            long_term_meta_value,
            route,
            best_upgrade_index: upgrade.best_upgrade_index,
            best_purge_index: purge.best_purge_index,
            upgrade,
            purge,
        }
    }

    pub(crate) fn build_shop_need_profile(&self, rs: &RunState) -> ShopNeedProfile {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let has_premium_damage = rs.master_deck.iter().any(|card| {
            matches!(
                card.id,
                crate::content::cards::CardId::SearingBlow
                    | crate::content::cards::CardId::Hemokinesis
                    | crate::content::cards::CardId::Carnage
                    | crate::content::cards::CardId::Immolate
                    | crate::content::cards::CardId::Whirlwind
                    | crate::content::cards::CardId::Pummel
                    | crate::content::cards::CardId::Bludgeon
            )
        });
        let has_anchor_defense = rs.master_deck.iter().any(|card| {
            matches!(
                card.id,
                crate::content::cards::CardId::ShrugItOff
                    | crate::content::cards::CardId::FlameBarrier
                    | crate::content::cards::CardId::GhostlyArmor
                    | crate::content::cards::CardId::Impervious
                    | crate::content::cards::CardId::PowerThrough
            )
        });
        let has_damage_control = rs.master_deck.iter().any(|card| {
            matches!(
                card.id,
                crate::content::cards::CardId::Disarm
                    | crate::content::cards::CardId::Shockwave
                    | crate::content::cards::CardId::Uppercut
                    | crate::content::cards::CardId::Clothesline
            )
        });
        let shell_incomplete = (profile.strength_enablers > 0 && profile.strength_payoffs == 0)
            || (profile.exhaust_engines > 0 && profile.exhaust_outlets == 0)
            || (profile.block_core >= 2 && profile.block_payoffs == 0);
        let mut damage_gap = 0;
        if !has_premium_damage {
            damage_gap += 28;
        }
        if profile.attack_count <= 6 && profile.strength_payoffs == 0 {
            damage_gap += 18;
        }
        if rs.act_num == 1 {
            damage_gap += 8;
        }

        let mut block_gap = 0;
        if profile.block_core < 2 {
            block_gap += 22;
        }
        if !has_anchor_defense {
            block_gap += 18;
        }
        block_gap += i32::from(rs.act_num >= 2) * 6;

        let mut control_gap = 0;
        if !has_damage_control {
            control_gap += 20;
        }
        control_gap += i32::from(rs.act_num >= 2) * 4;

        let upgrade_assessment = crate::bot::run_deck_improvement::assess_deck_operation(
            rs,
            crate::bot::run_deck_improvement::DeckOperationKind::Upgrade,
        );
        let upgrade_hunger = upgrade_assessment.total_prior_delta.max(0) * 2
            + upgrade_assessment
                .best_candidate
                .as_ref()
                .and_then(|candidate| candidate.target_index)
                .map(|idx| {
                    crate::bot::evaluator::CardEvaluator::evaluate_owned_card(
                        rs.master_deck[idx].id,
                        rs,
                    ) / 3
                })
                .unwrap_or(0);
        let purge_hunger = crate::bot::deck_delta_eval::compare_purge_vs_keep(rs)
            .total
            .max(0)
            * 2;

        ShopNeedProfile {
            damage_gap,
            block_gap,
            control_gap,
            upgrade_hunger,
            purge_hunger,
            shell_incomplete,
        }
    }

    fn assess_best_upgrade(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> UpgradeAssessment {
        let upgrade_assessment = crate::bot::run_deck_improvement::assess_deck_operation(
            rs,
            crate::bot::run_deck_improvement::DeckOperationKind::Upgrade,
        );
        let best_upgrade_index = upgrade_assessment
            .best_candidate
            .as_ref()
            .and_then(|candidate| candidate.target_index);
        let base_delta = upgrade_assessment.total_prior_delta.max(0);
        let Some(idx) = best_upgrade_index else {
            return UpgradeAssessment {
                best_upgrade_index: None,
                best_upgrade_value: 0,
                upgrade_rationale_key: "no_upgrade_target",
            };
        };

        let card = &rs.master_deck[idx];
        let owned_value = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs);
        let searing_plan = self.searing_blow_plan_score(rs, profile);
        let shell_bonus = self.upgrade_shell_bonus(card.id, profile);
        let upgrade_facts = crate::bot::upgrade_facts::upgrade_facts(card.id);
        let mut best_upgrade_value =
            120 + base_delta * 18 + owned_value.max(0) * 2 + shell_bonus * 3;
        let upgrade_rationale_key = match card.id {
            crate::content::cards::CardId::SearingBlow if searing_plan > 0 => {
                best_upgrade_value += searing_plan * 2;
                "searing_blow_upgrade_plan"
            }
            _ if upgrade_facts.changes_cost
                || upgrade_facts.improves_target_control
                || upgrade_facts.extends_debuff_duration
                || upgrade_facts.improves_draw_consistency
                || upgrade_facts.improves_exhaust_control
                || upgrade_facts.improves_scaling =>
            {
                best_upgrade_value += 70;
                crate::bot::upgrade_facts::dominant_upgrade_semantic_key(card.id)
            }
            _ if shell_bonus > 0 => "shell_completion_upgrade",
            _ => upgrade_assessment.rationale_key,
        };

        UpgradeAssessment {
            best_upgrade_index,
            best_upgrade_value,
            upgrade_rationale_key,
        }
    }

    fn assess_best_purge(&self, rs: &RunState) -> PurgeAssessment {
        if rs.master_deck.is_empty() {
            return PurgeAssessment {
                best_purge_index: 0,
                purge_value: 0,
                purge_rationale_key: "no_purge_target",
            };
        }

        let best_purge_index = self.best_purge_index(rs);
        let card = &rs.master_deck[best_purge_index];
        let delta = crate::bot::deck_delta_eval::compare_purge_vs_keep(rs);
        let remove_severity = crate::bot::evaluator::curse_remove_severity(card.id);
        let starter_basic_count = rs
            .master_deck
            .iter()
            .filter(|candidate| crate::content::cards::is_starter_basic(candidate.id))
            .count() as i32;
        let owned_value = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs);
        let mut purge_value =
            120 + delta.total.max(0) * 10 + remove_severity * 35 + starter_basic_count * 12;
        let purge_rationale_key = if matches!(
            crate::content::cards::get_card_definition(card.id).card_type,
            crate::content::cards::CardType::Curse | crate::content::cards::CardType::Status
        ) || remove_severity >= 8
        {
            purge_value += 120;
            "curse_or_burden_trim"
        } else if crate::content::cards::is_starter_basic(card.id) {
            purge_value += 80;
            "starter_density_trim"
        } else if owned_value <= 10 {
            "low_value_trim"
        } else {
            "marginal_density_trim"
        };

        PurgeAssessment {
            best_purge_index,
            purge_value,
            purge_rationale_key,
        }
    }

    fn noncombat_survival_pressure(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
        route: &RoutePressureSummary,
    ) -> i32 {
        let shop_need = self.build_shop_need_profile(rs);
        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
        let missing_hp_ratio = (1.0 - hp_ratio).max(0.0);
        let potion_slack = rs.potions.iter().flatten().count() as i32;

        let mut pressure = (missing_hp_ratio * 260.0).round() as i32;
        pressure += route.upcoming_normal_pressure;
        pressure += route.upcoming_elite_pressure * 2;
        pressure += route.upcoming_boss_pressure * 2;
        pressure -= route.nearby_recovery_windows * 35;
        pressure -= route.nearby_shop_windows * 18;
        pressure += shop_need.damage_gap * 3 / 2 + shop_need.block_gap * 2 + shop_need.control_gap;
        pressure -= potion_slack * 14;
        pressure -= profile.block_core.min(3) * 12;
        pressure -= profile.block_payoffs.min(2) * 15;
        pressure -= profile.draw_sources.min(3) * 8;

        pressure.max(0)
    }

    fn noncombat_key_urgency(
        &self,
        rs: &RunState,
        route: &RoutePressureSummary,
        survival_pressure: i32,
        best_upgrade_value: i32,
    ) -> i32 {
        if !rs.is_final_act_available || rs.keys[0] {
            return 0;
        }

        let remaining_rows = if rs.map.current_y < 0 {
            15
        } else {
            (14 - rs.map.current_y).max(0)
        };
        let act_pressure = match rs.act_num {
            3 => 180,
            2 => 60,
            _ => 0,
        };
        let proximity_pressure = match remaining_rows {
            0..=1 => 220,
            2..=3 => 170,
            4..=5 => 110,
            _ => 40,
        };
        let opportunity_cost = survival_pressure / 3 + best_upgrade_value / 4;

        (act_pressure + proximity_pressure + route.recall_window_pressure - opportunity_cost).max(0)
    }

    fn noncombat_long_term_meta_value(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
        route: &RoutePressureSummary,
        survival_pressure: i32,
        best_upgrade_value: i32,
        purge_value: i32,
    ) -> i32 {
        use crate::content::relics::RelicId;

        let floors_remaining = if rs.map.current_y < 0 {
            15
        } else {
            (14 - rs.map.current_y).max(0)
        };
        let mut value = 0;

        if let Some(girya) = rs.relics.iter().find(|relic| relic.id == RelicId::Girya) {
            value += 70
                + (3 - girya.counter).max(0) * 40
                + profile.attack_count.min(10) * 6
                + floors_remaining * 4;
        }
        if rs.relics.iter().any(|relic| relic.id == RelicId::Shovel) {
            value += 55 + floors_remaining * 5;
        }
        if rs.relics.iter().any(|relic| relic.id == RelicId::PeacePipe) {
            value += 30 + purge_value / 5 + route.remaining_recall_windows * 8;
        }

        value -= survival_pressure / 2;
        value -= best_upgrade_value / 5;
        value.max(0)
    }
}

pub(super) fn build_route_pressure_summary(rs: &RunState) -> RoutePressureSummary {
    let nearby_nodes = reachable_room_nodes(rs, 4);
    let remaining_rests = reachable_room_count(rs, RoomType::RestRoom, 15);
    let remaining_rows = if rs.map.current_y < 0 {
        15
    } else {
        (14 - rs.map.current_y).max(0)
    };
    let upcoming_normal_pressure = nearby_nodes
        .iter()
        .filter(|(_, room_type, _)| *room_type == RoomType::MonsterRoom)
        .map(|(_, _, depth)| (5 - *depth).max(1) * 6)
        .sum();
    let upcoming_elite_pressure = nearby_nodes
        .iter()
        .filter(|(_, room_type, _)| *room_type == RoomType::MonsterRoomElite)
        .map(|(_, _, depth)| (6 - *depth).max(1) * 14)
        .sum();
    let nearby_recovery_windows = nearby_nodes
        .iter()
        .filter(|(_, room_type, depth)| {
            *depth <= 3 && matches!(room_type, RoomType::RestRoom | RoomType::ShopRoom)
        })
        .count() as i32;
    let nearby_shop_windows = nearby_nodes
        .iter()
        .filter(|(_, room_type, depth)| *depth <= 3 && *room_type == RoomType::ShopRoom)
        .count() as i32;
    let upcoming_boss_pressure = match remaining_rows {
        0..=1 => 65,
        2..=3 => 45,
        4..=5 => 25,
        _ => 0,
    };
    let recall_window_pressure = if rs.is_final_act_available && !rs.keys[0] {
        match remaining_rests {
            0 => 220,
            1 => 160,
            2 => 95,
            _ => 30,
        }
    } else {
        0
    };

    RoutePressureSummary {
        upcoming_normal_pressure,
        upcoming_elite_pressure,
        upcoming_boss_pressure,
        nearby_recovery_windows,
        nearby_shop_windows,
        recall_window_pressure,
        remaining_recall_windows: remaining_rests,
    }
}

fn reachable_room_count(rs: &RunState, target: RoomType, max_depth: i32) -> i32 {
    reachable_room_nodes(rs, max_depth)
        .into_iter()
        .filter(|(_, room_type, _)| *room_type == target)
        .count() as i32
}

fn reachable_room_nodes(rs: &RunState, max_depth: i32) -> Vec<((usize, usize), RoomType, i32)> {
    let mut seen: HashMap<(usize, usize), i32> = HashMap::new();
    let mut queue = initial_frontier(rs);

    while let Some(((x, y), depth)) = queue.pop_front() {
        if depth > max_depth {
            continue;
        }

        if let Some(existing_depth) = seen.get(&(x, y)) {
            if *existing_depth <= depth {
                continue;
            }
        }
        seen.insert((x, y), depth);

        let Some(node) = rs.map.graph.get(y).and_then(|row| row.get(x)) else {
            continue;
        };
        for edge in &node.edges {
            if edge.dst_x < 0 || edge.dst_y < 0 {
                continue;
            }
            queue.push_back(((edge.dst_x as usize, edge.dst_y as usize), depth + 1));
        }
    }

    let mut nodes = seen
        .into_iter()
        .filter_map(|((x, y), depth)| {
            if depth <= 0 {
                return None;
            }
            rs.map
                .graph
                .get(y)
                .and_then(|row| row.get(x))
                .and_then(|node| node.class.map(|room_type| ((x, y), room_type, depth)))
        })
        .collect::<Vec<_>>();
    nodes.sort_by_key(|(_, _, depth)| *depth);
    nodes
}

fn initial_frontier(rs: &RunState) -> VecDeque<((usize, usize), i32)> {
    let mut queue = VecDeque::new();
    if rs.map.current_y < 0 || rs.map.current_x < 0 {
        if let Some(row) = rs.map.graph.first() {
            for (x, node) in row.iter().enumerate() {
                if !node.edges.is_empty() {
                    queue.push_back(((x, 0usize), 1));
                }
            }
        }
        return queue;
    }

    let current = (rs.map.current_x as usize, rs.map.current_y as usize);
    queue.push_back((current, 0));
    queue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::Agent;
    use crate::content::cards::CardId;
    use crate::map::node::{Map, MapEdge, MapRoomNode};
    use crate::map::state::MapState;

    #[test]
    fn need_snapshot_tracks_route_pressure_upgrade_purge_and_key_urgency() {
        let agent = Agent::new();

        let mut safe_run = RunState::new(1, 0, true, "Ironclad");
        safe_run.current_hp = 72;
        safe_run.max_hp = 80;
        safe_run.map = linear_map_state(
            &[
                RoomType::MonsterRoom,
                RoomType::EventRoom,
                RoomType::ShopRoom,
                RoomType::RestRoom,
            ],
            0,
        );

        let mut dangerous_run = safe_run.clone();
        dangerous_run.current_hp = 18;
        dangerous_run.max_hp = 80;
        dangerous_run.act_num = 3;
        dangerous_run
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(
                CardId::Parasite,
                10_001,
            ));
        dangerous_run
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(
                CardId::Shockwave,
                10_002,
            ));
        dangerous_run.map = linear_map_state(
            &[
                RoomType::MonsterRoomElite,
                RoomType::MonsterRoom,
                RoomType::RestRoom,
                RoomType::MonsterRoomElite,
            ],
            0,
        );

        let safe = agent.build_noncombat_need_snapshot(&safe_run);
        let dangerous = agent.build_noncombat_need_snapshot(&dangerous_run);

        assert!(safe.survival_pressure < dangerous.survival_pressure);
        assert!(dangerous.best_upgrade_value > 0);
        assert!(dangerous.purge_value > safe.purge_value);
        assert!(dangerous.route.upcoming_elite_pressure > safe.route.upcoming_elite_pressure);
        assert!(dangerous.route.recall_window_pressure >= safe.route.recall_window_pressure);
    }

    #[test]
    fn shop_need_profile_surfaces_gaps_and_hunger() {
        let agent = Agent::new();
        let mut weak_run = RunState::new(3, 0, true, "Ironclad");
        weak_run.current_hp = 20;
        weak_run.max_hp = 80;
        weak_run.map = linear_map_state(
            &[
                RoomType::MonsterRoomElite,
                RoomType::MonsterRoom,
                RoomType::ShopRoom,
            ],
            0,
        );

        let mut stronger_run = weak_run.clone();
        stronger_run.current_hp = 72;
        stronger_run
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(
                CardId::Hemokinesis,
                10_101,
            ));
        stronger_run
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(
                CardId::ShrugItOff,
                10_102,
            ));
        stronger_run
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(
                CardId::Disarm,
                10_103,
            ));

        let weak = agent.build_shop_need_profile(&weak_run);
        let strong = agent.build_shop_need_profile(&stronger_run);

        assert!(weak.damage_gap > strong.damage_gap);
        assert!(weak.block_gap > strong.block_gap);
        assert!(weak.control_gap > strong.control_gap);
        assert!(weak.upgrade_hunger >= 0);
        assert!(weak.purge_hunger >= 0);
    }

    #[test]
    fn key_urgency_rises_when_remaining_recall_windows_collapse() {
        let agent = Agent::new();
        let mut many_windows = RunState::new(2, 0, true, "Ironclad");
        many_windows.act_num = 3;
        many_windows.map = linear_map_state(
            &[
                RoomType::MonsterRoom,
                RoomType::RestRoom,
                RoomType::MonsterRoom,
                RoomType::RestRoom,
            ],
            0,
        );

        let mut last_window = many_windows.clone();
        last_window.map = linear_map_state(&[RoomType::MonsterRoomElite], 12);
        last_window.floor_num = 48;

        let many = agent.build_noncombat_need_snapshot(&many_windows);
        let last = agent.build_noncombat_need_snapshot(&last_window);

        assert!(many.route.remaining_recall_windows > last.route.remaining_recall_windows);
        assert!(many.key_urgency < last.key_urgency);
    }

    fn linear_map_state(rooms: &[RoomType], current_y: i32) -> MapState {
        let mut graph: Map = Vec::new();
        for (y, room_type) in rooms.iter().enumerate() {
            let mut node = MapRoomNode::new(0, y as i32);
            node.class = Some(*room_type);
            if y + 1 < rooms.len() {
                node.edges
                    .insert(MapEdge::new(0, y as i32, 0, y as i32 + 1));
            }
            graph.push(vec![node]);
        }
        let mut map = MapState::new(graph);
        map.current_x = 0;
        map.current_y = current_y;
        map
    }
}
