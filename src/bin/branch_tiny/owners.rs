use sts_simulator::eval::run_control::{DecisionSurface, RunControlSession};

use super::boss_relic_owner::boss_relic_owner_choices;
use super::campfire_owner::campfire_owner_decision;
use super::card_reward_owner::card_reward_owner_choices;
use super::event_owner_bridge::event_owner_decision;
use super::neow_owner::neow_owner_decision;
use super::owner_model::OwnerDecision;
use super::reward_tiny_owner::reward_tiny_owner_decision;
use super::run_choice_owner::run_choice_owner_decision;
use super::shop_tiny_owner::shop_tiny_owner_choices;
use super::Owner;

pub(super) fn owner_decision(
    session: &RunControlSession,
    owner: Owner,
    surface: &DecisionSurface,
) -> OwnerDecision {
    match owner {
        Owner::NeowStart => neow_owner_decision(session, surface),
        Owner::CardReward => OwnerDecision::Candidates(card_reward_owner_choices(session, surface)),
        Owner::BossRelic => OwnerDecision::Candidates(boss_relic_owner_choices(session, surface)),
        Owner::ShopTiny => OwnerDecision::Candidates(shop_tiny_owner_choices(session, surface)),
        Owner::Event(_) => event_owner_decision(session, surface),
        Owner::RewardTiny => reward_tiny_owner_decision(surface),
        Owner::Campfire => campfire_owner_decision(session, surface),
        Owner::RunChoice => run_choice_owner_decision(session),
    }
}
