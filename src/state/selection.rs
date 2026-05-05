use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::events::EventId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionScope {
    Hand,
    Deck,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionReason {
    Upgrade,
    Purge,
    Transform,
    TransformUpgraded,
    Duplicate,
    Exhaust,
    Discard,
    Retain,
    PutOnDrawPile,
    PutToBottomOfDraw,
    Copy,
    GamblingChip,
    MoveToDrawPile,
    Exhume,
    SkillFromDeckToHand,
    AttackFromDeckToHand,
    DiscardToHand,
    Discovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionConstraint {
    Exactly(usize),
    Between { min: usize, max: usize },
    UpToAvailable,
    OptionalUpToAvailable,
}

impl SelectionConstraint {
    pub fn from_bounds(min: usize, max: usize, available: usize) -> Self {
        if available == 0 {
            return if min == 0 {
                SelectionConstraint::OptionalUpToAvailable
            } else {
                SelectionConstraint::Exactly(min)
            };
        }

        if min == 0 && max >= available {
            SelectionConstraint::OptionalUpToAvailable
        } else if min == 1 && max >= available {
            SelectionConstraint::UpToAvailable
        } else if min == max {
            SelectionConstraint::Exactly(min)
        } else {
            SelectionConstraint::Between { min, max }
        }
    }

    pub fn describe(self, available: usize) -> String {
        match self {
            SelectionConstraint::Exactly(n) => format!("choose exactly {n}"),
            SelectionConstraint::Between { min, max } => format!("choose {min}-{max}"),
            SelectionConstraint::UpToAvailable => {
                format!("choose up to {}", available.max(1))
            }
            SelectionConstraint::OptionalUpToAvailable => {
                format!("choose 0-{}", available)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionTargetRef {
    CardUuid(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionRequest {
    pub scope: SelectionScope,
    pub reason: SelectionReason,
    pub constraint: SelectionConstraint,
    pub can_cancel: bool,
    pub targets: Vec<SelectionTargetRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionResolution {
    pub scope: SelectionScope,
    pub selected: Vec<SelectionTargetRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainEventSource {
    Event(EventId),
    CampfireSmith,
    CampfireToke,
    Selection(SelectionReason),
    DeckMutation,
    RewardScreen,
    Shop,
    BossRelicChoice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DomainCardSnapshot {
    pub id: CardId,
    pub upgrades: u8,
    pub uuid: u32,
}

impl DomainCardSnapshot {
    pub fn upgraded(mut self) -> Self {
        self.upgrades += 1;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainEvent {
    RelicObtained {
        relic_id: RelicId,
        source: DomainEventSource,
    },
    RelicLost {
        relic_id: RelicId,
        source: DomainEventSource,
    },
    GoldChanged {
        delta: i32,
        new_total: i32,
        source: DomainEventSource,
    },
    HpChanged {
        delta: i32,
        current_hp: i32,
        max_hp: i32,
        source: DomainEventSource,
    },
    MaxHpChanged {
        delta: i32,
        current_hp: i32,
        max_hp: i32,
        source: DomainEventSource,
    },
    PotionObtained {
        potion_id: PotionId,
        slot: usize,
        source: DomainEventSource,
    },
    SelectionResolved {
        scope: SelectionScope,
        reason: SelectionReason,
        selected: Vec<SelectionTargetRef>,
        source: DomainEventSource,
    },
    CardObtained {
        card: DomainCardSnapshot,
        source: DomainEventSource,
    },
    CardRemoved {
        card: DomainCardSnapshot,
        source: DomainEventSource,
    },
    CardUpgraded {
        before: DomainCardSnapshot,
        after: DomainCardSnapshot,
        source: DomainEventSource,
    },
    CardTransformed {
        before: DomainCardSnapshot,
        after: DomainCardSnapshot,
        source: DomainEventSource,
    },
    CardsExhausted {
        cards: Vec<DomainCardSnapshot>,
        source: DomainEventSource,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineDiagnosticClass {
    Normalization,
    Suspicious,
    Broken,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineDiagnostic {
    pub severity: EngineDiagnosticSeverity,
    pub class: EngineDiagnosticClass,
    pub message: String,
}
