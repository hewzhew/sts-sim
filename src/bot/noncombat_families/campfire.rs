use super::model::NoncombatNeedSnapshot;
use crate::bot::agent::Agent;
use crate::state::core::{CampfireChoice, ClientInput};
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug)]
struct CampfireDecisionContext {
    act_num: u8,
    hp_ratio: f32,
    pre_boss_floor: bool,
    can_recall: bool,
    can_toke: bool,
    can_lift: bool,
    can_dig: bool,
    can_rest: bool,
    can_smith: bool,
    best_purge_index: usize,
    best_upgrade_index: Option<usize>,
    needs_smith_shell_support: bool,
    need: NoncombatNeedSnapshot,
}

#[derive(Clone, Copy, Debug)]
struct CampfireCandidate {
    action: CampfireAction,
    score: i32,
    rationale_key: &'static str,
}

impl CampfireCandidate {
    fn into_input(self) -> ClientInput {
        let _ = self.rationale_key;
        self.action.into_input()
    }
}

#[derive(Clone, Copy, Debug)]
enum CampfireAction {
    Recall,
    Toke(usize),
    Lift,
    Dig,
    Rest,
    Smith(usize),
    Proceed,
}

impl CampfireAction {
    fn into_input(self) -> ClientInput {
        match self {
            Self::Recall => ClientInput::CampfireOption(CampfireChoice::Recall),
            Self::Toke(idx) => ClientInput::CampfireOption(CampfireChoice::Toke(idx)),
            Self::Lift => ClientInput::CampfireOption(CampfireChoice::Lift),
            Self::Dig => ClientInput::CampfireOption(CampfireChoice::Dig),
            Self::Rest => ClientInput::CampfireOption(CampfireChoice::Rest),
            Self::Smith(idx) => ClientInput::CampfireOption(CampfireChoice::Smith(idx)),
            Self::Proceed => ClientInput::Proceed,
        }
    }
}

impl Agent {
    pub(crate) fn decide_campfire(&self, rs: &RunState) -> ClientInput {
        let context = self.build_campfire_decision_context(rs);
        self.campfire_candidates(&context)
            .into_iter()
            .max_by_key(|candidate| candidate.score)
            .map(CampfireCandidate::into_input)
            .unwrap_or(ClientInput::Proceed)
    }

    fn build_campfire_decision_context(&self, rs: &RunState) -> CampfireDecisionContext {
        use crate::content::relics::RelicId;

        let has_relic = |id: RelicId| rs.relics.iter().any(|relic| relic.id == id);
        let need = self.build_noncombat_need_snapshot(rs);
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
        let best_upgrade_index = need.best_upgrade_index;
        let needs_smith_shell_support = best_upgrade_index
            .map(|idx| self.upgrade_shell_bonus(rs.master_deck[idx].id, &profile) >= 14)
            .unwrap_or(false);

        CampfireDecisionContext {
            act_num: rs.act_num,
            hp_ratio,
            pre_boss_floor: rs.floor_num % 17 == 15,
            can_recall: rs.is_final_act_available && !rs.keys[0],
            can_toke: has_relic(RelicId::PeacePipe) && !rs.master_deck.is_empty(),
            can_lift: rs
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::Girya && relic.counter < 3),
            can_dig: has_relic(RelicId::Shovel),
            can_rest: !has_relic(RelicId::CoffeeDripper),
            can_smith: !has_relic(RelicId::FusionHammer) && best_upgrade_index.is_some(),
            best_purge_index: need.best_purge_index,
            best_upgrade_index,
            needs_smith_shell_support,
            need,
        }
    }

    fn campfire_candidates(&self, context: &CampfireDecisionContext) -> Vec<CampfireCandidate> {
        let mut candidates = Vec::new();

        if context.can_rest {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Rest,
                score: self.rest_score(context),
                rationale_key: "rest_for_route_survival_pressure",
            });
        }

        if context.can_smith {
            if let Some(idx) = context.best_upgrade_index {
                candidates.push(CampfireCandidate {
                    action: CampfireAction::Smith(idx),
                    score: self.smith_score(context),
                    rationale_key: context.need.upgrade.upgrade_rationale_key,
                });
            }
        }

        if context.can_toke {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Toke(context.best_purge_index),
                score: self.toke_score(context),
                rationale_key: context.need.purge.purge_rationale_key,
            });
        }

        if context.can_recall {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Recall,
                score: self.recall_score(context),
                rationale_key: "recall_when_key_urgency_beats_campfire_opportunity_cost",
            });
        }

        if context.can_lift {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Lift,
                score: self.lift_score(context),
                rationale_key: "lift_when_long_term_scaling_beats_immediate_needs",
            });
        }

        if context.can_dig {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Dig,
                score: self.dig_score(context),
                rationale_key: "dig_when_relic_ev_beats_immediate_needs",
            });
        }

        candidates.push(CampfireCandidate {
            action: CampfireAction::Proceed,
            score: 0,
            rationale_key: "no_campfire_option_beats_proceed",
        });

        candidates
    }

    fn rest_score(&self, context: &CampfireDecisionContext) -> i32 {
        let urgent_boss_window = i32::from(context.pre_boss_floor) * 80;
        let shell_support_penalty = i32::from(context.needs_smith_shell_support) * 30;
        let critical_hp_bonus = i32::from(context.hp_ratio < 0.35) * 180;
        let survival_floor =
            i32::from(context.hp_ratio < 0.40 || context.need.survival_pressure >= 140) * 220;
        context.need.survival_pressure * 2 + urgent_boss_window + critical_hp_bonus + survival_floor
            - context.need.best_upgrade_value / 3
            - context.need.purge_value / 4
            - context.need.long_term_meta_value / 5
            - shell_support_penalty
    }

    fn smith_score(&self, context: &CampfireDecisionContext) -> i32 {
        let urgent_boss_window = i32::from(context.pre_boss_floor) * 45;
        let critical_hp_penalty = i32::from(context.hp_ratio < 0.40) * 120;
        context.need.best_upgrade_value * 3 / 2
            - context.need.survival_pressure
            - context.need.key_urgency / 5
            + urgent_boss_window
            + i32::from(context.needs_smith_shell_support) * 35
            - critical_hp_penalty
    }

    fn toke_score(&self, context: &CampfireDecisionContext) -> i32 {
        context.need.purge_value * 7 / 5
            - context.need.best_upgrade_value / 2
            - context.need.survival_pressure / 3
            - context.need.key_urgency / 5
            + i32::from(context.need.route.nearby_shop_windows == 0) * 40
    }

    fn recall_score(&self, context: &CampfireDecisionContext) -> i32 {
        let immediate_cost = self
            .rest_score(context)
            .max(self.smith_score(context))
            .max(self.toke_score(context));
        context.need.key_urgency * 2 - immediate_cost / 2 - i32::from(context.pre_boss_floor) * 80
    }

    fn lift_score(&self, context: &CampfireDecisionContext) -> i32 {
        context.need.long_term_meta_value
            + context.act_num as i32 * 18
            + (context.hp_ratio * 40.0).round() as i32
            - context.need.survival_pressure
            - context.need.best_upgrade_value / 2
            - context.need.key_urgency / 2
    }

    fn dig_score(&self, context: &CampfireDecisionContext) -> i32 {
        context.need.long_term_meta_value * 4 / 5 + context.need.route.nearby_recovery_windows * 20
            - context.need.survival_pressure
            - context.need.best_upgrade_value / 2
            - context.need.key_urgency / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::Agent;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::map::node::{Map, MapEdge, MapRoomNode, RoomType};
    use crate::map::state::MapState;

    #[test]
    fn campfire_prefers_rest_over_long_term_actions_under_pressure() {
        let agent = Agent::new();
        let mut rs = campfire_run_with_relic(RelicId::Shovel);
        rs.relics.push(RelicState::new(RelicId::FusionHammer));
        rs.current_hp = 16;
        rs.max_hp = 80;
        rs.map = linear_map_state(
            &[
                RoomType::MonsterRoomElite,
                RoomType::MonsterRoom,
                RoomType::MonsterRoomElite,
            ],
            0,
        );

        assert_eq!(
            agent.decide_campfire(&rs),
            ClientInput::CampfireOption(CampfireChoice::Rest)
        );
    }

    #[test]
    fn campfire_prefers_smith_when_upgrade_value_beats_healing() {
        let agent = Agent::new();
        let mut rs = campfire_run();
        rs.current_hp = 62;
        rs.max_hp = 80;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Shockwave,
            9_001,
        ));

        assert_eq!(
            agent.decide_campfire(&rs),
            ClientInput::CampfireOption(CampfireChoice::Smith(
                agent.best_upgrade_index(&rs).unwrap()
            ))
        );
    }

    #[test]
    fn campfire_prefers_toke_when_trim_value_is_cleanest() {
        let agent = Agent::new();
        let mut rs = campfire_run_with_relic(RelicId::PeacePipe);
        rs.current_hp = 70;
        rs.max_hp = 80;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::CurseOfTheBell,
            9_002,
        ));

        assert_eq!(
            agent.decide_campfire(&rs),
            ClientInput::CampfireOption(CampfireChoice::Toke(agent.best_purge_index(&rs)))
        );
    }

    #[test]
    fn recall_does_not_auto_win_when_window_is_expensive() {
        let agent = Agent::new();
        let mut rs = campfire_run();
        rs.act_num = 3;
        rs.current_hp = 20;
        rs.max_hp = 80;
        rs.floor_num = 49;
        rs.map = linear_map_state(&[RoomType::MonsterRoomElite], 13);
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Shockwave,
            9_003,
        ));

        assert_ne!(
            agent.decide_campfire(&rs),
            ClientInput::CampfireOption(CampfireChoice::Recall)
        );
    }

    fn campfire_run() -> RunState {
        let mut rs = RunState::new(7, 0, true, "Ironclad");
        rs.map = linear_map_state(
            &[
                RoomType::MonsterRoom,
                RoomType::MonsterRoom,
                RoomType::RestRoom,
            ],
            0,
        );
        rs.floor_num = 10;
        rs
    }

    fn campfire_run_with_relic(relic_id: RelicId) -> RunState {
        let mut rs = campfire_run();
        rs.relics.push(RelicState::new(relic_id));
        rs
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
