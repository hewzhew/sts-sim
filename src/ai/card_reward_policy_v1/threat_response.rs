use crate::ai::noncombat_strategy_v1::{StrategyPlanEffectV1, StrategyThreatTagV1};
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

    if has_threat(context, StrategyThreatTagV1::StrengthDebuffValuable)
        && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.18;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_strength_down_response".to_string(),
            value,
        });
    }

    if has_threat(context, StrategyThreatTagV1::MultiHit) && candidate.facts.enemy_strength_down > 0
    {
        let value = candidate.facts.enemy_strength_down as f32 * 0.12;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_multi_hit_strength_down_response".to_string(),
            value,
        });
    }

    if has_threat(context, StrategyThreatTagV1::WeakValuable) && candidate.facts.weak > 0 {
        let value = candidate.facts.weak as f32 * 0.08;
        response.survival_delta += value;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_weak_response".to_string(),
            value,
        });
    }

    if has_threat(context, StrategyThreatTagV1::AoEValuable) && candidate.facts.is_aoe {
        response.progress_delta += 0.10;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_aoe_response".to_string(),
            value: 0.10,
        });
    }

    if has_threat(context, StrategyThreatTagV1::StatusFlood)
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

    if has_threat(context, StrategyThreatTagV1::LongFightScaling) && scaling_candidate(candidate) {
        response.progress_delta += 0.12;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_long_fight_scaling_response".to_string(),
            value: 0.12,
        });
    }

    if has_threat(context, StrategyThreatTagV1::SetupWindow) && setup_candidate(candidate) {
        response.progress_delta += 0.10;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_setup_window_response".to_string(),
            value: 0.10,
        });
    }

    if has_threat(context, StrategyThreatTagV1::SkillPunish)
        && candidate.facts.card_type == CardType::Skill
    {
        response.survival_delta -= 0.05;
        response.progress_delta -= 0.03;
        response.components.push(CardRewardValueComponentV1 {
            name: "elite_pool_skill_punish_penalty".to_string(),
            value: -0.05,
        });
    }

    if has_threat(context, StrategyThreatTagV1::ArtifactBlocksDebuff)
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

    if has_threat(context, StrategyThreatTagV1::PowerPunish)
        && candidate.facts.card_type == CardType::Power
    {
        response.survival_delta -= 0.08;
        response.progress_delta -= 0.08;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_power_punish_penalty".to_string(),
            value: -0.08,
        });
    }

    if has_threat(context, StrategyThreatTagV1::CardPlayLimit) {
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

    if has_threat(context, StrategyThreatTagV1::SplitThreshold)
        && candidate.facts.damage.total_damage >= 15
    {
        response.progress_delta += 0.12;
        response.components.push(CardRewardValueComponentV1 {
            name: "boss_threat_split_burst_response".to_string(),
            value: 0.12,
        });
    }

    response
}

fn has_threat(context: &CardRewardDecisionContextV1, tag: StrategyThreatTagV1) -> bool {
    context.strategy.threats.tags.contains(&tag)
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
