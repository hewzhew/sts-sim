use crate::bot::agent::Agent;
use crate::state::core::{CampfireChoice, ClientInput};
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn boss_relic_score(
        &self,
        rs: &RunState,
        relic_id: crate::content::relics::RelicId,
    ) -> i32 {
        use crate::content::relics::RelicId;

        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let bad_basics = rs
            .master_deck
            .iter()
            .filter(|c| {
                matches!(
                    c.id,
                    id if crate::content::cards::is_starter_basic(id)
                )
            })
            .count() as i32;
        let avg_cost = if rs.master_deck.is_empty() {
            0.0
        } else {
            let total_cost: i32 = rs
                .master_deck
                .iter()
                .map(|c| crate::content::cards::get_card_definition(c.id).cost.max(0) as i32)
                .sum();
            total_cost as f32 / rs.master_deck.len() as f32
        };

        let mut score = match relic_id {
            RelicId::Sozu => 100,
            RelicId::CursedKey => 90,
            RelicId::Astrolabe => 80,
            RelicId::SneckoEye => 70,
            RelicId::BustedCrown => 60,
            RelicId::CoffeeDripper => 50,
            RelicId::FusionHammer => 45,
            RelicId::Ectoplasm => 40,
            RelicId::PhilosopherStone => 35,
            RelicId::VelvetChoker => 30,
            RelicId::EmptyCage => 20,
            RelicId::CallingBell => 10,
            _ => 0,
        };

        match relic_id {
            RelicId::Astrolabe | RelicId::EmptyCage => {
                score += bad_basics * 4;
            }
            RelicId::SneckoEye => {
                if avg_cost >= 1.4 {
                    score += 18;
                }
                if profile.x_cost_payoffs > 0 {
                    score -= 12;
                }
            }
            RelicId::FusionHammer => {
                if self.best_upgrade_index(rs).is_some() {
                    score -= 12;
                }
            }
            RelicId::PhilosopherStone => {
                score += profile.strength_payoffs * 3 + profile.block_core;
            }
            RelicId::VelvetChoker => {
                if profile.attack_count >= 8 {
                    score -= 10;
                }
            }
            RelicId::CoffeeDripper => {
                let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
                if hp_ratio >= 0.7 {
                    score += 15;
                } else {
                    score -= 15;
                }
            }
            RelicId::BustedCrown => {
                if rs.act_num == 1 {
                    score -= 18;
                }
            }
            _ => {}
        }

        score
    }

    pub(crate) fn is_high_value_tactical_card(
        &self,
        card_id: crate::content::cards::CardId,
    ) -> bool {
        use crate::content::cards::CardId;
        matches!(
            card_id,
            CardId::Apotheosis
                | CardId::Panacea
                | CardId::Blind
                | CardId::DarkShackles
                | CardId::Trip
                | CardId::GoodInstincts
                | CardId::Finesse
                | CardId::FlashOfSteel
                | CardId::MasterOfStrategy
                | CardId::Corruption
                | CardId::FeelNoPain
                | CardId::DarkEmbrace
                | CardId::Shockwave
        )
    }

    pub(crate) fn decide_map(&mut self, rs: &RunState) -> ClientInput {
        if rs.map.current_y < 0 {
            self.map_path = Self::compute_map_path_with_target(rs, self.active_curiosity_target());
            let archetypes = crate::bot::evaluator::CardEvaluator::archetype_tags(
                &crate::bot::evaluator::CardEvaluator::deck_profile(rs),
            );
            eprintln!(
                "  [BOT] Computed map path: {:?} | Archetypes: {:?}",
                self.map_path, archetypes
            );
        }

        let path_idx = (rs.map.current_y + 1) as usize;
        if path_idx < self.map_path.len() {
            let target_x = self.map_path[path_idx];
            let next_y = rs.map.current_y + 1;
            if rs.map.can_travel_to(target_x, next_y, false) {
                ClientInput::SelectMapNode(target_x as usize)
            } else {
                for x in 0..7 {
                    if rs.map.can_travel_to(x, next_y, false) {
                        return ClientInput::SelectMapNode(x as usize);
                    }
                }
                ClientInput::SelectMapNode(0)
            }
        } else {
            let next_y = rs.map.current_y + 1;
            for x in 0..7 {
                if rs.map.can_travel_to(x, next_y, false) {
                    return ClientInput::SelectMapNode(x as usize);
                }
            }
            ClientInput::SelectMapNode(0)
        }
    }

    pub(crate) fn compute_map_path_with_target(
        rs: &RunState,
        curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    ) -> Vec<i32> {
        let graph = &rs.map.graph;
        let weights = Self::map_room_weights(rs, curiosity_target);

        let mut paths_a: Vec<(Vec<i32>, i32)> = vec![(vec![], 0); 7];
        let mut paths_b: Vec<(Vec<i32>, i32)> = vec![(vec![], 0); 7];

        if !graph.is_empty() {
            for x in 0..7 {
                if x < graph[0].len() {
                    let node = &graph[0][x];
                    if !node.edges.is_empty() {
                        let w = node
                            .class
                            .map(|rt| weights[Self::room_type_to_weight_index(rt)])
                            .unwrap_or(0);
                        paths_a[x] = (vec![x as i32], w);
                    }
                }
            }
        }

        let max_y = graph.len().min(15);
        for y in 0..max_y.saturating_sub(1) {
            for slot in paths_b.iter_mut().take(7) {
                *slot = (vec![], 0);
            }

            for x in 0..7 {
                if x >= graph[y].len() {
                    continue;
                }
                let node = &graph[y][x];
                if node.edges.is_empty() {
                    continue;
                }
                let cur_path = &paths_a[x];

                for edge in &node.edges {
                    let next_x = edge.dst_x as usize;
                    let next_y = edge.dst_y as usize;
                    if next_y >= graph.len() || next_x >= graph[next_y].len() {
                        continue;
                    }

                    let next_node = &graph[next_y][next_x];
                    let room_w = next_node
                        .class
                        .map(|rt| weights[Self::room_type_to_weight_index(rt)])
                        .unwrap_or(0);
                    let new_weight = cur_path.1 + room_w;

                    let dest = &paths_b[next_x];
                    if dest.0.len() < cur_path.0.len() + 1 || dest.1 < new_weight {
                        let mut new_route = cur_path.0.clone();
                        new_route.push(next_x as i32);
                        paths_b[next_x] = (new_route, new_weight);
                    }
                }
            }

            std::mem::swap(&mut paths_a, &mut paths_b);
        }

        let mut best_x = 0;
        let mut best_weight = i32::MIN;
        for (x, path) in paths_a.iter().enumerate().take(7) {
            if path.1 > best_weight && !path.0.is_empty() {
                best_weight = path.1;
                best_x = x;
            }
        }

        let mut route = paths_a[best_x].0.clone();
        route.push(0);
        route
    }

    pub(crate) fn room_type_to_weight_index(rt: crate::map::node::RoomType) -> usize {
        use crate::map::node::RoomType;
        match rt {
            RoomType::ShopRoom => 0,
            RoomType::RestRoom => 1,
            RoomType::EventRoom => 2,
            RoomType::MonsterRoomElite => 3,
            RoomType::MonsterRoom => 4,
            RoomType::TreasureRoom => 5,
            _ => 4,
        }
    }

    pub(crate) fn map_room_weights(
        rs: &RunState,
        curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    ) -> [i32; 6] {
        let act_idx = ((rs.act_num as usize).saturating_sub(1)).min(2);
        let mut weights: [i32; 6] = match act_idx {
            0 => [100, 1000, 100, 10, 1, 0],
            1 => [10, 1000, 10, 100, 1, 0],
            _ => [100, 1000, 100, 1, 10, 0],
        };

        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
        let bad_basics = rs
            .master_deck
            .iter()
            .filter(|c| {
                matches!(
                    c.id,
                    id if crate::content::cards::is_starter_basic(id)
                )
            })
            .count() as i32;

        if hp_ratio < 0.45 {
            weights[1] += 500;
            weights[3] -= 120;
            weights[4] -= 10;
        } else if hp_ratio > 0.75 && !Self::profile_needs_support(&profile) {
            weights[3] += 80;
        }

        if bad_basics >= 4 {
            weights[0] += 60;
            weights[2] += 20;
        }

        if Self::profile_needs_support(&profile) {
            weights[0] += 40;
            weights[2] += 25;
            weights[3] -= 40;
        } else if Self::profile_is_online(&profile) {
            weights[3] += 40;
        }

        if let Some(crate::bot::coverage::CuriosityTarget::Archetype(target)) = curiosity_target {
            let target = Self::normalize_lookup_name(target);
            let target_online = crate::bot::evaluator::CardEvaluator::archetype_tags(&profile)
                .iter()
                .any(|tag| Self::normalize_lookup_name(tag) == target);
            if !target_online {
                weights[0] += 45;
                weights[2] += 35;
                weights[3] -= 35;
                if target == "block" {
                    weights[1] += 25;
                }
            } else {
                weights[3] += 30;
            }
        }

        weights
    }

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

        let has_relic = |id: RelicId| rs.relics.iter().any(|r| r.id == id);
        let can_rest = !has_relic(RelicId::CoffeeDripper);
        let best_upgrade_index = self.best_upgrade_index(rs);
        let can_smith = !has_relic(RelicId::FusionHammer) && best_upgrade_index.is_some();
        let can_toke = has_relic(RelicId::PeacePipe) && !rs.master_deck.is_empty();
        let can_lift = rs
            .relics
            .iter()
            .any(|r| r.id == RelicId::Girya && r.counter < 3);
        let can_dig = has_relic(RelicId::Shovel);
        let can_recall = rs.is_final_act_available && !rs.keys[0];
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let searing_plan = self.searing_blow_plan_score(rs, &profile);

        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
        let pre_boss_floor = rs.floor_num % 17 == 15;
        let worst_card_score = rs
            .master_deck
            .iter()
            .map(|c| crate::bot::evaluator::CardEvaluator::evaluate_owned_card(c.id, rs))
            .min()
            .unwrap_or(0);
        let strong_upgrade_exists = self
            .best_upgrade_index(rs)
            .map(|idx| {
                crate::bot::evaluator::CardEvaluator::evaluate_owned_card(
                    rs.master_deck[idx].id,
                    rs,
                ) >= 60
            })
            .unwrap_or(false);
        let shell_needs_smith = self
            .best_upgrade_index(rs)
            .map(|idx| self.upgrade_shell_bonus(rs.master_deck[idx].id, &profile) >= 14)
            .unwrap_or(false);
        let bad_basic_count = rs
            .master_deck
            .iter()
            .filter(|c| {
                matches!(
                    c.id,
                    id if crate::content::cards::is_starter_basic(id)
                )
            })
            .count();

        let searing_blow_index = if searing_plan > 0 {
            rs.master_deck
                .iter()
                .enumerate()
                .find(|(_, c)| c.id == crate::content::cards::CardId::SearingBlow)
                .map(|(idx, _)| idx)
        } else {
            None
        };

        CampfireDecisionContext {
            act_num: rs.act_num,
            hp_ratio,
            pre_boss_floor,
            can_recall,
            can_toke,
            can_lift,
            can_dig,
            can_rest,
            can_smith,
            best_purge_index: self.best_purge_index(rs),
            best_upgrade_index,
            searing_blow_index,
            searing_plan,
            worst_card_score,
            strong_upgrade_exists,
            shell_needs_smith,
            bad_basic_count,
        }
    }

    fn campfire_candidates(&self, context: &CampfireDecisionContext) -> Vec<CampfireCandidate> {
        let mut candidates = Vec::new();

        if context.can_recall
            && context.act_num >= 3
            && context.hp_ratio >= 0.45
            && !context.pre_boss_floor
        {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Recall,
                score: 1000,
                rationale: "recall_when_final_act_key_window_is_safe",
            });
        }

        if context.can_toke
            && context.hp_ratio >= 0.75
            && context.worst_card_score <= 10
            && !context.strong_upgrade_exists
            && !context.shell_needs_smith
            && context.bad_basic_count >= 3
        {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Toke(context.best_purge_index),
                score: 900 + context.bad_basic_count as i32 * 5,
                rationale: "toke_when_trim_is_cleaner_than_other_campfire_value",
            });
        }

        if context.can_lift && context.hp_ratio >= 0.75 {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Lift,
                score: 800,
                rationale: "lift_when_safe_and_scaling_is_free",
            });
        }

        if context.can_dig && context.hp_ratio >= 0.85 {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Dig,
                score: 700,
                rationale: "dig_when_safe_and_relic_value_is_free",
            });
        }

        if context.can_rest
            && (context.hp_ratio < 0.5
                || (context.act_num != 1
                    && context.pre_boss_floor
                    && context.hp_ratio < 0.9))
        {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Rest,
                score: 600,
                rationale: "rest_when_hp_window_is_dangerous",
            });
        }

        if context.can_smith {
            let smith_index = if context.searing_plan > 0
                && (context.hp_ratio >= 0.4 || context.act_num == 1)
            {
                context.searing_blow_index.or(context.best_upgrade_index)
            } else {
                context.best_upgrade_index
            };
            if let Some(idx) = smith_index {
                let score = if context.searing_plan > 0 { 560 } else { 500 };
                let rationale = if context.searing_plan > 0 {
                    "smith_searing_blow_when_upgrade_plan_is_active"
                } else {
                    "smith_best_upgrade_when_rest_is_not_urgent"
                };
                candidates.push(CampfireCandidate {
                    action: CampfireAction::Smith(idx),
                    score,
                    rationale,
                });
            }
        }

        if context.can_rest {
            candidates.push(CampfireCandidate {
                action: CampfireAction::Rest,
                score: 200,
                rationale: "rest_as_safe_fallback",
            });
        }

        candidates.push(CampfireCandidate {
            action: CampfireAction::Proceed,
            score: 0,
            rationale: "no_campfire_option_beats_proceed",
        });

        candidates
    }

    pub(crate) fn profile_needs_support(profile: &crate::bot::evaluator::DeckProfile) -> bool {
        (profile.strength_enablers > 0 && profile.strength_payoffs == 0)
            || (profile.strength_payoffs >= 2 && profile.strength_enablers == 0)
            || (profile.exhaust_engines > 0 && profile.exhaust_outlets == 0)
            || (profile.exhaust_outlets >= 2 && profile.exhaust_engines == 0)
            || (profile.block_core >= 2 && profile.block_payoffs == 0)
    }

    pub(crate) fn shop_needs_frontload_damage(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> bool {
        let has_premium_damage = rs.master_deck.iter().any(|c| {
            matches!(
                c.id,
                crate::content::cards::CardId::SearingBlow
                    | crate::content::cards::CardId::Hemokinesis
                    | crate::content::cards::CardId::Carnage
                    | crate::content::cards::CardId::Immolate
                    | crate::content::cards::CardId::Whirlwind
                    | crate::content::cards::CardId::Pummel
                    | crate::content::cards::CardId::Bludgeon
            )
        });
        !has_premium_damage || (profile.attack_count <= 6 && profile.strength_payoffs == 0)
    }

    pub(crate) fn shop_needs_reliable_block(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> bool {
        let has_anchor_defense = rs.master_deck.iter().any(|c| {
            matches!(
                c.id,
                crate::content::cards::CardId::ShrugItOff
                    | crate::content::cards::CardId::FlameBarrier
                    | crate::content::cards::CardId::GhostlyArmor
                    | crate::content::cards::CardId::Impervious
                    | crate::content::cards::CardId::PowerThrough
            )
        });
        profile.block_core < 2 || !has_anchor_defense
    }

    pub(crate) fn shop_needs_damage_control(&self, rs: &RunState) -> bool {
        !rs.master_deck.iter().any(|c| {
            matches!(
                c.id,
                crate::content::cards::CardId::Disarm
                    | crate::content::cards::CardId::Shockwave
                    | crate::content::cards::CardId::Uppercut
                    | crate::content::cards::CardId::Clothesline
            )
        })
    }

    pub(crate) fn profile_is_online(profile: &crate::bot::evaluator::DeckProfile) -> bool {
        (profile.strength_enablers >= 1 && profile.strength_payoffs >= 2)
            || (profile.exhaust_engines >= 2 && profile.exhaust_outlets >= 1)
            || (profile.block_core >= 3 && profile.block_payoffs >= 1)
    }
}

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
    searing_blow_index: Option<usize>,
    searing_plan: i32,
    worst_card_score: i32,
    strong_upgrade_exists: bool,
    shell_needs_smith: bool,
    bad_basic_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct CampfireCandidate {
    action: CampfireAction,
    score: i32,
    rationale: &'static str,
}

impl CampfireCandidate {
    fn into_input(self) -> ClientInput {
        let _ = self.rationale;
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
