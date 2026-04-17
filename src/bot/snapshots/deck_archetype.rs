use crate::bot::deck_profile::DeckProfile;

pub fn archetype_tags(profile: &DeckProfile) -> Vec<String> {
    let mut tags = Vec::new();

    let strength_online = profile.strength_enablers >= 1 && profile.strength_payoffs >= 2;
    let exhaust_online = profile.exhaust_engines >= 2 && profile.exhaust_outlets >= 1;
    let block_online = profile.block_core >= 3 && profile.block_payoffs >= 1;
    let self_damage_online = profile.self_damage_sources >= 2;
    let draw_cycle_online = profile.draw_sources >= 3;
    let power_scaling_online = profile.power_scalers >= 2 || profile.power_count >= 4;
    let status_online = profile.status_generators >= 1 && profile.status_payoffs >= 1;
    let searing_blow_online = profile.searing_blow_count > 0;

    if strength_online {
        tags.push("strength".to_string());
    }
    if exhaust_online {
        tags.push("exhaust".to_string());
    }
    if block_online {
        tags.push("block".to_string());
    }
    if self_damage_online {
        tags.push("self_damage".to_string());
    }
    if draw_cycle_online {
        tags.push("draw_cycle".to_string());
    }
    if power_scaling_online {
        tags.push("power_scaling".to_string());
    }
    if status_online {
        tags.push("status".to_string());
    }
    if searing_blow_online {
        tags.push("searing_blow".to_string());
    }

    if tags.len() >= 2 {
        tags.push("hybrid".to_string());
    }

    if (profile.strength_enablers > 0 && profile.strength_payoffs == 0)
        || (profile.strength_payoffs >= 2 && profile.strength_enablers == 0)
        || (profile.exhaust_engines > 0 && profile.exhaust_outlets == 0)
        || (profile.exhaust_outlets >= 2 && profile.exhaust_engines == 0)
        || (profile.block_core >= 2 && profile.block_payoffs == 0)
        || (profile.status_generators > 0 && profile.status_payoffs == 0)
    {
        tags.push("shell_incomplete".to_string());
    }

    if tags.is_empty() {
        tags.push("goodstuff".to_string());
    }

    tags.sort();
    tags.dedup();
    tags
}

pub fn archetype_summary(profile: &DeckProfile) -> String {
    let tags = archetype_tags(profile).join(",");
    format!(
        "tags=[{}] str={}/{} exh={}/{}/{} block={}/{}",
        tags,
        profile.strength_enablers,
        profile.strength_payoffs,
        profile.exhaust_engines,
        profile.exhaust_outlets,
        profile.exhaust_fodder,
        profile.block_core,
        profile.block_payoffs
    )
}
