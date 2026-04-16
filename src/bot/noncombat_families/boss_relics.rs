use crate::bot::agent::Agent;
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn boss_relic_score(
        &self,
        rs: &RunState,
        relic_id: crate::content::relics::RelicId,
    ) -> i32 {
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let need = self.build_noncombat_need_snapshot(rs);
        let avg_cost = average_deck_cost(rs);

        base_tier(relic_id, rs.player_class)
            + relic_modifier_score(self, rs, relic_id, &profile, &need, avg_cost)
    }
}

fn average_deck_cost(rs: &RunState) -> f32 {
    if rs.master_deck.is_empty() {
        return 0.0;
    }
    let total_cost: i32 = rs
        .master_deck
        .iter()
        .map(|card| crate::content::cards::get_card_definition(card.id).cost.max(0) as i32)
        .sum();
    total_cost as f32 / rs.master_deck.len() as f32
}

fn base_tier(relic_id: crate::content::relics::RelicId, player_class: &str) -> i32 {
    base_tier_band(relic_id).score() + class_tier_adjustment(relic_id, player_class)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BossRelicTierBand {
    Premium,
    Strong,
    AboveAverage,
    Solid,
    LeanPositive,
    RiskyUpside,
    Narrow,
    Weak,
    Unknown,
}

impl BossRelicTierBand {
    const fn score(self) -> i32 {
        match self {
            Self::Premium => 100,
            Self::Strong => 90,
            Self::AboveAverage => 80,
            Self::Solid => 70,
            Self::LeanPositive => 60,
            Self::RiskyUpside => 50,
            Self::Narrow => 40,
            Self::Weak => 20,
            Self::Unknown => 0,
        }
    }
}

fn base_tier_band(relic_id: crate::content::relics::RelicId) -> BossRelicTierBand {
    use crate::content::relics::RelicId;

    match relic_id {
        RelicId::Sozu => BossRelicTierBand::Premium,
        RelicId::CursedKey => BossRelicTierBand::Strong,
        RelicId::Astrolabe => BossRelicTierBand::AboveAverage,
        RelicId::SneckoEye => BossRelicTierBand::Solid,
        RelicId::BustedCrown => BossRelicTierBand::LeanPositive,
        RelicId::CoffeeDripper => BossRelicTierBand::RiskyUpside,
        RelicId::FusionHammer => BossRelicTierBand::Narrow,
        RelicId::Ectoplasm => BossRelicTierBand::Narrow,
        RelicId::PhilosopherStone => BossRelicTierBand::Narrow,
        RelicId::VelvetChoker => BossRelicTierBand::Narrow,
        RelicId::EmptyCage => BossRelicTierBand::Weak,
        RelicId::CallingBell => BossRelicTierBand::Weak,
        _ => BossRelicTierBand::Unknown,
    }
}

fn class_tier_adjustment(relic_id: crate::content::relics::RelicId, player_class: &str) -> i32 {
    use crate::content::relics::RelicId;

    match (player_class, relic_id) {
        ("Ironclad", RelicId::CoffeeDripper) => 8,
        _ => 0,
    }
}

fn relic_modifier_score(
    agent: &Agent,
    rs: &RunState,
    relic_id: crate::content::relics::RelicId,
    profile: &crate::bot::evaluator::DeckProfile,
    need: &super::model::NoncombatNeedSnapshot,
    avg_cost: f32,
) -> i32 {
    use crate::content::relics::RelicId;

    let bad_basics = rs
        .master_deck
        .iter()
        .filter(|card| crate::content::cards::is_starter_basic(card.id))
        .count() as i32;
    let mut modifier = 0;

    match relic_id {
        RelicId::Astrolabe | RelicId::EmptyCage => {
            modifier += bad_basics * 4 + need.purge_value / 8;
        }
        RelicId::SneckoEye => {
            if avg_cost >= 1.4 {
                modifier += 18;
            }
            if profile.x_cost_payoffs > 0 {
                modifier -= 12;
            }
            modifier += profile.draw_sources * 2;
        }
        RelicId::FusionHammer => {
            modifier -= need.best_upgrade_value / 18;
            if agent.best_upgrade_index(rs).is_some() {
                modifier -= 12;
            }
        }
        RelicId::PhilosopherStone => {
            modifier += profile.strength_payoffs * 3 + profile.block_core;
            modifier -= need.survival_pressure / 22;
        }
        RelicId::VelvetChoker => {
            if profile.attack_count >= 8 {
                modifier -= 10;
            }
            modifier -= profile.draw_sources * 2;
        }
        RelicId::CoffeeDripper => {
            modifier += 18 - need.survival_pressure / 14;
        }
        RelicId::BustedCrown => {
            if rs.act_num == 1 {
                modifier -= 18;
            }
            modifier -= need.best_upgrade_value / 30;
        }
        RelicId::Sozu => {
            let open_slots = rs.potions.iter().filter(|slot| slot.is_none()).count() as i32;
            modifier -= open_slots * 4;
            modifier += need.survival_pressure / 26;
        }
        RelicId::CursedKey => {
            modifier += need.long_term_meta_value / 20;
        }
        RelicId::Ectoplasm => {
            modifier -= need.purge_value / 28;
        }
        _ => {}
    }

    modifier
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::Agent;
    use crate::content::cards::CardId;
    use crate::content::relics::RelicId;

    #[test]
    fn base_tier_uses_explicit_band_ordering_and_class_adjustments() {
        assert!(base_tier(RelicId::Sozu, "Ironclad") > base_tier(RelicId::CursedKey, "Ironclad"));
        assert!(
            base_tier(RelicId::CursedKey, "Ironclad")
                > base_tier(RelicId::SneckoEye, "Ironclad")
        );
        assert!(
            base_tier(RelicId::CoffeeDripper, "Ironclad")
                > base_tier(RelicId::CoffeeDripper, "Silent")
        );
        assert_eq!(base_tier(RelicId::CallingBell, "Ironclad"), BossRelicTierBand::Weak.score());
    }

    #[test]
    fn boss_relic_scoring_uses_base_tier_and_need_modifiers() {
        let agent = Agent::new();
        let mut thin_run = RunState::new(11, 0, true, "Ironclad");
        thin_run.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Shockwave,
            11_001,
        ));

        let mut cluttered_run = thin_run.clone();
        cluttered_run.current_hp = 24;
        cluttered_run.max_hp = 80;
        cluttered_run.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            11_002,
        ));

        assert!(
            agent.boss_relic_score(&cluttered_run, RelicId::EmptyCage)
                > agent.boss_relic_score(&thin_run, RelicId::EmptyCage)
        );
        assert!(
            agent.boss_relic_score(&cluttered_run, RelicId::CoffeeDripper)
                < agent.boss_relic_score(&thin_run, RelicId::CoffeeDripper)
        );
    }
}
