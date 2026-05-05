use crate::content::relics::RelicId;

use super::types::{BossRelicCandidate, RelicJudgement};

pub(super) fn candidate_from_judgement(
    index: usize,
    relic_id: RelicId,
    judgement: RelicJudgement,
) -> BossRelicCandidate {
    let rank_score = judgement.upside - judgement.downside;
    BossRelicCandidate {
        index,
        relic_id: format!("{relic_id:?}"),
        compatibility: judgement.compatibility,
        rank_score,
        upside: judgement.upside,
        downside: judgement.downside,
        volatility: judgement.volatility,
        confidence: judgement.confidence,
        primary_reason: judgement.primary_reason,
        positive_tags: judgement.positive_tags,
        negative_tags: judgement.negative_tags,
    }
}

pub(super) fn sort_candidates(candidates: &mut [BossRelicCandidate]) {
    candidates.sort_by(|lhs, rhs| {
        rhs.compatibility
            .bucket()
            .cmp(&lhs.compatibility.bucket())
            .then_with(|| rhs.rank_score.cmp(&lhs.rank_score))
            .then_with(|| rhs.confidence.cmp(&lhs.confidence))
            .then_with(|| lhs.volatility.cmp(&rhs.volatility))
            .then_with(|| lhs.index.cmp(&rhs.index))
    });
}
