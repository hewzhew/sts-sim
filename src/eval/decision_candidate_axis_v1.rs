fn normalize_axis_part_v1(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch == '_' || ch == '-' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

pub(crate) fn decision_candidate_axis_key_v1(
    boundary_kind: &str,
    boundary_scope: &str,
    family: &str,
) -> Option<String> {
    let boundary_kind = normalize_axis_part_v1(boundary_kind);
    let boundary_scope = normalize_axis_part_v1(boundary_scope);
    let family = normalize_axis_part_v1(family);
    if boundary_kind.is_empty() || boundary_scope.is_empty() || family.is_empty() {
        return None;
    }
    Some(format!("{boundary_kind}:{boundary_scope}:{family}"))
}

pub(crate) fn event_decision_candidate_axis_v1(
    boundary_scope: &str,
    effect_kind: &str,
) -> Option<String> {
    decision_candidate_axis_key_v1("event", boundary_scope, effect_kind)
}

pub(crate) fn shop_decision_candidate_axis_v1(family: &str) -> Option<String> {
    decision_candidate_axis_key_v1("shop", "shop", family)
}

pub(crate) fn shop_candidate_axis_family_v1(axis: &str) -> Option<&str> {
    axis.strip_prefix("shop:shop:")
        .filter(|family| !family.trim().is_empty())
}

pub(crate) fn compose_shop_decision_candidate_axes_v1<'a>(
    axes: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut has_purge = false;
    let mut has_buy_card = false;
    let mut has_buy_relic = false;
    let mut has_buy_potion = false;
    let mut has_leave = false;
    let mut has_stop = false;

    for axis in axes {
        let Some(family) = shop_candidate_axis_family_v1(axis) else {
            continue;
        };
        for part in shop_axis_family_parts_v1(family) {
            match part {
                "purge" => has_purge = true,
                "buy_card" => has_buy_card = true,
                "buy_relic" => has_buy_relic = true,
                "buy_potion" => has_buy_potion = true,
                "leave" => has_leave = true,
                "stop" => has_stop = true,
                _ => {}
            }
        }
    }

    let mut parts = Vec::new();
    if has_purge {
        parts.push("purge");
    }
    if has_buy_card {
        parts.push("buy_card");
    }
    if has_buy_relic {
        parts.push("buy_relic");
    }
    if has_buy_potion {
        parts.push("buy_potion");
    }
    if has_leave {
        parts.push("leave");
    }
    if has_stop {
        parts.push("stop");
    }

    let family = match parts.as_slice() {
        [] => return None,
        ["purge"] => "purge_only".to_string(),
        ["buy_card"] => "buy_card_only".to_string(),
        ["buy_relic"] => "buy_relic_only".to_string(),
        ["buy_potion"] => "buy_potion_only".to_string(),
        ["leave"] => "leave_or_save_gold".to_string(),
        ["stop"] => "stop".to_string(),
        _ => parts.join("_plus_"),
    };
    shop_decision_candidate_axis_v1(&family)
}

fn shop_axis_family_parts_v1(family: &str) -> Vec<&'static str> {
    match family {
        "purge_only" => vec!["purge"],
        "buy_card_only" => vec!["buy_card"],
        "buy_relic_only" => vec!["buy_relic"],
        "buy_potion_only" => vec!["buy_potion"],
        "leave_or_save_gold" => vec!["leave"],
        "stop" => vec!["stop"],
        _ => family
            .split("_plus_")
            .filter_map(|part| match part {
                "purge" => Some("purge"),
                "buy_card" => Some("buy_card"),
                "buy_relic" => Some("buy_relic"),
                "buy_potion" => Some("buy_potion"),
                "leave" => Some("leave"),
                "stop" => Some("stop"),
                _ => None,
            })
            .collect(),
    }
}
