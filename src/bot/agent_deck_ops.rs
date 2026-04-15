use crate::bot::agent::Agent;
use crate::bot::card_disposition::{self, DeckDispositionMode};
use crate::state::run::RunState;

impl Agent {
    fn deck_cut_score(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
        card: &crate::combat::CombatCard,
        mode: DeckCutMode,
    ) -> i32 {
        let bash_preservation_bonus = if card.id == crate::content::cards::CardId::Bash {
            self.bash_preservation_bonus(rs, profile, mode)
        } else {
            0
        };
        card_disposition::deck_cut_score(
            rs,
            profile,
            card,
            match mode {
                DeckCutMode::Purge => DeckDispositionMode::Purge,
                DeckCutMode::Transform => DeckDispositionMode::Transform,
                DeckCutMode::TransformUpgraded => DeckDispositionMode::TransformUpgraded,
            },
            bash_preservation_bonus,
        )
    }

    fn bash_preservation_bonus(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
        mode: DeckCutMode,
    ) -> i32 {
        let early_opening = rs.act_num <= 1 && rs.floor_num <= 4;
        let deck_is_thin = rs.master_deck.len() <= 14;
        let vuln_sources = rs
            .master_deck
            .iter()
            .filter(|card| {
                matches!(
                    card.id,
                    crate::content::cards::CardId::Bash
                        | crate::content::cards::CardId::Uppercut
                        | crate::content::cards::CardId::Shockwave
                        | crate::content::cards::CardId::ThunderClap
                )
            })
            .count() as i32;
        let frontload_gap = self.shop_needs_frontload_damage(rs, profile);

        let mut bonus = 0;
        if early_opening && deck_is_thin {
            bonus += 260;
        }
        if vuln_sources <= 1 {
            bonus += 120;
        }
        if frontload_gap {
            bonus += 80;
        }
        if matches!(mode, DeckCutMode::TransformUpgraded) {
            bonus += 80;
        }
        if rs
            .master_deck
            .iter()
            .filter(|card| crate::content::cards::is_starter_basic(card.id))
            .count()
            >= 4
        {
            bonus += 40;
        }

        bonus
    }

    fn ranked_deck_cut_indices(
        &self,
        rs: &RunState,
        count: usize,
        mode: DeckCutMode,
    ) -> Vec<usize> {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut ranked = rs
            .master_deck
            .iter()
            .enumerate()
            .filter(|(_, card)| {
                !matches!(
                    mode,
                    DeckCutMode::TransformUpgraded
                        if matches!(
                            crate::content::cards::get_card_definition(card.id).card_type,
                            crate::content::cards::CardType::Curse
                                | crate::content::cards::CardType::Status
                        )
                )
            })
            .map(|(idx, card)| (idx, self.deck_cut_score(rs, &profile, card, mode)))
            .collect::<Vec<_>>();

        ranked.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        ranked.into_iter().take(count).map(|(idx, _)| idx).collect()
    }

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
        self.best_purge_indices(rs, 1)
            .into_iter()
            .next()
            .unwrap_or(0)
    }

    pub(crate) fn best_purge_indices(&self, rs: &RunState, count: usize) -> Vec<usize> {
        self.ranked_deck_cut_indices(rs, count.min(rs.master_deck.len()), DeckCutMode::Purge)
    }

    pub(crate) fn best_upgrade_index(&self, rs: &RunState) -> Option<usize> {
        if let Some(crate::bot::coverage::CuriosityTarget::Card(target_name)) =
            self.active_curiosity_target()
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

    #[allow(dead_code)]
    pub(crate) fn best_transform_index(&self, rs: &RunState) -> usize {
        self.best_transform_indices(rs, 1, false)
            .into_iter()
            .next()
            .unwrap_or(0)
    }

    pub(crate) fn best_transform_indices(
        &self,
        rs: &RunState,
        count: usize,
        upgraded_context: bool,
    ) -> Vec<usize> {
        let mode = if upgraded_context {
            DeckCutMode::TransformUpgraded
        } else {
            DeckCutMode::Transform
        };
        self.ranked_deck_cut_indices(rs, count.min(rs.master_deck.len()), mode)
    }

    pub(crate) fn best_duplicate_index(&self, rs: &RunState) -> Option<usize> {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let mut best_score = i32::MIN;
        let mut best_idx = None;

        for (i, c) in rs.master_deck.iter().enumerate() {
            let mut score = card_disposition::duplicate_score(rs, &profile, c);
            if let Some(target) = self.curiosity_archetype_target() {
                score += self.archetype_card_bonus(c.id, target);
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
        card_disposition::shell_core_preservation_penalty(card_id, profile)
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

#[derive(Clone, Copy)]
enum DeckCutMode {
    Purge,
    Transform,
    TransformUpgraded,
}
