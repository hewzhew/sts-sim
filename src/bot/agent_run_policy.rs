use crate::bot::agent::Agent;
use crate::state::core::{CampfireChoice, ClientInput};
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn decide_shop(&self, rs: &RunState, shop: &crate::shop::ShopState) -> ClientInput {
        if let Some(cmd) = self.curiosity_shop_pick(rs, shop) {
            return cmd;
        }
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);

        if let Some((idx, _)) = shop
            .relics
            .iter()
            .enumerate()
            .filter(|(_, relic)| rs.gold >= relic.price)
            .map(|(idx, relic)| (idx, self.shop_relic_score(rs, relic.relic_id)))
            .filter(|(_, score)| *score >= 88)
            .max_by_key(|(_, score)| *score)
        {
            return ClientInput::BuyRelic(idx);
        }

        if let Some((idx, _)) = shop
            .cards
            .iter()
            .enumerate()
            .filter(|(_, card)| rs.gold >= card.price)
            .map(|(idx, card)| (idx, self.shop_card_score(rs, card.card_id)))
            .filter(|(_, score)| *score >= self.shop_card_buy_threshold(rs, *score))
            .max_by_key(|(_, score)| *score)
        {
            return ClientInput::BuyCard(idx);
        }

        if let Some((idx, _)) = shop
            .potions
            .iter()
            .enumerate()
            .filter(|(_, potion)| rs.gold >= potion.price && rs.potions.iter().any(|p| p.is_none()))
            .map(|(idx, potion)| (idx, self.shop_potion_score(rs, potion.potion_id)))
            .filter(|(_, score)| *score >= 80)
            .max_by_key(|(_, score)| *score)
        {
            return ClientInput::BuyPotion(idx);
        }

        if shop.purge_available
            && rs.gold >= shop.purge_cost
            && !rs.master_deck.is_empty()
            && self.should_purge_at_shop(rs, shop)
            && self.searing_blow_plan_score(rs, &profile) <= 0
        {
            return ClientInput::PurgeCard(self.best_purge_index(rs));
        }

        ClientInput::Proceed
    }

    pub(crate) fn curiosity_shop_pick(
        &self,
        rs: &RunState,
        shop: &crate::shop::ShopState,
    ) -> Option<ClientInput> {
        match self.curiosity_target.as_ref()? {
            crate::bot::coverage::CuriosityTarget::Card(target_name) => shop
                .cards
                .iter()
                .enumerate()
                .find(|(_, card)| {
                    rs.gold >= card.price && Self::matches_card_target(card.card_id, target_name)
                })
                .map(|(idx, _)| ClientInput::BuyCard(idx)),
            crate::bot::coverage::CuriosityTarget::Relic(target_name) => shop
                .relics
                .iter()
                .enumerate()
                .find(|(_, relic)| {
                    rs.gold >= relic.price
                        && Self::matches_relic_target(relic.relic_id, target_name)
                })
                .map(|(idx, _)| ClientInput::BuyRelic(idx)),
            crate::bot::coverage::CuriosityTarget::Potion(target_name) => shop
                .potions
                .iter()
                .enumerate()
                .find(|(_, potion)| {
                    rs.gold >= potion.price
                        && rs.potions.iter().any(|p| p.is_none())
                        && crate::content::potions::get_potion_definition(potion.potion_id)
                            .name
                            .eq_ignore_ascii_case(target_name)
                })
                .map(|(idx, _)| ClientInput::BuyPotion(idx)),
            crate::bot::coverage::CuriosityTarget::Archetype(target_name) => {
                let relic_pick = shop
                    .relics
                    .iter()
                    .enumerate()
                    .filter(|(_, relic)| rs.gold >= relic.price)
                    .map(|(idx, relic)| {
                        (
                            ClientInput::BuyRelic(idx),
                            self.archetype_relic_bonus(relic.relic_id, target_name),
                        )
                    })
                    .filter(|(_, score)| *score > 0)
                    .max_by_key(|(_, score)| *score);
                let card_pick = shop
                    .cards
                    .iter()
                    .enumerate()
                    .filter(|(_, card)| rs.gold >= card.price)
                    .map(|(idx, card)| {
                        (
                            ClientInput::BuyCard(idx),
                            self.archetype_card_bonus(card.card_id, target_name),
                        )
                    })
                    .filter(|(_, score)| *score > 0)
                    .max_by_key(|(_, score)| *score);
                match (relic_pick, card_pick) {
                    (Some((cmd_a, score_a)), Some((cmd_b, score_b))) => {
                        if score_a >= score_b {
                            Some(cmd_a)
                        } else {
                            Some(cmd_b)
                        }
                    }
                    (Some((cmd, _)), None) | (None, Some((cmd, _))) => Some(cmd),
                    (None, None) => None,
                }
            }
            crate::bot::coverage::CuriosityTarget::Source(target_name) => shop
                .relics
                .iter()
                .enumerate()
                .find(|(_, relic)| {
                    rs.gold >= relic.price
                        && Self::matches_relic_target(relic.relic_id, target_name)
                })
                .map(|(idx, _)| ClientInput::BuyRelic(idx))
                .or_else(|| {
                    shop.cards
                        .iter()
                        .enumerate()
                        .find(|(_, card)| {
                            rs.gold >= card.price
                                && Self::matches_card_target(card.card_id, target_name)
                        })
                        .map(|(idx, _)| ClientInput::BuyCard(idx))
                })
                .or_else(|| {
                    shop.potions
                        .iter()
                        .enumerate()
                        .find(|(_, potion)| {
                            rs.gold >= potion.price
                                && rs.potions.iter().any(|p| p.is_none())
                                && crate::content::potions::get_potion_definition(potion.potion_id)
                                    .name
                                    .eq_ignore_ascii_case(target_name)
                        })
                        .map(|(idx, _)| ClientInput::BuyPotion(idx))
                }),
            _ => None,
        }
    }

    pub(crate) fn should_purge_at_shop(
        &self,
        rs: &RunState,
        shop: &crate::shop::ShopState,
    ) -> bool {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let searing_plan = self.searing_blow_plan_score(rs, &profile);
        let worst_idx = self.best_purge_index(rs);
        let worst = &rs.master_deck[worst_idx];
        let worst_score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(worst.id, rs);
        if crate::content::cards::is_starter_basic(worst.id) {
            return true;
        }
        if crate::content::cards::get_card_definition(worst.id).card_type
            == crate::content::cards::CardType::Curse
        {
            return true;
        }
        if crate::bot::evaluator::curse_remove_severity(worst.id) >= 8 {
            return true;
        }
        if searing_plan > 0 {
            let affordable_deficit_pick = shop.cards.iter().any(|card| {
                rs.gold >= card.price + shop.purge_cost
                    && self.shop_card_score(rs, card.card_id) >= 60
            }) || shop.potions.iter().any(|potion| {
                rs.gold >= potion.price + shop.purge_cost
                    && rs.potions.iter().any(|slot| slot.is_none())
                    && self.shop_potion_score(rs, potion.potion_id) >= 90
            });
            if affordable_deficit_pick {
                return false;
            }
        }
        if worst_score <= 10 {
            let affordable_high_value_card = shop.cards.iter().any(|card| {
                rs.gold >= card.price + shop.purge_cost
                    && self.shop_card_score(rs, card.card_id) >= 65
            });
            return !affordable_high_value_card;
        }
        if self.shell_core_preservation_penalty(worst.id, &profile) >= 50 {
            return false;
        }
        false
    }

    pub(crate) fn shop_relic_score(
        &self,
        rs: &RunState,
        relic_id: crate::content::relics::RelicId,
    ) -> i32 {
        use crate::content::relics::RelicId;

        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let searing_plan = self.searing_blow_plan_score(rs, &profile);
        let output_gap = self.shop_needs_frontload_damage(rs, &profile);
        let defense_gap = self.shop_needs_reliable_block(rs, &profile);
        let control_gap = self.shop_needs_damage_control(rs);

        let mut score = match relic_id {
            RelicId::MembershipCard => 115,
            RelicId::OrangePellets => 112,
            RelicId::DeadBranch => 110,
            RelicId::Pocketwatch => 108,
            RelicId::MedicalKit => 104,
            RelicId::ClockworkSouvenir => 102,
            RelicId::Shuriken => 100,
            RelicId::ChemicalX => 98,
            RelicId::ToughBandages => 96,
            RelicId::ToyOrnithopter => 94,
            RelicId::PrayerWheel => 93,
            RelicId::Sundial => 92,
            RelicId::Calipers => 92,
            RelicId::IceCream => 92,
            RelicId::IncenseBurner => 91,
            RelicId::Girya => 90,
            RelicId::Shovel => 88,
            RelicId::PeacePipe => 88,
            RelicId::LetterOpener => 84,
            RelicId::PenNib => 83,
            RelicId::Kunai => 83,
            RelicId::Nunchaku => 82,
            RelicId::DataDisk | RelicId::MercuryHourglass | RelicId::PreservedInsect => 80,
            _ => 50,
        };

        match relic_id {
            RelicId::Shuriken | RelicId::PenNib | RelicId::Kunai | RelicId::Nunchaku => {
                score += profile.attack_count * 2;
            }
            RelicId::ChemicalX => {
                score += profile.x_cost_payoffs * 18;
            }
            RelicId::DeadBranch => {
                score += profile.exhaust_outlets * 4 + profile.exhaust_engines * 5;
            }
            RelicId::MedicalKit => {
                score += profile.exhaust_fodder * 5 + profile.exhaust_engines * 4;
            }
            RelicId::OrangePellets => {
                score += profile.power_count * 2;
            }
            RelicId::ClockworkSouvenir => {
                score += profile.strength_enablers * 4 + profile.self_damage_sources * 2;
            }
            RelicId::LetterOpener => {
                score += profile.skill_count;
            }
            RelicId::Calipers => {
                score += profile.block_core * 3 + profile.block_payoffs * 5;
            }
            RelicId::IceCream => {
                score += profile.x_cost_payoffs * 6;
            }
            RelicId::PeacePipe => {
                let bad_basics = rs
                    .master_deck
                    .iter()
                    .filter(|c| crate::content::cards::is_starter_basic(c.id))
                    .count() as i32;
                score += bad_basics * 2;
            }
            _ => {}
        }

        if output_gap {
            match relic_id {
                RelicId::PenNib | RelicId::Shuriken => score += 8,
                RelicId::PreservedInsect => score += 10,
                _ => {}
            }
        }
        if defense_gap {
            match relic_id {
                RelicId::IncenseBurner => score += 10,
                RelicId::Calipers => score += 6,
                _ => {}
            }
        }
        if control_gap {
            if relic_id == RelicId::ClockworkSouvenir {
                score += 8;
            }
        }
        if searing_plan > 0 {
            match relic_id {
                RelicId::PenNib | RelicId::Nunchaku => score += 10,
                _ => {}
            }
        }

        if let Some(target) = self.curiosity_archetype_target() {
            score += self.archetype_relic_bonus(relic_id, target);
        }

        score
    }

    pub(crate) fn shop_card_score(
        &self,
        rs: &RunState,
        card_id: crate::content::cards::CardId,
    ) -> i32 {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut score = crate::bot::evaluator::CardEvaluator::evaluate_card(card_id, rs);
        if self.is_high_value_tactical_card(card_id) {
            score += 15;
        }
        score += self.shop_shell_card_bonus(card_id, &profile);
        score += self.shop_deficit_card_bonus(rs, card_id, &profile);
        if let Some(target) = self.curiosity_archetype_target() {
            score += self.archetype_card_bonus(card_id, target);
        }
        score
    }

    pub(crate) fn shop_card_buy_threshold(&self, rs: &RunState, score: i32) -> i32 {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let shell_incomplete = (profile.strength_enablers > 0 && profile.strength_payoffs == 0)
            || (profile.exhaust_engines > 0 && profile.exhaust_outlets == 0)
            || (profile.block_core >= 2 && profile.block_payoffs == 0);
        let acute_deficits = self.shop_needs_frontload_damage(rs, &profile) as i32
            + self.shop_needs_reliable_block(rs, &profile) as i32
            + self.shop_needs_damage_control(rs) as i32;

        if let Some(target) = self.curiosity_archetype_target() {
            if self.archetype_alignment_bonus(&profile, target) <= 0 {
                return 45;
            }
        }

        if acute_deficits >= 2 {
            return if score >= 60 { 42 } else { 46 };
        }

        if score >= 72 {
            55
        } else if shell_incomplete || rs.act_num == 1 {
            48
        } else {
            52
        }
    }

    pub(crate) fn shop_shell_card_bonus(
        &self,
        card_id: crate::content::cards::CardId,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> i32 {
        use crate::content::cards::CardId;

        match card_id {
            CardId::LimitBreak if profile.strength_enablers >= 1 => 18,
            CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm
                if profile.strength_payoffs >= 2 =>
            {
                12
            }
            CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Pummel | CardId::Whirlwind
                if profile.strength_enablers >= 2 =>
            {
                8
            }
            CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace
                if profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1 =>
            {
                18
            }
            CardId::SecondWind | CardId::BurningPact | CardId::SeverSoul | CardId::FiendFire
                if profile.exhaust_engines >= 2 =>
            {
                10
            }
            CardId::Barricade | CardId::Entrench if profile.block_core >= 3 => 16,
            CardId::BodySlam | CardId::FlameBarrier | CardId::Impervious
                if profile.block_payoffs >= 1 =>
            {
                10
            }
            _ => 0,
        }
    }

    pub(crate) fn shop_potion_score(
        &self,
        rs: &RunState,
        potion_id: crate::content::potions::PotionId,
    ) -> i32 {
        use crate::content::potions::PotionId;
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut score = match potion_id {
            PotionId::AncientPotion => 100,
            PotionId::PowerPotion | PotionId::ColorlessPotion => 94,
            PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
            PotionId::BlessingOfTheForge => 84,
            PotionId::StrengthPotion
            | PotionId::DexterityPotion
            | PotionId::SpeedPotion
            | PotionId::SteroidPotion
            | PotionId::EssenceOfSteel
            | PotionId::LiquidBronze
            | PotionId::RegenPotion => 85,
            PotionId::EnergyPotion | PotionId::SwiftPotion => 82,
            _ => 55,
        };

        if self.shop_needs_frontload_damage(rs, &profile) {
            match potion_id {
                PotionId::FearPotion
                | PotionId::FirePotion
                | PotionId::ExplosivePotion
                | PotionId::AttackPotion => score += 16,
                PotionId::StrengthPotion | PotionId::DuplicationPotion => score += 14,
                _ => {}
            }
        }
        if self.shop_needs_reliable_block(rs, &profile) {
            match potion_id {
                PotionId::GhostInAJar => score += 24,
                PotionId::BlockPotion
                | PotionId::WeakenPotion
                | PotionId::DexterityPotion
                | PotionId::EssenceOfSteel
                | PotionId::LiquidBronze => score += 16,
                _ => {}
            }
        }
        if self.shop_needs_damage_control(rs) {
            match potion_id {
                PotionId::WeakenPotion | PotionId::FearPotion => score += 12,
                _ => {}
            }
        }
        if self.searing_blow_plan_score(rs, &profile) > 0 {
            match potion_id {
                PotionId::DuplicationPotion => score += 20,
                PotionId::StrengthPotion => score += 12,
                PotionId::FearPotion => score += 10,
                PotionId::BlessingOfTheForge => score += 18,
                _ => {}
            }
        }

        score
    }

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
            self.map_path = Self::compute_map_path_with_target(rs, self.curiosity_target.as_ref());
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
        use crate::content::relics::RelicId;

        let has_relic = |id: RelicId| rs.relics.iter().any(|r| r.id == id);
        let can_rest = !has_relic(RelicId::CoffeeDripper);
        let can_smith = !has_relic(RelicId::FusionHammer) && self.best_upgrade_index(rs).is_some();
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

        if can_recall && rs.act_num >= 3 && hp_ratio >= 0.45 && !pre_boss_floor {
            return ClientInput::CampfireOption(CampfireChoice::Recall);
        }

        if can_toke
            && hp_ratio >= 0.75
            && worst_card_score <= 10
            && !strong_upgrade_exists
            && !shell_needs_smith
            && bad_basic_count >= 3
        {
            return ClientInput::CampfireOption(CampfireChoice::Toke(self.best_purge_index(rs)));
        }

        if can_lift && hp_ratio >= 0.75 {
            return ClientInput::CampfireOption(CampfireChoice::Lift);
        }

        if can_dig && hp_ratio >= 0.85 {
            return ClientInput::CampfireOption(CampfireChoice::Dig);
        }

        if can_rest && (hp_ratio < 0.5 || (rs.act_num != 1 && pre_boss_floor && hp_ratio < 0.9)) {
            ClientInput::CampfireOption(CampfireChoice::Rest)
        } else if can_smith {
            if searing_plan > 0 {
                if let Some((idx, _)) = rs
                    .master_deck
                    .iter()
                    .enumerate()
                    .find(|(_, c)| c.id == crate::content::cards::CardId::SearingBlow)
                {
                    if hp_ratio >= 0.4 || rs.act_num == 1 {
                        return ClientInput::CampfireOption(CampfireChoice::Smith(idx));
                    }
                }
            }
            ClientInput::CampfireOption(CampfireChoice::Smith(
                self.best_upgrade_index(rs).unwrap_or(0),
            ))
        } else if can_rest {
            ClientInput::CampfireOption(CampfireChoice::Rest)
        } else {
            ClientInput::Proceed
        }
    }

    pub(crate) fn decide_event(&self, rs: &RunState) -> ClientInput {
        use crate::content::relics::RelicId;
        use crate::state::events::EventId;

        if let Some(event) = &rs.event_state {
            let hp_per = (rs.current_hp as f32 / rs.max_hp.max(1) as f32) * 100.0;
            let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
            let choice = match event.id {
                EventId::ScrapOoze => {
                    if rs.current_hp < 25 && event.current_screen == 0 {
                        1
                    } else if event.current_screen == 1 {
                        0
                    } else {
                        0
                    }
                }
                EventId::BigFish => {
                    if hp_per <= 30.0 {
                        0
                    } else {
                        1
                    }
                }
                EventId::Cleric => {
                    if hp_per <= 65.0 {
                        0
                    } else if hp_per >= 90.0 {
                        2
                    } else {
                        1
                    }
                }
                EventId::DeadAdventurer => 1,
                EventId::GoldenIdol => {
                    if rs.relics.iter().any(|r| r.id == RelicId::Ectoplasm) {
                        1
                    } else if hp_per >= 90.0 {
                        1
                    } else {
                        2
                    }
                }
                EventId::Mushrooms => {
                    if hp_per >= 40.0 {
                        0
                    } else {
                        1
                    }
                }
                EventId::LivingWall => {
                    if event.current_screen > 0 {
                        0
                    } else if self.has_good_purge_target(rs) && self.profile_has_too_many_basics(rs)
                    {
                        0
                    } else if self.best_upgrade_index(rs).is_some() {
                        2
                    } else {
                        1
                    }
                }
                EventId::ShiningLight => {
                    if hp_per >= 70.0 {
                        0
                    } else {
                        1
                    }
                }
                EventId::Ssssserpent => 1,
                EventId::WorldOfGoop => {
                    if hp_per >= 80.0 {
                        0
                    } else {
                        1
                    }
                }
                EventId::MatchAndKeep => 0,
                EventId::GoldenWing => {
                    if hp_per >= 70.0 {
                        0
                    } else {
                        1
                    }
                }
                EventId::GoldenShrine => 0,
                EventId::Purifier => {
                    if event.current_screen > 0 || self.has_good_purge_target(rs) {
                        0
                    } else {
                        1
                    }
                }
                EventId::UpgradeShrine => {
                    if event.current_screen > 0 || self.best_upgrade_index(rs).is_some() {
                        0
                    } else {
                        1
                    }
                }
                EventId::Transmorgrifier => {
                    if event.current_screen > 0 || self.has_good_purge_target(rs) {
                        0
                    } else {
                        1
                    }
                }
                EventId::Lab => 0,
                EventId::Duplicator => {
                    if event.current_screen > 0 || self.has_good_duplicate_target(rs, &profile) {
                        0
                    } else {
                        1
                    }
                }
                EventId::MaskedBandits => {
                    if hp_per >= 65.0 {
                        1
                    } else {
                        0
                    }
                }
                EventId::Vampires => 2,
                EventId::Ghosts => 1,
                EventId::TheLibrary => {
                    if event.current_screen > 0 {
                        0
                    } else if hp_per < 45.0 && !Self::profile_needs_support(&profile) {
                        1
                    } else {
                        0
                    }
                }
                EventId::CursedTome => 1,
                EventId::ForgottenAltar => 1,
                EventId::KnowingSkull => 3,
                EventId::Beggar => 0,
                EventId::Falling => 0,
                EventId::MindBloom => 0,
                EventId::Colosseum => 0,
                EventId::MysteriousSphere => {
                    if hp_per >= 70.0 {
                        0
                    } else {
                        1
                    }
                }
                EventId::TombRedMask => 1,
                EventId::WindingHalls => 2,
                EventId::Nest => 0,
                EventId::FaceTrader => 2,
                EventId::Nloth => 2,
                EventId::WomanInBlue => 0,
                _ => 0,
            };
            ClientInput::EventChoice(choice)
        } else {
            ClientInput::EventChoice(0)
        }
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

    pub(crate) fn shop_deficit_card_bonus(
        &self,
        rs: &RunState,
        card_id: crate::content::cards::CardId,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> i32 {
        use crate::content::cards::CardId;

        let mut bonus = 0;
        let searing_plan = self.searing_blow_plan_score(rs, profile);

        if self.shop_needs_frontload_damage(rs, profile) {
            bonus += match card_id {
                CardId::Hemokinesis => 34,
                CardId::Carnage => 28,
                CardId::Pummel | CardId::Whirlwind => 22,
                CardId::SearingBlow => 24,
                CardId::Immolate => 26,
                CardId::Uppercut => 12,
                _ => 0,
            };
        }
        if self.shop_needs_reliable_block(rs, profile) {
            bonus += match card_id {
                CardId::ShrugItOff => 20,
                CardId::FlameBarrier => 22,
                CardId::GhostlyArmor => 16,
                CardId::Impervious => 26,
                CardId::PowerThrough => 14,
                CardId::Disarm => 12,
                _ => 0,
            };
        }
        if self.shop_needs_damage_control(rs) {
            bonus += match card_id {
                CardId::Disarm => 24,
                CardId::Shockwave => 22,
                CardId::Uppercut => 18,
                CardId::Clothesline => 10,
                _ => 0,
            };
        }
        if searing_plan > 0 {
            bonus += match card_id {
                CardId::SearingBlow => 40 + profile.searing_blow_upgrades * 10,
                CardId::Armaments => 18,
                CardId::Offering => 18,
                CardId::BattleTrance | CardId::Headbutt | CardId::SeeingRed => 12,
                CardId::ShrugItOff => 8,
                CardId::DoubleTap => 10,
                _ => 0,
            };
        }

        bonus
    }

    pub(crate) fn profile_is_online(profile: &crate::bot::evaluator::DeckProfile) -> bool {
        (profile.strength_enablers >= 1 && profile.strength_payoffs >= 2)
            || (profile.exhaust_engines >= 2 && profile.exhaust_outlets >= 1)
            || (profile.block_core >= 3 && profile.block_payoffs >= 1)
    }

    pub(crate) fn profile_has_too_many_basics(&self, rs: &RunState) -> bool {
        rs.master_deck
            .iter()
            .filter(|c| {
                matches!(
                    c.id,
                    id if crate::content::cards::is_starter_basic(id)
                )
            })
            .count()
            >= 4
    }

    pub(crate) fn has_good_purge_target(&self, rs: &RunState) -> bool {
        if rs.master_deck.is_empty() {
            return false;
        }
        let idx = self.best_purge_index(rs);
        let card = &rs.master_deck[idx];
        let score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs);
        matches!(
            card.id,
            id if crate::content::cards::is_starter_basic(id)
        ) || score <= 10
    }

    pub(crate) fn has_good_duplicate_target(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> bool {
        rs.master_deck.iter().any(|card| {
            crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs)
                + self.upgrade_shell_bonus(card.id, profile)
                >= 70
        })
    }
}
