//! Shared identity-independent card and potion mechanics for semantic schema v6.

use sts_oracle_eval::content::cards::{
    get_card_definition, CardId, CardRarity, CardTag, CardTarget, CardType,
};
use sts_oracle_eval::content::potions::{
    get_potion_definition, potion_mechanic_roles, PotionClass, PotionId, PotionMechanicRole,
    PotionRarity, POTION_MECHANIC_ROLE_COUNT,
};

use super::{bool_value, CategoricalField, ScalarField, SemanticBatchBuilder};

/// Identity embeddings in these fields are residuals over shared mechanics.
/// The model initializes their full vocabulary slices to zero, so an identity
/// absent from training falls back to typed mechanical attributes instead of
/// contributing a random vector.
pub(crate) const IDENTITY_RESIDUAL_CATEGORICAL_FIELDS: &[u16] = &[
    CategoricalField::CardId as u16,
    CategoricalField::PotionId as u16,
    CategoricalField::ActionCardId as u16,
    CategoricalField::ActionPotionId as u16,
];

pub(crate) const CARD_TYPE_SCHEMA: &[(&str, i64)] = &[
    ("Attack", CardType::Attack as i64),
    ("Skill", CardType::Skill as i64),
    ("Power", CardType::Power as i64),
    ("Status", CardType::Status as i64),
    ("Curse", CardType::Curse as i64),
];
pub(crate) const CARD_RARITY_SCHEMA: &[(&str, i64)] = &[
    ("Basic", CardRarity::Basic as i64),
    ("Common", CardRarity::Common as i64),
    ("Uncommon", CardRarity::Uncommon as i64),
    ("Rare", CardRarity::Rare as i64),
    ("Special", CardRarity::Special as i64),
    ("Curse", CardRarity::Curse as i64),
];
pub(crate) const CARD_TARGET_SCHEMA: &[(&str, i64)] = &[
    ("Enemy", CardTarget::Enemy as i64),
    ("AllEnemy", CardTarget::AllEnemy as i64),
    ("All", CardTarget::All as i64),
    ("SelfAndEnemy", CardTarget::SelfAndEnemy as i64),
    ("SelfTarget", CardTarget::SelfTarget as i64),
    ("None", CardTarget::None as i64),
];
pub(crate) const CARD_TAG_SCHEMA: &[(&str, i64)] = &[
    ("Strike", CardTag::Strike as i64),
    ("StarterStrike", CardTag::StarterStrike as i64),
    ("StarterDefend", CardTag::StarterDefend as i64),
    ("Healing", CardTag::Healing as i64),
    ("Empty", CardTag::Empty as i64),
];
pub(crate) const POTION_RARITY_SCHEMA: &[(&str, i64)] = &[
    ("Common", PotionRarity::Common as i64),
    ("Uncommon", PotionRarity::Uncommon as i64),
    ("Rare", PotionRarity::Rare as i64),
];
pub(crate) const POTION_CLASS_SCHEMA: &[(&str, i64)] = &[
    ("Any", PotionClass::Any as i64),
    ("Ironclad", PotionClass::Ironclad as i64),
    ("Silent", PotionClass::Silent as i64),
    ("Defect", PotionClass::Defect as i64),
    ("Watcher", PotionClass::Watcher as i64),
];
pub(crate) const POTION_MECHANIC_ROLE_SCHEMA: &[(&str, i64)] = &[
    ("DirectDamage", PotionMechanicRole::DirectDamage as i64),
    ("MultiTarget", PotionMechanicRole::MultiTarget as i64),
    ("ApplyPoison", PotionMechanicRole::ApplyPoison as i64),
    ("ApplyWeak", PotionMechanicRole::ApplyWeak as i64),
    (
        "ApplyVulnerable",
        PotionMechanicRole::ApplyVulnerable as i64,
    ),
    ("GainBlock", PotionMechanicRole::GainBlock as i64),
    ("Heal", PotionMechanicRole::Heal as i64),
    ("GainEnergy", PotionMechanicRole::GainEnergy as i64),
    ("GainStrength", PotionMechanicRole::GainStrength as i64),
    ("GainDexterity", PotionMechanicRole::GainDexterity as i64),
    ("Temporary", PotionMechanicRole::Temporary as i64),
    ("DrawCards", PotionMechanicRole::DrawCards as i64),
    ("GainFocus", PotionMechanicRole::GainFocus as i64),
    ("DiscoverAttack", PotionMechanicRole::DiscoverAttack as i64),
    ("DiscoverSkill", PotionMechanicRole::DiscoverSkill as i64),
    ("DiscoverPower", PotionMechanicRole::DiscoverPower as i64),
    (
        "DiscoverColorless",
        PotionMechanicRole::DiscoverColorless as i64,
    ),
    ("AddCardToHand", PotionMechanicRole::AddCardToHand as i64),
    ("UpgradeHand", PotionMechanicRole::UpgradeHand as i64),
    ("GainArtifact", PotionMechanicRole::GainArtifact as i64),
    (
        "GainRegeneration",
        PotionMechanicRole::GainRegeneration as i64,
    ),
    (
        "GainPlatedArmor",
        PotionMechanicRole::GainPlatedArmor as i64,
    ),
    ("GainThorns", PotionMechanicRole::GainThorns as i64),
    ("PlayTopCards", PotionMechanicRole::PlayTopCards as i64),
    ("DuplicateCards", PotionMechanicRole::DuplicateCards as i64),
    ("GainOrbSlots", PotionMechanicRole::GainOrbSlots as i64),
    (
        "RetrieveFromDiscard",
        PotionMechanicRole::RetrieveFromDiscard as i64,
    ),
    ("DiscardAndDraw", PotionMechanicRole::DiscardAndDraw as i64),
    ("ExhaustCards", PotionMechanicRole::ExhaustCards as i64),
    ("ChooseStance", PotionMechanicRole::ChooseStance as i64),
    ("PreventDeath", PotionMechanicRole::PreventDeath as i64),
    ("EscapeCombat", PotionMechanicRole::EscapeCombat as i64),
    ("GainMaxHp", PotionMechanicRole::GainMaxHp as i64),
    (
        "GeneratePotions",
        PotionMechanicRole::GeneratePotions as i64,
    ),
    (
        "RandomizeHandCosts",
        PotionMechanicRole::RandomizeHandCosts as i64,
    ),
    ("GainIntangible", PotionMechanicRole::GainIntangible as i64),
    (
        "GainMetallicize",
        PotionMechanicRole::GainMetallicize as i64,
    ),
    ("GainRitual", PotionMechanicRole::GainRitual as i64),
    ("EnterDivinity", PotionMechanicRole::EnterDivinity as i64),
    ("ChannelDark", PotionMechanicRole::ChannelDark as i64),
];

const _: () = {
    assert!(CardType::Attack as i64 == 0);
    assert!(CardType::Curse as usize + 1 == CARD_TYPE_SCHEMA.len());
    assert!(CardRarity::Basic as i64 == 0);
    assert!(CardRarity::Curse as usize + 1 == CARD_RARITY_SCHEMA.len());
    assert!(CardTarget::Enemy as i64 == 0);
    assert!(CardTarget::None as usize + 1 == CARD_TARGET_SCHEMA.len());
    assert!(CardTag::Strike as i64 == 0);
    assert!(CardTag::Empty as usize + 1 == CARD_TAG_SCHEMA.len());
    assert!(PotionRarity::Common as i64 == 0);
    assert!(PotionRarity::Rare as usize + 1 == POTION_RARITY_SCHEMA.len());
    assert!(PotionClass::Any as i64 == 0);
    assert!(PotionClass::Watcher as usize + 1 == POTION_CLASS_SCHEMA.len());
    assert!(PotionMechanicRole::DirectDamage as i64 == 0);
    assert!(PotionMechanicRole::ChannelDark as u64 + 1 == POTION_MECHANIC_ROLE_COUNT);
    assert!(POTION_MECHANIC_ROLE_SCHEMA.len() as u64 == POTION_MECHANIC_ROLE_COUNT);
};

impl SemanticBatchBuilder {
    pub(super) fn card_identity_with_mechanics(
        &mut self,
        token: u64,
        identity_field: CategoricalField,
        card: CardId,
    ) {
        self.category(token, identity_field, card as i64);
        let definition = get_card_definition(card);
        self.category(
            token,
            CategoricalField::CardType,
            definition.card_type as i64,
        );
        self.category(
            token,
            CategoricalField::CardRarity,
            definition.rarity as i64,
        );
        self.category(
            token,
            CategoricalField::CardTarget,
            definition.target as i64,
        );
        self.category(
            token,
            CategoricalField::CardIsMultiDamage,
            bool_value(definition.is_multi_damage),
        );
        self.category(
            token,
            CategoricalField::CardExhaust,
            bool_value(definition.exhaust),
        );
        self.category(
            token,
            CategoricalField::CardEthereal,
            bool_value(definition.ethereal),
        );
        self.category(
            token,
            CategoricalField::CardInnate,
            bool_value(definition.innate),
        );
        for tag in definition.tags {
            self.category(token, CategoricalField::CardTag, *tag as i64);
        }
        self.scalar(token, ScalarField::CardBaseCost, definition.cost);
        self.scalar(
            token,
            ScalarField::CardDefinitionDamage,
            definition.base_damage,
        );
        self.scalar(
            token,
            ScalarField::CardDefinitionBlock,
            definition.base_block,
        );
        self.scalar(
            token,
            ScalarField::CardDefinitionMagic,
            definition.base_magic,
        );
        self.scalar(
            token,
            ScalarField::CardUpgradeDamage,
            definition.upgrade_damage,
        );
        self.scalar(
            token,
            ScalarField::CardUpgradeBlock,
            definition.upgrade_block,
        );
        self.scalar(
            token,
            ScalarField::CardUpgradeMagic,
            definition.upgrade_magic,
        );
    }

    pub(super) fn potion_identity_with_mechanics(
        &mut self,
        token: u64,
        identity_field: CategoricalField,
        potion: PotionId,
    ) {
        self.category(token, identity_field, potion as i64);
        let definition = get_potion_definition(potion);
        self.category(
            token,
            CategoricalField::PotionRarity,
            definition.rarity as i64,
        );
        self.category(
            token,
            CategoricalField::PotionClass,
            definition.class as i64,
        );
        self.category(
            token,
            CategoricalField::PotionIsThrown,
            bool_value(definition.is_thrown),
        );
        for role in potion_mechanic_roles(potion) {
            self.category(token, CategoricalField::PotionMechanicRole, *role as i64);
        }
        self.scalar(
            token,
            ScalarField::PotionBasePotency,
            definition.base_potency,
        );
    }

    pub(super) fn action_card(&mut self, token: u64, card: CardId, upgrades: u8) {
        self.card_identity_with_mechanics(token, CategoricalField::ActionCardId, card);
        self.scalar(token, ScalarField::ActionUpgrades, upgrades);
    }

    pub(super) fn action_potion(&mut self, token: u64, potion: PotionId) {
        self.potion_identity_with_mechanics(token, CategoricalField::ActionPotionId, potion);
    }
}
