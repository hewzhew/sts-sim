use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::reward_admission::RewardAdmission;
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BossSurvivalEvidence {
    pub label: &'static str,
    pub score_delta: i32,
    pub relevant_to_boss_survival_plan: bool,
}

impl BossSurvivalEvidence {
    const fn relevant(label: &'static str, score_delta: i32) -> Self {
        Self {
            label,
            score_delta,
            relevant_to_boss_survival_plan: true,
        }
    }

    const fn score_only(label: &'static str, score_delta: i32) -> Self {
        Self {
            label,
            score_delta,
            relevant_to_boss_survival_plan: false,
        }
    }

    const fn none() -> Self {
        Self {
            label: "",
            score_delta: 0,
            relevant_to_boss_survival_plan: false,
        }
    }
}

pub fn assess_boss_survival_evidence(
    deck: DeckPlanSnapshot,
    card: Option<(CardId, u8)>,
    _admission: &RewardAdmission,
) -> BossSurvivalEvidence {
    let Some((card, upgrades)) = card else {
        return BossSurvivalEvidence::none();
    };
    match deck.boss_key {
        Some(EncounterId::AwakenedOne) => awakened_one_survival_evidence(deck, card, upgrades),
        _ => BossSurvivalEvidence::none(),
    }
}

fn awakened_one_survival_evidence(
    deck: DeckPlanSnapshot,
    card: CardId,
    upgrades: u8,
) -> BossSurvivalEvidence {
    if !awakened_one_survival_pressure_open(deck) {
        return BossSurvivalEvidence::none();
    }
    match card {
        CardId::Disarm => {
            BossSurvivalEvidence::relevant("awakened-one-strength-down-survival", 100)
        }
        CardId::Shockwave => {
            BossSurvivalEvidence::relevant("awakened-one-weak-strength-down-survival", 95)
        }
        CardId::Impervious | CardId::PowerThrough => {
            BossSurvivalEvidence::relevant("awakened-one-dark-echo-block-plan", 85)
        }
        CardId::FlameBarrier => {
            BossSurvivalEvidence::relevant("awakened-one-repeatable-block-plan", 70)
        }
        CardId::SecondWind
            if deck.roles.exhaust_stream_units > 0 || deck.roles.corruption_units > 0 =>
        {
            BossSurvivalEvidence::relevant("awakened-one-exhaust-block-plan", 65)
        }
        CardId::FeelNoPain
            if deck.roles.exhaust_stream_units > 0 || deck.roles.corruption_units > 0 =>
        {
            BossSurvivalEvidence::relevant("awakened-one-exhaust-block-engine", 60)
        }
        CardId::ShrugItOff if upgrades > 0 => {
            BossSurvivalEvidence::score_only("awakened-one-generic-block-access", 20)
        }
        _ => BossSurvivalEvidence::none(),
    }
}

fn awakened_one_survival_pressure_open(deck: DeckPlanSnapshot) -> bool {
    deck.context.act >= 3
        && deck.roles.mitigation_units == 0
        && (deck.roles.strength_source_units > 0 || deck.roles.aoe_units > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::analysis::card_semantics::Mechanic;
    use crate::ai::strategy::deck_admission::DeckAdmissionContext;
    use crate::ai::strategy::reward_admission::RewardAdmissionReason;
    use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
    use crate::runtime::combat::CombatCard;

    fn deck_plan(cards: &[CardId], boss: Option<EncounterId>) -> DeckPlanSnapshot {
        let deck = cards
            .iter()
            .enumerate()
            .map(|(index, card)| CombatCard::new(*card, index as u32 + 1))
            .collect::<Vec<_>>();
        DeckPlanSnapshot::from_deck(
            &deck,
            DeckAdmissionContext {
                act: 3,
                current_hp: 70,
                max_hp: 80,
            },
            RunStrategicFacts {
                entering_act: 3,
                starter_basic_count: 2,
                curse_count: 0,
                has_energy_relic: true,
            },
        )
        .with_boss_key(boss)
    }

    #[test]
    fn awakened_one_mitigation_card_repairs_open_survival_plan() {
        let plan = deck_plan(
            &[
                CardId::DemonForm,
                CardId::Rupture,
                CardId::Whirlwind,
                CardId::Cleave,
                CardId::ShrugItOff,
            ],
            Some(EncounterId::AwakenedOne),
        );
        let admission = RewardAdmission {
            card: Some(CardId::Disarm),
            class: crate::ai::strategy::reward_admission::RewardAdmissionClass::ImmediateWork,
            reasons: vec![RewardAdmissionReason::Provides(Mechanic::EnemyStrengthDown)],
        };

        let evidence = assess_boss_survival_evidence(plan, Some((CardId::Disarm, 0)), &admission);

        assert_eq!(evidence.label, "awakened-one-strength-down-survival");
        assert!(evidence.relevant_to_boss_survival_plan);
    }

    #[test]
    fn awakened_one_generic_block_draw_is_not_a_survival_plan_repair() {
        let plan = deck_plan(
            &[
                CardId::DemonForm,
                CardId::Rupture,
                CardId::Whirlwind,
                CardId::Cleave,
                CardId::ShrugItOff,
            ],
            Some(EncounterId::AwakenedOne),
        );
        let admission = RewardAdmission {
            card: Some(CardId::ShrugItOff),
            class: crate::ai::strategy::reward_admission::RewardAdmissionClass::ImmediateWork,
            reasons: vec![
                RewardAdmissionReason::Provides(Mechanic::Block),
                RewardAdmissionReason::Provides(Mechanic::CardDraw),
            ],
        };

        let evidence =
            assess_boss_survival_evidence(plan, Some((CardId::ShrugItOff, 1)), &admission);

        assert_eq!(evidence.label, "awakened-one-generic-block-access");
        assert!(!evidence.relevant_to_boss_survival_plan);
    }
}
