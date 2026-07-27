use crate::runtime::combat::{
    AwakenedOneRuntimeState, BookOfStabbingRuntimeState, BronzeAutomatonRuntimeState,
    BronzeOrbRuntimeState, ByrdRuntimeState, ChampRuntimeState, ChosenRuntimeState,
    CollectorRuntimeState, CorruptHeartRuntimeState, CultistRuntimeState, DarklingRuntimeState,
    DecaRuntimeState, DonuRuntimeState, ExploderRuntimeState, GiantHeadRuntimeState,
    GremlinLeaderRuntimeState, GremlinNobRuntimeState, GremlinWizardRuntimeState,
    GuardianRuntimeState, HexaghostRuntimeState, JawWormRuntimeState, LagavulinRuntimeState,
    LargeSlimeRuntimeState, LouseRuntimeState, MawRuntimeState, MonsterMoveState,
    NemesisRuntimeState, ReptomancerRuntimeState, SentryRuntimeState, ShelledParasiteRuntimeState,
    SlaverRedRuntimeState, SlimeBossRuntimeState, SnakeDaggerRuntimeState, SneckoRuntimeState,
    SphericGuardianRuntimeState, SpikerRuntimeState, SpireShieldRuntimeState,
    SpireSpearRuntimeState, ThiefRuntimeState, TimeEaterRuntimeState, TransientRuntimeState,
    WrithingMassRuntimeState,
};
use crate::runtime::monster_move::{MonsterMoveSpec, MoveStep};
use std::hash::{Hash, Hasher};

/// Exact monster identity without duplicating its derived turn plan.
///
/// `MonsterEntity::turn_plan()` is wholly determined by `move_state`,
/// `is_dying`, and `half_dead`, all of which are already part of this key.
/// Keeping a second owned plan cloned its steps and visible specification for
/// every transposition. Custom `Debug` and `Hash` still emit the former
/// derived field so durable identities remain unchanged.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CombatMonsterKey {
    pub(crate) entity_id: usize,
    pub(crate) monster_type: usize,
    pub(crate) current_hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) block: i32,
    pub(crate) slot: u8,
    pub(crate) logical_position: i32,
    pub(crate) is_dying: bool,
    pub(crate) is_escaped: bool,
    pub(crate) half_dead: bool,
    pub(crate) move_state: MonsterMoveState,
    pub(crate) runtime: CombatMonsterRuntimeKey,
}

impl std::fmt::Debug for CombatMonsterKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CombatMonsterKey")
            .field("entity_id", &self.entity_id)
            .field("monster_type", &self.monster_type)
            .field("current_hp", &self.current_hp)
            .field("max_hp", &self.max_hp)
            .field("block", &self.block)
            .field("slot", &self.slot)
            .field("logical_position", &self.logical_position)
            .field("is_dying", &self.is_dying)
            .field("is_escaped", &self.is_escaped)
            .field("half_dead", &self.half_dead)
            .field("move_state", &self.move_state)
            .field("turn_plan", &self.turn_plan_view())
            .field("runtime", &self.runtime)
            .finish()
    }
}

impl Hash for CombatMonsterKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity_id.hash(state);
        self.monster_type.hash(state);
        self.current_hp.hash(state);
        self.max_hp.hash(state);
        self.block.hash(state);
        self.slot.hash(state);
        self.logical_position.hash(state);
        self.is_dying.hash(state);
        self.is_escaped.hash(state);
        self.half_dead.hash(state);
        self.move_state.hash(state);
        self.turn_plan_view().hash(state);
        self.runtime.hash(state);
    }
}

impl CombatMonsterKey {
    fn turn_plan_view(&self) -> CombatMonsterTurnPlanView<'_> {
        CombatMonsterTurnPlanView {
            move_state: &self.move_state,
            inactive: self.is_dying || self.half_dead,
        }
    }
}

#[derive(Clone, Copy)]
struct CombatMonsterTurnPlanView<'a> {
    move_state: &'a MonsterMoveState,
    inactive: bool,
}

impl std::fmt::Debug for CombatMonsterTurnPlanView<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut plan = formatter.debug_struct("MonsterTurnPlan");
        plan.field("move_id", &self.move_state.planned_move_id);
        if self.inactive {
            let empty_steps: &[MoveStep] = &[];
            plan.field("steps", &empty_steps)
                .field("visible_spec", &Some(MonsterMoveSpec::Unknown));
        } else {
            let steps = self.move_state.planned_steps.as_deref().unwrap_or(&[]);
            plan.field("steps", &steps)
                .field("visible_spec", &self.move_state.planned_visible_spec);
        }
        plan.finish()
    }
}

impl Hash for CombatMonsterTurnPlanView<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.move_state.planned_move_id.hash(state);
        if self.inactive {
            let empty_steps: &[MoveStep] = &[];
            empty_steps.hash(state);
            Some(MonsterMoveSpec::Unknown).hash(state);
        } else {
            self.move_state
                .planned_steps
                .as_deref()
                .unwrap_or(&[])
                .hash(state);
            self.move_state.planned_visible_spec.hash(state);
        }
    }
}

#[cfg(test)]
mod turn_plan_view_tests {
    use super::*;
    use crate::runtime::monster_move::MonsterTurnPlan;
    use std::collections::hash_map::DefaultHasher;

    fn hash_value(value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn borrowed_turn_plan_view_preserves_owned_plan_debug_and_hash() {
        let move_state = MonsterMoveState {
            planned_move_id: 7,
            planned_steps: Some(smallvec::smallvec![MoveStep::Stun]),
            planned_visible_spec: Some(MonsterMoveSpec::Stun),
            ..MonsterMoveState::default()
        };
        let owned = MonsterTurnPlan {
            move_id: 7,
            steps: smallvec::smallvec![MoveStep::Stun],
            visible_spec: Some(MonsterMoveSpec::Stun),
        };
        let borrowed = CombatMonsterTurnPlanView {
            move_state: &move_state,
            inactive: false,
        };

        assert_eq!(format!("{borrowed:?}"), format!("{owned:?}"));
        assert_eq!(hash_value(&borrowed), hash_value(&owned));
    }

    #[test]
    fn inactive_turn_plan_view_preserves_owned_unknown_plan_debug_and_hash() {
        let move_state = MonsterMoveState {
            planned_move_id: 9,
            planned_steps: Some(smallvec::smallvec![MoveStep::Stun]),
            planned_visible_spec: Some(MonsterMoveSpec::Stun),
            ..MonsterMoveState::default()
        };
        let owned = MonsterTurnPlan::unknown(9);
        let borrowed = CombatMonsterTurnPlanView {
            move_state: &move_state,
            inactive: true,
        };

        assert_eq!(format!("{borrowed:?}"), format!("{owned:?}"));
        assert_eq!(hash_value(&borrowed), hash_value(&owned));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatMonsterRuntimeKey {
    None,
    Hexaghost(HexaghostRuntimeState),
    Louse(LouseRuntimeState),
    JawWorm(JawWormRuntimeState),
    Thief(ThiefRuntimeState),
    Byrd(ByrdRuntimeState),
    Chosen(ChosenRuntimeState),
    Snecko(SneckoRuntimeState),
    ShelledParasite(ShelledParasiteRuntimeState),
    BronzeAutomaton(BronzeAutomatonRuntimeState),
    BronzeOrb(BronzeOrbRuntimeState),
    BookOfStabbing(BookOfStabbingRuntimeState),
    Collector(CollectorRuntimeState),
    Champ(ChampRuntimeState),
    AwakenedOne(AwakenedOneRuntimeState),
    CorruptHeart(CorruptHeartRuntimeState),
    WrithingMass(WrithingMassRuntimeState),
    Spiker(SpikerRuntimeState),
    SpireShield(SpireShieldRuntimeState),
    SpireSpear(SpireSpearRuntimeState),
    SlaverRed(SlaverRedRuntimeState),
    GremlinLeader(GremlinLeaderRuntimeState),
    GremlinNob(GremlinNobRuntimeState),
    GremlinWizard(GremlinWizardRuntimeState),
    Cultist(CultistRuntimeState),
    Sentry(SentryRuntimeState),
    SlimeBoss(SlimeBossRuntimeState),
    LargeSlime(LargeSlimeRuntimeState),
    SphericGuardian(SphericGuardianRuntimeState),
    Reptomancer(ReptomancerRuntimeState),
    Darkling(DarklingRuntimeState),
    Nemesis(NemesisRuntimeState),
    GiantHead(GiantHeadRuntimeState),
    TimeEater(TimeEaterRuntimeState),
    Donu(DonuRuntimeState),
    Deca(DecaRuntimeState),
    Transient(TransientRuntimeState),
    Exploder(ExploderRuntimeState),
    Maw(MawRuntimeState),
    SnakeDagger(SnakeDaggerRuntimeState),
    Lagavulin(LagavulinRuntimeState),
    Guardian(GuardianRuntimeState),
    /// Preserves exactness for simulator extensions whose numeric monster id
    /// is not yet represented by `EnemyId`, without making every known
    /// monster carry all inactive runtime records inline.
    Unknown(Box<CombatMonsterRuntimeFallbackKey>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterRuntimeFallbackKey {
    pub(crate) hexaghost: HexaghostRuntimeState,
    pub(crate) louse: LouseRuntimeState,
    pub(crate) jaw_worm: JawWormRuntimeState,
    pub(crate) thief: ThiefRuntimeState,
    pub(crate) byrd: ByrdRuntimeState,
    pub(crate) chosen: ChosenRuntimeState,
    pub(crate) snecko: SneckoRuntimeState,
    pub(crate) shelled_parasite: ShelledParasiteRuntimeState,
    pub(crate) bronze_automaton: BronzeAutomatonRuntimeState,
    pub(crate) bronze_orb: BronzeOrbRuntimeState,
    pub(crate) book_of_stabbing: BookOfStabbingRuntimeState,
    pub(crate) collector: CollectorRuntimeState,
    pub(crate) champ: ChampRuntimeState,
    pub(crate) awakened_one: AwakenedOneRuntimeState,
    pub(crate) corrupt_heart: CorruptHeartRuntimeState,
    pub(crate) writhing_mass: WrithingMassRuntimeState,
    pub(crate) spiker: SpikerRuntimeState,
    pub(crate) spire_shield: SpireShieldRuntimeState,
    pub(crate) spire_spear: SpireSpearRuntimeState,
    pub(crate) slaver_red: SlaverRedRuntimeState,
    pub(crate) gremlin_leader: GremlinLeaderRuntimeState,
    pub(crate) gremlin_nob: GremlinNobRuntimeState,
    pub(crate) gremlin_wizard: GremlinWizardRuntimeState,
    pub(crate) cultist: CultistRuntimeState,
    pub(crate) sentry: SentryRuntimeState,
    pub(crate) slime_boss: SlimeBossRuntimeState,
    pub(crate) large_slime: LargeSlimeRuntimeState,
    pub(crate) spheric_guardian: SphericGuardianRuntimeState,
    pub(crate) reptomancer: ReptomancerRuntimeState,
    pub(crate) darkling: DarklingRuntimeState,
    pub(crate) nemesis: NemesisRuntimeState,
    pub(crate) giant_head: GiantHeadRuntimeState,
    pub(crate) time_eater: TimeEaterRuntimeState,
    pub(crate) donu: DonuRuntimeState,
    pub(crate) deca: DecaRuntimeState,
    pub(crate) transient: TransientRuntimeState,
    pub(crate) exploder: ExploderRuntimeState,
    pub(crate) maw: MawRuntimeState,
    pub(crate) snake_dagger: SnakeDaggerRuntimeState,
    pub(crate) lagavulin: LagavulinRuntimeState,
    pub(crate) guardian: GuardianRuntimeState,
}
