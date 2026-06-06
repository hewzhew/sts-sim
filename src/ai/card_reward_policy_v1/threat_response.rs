use crate::ai::noncombat_strategy_v1::{
    StrategyPlanEffectV1, StrategyThreatSourceV1, StrategyThreatTagV1,
};
use crate::content::cards::CardType;

use super::types::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1, CardRewardPickDependencyV1,
    CardRewardValueComponentV1,
};

#[derive(Default)]
pub(crate) struct CardRewardThreatResponseDeltaV1 {
    pub(crate) survival_delta: f32,
    pub(crate) progress_delta: f32,
    pub(crate) components: Vec<CardRewardValueComponentV1>,
}

pub(crate) fn threat_response_delta(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> CardRewardThreatResponseDeltaV1 {
    let mut response = CardRewardThreatResponseDeltaV1::default();

    if has_threat(context, StrategyThreatTagV1::HighIncomingDamage) {
        response.components.push(CardRewardValueComponentV1 {
            name: "strategy_threat_high_incoming_damage".to_string(),
            value: 1.0,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::StrengthDebuffValuable)
        && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.18;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_strength_down_response".to_string(),
            value,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::MultiHit)
        && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.12;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_multi_hit_strength_down_response".to_string(),
            value,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::WeakValuable) && candidate.facts.weak > 0 {
        let value = candidate.facts.weak as f32 * 0.08;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_weak_response".to_string(),
            value,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::AoEValuable) && candidate.facts.is_aoe {
        response.progress_delta += 0.10;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_aoe_response".to_string(),
            value: 0.10,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::StatusFlood)
        && candidate
            .facts
            .pick_dependencies
            .contains(&CardRewardPickDependencyV1::StatusPackage)
    {
        response.survival_delta += 0.08;
        response.progress_delta += 0.12;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_status_flood_payoff_response".to_string(),
            value: 0.12,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::LongFightScaling)
        && scaling_candidate(candidate)
    {
        response.progress_delta += 0.12;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_long_fight_scaling_response".to_string(),
            value: 0.12,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::SetupWindow) && setup_candidate(candidate) {
        response.progress_delta += 0.10;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_setup_window_response".to_string(),
            value: 0.10,
        });
    }

    if has_elite_pool_threat(context, StrategyThreatTagV1::SkillPunish)
        && candidate.facts.card_type == CardType::Skill
    {
        response.survival_delta -= 0.05;
        response.progress_delta -= 0.03;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_pool_skill_punish_penalty".to_string(),
            value: -0.05,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::ArtifactBlocksDebuff)
        && (candidate.facts.weak > 0
            || candidate.facts.vulnerable > 0
            || candidate.facts.enemy_strength_down > 0)
    {
        response.survival_delta -= 0.12;
        response.progress_delta -= 0.04;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_artifact_debuff_penalty".to_string(),
            value: -0.12,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::PowerPunish)
        && candidate.facts.card_type == CardType::Power
    {
        response.survival_delta -= 0.08;
        response.progress_delta -= 0.08;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_power_punish_penalty".to_string(),
            value: -0.08,
        });
    }

    if has_boss_threat(context, StrategyThreatTagV1::CardPlayLimit) {
        if low_density_card_play(candidate) {
            response.survival_delta -= 0.06;
            response.progress_delta -= 0.06;
            response.components.push(CardRewardValueComponentV1 {
                name: "boss_threat_card_play_limit_low_density_penalty".to_string(),
                value: -0.06,
            });
        }
        if dense_card_play(candidate) {
            response.progress_delta += 0.08;
            response.components.push(CardRewardValueComponentV1 {
                name: "boss_threat_card_play_limit_dense_card_response".to_string(),
                value: 0.08,
            });
        }
    }

    if has_boss_threat(context, StrategyThreatTagV1::SplitThreshold)
        && candidate.facts.damage.total_damage >= 15
    {
        response.progress_delta += 0.12;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_split_burst_response".to_string(),
            value: 0.12,
        });
    }

    if has_elite_pool_threat(context, StrategyThreatTagV1::StrengthDebuffValuable)
        && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.08;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_pool_strength_down_response".to_string(),
            value,
        });
    }

    if has_elite_pool_threat(context, StrategyThreatTagV1::MultiHit)
        && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.06;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_pool_multi_hit_strength_down_response".to_string(),
            value,
        });
    }

    if has_elite_encounter_threat(context, "GremlinNob", StrategyThreatTagV1::SkillPunish)
        && candidate.facts.card_type == CardType::Skill
    {
        response.survival_delta -= 0.04;
        response.progress_delta -= 0.03;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_encounter_nob_skill_punish_penalty".to_string(),
            value: -0.04,
        });
    }

    if has_elite_encounter_threat(context, "ThreeSentries", StrategyThreatTagV1::StatusFlood)
        && candidate
            .facts
            .pick_dependencies
            .contains(&CardRewardPickDependencyV1::StatusPackage)
    {
        response.survival_delta += 0.06;
        response.progress_delta += 0.08;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_encounter_sentries_status_payoff_response".to_string(),
            value: 0.08,
        });
    }

    if has_elite_encounter_threat(
        context,
        "BookOfStabbing",
        StrategyThreatTagV1::StrengthDebuffValuable,
    ) && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.10;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_encounter_book_strength_down_response".to_string(),
            value,
        });
    }

    response
}

pub(crate) fn candidate_response_threat_tags_v1(
    candidate: &CardRewardCandidateEvidenceV1,
) -> Vec<StrategyThreatTagV1> {
    let mut tags = Vec::new();

    if candidate.facts.block > 0
        || candidate.facts.weak > 0
        || candidate.facts.enemy_strength_down > 0
        || candidate.plan_delta.effects.iter().any(|effect| {
            matches!(
                effect,
                StrategyPlanEffectV1::DamageMitigation
                    | StrategyPlanEffectV1::BlockRetention
                    | StrategyPlanEffectV1::BlockMultiplier
            )
        })
    {
        push_unique(&mut tags, StrategyThreatTagV1::HighIncomingDamage);
    }
    if candidate.facts.enemy_strength_down > 0 {
        push_unique(&mut tags, StrategyThreatTagV1::StrengthDebuffValuable);
        push_unique(&mut tags, StrategyThreatTagV1::MultiHit);
    }
    if candidate.facts.weak > 0 {
        push_unique(&mut tags, StrategyThreatTagV1::WeakValuable);
    }
    if candidate.facts.is_aoe {
        push_unique(&mut tags, StrategyThreatTagV1::AoEValuable);
    }
    if candidate
        .facts
        .pick_dependencies
        .contains(&CardRewardPickDependencyV1::StatusPackage)
        || candidate
            .plan_delta
            .effects
            .contains(&StrategyPlanEffectV1::StatusPayoff)
    {
        push_unique(&mut tags, StrategyThreatTagV1::StatusFlood);
    }
    if scaling_candidate(candidate) {
        push_unique(&mut tags, StrategyThreatTagV1::LongFightScaling);
    }
    if setup_candidate(candidate) {
        push_unique(&mut tags, StrategyThreatTagV1::SetupWindow);
    }
    if candidate.facts.card_type == CardType::Skill {
        push_unique(&mut tags, StrategyThreatTagV1::SkillPunish);
    }
    if candidate.facts.weak > 0
        || candidate.facts.vulnerable > 0
        || candidate.facts.enemy_strength_down > 0
    {
        push_unique(&mut tags, StrategyThreatTagV1::ArtifactBlocksDebuff);
    }
    if candidate.facts.card_type == CardType::Power {
        push_unique(&mut tags, StrategyThreatTagV1::PowerPunish);
    }
    if low_density_card_play(candidate) || dense_card_play(candidate) {
        push_unique(&mut tags, StrategyThreatTagV1::CardPlayLimit);
    }
    if candidate.facts.damage.total_damage >= 15 {
        push_unique(&mut tags, StrategyThreatTagV1::SplitThreshold);
        push_unique(&mut tags, StrategyThreatTagV1::ModeShiftThreshold);
    }

    tags
}

fn push_unique(tags: &mut Vec<StrategyThreatTagV1>, tag: StrategyThreatTagV1) {
    if !tags.contains(&tag) {
        tags.push(tag);
    }
}

fn has_threat(context: &CardRewardDecisionContextV1, tag: StrategyThreatTagV1) -> bool {
    context.strategy.threats.tags.contains(&tag)
}

fn has_boss_threat(context: &CardRewardDecisionContextV1, tag: StrategyThreatTagV1) -> bool {
    context
        .strategy
        .threats
        .sources
        .iter()
        .any(|source| source.source == StrategyThreatSourceV1::ActBoss && source.tag == tag)
}

fn has_elite_pool_threat(context: &CardRewardDecisionContextV1, tag: StrategyThreatTagV1) -> bool {
    route_allows_elite_pool_response(context)
        && context.strategy.threats.sources.iter().any(|source| {
            source.source == StrategyThreatSourceV1::ActElitePool && source.tag == tag
        })
}

fn has_elite_encounter_threat(
    context: &CardRewardDecisionContextV1,
    subject: &str,
    tag: StrategyThreatTagV1,
) -> bool {
    route_allows_elite_pool_response(context)
        && context.strategy.threats.sources.iter().any(|source| {
            source.source == StrategyThreatSourceV1::ActEliteEncounter
                && source.subject == subject
                && source.tag == tag
        })
}

fn route_allows_elite_pool_response(context: &CardRewardDecisionContextV1) -> bool {
    context
        .route
        .as_ref()
        .and_then(|route| route.selected_route.as_ref())
        .map(|route| route.max_elites > 0)
        .unwrap_or(true)
}

fn low_density_card_play(candidate: &CardRewardCandidateEvidenceV1) -> bool {
    candidate.facts.cost <= 0 || candidate.facts.draw_cards > 0
}

fn dense_card_play(candidate: &CardRewardCandidateEvidenceV1) -> bool {
    candidate.facts.cost >= 2
        && (candidate.facts.damage.total_damage >= 15
            || candidate.facts.block >= 12
            || candidate.facts.enemy_strength_down > 0)
}

fn scaling_candidate(candidate: &CardRewardCandidateEvidenceV1) -> bool {
    candidate.facts.strength_gain > 0
        || candidate
            .plan_delta
            .effects
            .iter()
            .any(|effect| matches!(effect, StrategyPlanEffectV1::StrengthPayoff))
}

fn setup_candidate(candidate: &CardRewardCandidateEvidenceV1) -> bool {
    candidate.facts.card_type == CardType::Power
        || candidate.plan_delta.effects.iter().any(|effect| {
            matches!(
                effect,
                StrategyPlanEffectV1::BlockRetention
                    | StrategyPlanEffectV1::ExhaustPayoff
                    | StrategyPlanEffectV1::StatusPayoff
            )
        })
}
