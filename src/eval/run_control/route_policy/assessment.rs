use super::{format_first_floor, MapRouteTarget, RouteAssessment, RouteSummary, RouteTier};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct RoutePreference {
    tier: RouteTier,
    elite_shape: EliteShape,
    recovery: RecoveryAccess,
    early_fights: EarlyFightAccess,
    shop_access: Access,
    fire_earliness: i32,
    path_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum EliteShape {
    Unknown,
    Forced,
    None,
    Optional,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RecoveryAccess {
    None,
    Possible,
    Guaranteed,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum EarlyFightAccess {
    None,
    One,
    TwoPlus,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Access {
    None,
    Some,
}

pub(super) fn assess_route(target: &MapRouteTarget, summary: &RouteSummary) -> RouteAssessment {
    let mut reasons = Vec::new();
    let mut cautions = Vec::new();
    if target.has_emerald_key {
        cautions.push("emerald elite on this immediate node".to_string());
    }
    if summary.path_count == 0 {
        cautions.push("no visible continuation under current map graph".to_string());
    }

    let elite_shape = elite_shape(summary);
    match elite_shape {
        EliteShape::Optional => {
            reasons.push("elite fights are optional on visible paths".to_string())
        }
        EliteShape::None => reasons.push("no visible elite pressure".to_string()),
        EliteShape::Forced => {
            cautions.push("at least one elite is forced on every visible path".to_string())
        }
        EliteShape::Unknown => cautions.push("elite access is unknown".to_string()),
    }

    let recovery = recovery_access(summary);
    match recovery {
        RecoveryAccess::Guaranteed => {
            reasons.push("rest site is guaranteed somewhere on the route".to_string())
        }
        RecoveryAccess::Possible => {
            reasons.push("rest site exists on some visible continuations".to_string())
        }
        RecoveryAccess::None => cautions.push("no visible rest site before boss".to_string()),
    }

    if summary.max_shops > 0 {
        reasons.push(format!(
            "shop access exists, first shop {}",
            format_first_floor(summary.first_shop_floor)
        ));
    }

    let early_fights = early_fight_access(summary);
    match early_fights {
        EarlyFightAccess::TwoPlus => reasons.push(
            "has multiple early monster/elite rooms for card rewards before the route commits"
                .to_string(),
        ),
        EarlyFightAccess::One => {
            reasons.push("has at least one early monster/elite room".to_string())
        }
        EarlyFightAccess::None => cautions.push("little early combat reward access".to_string()),
    }

    if summary.path_count > 1 {
        reasons.push(format!(
            "keeps {} visible continuations open",
            summary.path_count
        ));
    }

    let tier = route_tier(summary, elite_shape, recovery);
    let preference = RoutePreference {
        tier,
        elite_shape,
        recovery,
        early_fights,
        shop_access: if summary.max_shops > 0 {
            Access::Some
        } else {
            Access::None
        },
        fire_earliness: summary
            .first_fire_floor
            .map(|floor| 20 - floor)
            .unwrap_or_default(),
        path_count: summary.path_count,
    };
    RouteAssessment {
        tier,
        reasons,
        cautions,
        preference,
    }
}

pub(super) fn tier_label(tier: RouteTier) -> &'static str {
    match tier {
        RouteTier::Avoid => "avoid",
        RouteTier::Conservative => "conservative",
        RouteTier::Flexible => "flexible",
        RouteTier::Preferred => "preferred",
    }
}

fn route_tier(
    summary: &RouteSummary,
    elite_shape: EliteShape,
    recovery: RecoveryAccess,
) -> RouteTier {
    if summary.path_count == 0
        || elite_shape == EliteShape::Forced && recovery == RecoveryAccess::None
    {
        return RouteTier::Avoid;
    }
    if elite_shape == EliteShape::Optional && recovery != RecoveryAccess::None {
        return RouteTier::Preferred;
    }
    if recovery != RecoveryAccess::None || summary.path_count > 1 {
        return RouteTier::Flexible;
    }
    RouteTier::Conservative
}

fn elite_shape(summary: &RouteSummary) -> EliteShape {
    if summary.path_count == 0 {
        EliteShape::Unknown
    } else if summary.min_elites > 0 {
        EliteShape::Forced
    } else if summary.max_elites > 0 {
        EliteShape::Optional
    } else {
        EliteShape::None
    }
}

fn recovery_access(summary: &RouteSummary) -> RecoveryAccess {
    if summary.min_fires > 0 {
        RecoveryAccess::Guaranteed
    } else if summary.max_fires > 0 {
        RecoveryAccess::Possible
    } else {
        RecoveryAccess::None
    }
}

fn early_fight_access(summary: &RouteSummary) -> EarlyFightAccess {
    if summary.max_early_pressure >= 2 {
        EarlyFightAccess::TwoPlus
    } else if summary.max_early_pressure == 1 {
        EarlyFightAccess::One
    } else {
        EarlyFightAccess::None
    }
}
