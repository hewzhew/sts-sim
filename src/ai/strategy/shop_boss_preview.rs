use crate::ai::strategy::decision_pipeline::{CleanupTarget, DecisionCandidateKind};
use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopBossPreviewClass {
    BaselineLeave,
    DeterministicBossRepair,
    DeterministicSupport,
    DeterministicCleanup,
    RandomOrDeferred,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShopBossPreviewCandidate {
    pub kind: DecisionCandidateKind,
    pub class: ShopBossPreviewClass,
    pub include_in_v0: bool,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopBossPreviewBundleReason {
    Baseline,
    Single,
    MultiItem,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopBossPreviewBundle {
    pub items: Vec<DecisionCandidateKind>,
    pub total_cost: i32,
    pub gold_after: i32,
    pub reason: ShopBossPreviewBundleReason,
}

pub fn shop_boss_preview_candidates(
    candidates: impl IntoIterator<Item = DecisionCandidateKind>,
) -> Vec<ShopBossPreviewCandidate> {
    let mut preview_candidates = Vec::new();
    for candidate in candidates
        .into_iter()
        .map(classify_shop_boss_preview_candidate)
        .filter(|candidate| candidate.include_in_v0)
    {
        if preview_candidates
            .iter()
            .any(|existing: &ShopBossPreviewCandidate| existing.kind == candidate.kind)
        {
            continue;
        }
        preview_candidates.push(candidate);
    }
    preview_candidates
}

pub fn shop_boss_preview_bundles(
    candidates: impl IntoIterator<Item = DecisionCandidateKind>,
    current_gold: i32,
    max_bundles: usize,
) -> Vec<ShopBossPreviewBundle> {
    let preview = shop_boss_preview_candidates(candidates);
    if preview.is_empty() {
        return Vec::new();
    }
    let mut bundles = vec![ShopBossPreviewBundle {
        items: Vec::new(),
        total_cost: 0,
        gold_after: current_gold,
        reason: ShopBossPreviewBundleReason::Baseline,
    }];
    let items = preview
        .into_iter()
        .filter(|candidate| candidate.class != ShopBossPreviewClass::BaselineLeave)
        .collect::<Vec<_>>();

    for size in 1..=3 {
        let mut current = Vec::new();
        generate_bundle_combinations(&items, size, 0, &mut current, current_gold, &mut bundles);
    }
    bundles[1..].sort_by(|a, b| {
        bundle_score(b)
            .cmp(&bundle_score(a))
            .then_with(|| a.total_cost.cmp(&b.total_cost))
    });
    bundles.truncate(max_bundles.max(1));
    bundles
}

pub fn classify_shop_boss_preview_candidate(
    kind: DecisionCandidateKind,
) -> ShopBossPreviewCandidate {
    let (class, include_in_v0, reason) = match kind {
        DecisionCandidateKind::ShopLeave => {
            (ShopBossPreviewClass::BaselineLeave, true, "BaselineLeave")
        }
        DecisionCandidateKind::ShopPurge { target } => match target {
            CleanupTarget::Curse | CleanupTarget::StarterStrike | CleanupTarget::StarterDefend => (
                ShopBossPreviewClass::DeterministicCleanup,
                true,
                "DeterministicCleanup",
            ),
            _ => (
                ShopBossPreviewClass::Unsupported,
                false,
                "UnsupportedCleanupTarget",
            ),
        },
        DecisionCandidateKind::ShopBuyCard { card, .. } => match card {
            CardId::DemonForm => (
                ShopBossPreviewClass::DeterministicBossRepair,
                true,
                "DeterministicBossScalingRepair",
            ),
            CardId::FiendFire | CardId::Bludgeon | CardId::Immolate | CardId::Reaper => (
                ShopBossPreviewClass::DeterministicBossRepair,
                true,
                "DeterministicBossDamageRepair",
            ),
            CardId::TrueGrit
            | CardId::ShrugItOff
            | CardId::Disarm
            | CardId::Metallicize
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::SecondWind => (
                ShopBossPreviewClass::DeterministicSupport,
                true,
                "DeterministicSupportCard",
            ),
            _ => (ShopBossPreviewClass::Unsupported, false, "NotPreviewV0Card"),
        },
        DecisionCandidateKind::ShopBuyRelic { relic, .. } => match relic {
            RelicId::Vajra | RelicId::Lantern | RelicId::BagOfPreparation | RelicId::Anchor => (
                ShopBossPreviewClass::DeterministicSupport,
                true,
                "DeterministicSupportRelic",
            ),
            RelicId::Orrery => (
                ShopBossPreviewClass::RandomOrDeferred,
                false,
                "RandomRewardRelicExcluded",
            ),
            _ => (
                ShopBossPreviewClass::Unsupported,
                false,
                "NotPreviewV0Relic",
            ),
        },
        DecisionCandidateKind::ShopBuyPotion { potion, .. } => match potion {
            PotionId::FirePotion | PotionId::ExplosivePotion | PotionId::FearPotion => (
                ShopBossPreviewClass::DeterministicBossRepair,
                true,
                "DeterministicBossPotion",
            ),
            PotionId::BlockPotion | PotionId::DexterityPotion | PotionId::SpeedPotion => (
                ShopBossPreviewClass::DeterministicSupport,
                true,
                "DeterministicSupportPotion",
            ),
            PotionId::PowerPotion => (
                ShopBossPreviewClass::RandomOrDeferred,
                true,
                "HighCeilingPowerDiscoveryPotion",
            ),
            PotionId::AttackPotion | PotionId::EntropicBrew | PotionId::GamblersBrew => (
                ShopBossPreviewClass::RandomOrDeferred,
                false,
                "RandomPotionExcluded",
            ),
            _ => (
                ShopBossPreviewClass::Unsupported,
                false,
                "NotPreviewV0Potion",
            ),
        },
        _ => (
            ShopBossPreviewClass::Unsupported,
            false,
            "UnsupportedCandidateKind",
        ),
    };

    ShopBossPreviewCandidate {
        kind,
        class,
        include_in_v0,
        reason,
    }
}

fn generate_bundle_combinations(
    items: &[ShopBossPreviewCandidate],
    target_size: usize,
    start: usize,
    current: &mut Vec<ShopBossPreviewCandidate>,
    current_gold: i32,
    bundles: &mut Vec<ShopBossPreviewBundle>,
) {
    if current.len() == target_size {
        if !is_valid_bundle(current) {
            return;
        }
        let total_cost = current.iter().map(|item| candidate_cost(item.kind)).sum();
        if total_cost > current_gold {
            return;
        }
        let kinds = current.iter().map(|item| item.kind).collect::<Vec<_>>();
        if bundles.iter().any(|bundle| bundle.items == kinds) {
            return;
        }
        bundles.push(ShopBossPreviewBundle {
            items: kinds,
            total_cost,
            gold_after: current_gold - total_cost,
            reason: if target_size == 1 {
                ShopBossPreviewBundleReason::Single
            } else {
                ShopBossPreviewBundleReason::MultiItem
            },
        });
        return;
    }
    for index in start..items.len() {
        current.push(items[index]);
        generate_bundle_combinations(
            items,
            target_size,
            index + 1,
            current,
            current_gold,
            bundles,
        );
        current.pop();
    }
}

fn is_valid_bundle(items: &[ShopBossPreviewCandidate]) -> bool {
    if items.is_empty() {
        return true;
    }
    if count_by_slot(items, ShopBossPreviewSlot::Cleanup) > 1 {
        return false;
    }
    if count_by_slot(items, ShopBossPreviewSlot::Potion) > 1 {
        return false;
    }
    if count_by_slot(items, ShopBossPreviewSlot::BossDamageCard) > 1 {
        return false;
    }
    if items.len() > 1
        && items
            .iter()
            .all(|item| preview_slot(item.kind) == ShopBossPreviewSlot::Support)
    {
        return false;
    }
    true
}

fn count_by_slot(items: &[ShopBossPreviewCandidate], slot: ShopBossPreviewSlot) -> usize {
    items
        .iter()
        .filter(|item| preview_slot(item.kind) == slot)
        .count()
}

fn bundle_score(bundle: &ShopBossPreviewBundle) -> i32 {
    bundle
        .items
        .iter()
        .map(|item| match preview_slot(*item) {
            ShopBossPreviewSlot::BossDamageCard => 120,
            ShopBossPreviewSlot::Potion => 80,
            ShopBossPreviewSlot::Cleanup => 70,
            ShopBossPreviewSlot::Support => 45,
            ShopBossPreviewSlot::Leave | ShopBossPreviewSlot::Unsupported => 0,
        })
        .sum::<i32>()
        + (bundle.items.len() as i32 * 5)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShopBossPreviewSlot {
    Leave,
    BossDamageCard,
    Potion,
    Cleanup,
    Support,
    Unsupported,
}

fn preview_slot(kind: DecisionCandidateKind) -> ShopBossPreviewSlot {
    match kind {
        DecisionCandidateKind::ShopLeave => ShopBossPreviewSlot::Leave,
        DecisionCandidateKind::ShopPurge { .. } => ShopBossPreviewSlot::Cleanup,
        DecisionCandidateKind::ShopBuyPotion { .. } => ShopBossPreviewSlot::Potion,
        DecisionCandidateKind::ShopBuyCard { card, .. }
            if matches!(
                card,
                CardId::DemonForm
                    | CardId::FiendFire
                    | CardId::Bludgeon
                    | CardId::Immolate
                    | CardId::Reaper
            ) =>
        {
            ShopBossPreviewSlot::BossDamageCard
        }
        DecisionCandidateKind::ShopBuyCard { .. } | DecisionCandidateKind::ShopBuyRelic { .. } => {
            ShopBossPreviewSlot::Support
        }
        _ => ShopBossPreviewSlot::Unsupported,
    }
}

fn candidate_cost(kind: DecisionCandidateKind) -> i32 {
    match kind {
        DecisionCandidateKind::ShopBuyCard { price, .. }
        | DecisionCandidateKind::ShopBuyRelic { price, .. }
        | DecisionCandidateKind::ShopBuyPotion { price, .. } => price,
        DecisionCandidateKind::ShopPurge { .. } => 75,
        DecisionCandidateKind::ShopLeave
        | DecisionCandidateKind::ShopOpenRewards
        | DecisionCandidateKind::CardRewardPick { .. }
        | DecisionCandidateKind::CardRewardSkip
        | DecisionCandidateKind::BossRelicPick { .. }
        | DecisionCandidateKind::BossRelicSkip
        | DecisionCandidateKind::Unsupported => 0,
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::strategy::decision_pipeline::{CleanupTarget, DecisionCandidateKind};
    use crate::content::cards::CardId;
    use crate::content::potions::PotionId;
    use crate::content::relics::RelicId;

    use super::{
        classify_shop_boss_preview_candidate, shop_boss_preview_bundles,
        shop_boss_preview_candidates, ShopBossPreviewBundleReason, ShopBossPreviewClass,
    };

    #[test]
    fn keeps_deterministic_boss_repair_candidates() {
        let candidates = shop_boss_preview_candidates([
            DecisionCandidateKind::ShopLeave,
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::FiendFire,
                upgrades: 0,
                price: 170,
            },
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::Bludgeon,
                upgrades: 0,
                price: 155,
            },
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::FirePotion,
                price: 50,
            },
            DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterStrike,
            },
        ]);

        assert_eq!(candidates.len(), 5);
        assert!(candidates
            .iter()
            .any(|candidate| matches!(candidate.class, ShopBossPreviewClass::BaselineLeave)));
        assert!(candidates.iter().any(|candidate| matches!(
            candidate.kind,
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::FiendFire,
                ..
            }
        )));
        assert!(candidates.iter().any(|candidate| matches!(
            candidate.kind,
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::FirePotion,
                ..
            }
        )));
        assert!(candidates.iter().any(|candidate| matches!(
            candidate.kind,
            DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterStrike
            }
        )));
    }

    #[test]
    fn excludes_random_or_deferred_shop_items_from_v0_preview() {
        let candidates = shop_boss_preview_candidates([
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::AttackPotion,
                price: 50,
            },
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::EntropicBrew,
                price: 70,
            },
            DecisionCandidateKind::ShopBuyRelic {
                relic: RelicId::Orrery,
                price: 164,
            },
            DecisionCandidateKind::ShopBuyRelic {
                relic: RelicId::MawBank,
                price: 150,
            },
        ]);

        assert!(candidates.is_empty());
    }

    #[test]
    fn classifies_support_relics_but_does_not_call_them_hard_answers() {
        let vajra = classify_shop_boss_preview_candidate(DecisionCandidateKind::ShopBuyRelic {
            relic: RelicId::Vajra,
            price: 143,
        });
        let lantern = classify_shop_boss_preview_candidate(DecisionCandidateKind::ShopBuyRelic {
            relic: RelicId::Lantern,
            price: 150,
        });

        assert_eq!(vajra.class, ShopBossPreviewClass::DeterministicSupport);
        assert_eq!(lantern.class, ShopBossPreviewClass::DeterministicSupport);
        assert!(vajra.include_in_v0);
        assert!(lantern.include_in_v0);
    }

    #[test]
    fn deduplicates_repeated_cleanup_targets_for_preview() {
        let candidates = shop_boss_preview_candidates([
            DecisionCandidateKind::ShopLeave,
            DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterStrike,
            },
            DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterStrike,
            },
            DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterDefend,
            },
            DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterDefend,
            },
        ]);

        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| matches!(
                    candidate.kind,
                    DecisionCandidateKind::ShopPurge { .. }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn generates_affordable_multi_item_bundles_for_boss_preview() {
        let bundles = shop_boss_preview_bundles(
            [
                DecisionCandidateKind::ShopLeave,
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                },
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::Bludgeon,
                    upgrades: 0,
                    price: 162,
                },
                DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::FirePotion,
                    price: 51,
                },
                DecisionCandidateKind::ShopPurge {
                    target: CleanupTarget::StarterStrike,
                },
                DecisionCandidateKind::ShopBuyRelic {
                    relic: RelicId::Vajra,
                    price: 153,
                },
            ],
            338,
            12,
        );

        assert!(bundles
            .iter()
            .any(|bundle| bundle.reason == ShopBossPreviewBundleReason::Baseline));
        assert!(bundles.iter().any(|bundle| {
            bundle.items.contains(&DecisionCandidateKind::ShopBuyCard {
                card: CardId::FiendFire,
                upgrades: 0,
                price: 152,
            }) && bundle.items.contains(&DecisionCandidateKind::ShopPurge {
                target: CleanupTarget::StarterStrike,
            })
        }));
        assert!(bundles.iter().any(|bundle| {
            bundle.items.contains(&DecisionCandidateKind::ShopBuyCard {
                card: CardId::FiendFire,
                upgrades: 0,
                price: 152,
            }) && bundle
                .items
                .contains(&DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::FirePotion,
                    price: 51,
                })
        }));
    }

    #[test]
    fn includes_power_potion_as_high_ceiling_boss_preview_candidate() {
        let power = classify_shop_boss_preview_candidate(DecisionCandidateKind::ShopBuyPotion {
            potion: PotionId::PowerPotion,
            price: 78,
        });
        assert_eq!(power.class, ShopBossPreviewClass::RandomOrDeferred);
        assert!(power.include_in_v0);
        assert_eq!(power.reason, "HighCeilingPowerDiscoveryPotion");

        let bundles = shop_boss_preview_bundles(
            [
                DecisionCandidateKind::ShopLeave,
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                },
                DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::PowerPotion,
                    price: 78,
                },
                DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::AttackPotion,
                    price: 51,
                },
            ],
            240,
            12,
        );

        assert!(bundles.iter().any(|bundle| {
            bundle.items.contains(&DecisionCandidateKind::ShopBuyCard {
                card: CardId::FiendFire,
                upgrades: 0,
                price: 152,
            }) && bundle
                .items
                .contains(&DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::PowerPotion,
                    price: 78,
                })
        }));
        assert!(bundles.iter().all(|bundle| {
            !bundle
                .items
                .contains(&DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::AttackPotion,
                    price: 51,
                })
        }));
    }

    #[test]
    fn classifies_demon_form_as_deterministic_boss_scaling_repair() {
        let demon_form = classify_shop_boss_preview_candidate(DecisionCandidateKind::ShopBuyCard {
            card: CardId::DemonForm,
            upgrades: 0,
            price: 139,
        });

        assert_eq!(
            demon_form.class,
            ShopBossPreviewClass::DeterministicBossRepair
        );
        assert!(demon_form.include_in_v0);
        assert_eq!(demon_form.reason, "DeterministicBossScalingRepair");
    }

    #[test]
    fn bundle_generation_excludes_unaffordable_and_duplicate_cleanup_sequences() {
        let bundles = shop_boss_preview_bundles(
            [
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 300,
                },
                DecisionCandidateKind::ShopPurge {
                    target: CleanupTarget::StarterStrike,
                },
                DecisionCandidateKind::ShopPurge {
                    target: CleanupTarget::StarterDefend,
                },
                DecisionCandidateKind::ShopBuyPotion {
                    potion: PotionId::AttackPotion,
                    price: 50,
                },
            ],
            120,
            12,
        );

        assert!(bundles.iter().all(|bundle| bundle.total_cost <= 120));
        assert!(bundles.iter().all(|bundle| {
            bundle
                .items
                .iter()
                .filter(|item| matches!(item, DecisionCandidateKind::ShopPurge { .. }))
                .count()
                <= 1
        }));
        assert!(bundles
            .iter()
            .all(|bundle| !bundle.items.iter().any(|item| {
                matches!(
                    item,
                    DecisionCandidateKind::ShopBuyPotion {
                        potion: PotionId::AttackPotion,
                        ..
                    }
                )
            })));
    }

    #[test]
    fn does_not_emit_baseline_bundle_without_shop_preview_candidates() {
        let bundles = shop_boss_preview_bundles(
            [
                DecisionCandidateKind::CardRewardPick {
                    card: CardId::Cleave,
                    upgrades: 0,
                },
                DecisionCandidateKind::CardRewardSkip,
            ],
            100,
            12,
        );

        assert!(bundles.is_empty());
    }
}
