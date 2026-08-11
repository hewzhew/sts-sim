//! Identity-independent card mechanics shared by runtime and learned-policy adapters.
//!
//! `CardDefinition` remains the source for the ordinary printed attributes. This
//! projection adds upgrade-sensitive effective attributes and a deliberately
//! bounded vocabulary for effects that the printed definition cannot express.
//! Role coverage is explicit: an absent role is only a negative fact when the
//! profile is `Complete`.

use serde::{Deserialize, Serialize};

use super::{get_card_definition, CardId, CardTarget};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u8)]
pub enum CardMechanicCoverage {
    /// Only the printed `CardDefinition` and effective upgrade attributes are known.
    DefinitionOnly = 0,
    /// Every listed role is true, but other roles may still be missing.
    Partial = 1,
    /// The card was reviewed against its production play/trigger actions for this vocabulary.
    Complete = 2,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[repr(u8)]
pub enum CardMechanicRole {
    DrawCards = 0,
    GainEnergy = 1,
    ApplyWeak = 2,
    ApplyVulnerable = 3,
    ExhaustOtherCards = 4,
    SelectFromHand = 5,
    DiscardFromHand = 6,
    DrawPileControl = 7,
    MultiHit = 8,
    RandomTarget = 9,
    GenerateCard = 10,
    SelfHpLoss = 11,
    EndTurnHandTrigger = 12,
    DrawOnStatus = 13,
    HandSizeScaling = 14,
    RandomOutcome = 15,
}

pub const CARD_MECHANIC_ROLE_COUNT: u64 = CardMechanicRole::RandomOutcome as u64 + 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CardMechanicRoleSet(u32);

impl CardMechanicRoleSet {
    pub fn contains(self, role: CardMechanicRole) -> bool {
        self.0 & (1_u32 << role as u8) != 0
    }

    pub fn iter(self) -> impl Iterator<Item = CardMechanicRole> {
        const ROLES: [CardMechanicRole; CARD_MECHANIC_ROLE_COUNT as usize] = [
            CardMechanicRole::DrawCards,
            CardMechanicRole::GainEnergy,
            CardMechanicRole::ApplyWeak,
            CardMechanicRole::ApplyVulnerable,
            CardMechanicRole::ExhaustOtherCards,
            CardMechanicRole::SelectFromHand,
            CardMechanicRole::DiscardFromHand,
            CardMechanicRole::DrawPileControl,
            CardMechanicRole::MultiHit,
            CardMechanicRole::RandomTarget,
            CardMechanicRole::GenerateCard,
            CardMechanicRole::SelfHpLoss,
            CardMechanicRole::EndTurnHandTrigger,
            CardMechanicRole::DrawOnStatus,
            CardMechanicRole::HandSizeScaling,
            CardMechanicRole::RandomOutcome,
        ];
        ROLES.into_iter().filter(move |role| self.contains(*role))
    }

    fn insert(&mut self, role: CardMechanicRole) {
        self.0 |= 1_u32 << role as u8;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardMechanicProfile {
    pub coverage: CardMechanicCoverage,
    pub roles: CardMechanicRoleSet,
    pub effective_target: CardTarget,
    pub exhausts_when_played: bool,
    pub ethereal: bool,
}

pub fn effective_card_target(card: CardId, upgrades: u8) -> CardTarget {
    match card {
        CardId::Blind | CardId::Trip if upgrades > 0 => CardTarget::AllEnemy,
        _ => get_card_definition(card).target,
    }
}

pub fn card_exhausts_when_played(card: CardId, upgrades: u8) -> bool {
    match card {
        CardId::CalculatedGamble
        | CardId::LimitBreak
        | CardId::Discovery
        | CardId::SecretTechnique
        | CardId::SecretWeapon
        | CardId::ThinkingAhead
        | CardId::Hologram
        | CardId::Rainbow
        | CardId::Impulse => upgrades == 0,
        _ => get_card_definition(card).exhaust,
    }
}

pub fn card_is_ethereal(card: CardId, upgrades: u8) -> bool {
    match card {
        CardId::Apparition | CardId::EchoForm | CardId::DevaForm => upgrades == 0,
        _ => get_card_definition(card).ethereal,
    }
}

/// Returns stable mechanical facts for one concrete upgrade state.
///
/// The broad positive-role registry is intentionally allowed to be partial.
/// The small `Complete` set is the reviewed Ironclad Sentries transfer surface;
/// expanding that set requires production-action tests in the same change.
pub fn card_mechanic_profile(card: CardId, upgrades: u8) -> CardMechanicProfile {
    use CardId::*;
    use CardMechanicCoverage::{Complete, DefinitionOnly, Partial};
    use CardMechanicRole as Role;

    let upgraded = upgrades > 0;
    let mut roles = CardMechanicRoleSet::default();

    if matches!(
        card,
        PommelStrike
            | ShrugItOff
            | BurningPact
            | BattleTrance
            | Offering
            | Warcry
            | Finesse
            | FlashOfSteel
            | DeepBreath
            | Impatience
            | Insight
            | MasterOfStrategy
            | ThinkingAhead
            | Coolheaded
            | Overclock
            | Reboot
            | Scrape
            | Skim
            | SweepingBeam
            | Acrobatics
            | Adrenaline
            | Backflip
            | DaggerThrow
            | EscapePlan
            | Expertise
            | Prepared
            | QuickSlash
            | Reflex
            | CutThroughFate
            | EmptyMind
            | Sanctity
            | Scrawl
            | WheelKick
    ) {
        roles.insert(Role::DrawCards);
    }
    if matches!(
        card,
        Offering
            | SeeingRed
            | Bloodletting
            | Adrenaline
            | Turbo
            | Tactician
            | HeelHook
            | Dropkick
            | Aggregate
            | DoubleEnergy
    ) {
        roles.insert(Role::GainEnergy);
    }
    if matches!(
        card,
        Clothesline
            | Uppercut
            | Shockwave
            | Blind
            | SuckerPunch
            | GoForTheEyes
            | SashWhip
            | WaveOfTheHand
    ) {
        roles.insert(Role::ApplyWeak);
    }
    if matches!(
        card,
        Bash | ThunderClap | Uppercut | Shockwave | Trip | Terror | BeamCell | CrushJoints
    ) {
        roles.insert(Role::ApplyVulnerable);
    }
    if matches!(
        card,
        BurningPact | TrueGrit | SecondWind | SeverSoul | FiendFire | Recycle | Purity
    ) {
        roles.insert(Role::ExhaustOtherCards);
    }
    if matches!(
        card,
        BurningPact | DualWield | Armaments | Forethought | Setup
    ) || (card == TrueGrit && upgraded)
        || card == ThinkingAhead
    {
        roles.insert(Role::SelectFromHand);
    }
    if matches!(
        card,
        Acrobatics | Prepared | DaggerThrow | CalculatedGamble | Survivor
    ) {
        roles.insert(Role::DiscardFromHand);
    }
    if matches!(
        card,
        ThinkingAhead
            | Warcry
            | Headbutt
            | Forethought
            | Setup
            | DeepBreath
            | Reboot
            | CutThroughFate
            | ThirdEye
    ) {
        roles.insert(Role::DrawPileControl);
    }
    if matches!(
        card,
        TwinStrike
            | SwordBoomerang
            | Pummel
            | FiendFire
            | RiddleWithHoles
            | Flechettes
            | Finisher
            | DaggerSpray
            | GlassKnife
            | RipAndTear
            | ThunderStrike
            | Barrage
            | Ragnarok
            | Tantrum
            | FlyingSleeves
    ) {
        roles.insert(Role::MultiHit);
    }
    if matches!(card, SwordBoomerang | RipAndTear | ThunderStrike | Ragnarok) {
        roles.insert(Role::RandomTarget);
    }
    if matches!(
        card,
        WildStrike
            | RecklessCharge
            | PowerThrough
            | Immolate
            | Discovery
            | InfernalBlade
            | JackOfAllTrades
            | WhiteNoise
            | Chrysalis
            | Metamorphosis
            | Transmutation
            | Magnetism
            | BladeDance
            | EndlessAgony
            | HelloWorld
            | ForeignInfluence
            | DeceiveReality
            | BattleHymn
    ) {
        roles.insert(Role::GenerateCard);
    }
    if matches!(
        card,
        Bloodletting | Offering | Hemokinesis | Combust | Brutality | JAX | Regret | Pain
    ) {
        roles.insert(Role::SelfHpLoss);
    }
    if matches!(card, Burn | Regret | Decay | Doubt | Pride | Shame) {
        roles.insert(Role::EndTurnHandTrigger);
    }
    if card == Evolve {
        roles.insert(Role::DrawOnStatus);
    }
    if matches!(card, Regret | FiendFire) {
        roles.insert(Role::HandSizeScaling);
    }
    if matches!(
        card,
        SwordBoomerang
            | Discovery
            | InfernalBlade
            | JackOfAllTrades
            | WhiteNoise
            | Chrysalis
            | Metamorphosis
            | Transmutation
            | Magnetism
            | Mayhem
    ) || (card == TrueGrit && !upgraded)
    {
        roles.insert(Role::RandomOutcome);
    }

    let coverage = if matches!(
        card,
        Strike
            | Defend
            | Bash
            | AscendersBane
            | Evolve
            | PommelStrike
            | ShrugItOff
            | SwordBoomerang
            | ThinkingAhead
            | TrueGrit
            | Regret
    ) {
        Complete
    } else if roles.iter().next().is_some() {
        Partial
    } else {
        DefinitionOnly
    };

    CardMechanicProfile {
        coverage,
        roles,
        effective_target: effective_card_target(card, upgrades),
        exhausts_when_played: card_exhausts_when_played(card, upgrades),
        ethereal: card_is_ethereal(card, upgrades),
    }
}

#[cfg(test)]
mod tests {
    use crate::content::cards::{
        resolve_card_play, resolve_card_play_with_context, CardUseContext,
    };
    use crate::content::powers::{self, PowerId};
    use crate::runtime::action::{Action, DamageType};
    use crate::runtime::combat::CombatCard;

    use super::*;

    fn has(profile: CardMechanicProfile, role: CardMechanicRole) -> bool {
        profile.roles.contains(role)
    }

    #[test]
    fn reviewed_sentries_play_roles_match_production_actions() {
        use CardMechanicRole as Role;

        let mut state = crate::test_support::blank_test_combat();
        state.zones.hand = vec![CombatCard::new(CardId::Defend, 20)];

        let bash = card_mechanic_profile(CardId::Bash, 0);
        let bash_actions = resolve_card_play(
            CardId::Bash,
            &state,
            &CombatCard::new(CardId::Bash, 1),
            Some(7),
        );
        assert_eq!(bash.coverage, CardMechanicCoverage::Complete);
        assert!(has(bash, Role::ApplyVulnerable));
        assert!(bash_actions.iter().any(|action| matches!(
            action.action,
            Action::ApplyPower {
                power_id: PowerId::Vulnerable,
                ..
            }
        )));

        let evolve = card_mechanic_profile(CardId::Evolve, 1);
        let mut evolve_card = CombatCard::new(CardId::Evolve, 2);
        evolve_card.upgrades = 1;
        let evolve_actions = resolve_card_play(CardId::Evolve, &state, &evolve_card, None);
        assert!(has(evolve, Role::DrawOnStatus));
        assert!(evolve_actions.iter().any(|action| matches!(
            action.action,
            Action::ApplyPower {
                power_id: PowerId::Evolve,
                amount: 2,
                ..
            }
        )));

        let pommel = card_mechanic_profile(CardId::PommelStrike, 1);
        let mut pommel_card = CombatCard::new(CardId::PommelStrike, 3);
        pommel_card.upgrades = 1;
        let pommel_actions = resolve_card_play(CardId::PommelStrike, &state, &pommel_card, Some(7));
        assert!(has(pommel, Role::DrawCards));
        assert!(matches!(pommel_actions[1].action, Action::DrawCards(2)));

        let shrug = card_mechanic_profile(CardId::ShrugItOff, 0);
        let shrug_actions = resolve_card_play(
            CardId::ShrugItOff,
            &state,
            &CombatCard::new(CardId::ShrugItOff, 4),
            None,
        );
        assert!(has(shrug, Role::DrawCards));
        assert!(matches!(shrug_actions[1].action, Action::DrawCards(1)));

        let sword = card_mechanic_profile(CardId::SwordBoomerang, 0);
        let sword_actions = resolve_card_play(
            CardId::SwordBoomerang,
            &state,
            &CombatCard::new(CardId::SwordBoomerang, 5),
            None,
        );
        assert!(has(sword, Role::MultiHit));
        assert!(has(sword, Role::RandomTarget));
        assert_eq!(sword_actions.len(), 3);
        assert!(sword_actions
            .iter()
            .all(|action| matches!(action.action, Action::AttackDamageRandomEnemyCard { .. })));

        let thinking = card_mechanic_profile(CardId::ThinkingAhead, 0);
        let thinking_actions = resolve_card_play_with_context(
            CardId::ThinkingAhead,
            &state,
            &CombatCard::new(CardId::ThinkingAhead, 6),
            None,
            CardUseContext {
                played_from_hand: true,
            },
        );
        assert!(has(thinking, Role::DrawCards));
        assert!(has(thinking, Role::SelectFromHand));
        assert!(has(thinking, Role::DrawPileControl));
        assert!(matches!(thinking_actions[0].action, Action::DrawCards(2)));
        assert!(matches!(
            thinking_actions[1].action,
            Action::SuspendForHandSelect {
                reason: crate::state::HandSelectReason::PutOnDrawPile,
                ..
            }
        ));
    }

    #[test]
    fn reviewed_upgrade_and_trigger_roles_match_production_actions() {
        use CardMechanicRole as Role;

        let mut state = crate::test_support::blank_test_combat();
        state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 30),
            CombatCard::new(CardId::Defend, 31),
            CombatCard::new(CardId::Regret, 32),
            CombatCard::new(CardId::Wound, 33),
        ];

        let true_grit_base = card_mechanic_profile(CardId::TrueGrit, 0);
        let base_actions = resolve_card_play(
            CardId::TrueGrit,
            &state,
            &CombatCard::new(CardId::TrueGrit, 7),
            None,
        );
        assert!(has(true_grit_base, Role::ExhaustOtherCards));
        assert!(has(true_grit_base, Role::RandomOutcome));
        assert!(!has(true_grit_base, Role::SelectFromHand));
        assert!(matches!(
            base_actions[1].action,
            Action::ExhaustFromHand { random: true, .. }
        ));

        let true_grit_plus = card_mechanic_profile(CardId::TrueGrit, 1);
        let mut upgraded_card = CombatCard::new(CardId::TrueGrit, 8);
        upgraded_card.upgrades = 1;
        let upgraded_actions = resolve_card_play(CardId::TrueGrit, &state, &upgraded_card, None);
        assert!(has(true_grit_plus, Role::SelectFromHand));
        assert!(!has(true_grit_plus, Role::RandomOutcome));
        assert!(matches!(
            upgraded_actions[1].action,
            Action::ExhaustFromHand { random: false, .. }
        ));

        let thinking_base = card_mechanic_profile(CardId::ThinkingAhead, 0);
        let thinking_plus = card_mechanic_profile(CardId::ThinkingAhead, 1);
        assert!(thinking_base.exhausts_when_played);
        assert!(!thinking_plus.exhausts_when_played);

        let regret = card_mechanic_profile(CardId::Regret, 0);
        let regret_actions = crate::content::cards::curses::regret::on_end_turn_in_hand(&state);
        assert!(has(regret, Role::EndTurnHandTrigger));
        assert!(has(regret, Role::SelfHpLoss));
        assert!(has(regret, Role::HandSizeScaling));
        assert!(matches!(
            regret_actions[0].action,
            Action::Damage(ref info)
                if info.target == 0
                    && info.output == 4
                    && info.damage_type == DamageType::HpLoss
        ));

        let evolve_trigger = powers::resolve_power_on_card_drawn(PowerId::Evolve, &state, 0, 2, 33);
        assert_eq!(evolve_trigger.as_slice(), &[Action::DrawCards(2)]);
    }

    #[test]
    fn effective_attributes_and_role_coverage_are_explicit() {
        assert_eq!(
            card_mechanic_profile(CardId::Blind, 0).effective_target,
            CardTarget::Enemy
        );
        assert_eq!(
            card_mechanic_profile(CardId::Blind, 1).effective_target,
            CardTarget::AllEnemy
        );
        assert!(card_mechanic_profile(CardId::Apparition, 0).ethereal);
        assert!(!card_mechanic_profile(CardId::Apparition, 1).ethereal);
        assert_eq!(
            card_mechanic_profile(CardId::Offering, 0).coverage,
            CardMechanicCoverage::Partial
        );
        assert_eq!(
            card_mechanic_profile(CardId::Bludgeon, 0).coverage,
            CardMechanicCoverage::DefinitionOnly
        );
    }
}
