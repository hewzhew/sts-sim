mod energy;
mod special;
mod support;
mod transform;

use crate::content::relics::RelicId;

use super::context::BossRelicContext;
use super::types::RelicJudgement;

pub(super) fn evaluate_boss_relic(context: &BossRelicContext, relic_id: RelicId) -> RelicJudgement {
    match relic_id {
        RelicId::Astrolabe => transform::eval_astrolabe(context),
        RelicId::EmptyCage => transform::eval_empty_cage(context),
        RelicId::PandorasBox => transform::eval_pandoras_box(context),
        RelicId::BlackBlood => special::eval_black_blood(context),
        RelicId::CallingBell => special::eval_calling_bell(context),
        RelicId::RunicPyramid => special::eval_runic_pyramid(context),
        RelicId::SlaversCollar => special::eval_slavers_collar(context),
        RelicId::TinyHouse => special::eval_tiny_house(context),
        RelicId::CoffeeDripper => energy::eval_coffee_dripper(context),
        RelicId::FusionHammer => energy::eval_fusion_hammer(context),
        RelicId::BustedCrown => energy::eval_busted_crown(context),
        RelicId::Ectoplasm => energy::eval_ectoplasm(context),
        RelicId::CursedKey => energy::eval_cursed_key(context),
        RelicId::SneckoEye => energy::eval_snecko_eye(context),
        RelicId::Sozu => energy::eval_sozu(context),
        RelicId::VelvetChoker => energy::eval_velvet_choker(context),
        RelicId::PhilosopherStone => energy::eval_philosophers_stone(context),
        RelicId::MarkOfPain => energy::eval_mark_of_pain(context),
        _ => special::eval_unmodeled(),
    }
}
