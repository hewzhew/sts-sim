use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mechanic {
    Strength,
    TemporaryStrength,
    StrengthMultiplier,
    CardDraw,
    Energy,
    Block,
    Weak,
    Vulnerable,
    EnemyStrengthDown,
    TopdeckControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamageScalingAxis {
    EnergySpent,
    HandSize,
    PerHitStrength,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunRewardKind {
    MaxHpOnFatal,
    GoldOnFatal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatEvent {
    CardExhausted,
    CardSelfDamage,
    StatusDrawn,
    TurnStart,
    TurnEnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayEffect {
    Provide(Mechanic),
    FrontloadDamage,
    AreaDamage,
    DamageUses(Mechanic),
    DamageScalesWith(DamageScalingAxis),
    EmitEvent(CombatEvent),
    ExhaustsSelf,
    RunReward(RunRewardKind),
    RecoverCurrentHp,
    CostReducedByHpLossThisCombat,
    AddCombatDeckClutter,
    PlayTopCardAndExhaust,
    CombatUpgradeSingle,
    CombatUpgradeAll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstalledRule {
    SkillCardsCostZeroAndExhaust,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TriggeredEffect {
    Provide(Mechanic),
    LoseHpFromCard,
    DealAllDamage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventHandler {
    pub on: CombatEvent,
    pub effect: TriggeredEffect,
}

impl EventHandler {
    pub const fn new(on: CombatEvent, effect: TriggeredEffect) -> Self {
        Self { on, effect }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayoffRequirement {
    WantsMechanic(Mechanic),
    WantsEventStream(CombatEvent),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardBurden {
    PowerSetup,
    HpCost,
    DrawLockout,
    AddsCombatDeckClutter,
    RandomExhaust,
    ExhaustsHand,
    RequiresEnemyAttackIntent,
    CardBlockLockoutUntilNextTurn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicateBehavior {
    Normal,
    StackingHandler,
    StackingOutput,
    RedundantAfterInstalled,
    DiminishingReturn,
    AccessCopyUseful,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinition {
    pub card: CardId,
    pub play_effects: Vec<PlayEffect>,
    pub installed_rules: Vec<InstalledRule>,
    pub event_handlers: Vec<EventHandler>,
    pub payoff_requirements: Vec<PayoffRequirement>,
    pub burdens: Vec<CardBurden>,
    pub duplicate_behaviors: Vec<DuplicateBehavior>,
}

impl CardDefinition {
    pub fn new(card: CardId) -> Self {
        Self {
            card,
            play_effects: Vec::new(),
            installed_rules: Vec::new(),
            event_handlers: Vec::new(),
            payoff_requirements: Vec::new(),
            burdens: Vec::new(),
            duplicate_behaviors: Vec::new(),
        }
    }

    pub fn effect(mut self, effect: PlayEffect) -> Self {
        push_unique(&mut self.play_effects, effect);
        self
    }

    pub fn provides(self, mechanic: Mechanic) -> Self {
        self.effect(PlayEffect::Provide(mechanic))
    }

    pub fn installs(mut self, rule: InstalledRule) -> Self {
        push_unique(&mut self.installed_rules, rule);
        self
    }

    pub fn handles(mut self, handler: EventHandler) -> Self {
        push_unique(&mut self.event_handlers, handler);
        self
    }

    pub fn wants(mut self, requirement: PayoffRequirement) -> Self {
        push_unique(&mut self.payoff_requirements, requirement);
        self
    }

    pub fn burden(mut self, burden: CardBurden) -> Self {
        push_unique(&mut self.burdens, burden);
        self
    }

    pub fn duplicate(mut self, behavior: DuplicateBehavior) -> Self {
        if behavior != DuplicateBehavior::Normal {
            push_unique(&mut self.duplicate_behaviors, behavior);
        }
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeckMechanicContext {
    pub mechanics: Vec<Mechanic>,
    pub event_streams: Vec<CombatEvent>,
    pub installed_rules: Vec<InstalledRule>,
    pub event_handlers: Vec<EventHandler>,
    pub burdens: Vec<CardBurden>,
    pub duplicate_behaviors: Vec<DuplicateBehavior>,
    pub payoff_requirements: Vec<PayoffRequirement>,
    pub open_payoff_requirements: Vec<PayoffRequirement>,
}

impl DeckMechanicContext {
    pub fn from_definitions(definitions: &[CardDefinition]) -> Self {
        let mut context = Self::default();
        for definition in definitions {
            context.add_direct_definition_facts(definition);
        }
        context.derive_triggered_facts();
        for definition in definitions {
            for requirement in &definition.payoff_requirements {
                push_unique(&mut context.payoff_requirements, *requirement);
            }
            for requirement in &definition.payoff_requirements {
                if !context.satisfies(*requirement) {
                    push_unique(&mut context.open_payoff_requirements, *requirement);
                }
            }
        }
        context
    }

    pub fn satisfies(&self, requirement: PayoffRequirement) -> bool {
        match requirement {
            PayoffRequirement::WantsMechanic(mechanic) => self.mechanics.contains(&mechanic),
            PayoffRequirement::WantsEventStream(event) => self.event_streams.contains(&event),
        }
    }

    fn add_direct_definition_facts(&mut self, definition: &CardDefinition) {
        for effect in &definition.play_effects {
            match effect {
                PlayEffect::Provide(mechanic) => push_unique(&mut self.mechanics, *mechanic),
                PlayEffect::EmitEvent(event) => push_unique(&mut self.event_streams, *event),
                PlayEffect::PlayTopCardAndExhaust => {
                    push_unique(&mut self.event_streams, CombatEvent::CardExhausted);
                }
                PlayEffect::AddCombatDeckClutter => {
                    push_unique(&mut self.event_streams, CombatEvent::StatusDrawn);
                }
                PlayEffect::FrontloadDamage
                | PlayEffect::AreaDamage
                | PlayEffect::DamageUses(_)
                | PlayEffect::DamageScalesWith(_)
                | PlayEffect::ExhaustsSelf
                | PlayEffect::RunReward(_)
                | PlayEffect::RecoverCurrentHp
                | PlayEffect::CostReducedByHpLossThisCombat
                | PlayEffect::CombatUpgradeSingle
                | PlayEffect::CombatUpgradeAll => {}
            }
        }
        for rule in &definition.installed_rules {
            push_unique(&mut self.installed_rules, *rule);
        }
        for handler in &definition.event_handlers {
            push_unique(&mut self.event_handlers, *handler);
        }
        for burden in &definition.burdens {
            push_unique(&mut self.burdens, *burden);
        }
        for behavior in &definition.duplicate_behaviors {
            push_unique(&mut self.duplicate_behaviors, *behavior);
        }
    }

    fn derive_triggered_facts(&mut self) {
        let handlers = self.event_handlers.clone();
        for handler in handlers {
            if !self.event_is_available(handler.on) {
                continue;
            }
            match handler.effect {
                TriggeredEffect::Provide(mechanic) => push_unique(&mut self.mechanics, mechanic),
                TriggeredEffect::LoseHpFromCard => {
                    push_unique(&mut self.event_streams, CombatEvent::CardSelfDamage);
                }
                TriggeredEffect::DealAllDamage => {}
            }
        }
    }

    fn event_is_available(&self, event: CombatEvent) -> bool {
        matches!(event, CombatEvent::TurnStart | CombatEvent::TurnEnd)
            || self.event_streams.contains(&event)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateMechanicFinding {
    PlayEffect(PlayEffect),
    InstallsRule(InstalledRule),
    EventHandler(EventHandler),
    SupportedPayoff(PayoffRequirement),
    OpenPayoff(PayoffRequirement),
    Burden(CardBurden),
    DuplicateBehavior(DuplicateBehavior),
}

pub fn card_definition(card: CardId) -> CardDefinition {
    card_definition_with_upgrades(card, 0)
}

pub fn card_definition_with_upgrades(card: CardId, upgrades: u8) -> CardDefinition {
    use CardBurden::*;
    use CardId::*;
    use CombatEvent::*;
    use DamageScalingAxis::*;
    use DuplicateBehavior::*;
    use Mechanic::*;
    use PlayEffect::*;
    use RunRewardKind::*;

    match card {
        Inflame => CardDefinition::new(card)
            .provides(Strength)
            .duplicate(StackingOutput),
        SpotWeakness => CardDefinition::new(card)
            .provides(Strength)
            .burden(RequiresEnemyAttackIntent)
            .duplicate(StackingOutput),
        DemonForm => CardDefinition::new(card)
            .handles(EventHandler::new(
                TurnStart,
                TriggeredEffect::Provide(Strength),
            ))
            .burden(PowerSetup)
            .duplicate(StackingOutput),
        Flex => CardDefinition::new(card).provides(TemporaryStrength),
        LimitBreak => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Strength))
            .provides(StrengthMultiplier),
        HeavyBlade | SwordBoomerang | Reaper => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Strength))
            .effect(DamageUses(Strength)),
        Pummel => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Strength))
            .effect(DamageUses(Strength))
            .effect(FrontloadDamage),

        Offering => CardDefinition::new(card)
            .provides(CardDraw)
            .provides(Energy)
            .effect(EmitEvent(CardSelfDamage))
            .burden(HpCost),
        SeeingRed => CardDefinition::new(card).provides(Energy),
        Bloodletting => CardDefinition::new(card)
            .provides(Energy)
            .effect(EmitEvent(CardSelfDamage))
            .burden(HpCost),
        Rupture => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsEventStream(CardSelfDamage))
            .handles(EventHandler::new(
                CardSelfDamage,
                TriggeredEffect::Provide(Strength),
            ))
            .burden(PowerSetup)
            .duplicate(StackingHandler),

        Corruption => CardDefinition::new(card)
            .installs(InstalledRule::SkillCardsCostZeroAndExhaust)
            .burden(PowerSetup)
            .duplicate(RedundantAfterInstalled)
            .duplicate(AccessCopyUseful),
        FeelNoPain => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsEventStream(CardExhausted))
            .handles(EventHandler::new(
                CardExhausted,
                TriggeredEffect::Provide(Block),
            ))
            .burden(PowerSetup)
            .duplicate(StackingHandler),
        DarkEmbrace => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsEventStream(CardExhausted))
            .handles(EventHandler::new(
                CardExhausted,
                TriggeredEffect::Provide(CardDraw),
            ))
            .burden(PowerSetup)
            .duplicate(StackingHandler),
        TrueGrit => CardDefinition::new(card)
            .provides(Block)
            .effect(EmitEvent(CardExhausted))
            .burden(RandomExhaust),
        SecondWind => CardDefinition::new(card)
            .provides(Block)
            .effect(EmitEvent(CardExhausted)),
        SeverSoul => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(EmitEvent(CardExhausted)),
        BurningPact => CardDefinition::new(card)
            .provides(CardDraw)
            .effect(EmitEvent(CardExhausted)),
        Havoc => CardDefinition::new(card)
            .effect(PlayTopCardAndExhaust)
            .effect(EmitEvent(CardExhausted))
            .burden(RandomExhaust),

        Armaments => {
            let upgrade_effect = if upgrades > 0 {
                CombatUpgradeAll
            } else {
                CombatUpgradeSingle
            };
            CardDefinition::new(card)
                .provides(Block)
                .effect(upgrade_effect)
        }
        BattleTrance => CardDefinition::new(card)
            .provides(CardDraw)
            .burden(DrawLockout)
            .duplicate(DiminishingReturn)
            .duplicate(AccessCopyUseful),
        Finesse => CardDefinition::new(card).provides(Block).provides(CardDraw),
        FlashOfSteel => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(CardDraw),
        PommelStrike => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(CardDraw),
        ShrugItOff => CardDefinition::new(card).provides(Block).provides(CardDraw),
        Trip => CardDefinition::new(card).provides(Vulnerable),
        Bash | ThunderClap => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(Vulnerable),
        Shockwave => CardDefinition::new(card)
            .provides(Vulnerable)
            .provides(Weak)
            .provides(EnemyStrengthDown)
            .effect(ExhaustsSelf),
        Uppercut => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(Vulnerable)
            .provides(Weak),
        Clothesline => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(Weak),
        Disarm => CardDefinition::new(card).provides(EnemyStrengthDown),
        FlameBarrier | Impervious => CardDefinition::new(card).provides(Block),
        IronWave => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(Block),
        Carnage | Rampage | SwiftStrike | TwinStrike => {
            CardDefinition::new(card).effect(FrontloadDamage)
        }
        Feed => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(ExhaustsSelf)
            .effect(RunReward(MaxHpOnFatal)),
        FiendFire => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(EmitEvent(CardExhausted))
            .effect(DamageScalesWith(HandSize))
            .effect(DamageScalesWith(PerHitStrength))
            .burden(ExhaustsHand),
        Whirlwind => CardDefinition::new(card)
            .effect(AreaDamage)
            .effect(DamageScalesWith(EnergySpent))
            .effect(DamageScalesWith(PerHitStrength)),
        Cleave => CardDefinition::new(card).effect(AreaDamage),
        Immolate => CardDefinition::new(card)
            .effect(AreaDamage)
            .effect(AddCombatDeckClutter)
            .burden(AddsCombatDeckClutter),
        Hemokinesis => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(EmitEvent(CardSelfDamage))
            .burden(HpCost),
        BodySlam => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsMechanic(Block))
            .effect(DamageUses(Block)),
        Entrench => CardDefinition::new(card).wants(PayoffRequirement::WantsMechanic(Block)),
        PowerThrough => CardDefinition::new(card)
            .provides(Block)
            .effect(AddCombatDeckClutter)
            .burden(AddsCombatDeckClutter),
        WildStrike | RecklessCharge => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(AddCombatDeckClutter)
            .burden(AddsCombatDeckClutter),
        Headbutt => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .provides(TopdeckControl),
        Warcry => CardDefinition::new(card).provides(TopdeckControl),
        Evolve => CardDefinition::new(card)
            .wants(PayoffRequirement::WantsEventStream(StatusDrawn))
            .handles(EventHandler::new(
                StatusDrawn,
                TriggeredEffect::Provide(CardDraw),
            ))
            .burden(PowerSetup)
            .duplicate(StackingHandler),
        Metallicize => CardDefinition::new(card)
            .handles(EventHandler::new(TurnEnd, TriggeredEffect::Provide(Block)))
            .burden(PowerSetup)
            .duplicate(StackingOutput),
        Combust => CardDefinition::new(card)
            .handles(EventHandler::new(TurnEnd, TriggeredEffect::LoseHpFromCard))
            .handles(EventHandler::new(TurnEnd, TriggeredEffect::DealAllDamage))
            .burden(PowerSetup)
            .duplicate(StackingOutput),
        Brutality => CardDefinition::new(card)
            .handles(EventHandler::new(
                TurnStart,
                TriggeredEffect::LoseHpFromCard,
            ))
            .handles(EventHandler::new(
                TurnStart,
                TriggeredEffect::Provide(CardDraw),
            ))
            .burden(PowerSetup)
            .duplicate(StackingOutput),
        MasterOfStrategy => CardDefinition::new(card)
            .provides(CardDraw)
            .effect(ExhaustsSelf),
        GoodInstincts => CardDefinition::new(card).provides(Block),
        Intimidate => CardDefinition::new(card)
            .provides(Weak)
            .effect(ExhaustsSelf),
        BandageUp => CardDefinition::new(card)
            .effect(RecoverCurrentHp)
            .effect(ExhaustsSelf),
        PanicButton => CardDefinition::new(card)
            .provides(Block)
            .effect(ExhaustsSelf)
            .burden(CardBlockLockoutUntilNextTurn),
        HandOfGreed => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(RunReward(GoldOnFatal)),
        BloodForBlood => CardDefinition::new(card)
            .effect(FrontloadDamage)
            .effect(CostReducedByHpLossThisCombat),
        // Juggernaut needs a future BlockGained reactive-power model; do not
        // flatten it into ordinary frontload damage.
        Juggernaut => CardDefinition::new(card),
        _ => CardDefinition::new(card),
    }
}

pub fn evaluate_candidate_definition(
    candidate: &CardDefinition,
    deck: &DeckMechanicContext,
) -> Vec<CandidateMechanicFinding> {
    let mut findings = Vec::new();
    for effect in &candidate.play_effects {
        findings.push(CandidateMechanicFinding::PlayEffect(*effect));
    }
    for rule in &candidate.installed_rules {
        findings.push(CandidateMechanicFinding::InstallsRule(*rule));
    }
    for handler in &candidate.event_handlers {
        findings.push(CandidateMechanicFinding::EventHandler(*handler));
    }
    for requirement in &candidate.payoff_requirements {
        if deck.satisfies(*requirement) {
            findings.push(CandidateMechanicFinding::SupportedPayoff(*requirement));
        } else {
            findings.push(CandidateMechanicFinding::OpenPayoff(*requirement));
        }
    }
    for burden in &candidate.burdens {
        findings.push(CandidateMechanicFinding::Burden(*burden));
    }
    for behavior in &candidate.duplicate_behaviors {
        findings.push(CandidateMechanicFinding::DuplicateBehavior(*behavior));
    }
    findings
}

fn push_unique<T: Eq>(items: &mut Vec<T>, value: T) {
    if !items.contains(&value) {
        items.push(value);
    }
}
