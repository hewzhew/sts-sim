use crate::bot::agent::Agent;
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn curiosity_reward_pick(
        &self,
        offered_cards: &[crate::content::cards::CardId],
        rs: &RunState,
    ) -> Option<usize> {
        let target_name = match self.curiosity_target.as_ref()? {
            crate::bot::coverage::CuriosityTarget::Card(target_name)
            | crate::bot::coverage::CuriosityTarget::Source(target_name) => target_name,
            crate::bot::coverage::CuriosityTarget::Archetype(target_name) => {
                let mut best_idx = None;
                let mut best_bonus = 0;
                for (idx, &card_id) in offered_cards.iter().enumerate() {
                    let bonus = self.archetype_card_bonus(card_id, target_name.as_str());
                    if bonus > best_bonus {
                        best_bonus = bonus;
                        best_idx = Some(idx);
                    }
                }
                if best_bonus > 0 {
                    return best_idx;
                }
                return self.curiosity_shell_seed_pick(offered_cards, rs, target_name.as_str());
            }
            _ => return None,
        };
        offered_cards
            .iter()
            .position(|card_id| Self::matches_card_target(*card_id, target_name.as_str()))
    }

    pub(crate) fn curiosity_shell_seed_pick(
        &self,
        offered_cards: &[crate::content::cards::CardId],
        rs: &RunState,
        archetype: &str,
    ) -> Option<usize> {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let target_normalized = Self::normalize_lookup_name(archetype);
        let mut best_idx = None;
        let mut best_score = i32::MIN;

        for (idx, &card_id) in offered_cards.iter().enumerate() {
            let mut score = self.archetype_card_bonus(card_id, archetype);
            if score == 0 {
                score = match target_normalized.as_str() {
                    "strength" => match card_id {
                        crate::content::cards::CardId::Inflame
                        | crate::content::cards::CardId::SpotWeakness
                        | crate::content::cards::CardId::DemonForm
                        | crate::content::cards::CardId::HeavyBlade
                        | crate::content::cards::CardId::SwordBoomerang
                        | crate::content::cards::CardId::Whirlwind => 12,
                        _ => 0,
                    },
                    "exhaust" => match card_id {
                        crate::content::cards::CardId::Corruption
                        | crate::content::cards::CardId::FeelNoPain
                        | crate::content::cards::CardId::DarkEmbrace
                        | crate::content::cards::CardId::SecondWind
                        | crate::content::cards::CardId::BurningPact
                        | crate::content::cards::CardId::TrueGrit => 12,
                        _ => 0,
                    },
                    "block" => match card_id {
                        crate::content::cards::CardId::Barricade
                        | crate::content::cards::CardId::Entrench
                        | crate::content::cards::CardId::BodySlam
                        | crate::content::cards::CardId::FlameBarrier
                        | crate::content::cards::CardId::Impervious => 12,
                        _ => 0,
                    },
                    _ => 0,
                };
            }
            score +=
                crate::bot::reward_heuristics::pick_probability(card_id).mul_add(10.0, 0.0) as i32;
            score += self.archetype_alignment_bonus(&profile, archetype);
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }

        best_idx.filter(|_| best_score > 0)
    }

    pub(crate) fn curiosity_reward_claim(
        &self,
        reward_items: &[crate::rewards::state::RewardItem],
    ) -> Option<usize> {
        let target_name = match self.curiosity_target.as_ref()? {
            crate::bot::coverage::CuriosityTarget::Relic(target_name)
            | crate::bot::coverage::CuriosityTarget::Source(target_name) => target_name,
            _ => return None,
        };
        reward_items.iter().position(|item| {
            matches!(
                item,
                crate::rewards::state::RewardItem::Relic { relic_id }
                    if Self::matches_relic_target(*relic_id, target_name.as_str())
            )
        })
    }

    pub(crate) fn curiosity_boss_relic_pick(
        &self,
        relics: &[crate::content::relics::RelicId],
    ) -> Option<usize> {
        let target_name = match self.curiosity_target.as_ref()? {
            crate::bot::coverage::CuriosityTarget::Relic(target_name)
            | crate::bot::coverage::CuriosityTarget::Source(target_name) => target_name,
            _ => return None,
        };
        relics
            .iter()
            .position(|relic_id| Self::matches_relic_target(*relic_id, target_name.as_str()))
    }

    pub(crate) fn matches_card_target(
        card_id: crate::content::cards::CardId,
        target_name: &str,
    ) -> bool {
        crate::content::cards::get_card_definition(card_id)
            .name
            .eq_ignore_ascii_case(target_name)
    }

    pub(crate) fn matches_relic_target(
        relic_id: crate::content::relics::RelicId,
        target_name: &str,
    ) -> bool {
        Self::normalize_lookup_name(&format!("{relic_id:?}"))
            == Self::normalize_lookup_name(target_name)
    }

    pub(crate) fn curiosity_archetype_target(&self) -> Option<&str> {
        match self.curiosity_target.as_ref()? {
            crate::bot::coverage::CuriosityTarget::Archetype(target) => Some(target.as_str()),
            _ => None,
        }
    }

    pub(crate) fn archetype_alignment_bonus(
        &self,
        profile: &crate::bot::evaluator::DeckProfile,
        archetype: &str,
    ) -> i32 {
        let target = Self::normalize_lookup_name(archetype);
        let tags = crate::bot::evaluator::CardEvaluator::archetype_tags(profile);
        if tags
            .iter()
            .any(|tag| Self::normalize_lookup_name(tag) == target)
        {
            return 20;
        }
        match target.as_str() {
            "strength" => {
                profile.strength_enablers * 6
                    + profile.strength_payoffs * 6
                    + if profile.strength_enablers > 0 && profile.strength_payoffs == 0 {
                        10
                    } else {
                        0
                    }
            }
            "exhaust" => {
                profile.exhaust_engines * 6
                    + profile.exhaust_outlets * 8
                    + profile.exhaust_fodder * 2
            }
            "block" => profile.block_core * 6 + profile.block_payoffs * 8,
            _ => 0,
        }
    }

    pub(crate) fn archetype_card_bonus(
        &self,
        card_id: crate::content::cards::CardId,
        archetype: &str,
    ) -> i32 {
        use crate::content::cards::CardId;

        match Self::normalize_lookup_name(archetype).as_str() {
            "strength" => match card_id {
                CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm => 22,
                CardId::LimitBreak => 26,
                CardId::HeavyBlade
                | CardId::SwordBoomerang
                | CardId::Pummel
                | CardId::Whirlwind
                | CardId::TwinStrike => 14,
                CardId::Rupture | CardId::Flex => 12,
                _ => 0,
            },
            "exhaust" => match card_id {
                CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace => 24,
                CardId::SecondWind
                | CardId::BurningPact
                | CardId::TrueGrit
                | CardId::SeverSoul
                | CardId::FiendFire => 16,
                _ => 0,
            },
            "block" => match card_id {
                CardId::Barricade | CardId::Entrench => 24,
                CardId::BodySlam | CardId::FlameBarrier | CardId::Impervious => 16,
                CardId::Juggernaut | CardId::ShrugItOff => 10,
                _ => 0,
            },
            "selfdamage" => match card_id {
                CardId::Offering
                | CardId::Bloodletting
                | CardId::Hemokinesis
                | CardId::Combust
                | CardId::Brutality
                | CardId::Rupture => 18,
                CardId::Reaper => 10,
                _ => 0,
            },
            "drawcycle" => match card_id {
                CardId::BattleTrance
                | CardId::PommelStrike
                | CardId::ShrugItOff
                | CardId::Finesse
                | CardId::FlashOfSteel
                | CardId::MasterOfStrategy
                | CardId::BurningPact => 18,
                CardId::Brutality | CardId::Offering => 12,
                _ => 0,
            },
            "powerscaling" => match card_id {
                CardId::DemonForm
                | CardId::Corruption
                | CardId::FeelNoPain
                | CardId::DarkEmbrace
                | CardId::Barricade
                | CardId::Juggernaut
                | CardId::Panache
                | CardId::Mayhem
                | CardId::Magnetism
                | CardId::Evolve
                | CardId::FireBreathing => 18,
                CardId::Inflame | CardId::Brutality | CardId::Combust | CardId::Rupture => 10,
                _ => 0,
            },
            "status" => match card_id {
                CardId::Evolve | CardId::FireBreathing => 22,
                CardId::WildStrike | CardId::RecklessCharge | CardId::PowerThrough => 14,
                CardId::SecondWind => 10,
                _ => 0,
            },
            _ => 0,
        }
    }

    pub(crate) fn archetype_relic_bonus(
        &self,
        relic_id: crate::content::relics::RelicId,
        archetype: &str,
    ) -> i32 {
        use crate::content::relics::RelicId;

        match Self::normalize_lookup_name(archetype).as_str() {
            "strength" => match relic_id {
                RelicId::Shuriken | RelicId::PenNib | RelicId::Girya => 18,
                RelicId::ClockworkSouvenir | RelicId::Nunchaku | RelicId::ChemicalX => 10,
                _ => 0,
            },
            "exhaust" => match relic_id {
                RelicId::DeadBranch | RelicId::MedicalKit => 22,
                RelicId::ToughBandages | RelicId::OrangePellets => 12,
                _ => 0,
            },
            "block" => match relic_id {
                RelicId::Calipers | RelicId::IceCream => 18,
                RelicId::LetterOpener | RelicId::Kunai => 10,
                _ => 0,
            },
            "selfdamage" => match relic_id {
                RelicId::ToyOrnithopter | RelicId::ClockworkSouvenir | RelicId::OrangePellets => 12,
                _ => 0,
            },
            "drawcycle" => match relic_id {
                RelicId::Pocketwatch | RelicId::Sundial | RelicId::LetterOpener => 16,
                RelicId::Nunchaku => 8,
                _ => 0,
            },
            "powerscaling" => match relic_id {
                RelicId::OrangePellets | RelicId::ClockworkSouvenir | RelicId::Pocketwatch => 14,
                _ => 0,
            },
            "status" => match relic_id {
                RelicId::MedicalKit | RelicId::DeadBranch => 16,
                _ => 0,
            },
            _ => 0,
        }
    }

    pub(crate) fn normalize_lookup_name(name: &str) -> String {
        name.chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .map(|ch| ch.to_ascii_lowercase())
            .collect()
    }
}
