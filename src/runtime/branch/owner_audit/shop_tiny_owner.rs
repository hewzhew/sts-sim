use sts_simulator::ai::strategy::decision_pipeline::{
    CandidateOrderKey, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, RewardAdmission,
};
use sts_simulator::ai::strategy::shop_purchase_bundle::ShopGoldOpportunity;
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::run_control::{DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;

use super::candidate_ir_adapter::shop_tiny_kind;
use super::expansion_policy::shop_tiny_choice_expansion;
use super::owner_candidate_eval::candidate_annotation;
use super::owner_commands::executable_choices;
use super::owner_model::{ChoiceAnnotation, OwnerChoice};
use super::shop_investment::shop_investment_for_surface;

pub(super) fn shop_tiny_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let base_context = shop_tiny_context(session);
    let deck = &session.run_state.master_deck;
    let shop_investment = shop_investment_for_surface(session, surface, deck, base_context);
    let context = shop_investment
        .map(|shop| base_context.with_shop_investment(shop))
        .unwrap_or(base_context);
    let mut choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_candidate_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion = shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
    }
    choices.sort_by_key(|(index, choice)| (shop_tiny_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    let context = DecisionPipelineContext::shop(deck_plan, session.run_state.gold);
    if active_maw_bank(session) {
        context.with_shop_gold_opportunity(ShopGoldOpportunity {
            current_gold: session.run_state.gold,
            active_maw_bank: true,
            future_rooms_before_next_shop: 5,
            survival_purchase_needed: deck_plan.survival_pressure(),
            boss_answer_needed: matches!(
                deck_plan.strategic_deficit.boss_scaling_plan,
                StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
            ),
        })
    } else {
        context
    }
}

fn active_maw_bank(session: &RunControlSession) -> bool {
    session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::MawBank && !relic.used_up)
}

fn shop_tiny_candidate_for_choice(
    context: DecisionPipelineContext,
    deck: &[CombatCard],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    let kind = shop_tiny_kind(&choice.key);
    candidate_annotation(context, kind, shop_card_admission(deck, kind))
}

fn shop_card_admission(
    deck: &[CombatCard],
    kind: DecisionCandidateKind,
) -> Option<RewardAdmission> {
    if let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = kind {
        Some(assess_reward_admission_from_master_deck(
            deck, card, upgrades,
        ))
    } else {
        None
    }
}

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => decision.evaluation.auto_order_key(false),
        _ => (u8::MAX, CandidateOrderKey::fallback()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::relics::RelicState;
    use sts_simulator::eval::run_control::{
        DecisionCandidateKey, RunControlCommand, RunControlConfig,
    };
    use sts_simulator::runtime::combat::CombatCard;

    fn maw_bank_session() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 224;
        session.run_state.relics = vec![RelicState::new(RelicId::MawBank)];
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
            CombatCard::new(CardId::Defend, 4),
            CombatCard::new(CardId::Bash, 5),
            CombatCard::new(CardId::Immolate, 6),
            CombatCard::new(CardId::IronWave, 7),
            CombatCard::new(CardId::Cleave, 8),
            CombatCard::new(CardId::ShrugItOff, 9),
            CombatCard::new(CardId::PommelStrike, 10),
            CombatCard::new(CardId::Bloodletting, 11),
        ];
        session
    }

    fn choice(key: DecisionCandidateKey) -> OwnerChoice {
        OwnerChoice {
            key: Some(key),
            action: RunControlCommand::Noop,
            label: String::new(),
            annotation: ChoiceAnnotation::None,
            expansion: super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
        }
    }

    #[test]
    fn shop_tiny_owner_context_prefers_leave_over_generic_maw_bank_breaking_relic() {
        let session = maw_bank_session();
        let context = shop_tiny_context(&session);
        let mut clockwork = choice(DecisionCandidateKey::ShopBuyRelic {
            shop_slot: 0,
            relic: RelicId::ClockworkSouvenir,
            price: 149,
        });
        let mut leave = choice(DecisionCandidateKey::ShopLeave);

        clockwork.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &clockwork);
        leave.annotation =
            shop_tiny_candidate_for_choice(context, &session.run_state.master_deck, &leave);
        let mut auto_purge_targets = Vec::new();
        clockwork.expansion =
            shop_tiny_choice_expansion(&clockwork.annotation, &mut auto_purge_targets);
        leave.expansion = shop_tiny_choice_expansion(&leave.annotation, &mut auto_purge_targets);

        assert_eq!(
            clockwork.inspect_only_reason(),
            Some("BreaksMawBankWithoutHardNeed")
        );
        assert!(
            shop_tiny_choice_rank(&leave) < shop_tiny_choice_rank(&clockwork),
            "ShopTiny should prefer LeaveWithGold over generic Maw Bank-breaking relic"
        );
    }
}
