use sts_simulator::ai::strategy::decision_pipeline::DecisionCandidateKind;
use sts_simulator::eval::run_control::{
    build_decision_surface, DecisionCandidateKey, RunControlSession,
};

use super::candidate_ir_adapter::shop_tiny_kind;

pub(super) fn apply_shop_boss_preview_bundle(
    session: &mut RunControlSession,
    items: &[DecisionCandidateKind],
) -> Result<(), String> {
    for item in items {
        apply_shop_candidate_kind(session, *item)?;
    }
    apply_shop_candidate_kind(session, DecisionCandidateKind::ShopLeave)
}

fn apply_shop_candidate_kind(
    session: &mut RunControlSession,
    kind: DecisionCandidateKind,
) -> Result<(), String> {
    let surface = build_decision_surface(session);
    let candidate = surface
        .view
        .candidates
        .iter()
        .find(|candidate| candidate_kind_matches(&candidate.key, kind))
        .ok_or_else(|| format!("shop boss preview bundle candidate not visible: {kind:?}"))?;
    let command = candidate.action.executable_command().ok_or_else(|| {
        format!(
            "shop boss preview bundle candidate is not executable: {}",
            candidate.label
        )
    })?;
    session.apply_command(command)?;
    Ok(())
}

fn candidate_kind_matches(key: &Option<DecisionCandidateKey>, kind: DecisionCandidateKind) -> bool {
    shop_tiny_kind(key) == kind
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::strategy::decision_pipeline::DecisionCandidateKind;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};
    use sts_simulator::state::core::EngineState;
    use sts_simulator::state::shop::{ShopCard, ShopState};

    use super::apply_shop_boss_preview_bundle;

    #[test]
    fn applies_bundle_by_typed_candidate_after_shop_slots_shift() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 300;
        let mut shop = ShopState::new();
        shop.cards.push(ShopCard {
            card_id: CardId::FiendFire,
            upgrades: 0,
            price: 152,
            can_buy: true,
            blocked_reason: None,
        });
        shop.cards.push(ShopCard {
            card_id: CardId::TrueGrit,
            upgrades: 0,
            price: 49,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        apply_shop_boss_preview_bundle(
            &mut session,
            &[
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                },
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::TrueGrit,
                    upgrades: 0,
                    price: 49,
                },
            ],
        )
        .expect("bundle should apply");

        assert!(session
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::FiendFire));
        assert!(session
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::TrueGrit));
        assert_eq!(session.run_state.gold, 99);
        assert!(!matches!(session.engine_state, EngineState::Shop(_)));
    }
}
