use crate::bot::agent::Agent;
use crate::state::run::RunState;

impl Agent {
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
            if matches!(
                c.id,
                crate::content::cards::CardId::Strike | crate::content::cards::CardId::Defend
            ) {
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
            if matches!(
                c.id,
                crate::content::cards::CardId::Strike | crate::content::cards::CardId::Defend
            ) {
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

            if matches!(
                c.id,
                crate::content::cards::CardId::Strike | crate::content::cards::CardId::Defend
            ) {
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
