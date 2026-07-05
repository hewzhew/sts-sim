use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_json::{json, Map, Value};
use sts_simulator::ai::strategy::deck_strategic_deficit::assess_deck_strategic_deficit;
use sts_simulator::ai::strategy::run_strategic_facts::RunStrategicFacts;
use sts_simulator::content::relics::RelicState;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::run::RunState;

pub(super) fn strategic_deficit_value(run: &RunState) -> Value {
    serde_json::to_value(assess_deck_strategic_deficit(
        &run.master_deck,
        RunStrategicFacts::from_run_state(run),
    ))
    .unwrap_or(Value::Null)
}

pub(super) fn deck_value(run: &RunState) -> Value {
    json!(run.master_deck.iter().map(card_value).collect::<Vec<_>>())
}

pub(super) fn relics_value(run: &RunState) -> Value {
    json!(run.relics.iter().map(relic_value).collect::<Vec<_>>())
}

pub(super) fn potions_value(run: &RunState) -> Value {
    json!(run
        .potions
        .iter()
        .map(|slot| slot
            .as_ref()
            .map(|potion| json!({"id": potion.id, "uuid": potion.uuid})))
        .collect::<Vec<_>>())
}

pub(super) fn deck_hash(deck: &[CombatCard]) -> String {
    let mut hasher = DefaultHasher::new();
    for card in deck {
        card.id.hash(&mut hasher);
        card.uuid.hash(&mut hasher);
        card.upgrades.hash(&mut hasher);
        card.misc_value.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn card_value(card: &CombatCard) -> Value {
    let mut value = Map::from_iter([
        ("id".to_string(), json!(card.id)),
        ("uuid".to_string(), json!(card.uuid)),
    ]);
    if card.upgrades != 0 {
        value.insert("upgrades".to_string(), json!(card.upgrades));
    }
    if card.misc_value != 0 {
        value.insert("misc".to_string(), json!(card.misc_value));
    }
    Value::Object(value)
}

fn relic_value(relic: &RelicState) -> Value {
    let mut value = Map::from_iter([("id".to_string(), json!(relic.id))]);
    if relic.counter != -1 {
        value.insert("counter".to_string(), json!(relic.counter));
    }
    if relic.used_up {
        value.insert("used_up".to_string(), json!(true));
    }
    if relic.amount != 0 {
        value.insert("amount".to_string(), json!(relic.amount));
    }
    Value::Object(value)
}
