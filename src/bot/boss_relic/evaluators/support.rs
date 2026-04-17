use super::super::context::BossRelicContext;

pub(super) fn act4_setup_bonus(context: &BossRelicContext) -> i32 {
    if context.is_final_act_available && context.missing_keys > 0 {
        6
    } else {
        0
    }
}

pub(super) fn rest_soon(context: &BossRelicContext) -> bool {
    context.rest_distance.is_some_and(|distance| distance <= 2)
}

pub(super) fn smith_soon(context: &BossRelicContext) -> bool {
    context.rest_distance.is_some_and(|distance| distance <= 2)
}
