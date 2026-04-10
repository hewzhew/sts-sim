use crate::bot::agent::Agent;
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn searing_blow_plan_score(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> i32 {
        use crate::content::relics::RelicId;

        if profile.searing_blow_count <= 0 {
            return 0;
        }

        let upgrades = rs
            .master_deck
            .iter()
            .find(|c| c.id == crate::content::cards::CardId::SearingBlow)
            .map(|c| c.upgrades as i32)
            .unwrap_or(profile.searing_blow_upgrades);

        let mut score = 40 + upgrades * 18;
        if rs.act_num == 1 {
            score += 25;
        }
        if rs.floor_num <= 16 {
            score += 15;
        }
        if rs.master_deck.len() <= 14 {
            score += 10;
        }
        if profile.draw_sources >= 1 {
            score += 4;
        }
        if profile.block_core >= 1 {
            score += 4;
        }
        if rs.relics.iter().any(|r| r.id == RelicId::BustedCrown) {
            // Drafting is constrained, so existing scalable output should be committed harder.
            score += 24;
        }
        score
    }

    pub(crate) fn best_purge_index(&self, rs: &RunState) -> usize {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut best_score = i32::MAX;
        let mut best_idx = 0;
        for (i, c) in rs.master_deck.iter().enumerate() {
            let mut score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(c.id, rs);
            let def = crate::content::cards::get_card_definition(c.id);
            if matches!(
                def.card_type,
                crate::content::cards::CardType::Curse | crate::content::cards::CardType::Status
            ) {
                score -= 2_000;
            }
            score -= crate::bot::evaluator::curse_remove_severity(c.id) * 450;
            if crate::content::cards::is_starter_basic(c.id) {
                score -= 200;
            }
            score += self.shell_core_preservation_penalty(c.id, &profile);
            if score < best_score {
                best_score = score;
                best_idx = i;
            }
        }
        best_idx
    }

    pub(crate) fn best_upgrade_index(&self, rs: &RunState) -> Option<usize> {
        if let Some(crate::bot::coverage::CuriosityTarget::Card(target_name)) =
            self.curiosity_target.as_ref()
        {
            if let Some(idx) = rs.master_deck.iter().position(|c| {
                c.upgrades == 0
                    && crate::content::cards::get_card_definition(c.id)
                        .name
                        .eq_ignore_ascii_case(target_name.as_str())
            }) {
                return Some(idx);
            }
        }

        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let searing_plan = self.searing_blow_plan_score(rs, &profile);
        if let Some(committed) = self.searing_blow_committed_upgrade(rs, &profile, searing_plan) {
            return Some(committed);
        }
        let mut best_score = i32::MIN;
        let mut best_idx = None;
        for (i, c) in rs.master_deck.iter().enumerate() {
            let upgradable = c.id == crate::content::cards::CardId::SearingBlow || c.upgrades == 0;
            if !upgradable {
                continue;
            }
            let mut score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(c.id, rs);
            if matches!(
                c.id,
                crate::content::cards::CardId::Apotheosis
                    | crate::content::cards::CardId::Corruption
                    | crate::content::cards::CardId::FeelNoPain
                    | crate::content::cards::CardId::DarkEmbrace
                    | crate::content::cards::CardId::Shockwave
                    | crate::content::cards::CardId::Uppercut
                    | crate::content::cards::CardId::FlameBarrier
            ) {
                score += 20;
            }
            if c.id == crate::content::cards::CardId::SearingBlow {
                score += searing_plan + c.upgrades as i32 * 28;
            } else if searing_plan > 0 {
                match c.id {
                    crate::content::cards::CardId::Apotheosis => score += 45,
                    crate::content::cards::CardId::Armaments => score += 38,
                    crate::content::cards::CardId::Offering => score += 34,
                    crate::content::cards::CardId::BattleTrance
                    | crate::content::cards::CardId::Headbutt
                    | crate::content::cards::CardId::SeeingRed
                    | crate::content::cards::CardId::ShrugItOff => score += 10,
                    _ => {}
                }
                score -= searing_plan / 4;
            }
            score += self.upgrade_shell_bonus(c.id, &profile);
            if let Some(target) = self.curiosity_archetype_target() {
                score += self.archetype_card_bonus(c.id, target);
            }
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }
        best_idx
    }

    fn searing_blow_committed_upgrade(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
        searing_plan: i32,
    ) -> Option<usize> {
        use crate::content::cards::CardId;
        use crate::content::relics::RelicId;

        if profile.searing_blow_count <= 0 || searing_plan <= 0 {
            return None;
        }

        let searing_idx = rs
            .master_deck
            .iter()
            .position(|c| c.id == CardId::SearingBlow)?;
        let searing_upgrades = rs.master_deck[searing_idx].upgrades as i32;
        let has_busted_crown = rs.relics.iter().any(|r| r.id == RelicId::BustedCrown);
        let route_locked = searing_plan >= 92 || (has_busted_crown && searing_plan >= 70);
        if !route_locked {
            return None;
        }

        let premium_unupgraded = rs
            .master_deck
            .iter()
            .filter(|c| {
                c.upgrades == 0
                    && matches!(
                        c.id,
                        CardId::Apotheosis
                            | CardId::Armaments
                            | CardId::Offering
                            | CardId::BattleTrance
                            | CardId::ShrugItOff
                            | CardId::FlameBarrier
                            | CardId::Impervious
                            | CardId::DarkEmbrace
                            | CardId::Corruption
                            | CardId::FeelNoPain
                    )
            })
            .count() as i32;
        let upgradable_non_searing = rs
            .master_deck
            .iter()
            .filter(|c| c.id != CardId::SearingBlow && c.upgrades == 0)
            .count() as i32;
        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;

        let mut best_override: Option<(usize, i32)> = None;
        for (idx, card) in rs.master_deck.iter().enumerate() {
            if idx == searing_idx || card.upgrades > 0 {
                continue;
            }
            let override_score = match card.id {
                CardId::Apotheosis if upgradable_non_searing >= 5 && searing_upgrades >= 2 => {
                    Some(220 + upgradable_non_searing * 8)
                }
                CardId::Armaments
                    if premium_unupgraded >= 5 && searing_upgrades >= 4 && hp_ratio >= 0.55 =>
                {
                    Some(195 + premium_unupgraded * 5)
                }
                CardId::Offering if searing_upgrades >= 3 && hp_ratio >= 0.65 => Some(184),
                _ => None,
            };
            if let Some(score) = override_score {
                match best_override {
                    Some((_, current)) if current >= score => {}
                    _ => best_override = Some((idx, score)),
                }
            }
        }

        let searing_anchor = 210 + searing_plan + searing_upgrades * 32;
        if let Some((idx, _score)) = best_override.filter(|(_, score)| *score > searing_anchor) {
            Some(idx)
        } else {
            Some(searing_idx)
        }
    }

    pub(crate) fn best_transform_index(&self, rs: &RunState) -> usize {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut best_score = i32::MAX;
        let mut best_idx = 0;

        for (i, c) in rs.master_deck.iter().enumerate() {
            let mut score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(c.id, rs);
            let def = crate::content::cards::get_card_definition(c.id);

            if matches!(
                def.card_type,
                crate::content::cards::CardType::Curse | crate::content::cards::CardType::Status
            ) {
                score -= 2_000;
            }
            if crate::content::cards::is_starter_basic(c.id) {
                score -= 180;
            }
            score += self.shell_core_preservation_penalty(c.id, &profile);

            if score < best_score {
                best_score = score;
                best_idx = i;
            }
        }

        best_idx
    }

    pub(crate) fn best_duplicate_index(&self, rs: &RunState) -> Option<usize> {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut best_score = i32::MIN;
        let mut best_idx = None;

        for (i, c) in rs.master_deck.iter().enumerate() {
            let mut score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(c.id, rs);
            score += self.duplicate_shell_bonus(c.id, &profile);
            if let Some(target) = self.curiosity_archetype_target() {
                score += self.archetype_card_bonus(c.id, target);
            }

            if crate::content::cards::is_starter_basic(c.id) {
                score -= 120;
            }

            if score > best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        best_idx.filter(|_| best_score >= 35)
    }

    pub(crate) fn shell_core_preservation_penalty(
        &self,
        card_id: crate::content::cards::CardId,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> i32 {
        use crate::content::cards::CardId;

        match card_id {
            CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm | CardId::Rupture
                if profile.strength_payoffs >= 2 =>
            {
                80
            }
            CardId::HeavyBlade | CardId::SwordBoomerang | CardId::TwinStrike | CardId::Pummel
                if profile.strength_enablers >= 2 =>
            {
                45
            }
            CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace
                if profile.exhaust_outlets >= 2 || profile.exhaust_fodder >= 1 =>
            {
                90
            }
            CardId::SecondWind
            | CardId::FiendFire
            | CardId::SeverSoul
            | CardId::BurningPact
            | CardId::TrueGrit
                if profile.exhaust_engines >= 2 =>
            {
                55
            }
            CardId::Barricade | CardId::Entrench if profile.block_core >= 3 => 85,
            CardId::BodySlam | CardId::Juggernaut if profile.block_core >= 3 => 55,
            _ => 0,
        }
    }

    pub(crate) fn duplicate_shell_bonus(
        &self,
        card_id: crate::content::cards::CardId,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> i32 {
        use crate::content::cards::CardId;

        match card_id {
            CardId::LimitBreak if profile.strength_enablers >= 1 => 18,
            CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Whirlwind
                if profile.strength_enablers >= 2 =>
            {
                10
            }
            CardId::FeelNoPain | CardId::DarkEmbrace if profile.exhaust_outlets >= 2 => 14,
            CardId::SecondWind | CardId::BurningPact | CardId::FiendFire
                if profile.exhaust_engines >= 2 =>
            {
                12
            }
            CardId::BodySlam | CardId::Impervious | CardId::FlameBarrier
                if profile.block_payoffs >= 1 =>
            {
                10
            }
            CardId::Offering | CardId::Shockwave | CardId::Apotheosis => 18,
            _ => 0,
        }
    }

    pub(crate) fn upgrade_shell_bonus(
        &self,
        card_id: crate::content::cards::CardId,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> i32 {
        use crate::content::cards::CardId;

        match card_id {
            CardId::LimitBreak if profile.strength_enablers >= 1 => 24,
            CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Whirlwind
                if profile.strength_enablers >= 2 =>
            {
                10
            }
            CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace
                if profile.exhaust_outlets >= 1 || profile.exhaust_fodder >= 1 =>
            {
                18
            }
            CardId::SecondWind | CardId::BurningPact | CardId::TrueGrit
                if profile.exhaust_engines >= 2 =>
            {
                10
            }
            CardId::Barricade | CardId::Entrench if profile.block_core >= 3 => 18,
            CardId::BodySlam | CardId::FlameBarrier | CardId::Impervious
                if profile.block_payoffs >= 1 =>
            {
                10
            }
            _ => 0,
        }
    }
}
