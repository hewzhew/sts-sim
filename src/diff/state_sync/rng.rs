use serde_json::Value;

use crate::runtime::rng::{RngPool, StsRng};

pub fn sync_rng(rng: &mut RngPool, snapshot: &Value) {
    let rng_state = match snapshot.get("rng_state") {
        Some(v) if v.is_object() && !v.as_object().unwrap().is_empty() => v,
        _ => return,
    };

    if let Some(ai) = rng_state.get("ai_rng") {
        sync_rng_channel(&mut rng.ai_rng, ai);
    }
    if let Some(shuffle) = rng_state.get("shuffle_rng") {
        sync_rng_channel(&mut rng.shuffle_rng, shuffle);
    }
    if let Some(card) = rng_state.get("card_rng") {
        sync_rng_channel(&mut rng.card_random_rng, card);
    }
    if let Some(misc) = rng_state.get("misc_rng") {
        sync_rng_channel(&mut rng.misc_rng, misc);
    }
    if let Some(monster_hp) = rng_state.get("monster_hp_rng") {
        sync_rng_channel(&mut rng.monster_hp_rng, monster_hp);
    }
    if let Some(potion) = rng_state.get("potion_rng") {
        sync_rng_channel(&mut rng.potion_rng, potion);
    }
}

fn sync_rng_channel(rng: &mut StsRng, json: &Value) {
    if let Some(s0) = json.get("seed0").and_then(|v| v.as_i64()) {
        rng.seed0 = s0 as u64;
    }
    if let Some(s1) = json.get("seed1").and_then(|v| v.as_i64()) {
        rng.seed1 = s1 as u64;
    }
    if let Some(c) = json.get("counter").and_then(|v| v.as_u64()) {
        rng.counter = c as u32;
    }
}
