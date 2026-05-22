use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlayerEntity {
    pub id: EntityId,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    /// Java `AbstractPlayer.flipHorizontal`.
    ///
    /// This is a mechanical combat field in the Shield/Spear fight: while the
    /// player has Surrounded, Java uses the player's facing and monster drawX
    /// positions to decide which monster receives BackAttack.
    pub facing_left: bool,
    pub gold_delta_this_combat: i32,
    pub gold: i32,
    pub max_orbs: u8,
    pub orbs: Vec<OrbEntity>,
    pub stance: StanceId,
    pub relics: Vec<RelicState>,
    pub relic_buses: RelicBuses,
    /// Java: EnergyManager.energyMaster — base energy per turn.
    /// Starts at 3, boss relics with onEquip() { ++energyMaster } increment this.
    /// SlaversCollar conditionally adds +1 at battle start (handled separately).
    pub energy_master: u8,
}

impl PlayerEntity {
    pub fn has_relic(&self, id: crate::content::relics::RelicId) -> bool {
        self.relics.iter().any(|r| r.id == id)
    }

    pub fn add_relic(&mut self, state: RelicState) {
        let index = self.relics.len();
        let sub = crate::content::relics::get_relic_subscriptions(state.id);
        self.energy_master += crate::content::relics::energy_master_delta(state.id);

        self.relics.push(state);
        self.register_relic_subscriptions(index, sub);
    }

    fn register_relic_subscriptions(
        &mut self,
        index: usize,
        sub: crate::content::relics::RelicSubscriptions,
    ) {
        if sub.at_pre_battle {
            self.relic_buses.at_pre_battle.push(index);
        }
        if sub.at_battle_start_pre_draw {
            self.relic_buses.at_battle_start_pre_draw.push(index);
        }
        if sub.at_battle_start {
            self.relic_buses.at_battle_start.push(index);
        }
        if sub.at_turn_start {
            self.relic_buses.at_turn_start.push(index);
        }
        if sub.at_turn_start_post_draw {
            self.relic_buses.at_turn_start_post_draw.push(index);
        }
        if sub.on_use_card {
            self.relic_buses.on_use_card.push(index);
        }
        if sub.on_shuffle {
            self.relic_buses.on_shuffle.push(index);
        }
        if sub.on_exhaust {
            self.relic_buses.on_exhaust.push(index);
        }
        if sub.on_lose_hp {
            self.relic_buses.on_lose_hp.push(index);
        }
        if sub.on_victory {
            self.relic_buses.on_victory.push(index);
        }
        if sub.on_apply_power {
            self.relic_buses.on_apply_power.push(index);
        }
        if sub.on_monster_death {
            self.relic_buses.on_monster_death.push(index);
        }
        if sub.on_spawn_monster {
            self.relic_buses.on_spawn_monster.push(index);
        }
        if sub.at_end_of_turn {
            self.relic_buses.at_end_of_turn.push(index);
        }
        if sub.on_use_potion {
            self.relic_buses.on_use_potion.push(index);
        }
        if sub.on_discard {
            self.relic_buses.on_discard.push(index);
        }
        if sub.on_change_stance {
            self.relic_buses.on_change_stance.push(index);
        }
        if sub.on_attacked_to_change_damage {
            self.relic_buses.on_attacked_to_change_damage.push(index);
        }
        if sub.on_lose_hp_last {
            self.relic_buses.on_lose_hp_last.push(index);
        }

        if sub.on_calculate_heal {
            self.relic_buses.on_calculate_heal.push(index);
        }
        if sub.on_calculate_x_cost {
            self.relic_buses.on_calculate_x_cost.push(index);
        }
        if sub.on_calculate_block_retained {
            self.relic_buses.on_calculate_block_retained.push(index);
        }
        if sub.on_calculate_energy_retained {
            self.relic_buses.on_calculate_energy_retained.push(index);
        }
        if sub.on_scry {
            self.relic_buses.on_scry.push(index);
        }
        if sub.on_receive_power_modify {
            self.relic_buses.on_receive_power_modify.push(index);
        }
        if sub.on_calculate_vulnerable_multiplier {
            self.relic_buses
                .on_calculate_vulnerable_multiplier
                .push(index);
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Intent {
    Attack { damage: i32, hits: u8 },
    AttackBuff { damage: i32, hits: u8 },
    AttackDebuff { damage: i32, hits: u8 },
    AttackDefend { damage: i32, hits: u8 },
    Buff,
    Debuff,
    StrongDebuff,
    Debug,
    Defend,
    DefendDebuff,
    DefendBuff,
    Escape,
    Magic,
    None,
    Sleep,
    Stun,
    Unknown,
}

impl Intent {
    pub fn is_java_attack_intent(&self) -> bool {
        matches!(
            self,
            Intent::Attack { .. }
                | Intent::AttackBuff { .. }
                | Intent::AttackDebuff { .. }
                | Intent::AttackDefend { .. }
        )
    }

    pub fn base_damage(&self) -> Option<i32> {
        match self {
            Intent::Attack { damage, .. }
            | Intent::AttackBuff { damage, .. }
            | Intent::AttackDebuff { damage, .. }
            | Intent::AttackDefend { damage, .. } => Some(*damage),
            _ => None,
        }
    }

    pub fn hits(&self) -> i32 {
        match self {
            Intent::Attack { hits, .. }
            | Intent::AttackBuff { hits, .. }
            | Intent::AttackDebuff { hits, .. }
            | Intent::AttackDefend { hits, .. } => (*hits as i32).max(1),
            _ => 0,
        }
    }

    /// Legacy protocol/old-monster bridge only.
    ///
    /// Semantic main paths must not derive move truth from `Intent`.
    pub fn to_legacy_move_spec(&self) -> MonsterMoveSpec {
        match self {
            Intent::Attack { damage, hits } => MonsterMoveSpec::Attack(AttackSpec {
                base_damage: *damage,
                hits: *hits,
                damage_kind: DamageKind::Normal,
            }),
            Intent::AttackBuff { .. }
            | Intent::AttackDebuff { .. }
            | Intent::AttackDefend { .. }
            | Intent::Buff
            | Intent::Debuff
            | Intent::StrongDebuff
            | Intent::Defend
            | Intent::DefendDebuff
            | Intent::DefendBuff => MonsterMoveSpec::Unknown,
            Intent::Debug => MonsterMoveSpec::Debug,
            Intent::Escape => MonsterMoveSpec::Escape,
            Intent::Magic => MonsterMoveSpec::Magic,
            Intent::None => MonsterMoveSpec::None,
            Intent::Sleep => MonsterMoveSpec::Sleep,
            Intent::Stun => MonsterMoveSpec::Stun,
            Intent::Unknown => MonsterMoveSpec::Unknown,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MonsterEntity {
    pub id: EntityId,
    pub monster_type: MonsterId,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub slot: u8,
    pub is_dying: bool,
    pub is_escaped: bool,
    pub half_dead: bool,
    pub move_state: MonsterMoveState,
    pub logical_position: i32,
    pub hexaghost: HexaghostRuntimeState,
    pub louse: LouseRuntimeState,
    pub jaw_worm: JawWormRuntimeState,
    pub thief: ThiefRuntimeState,
    pub byrd: ByrdRuntimeState,
    pub chosen: ChosenRuntimeState,
    pub snecko: SneckoRuntimeState,
    pub shelled_parasite: ShelledParasiteRuntimeState,
    pub bronze_automaton: BronzeAutomatonRuntimeState,
    pub bronze_orb: BronzeOrbRuntimeState,
    pub book_of_stabbing: BookOfStabbingRuntimeState,
    pub collector: CollectorRuntimeState,
    pub champ: ChampRuntimeState,
    pub awakened_one: AwakenedOneRuntimeState,
    pub corrupt_heart: CorruptHeartRuntimeState,
    pub writhing_mass: WrithingMassRuntimeState,
    pub spiker: SpikerRuntimeState,
    pub spire_shield: SpireShieldRuntimeState,
    pub spire_spear: SpireSpearRuntimeState,
    pub slaver_red: SlaverRedRuntimeState,
    pub gremlin_leader: GremlinLeaderRuntimeState,
    pub gremlin_nob: GremlinNobRuntimeState,
    pub gremlin_wizard: GremlinWizardRuntimeState,
    pub cultist: CultistRuntimeState,
    pub sentry: SentryRuntimeState,
    pub slime_boss: SlimeBossRuntimeState,
    pub large_slime: LargeSlimeRuntimeState,
    pub spheric_guardian: SphericGuardianRuntimeState,
    pub reptomancer: ReptomancerRuntimeState,
    pub darkling: DarklingRuntimeState,
    pub nemesis: NemesisRuntimeState,
    pub giant_head: GiantHeadRuntimeState,
    pub time_eater: TimeEaterRuntimeState,
    pub donu: DonuRuntimeState,
    pub deca: DecaRuntimeState,
    pub transient: TransientRuntimeState,
    pub exploder: ExploderRuntimeState,
    pub maw: MawRuntimeState,
    pub snake_dagger: SnakeDaggerRuntimeState,
    pub lagavulin: LagavulinRuntimeState,
    pub guardian: GuardianRuntimeState,
}

impl MonsterEntity {
    pub fn is_dead_or_escaped(&self) -> bool {
        self.is_dying || self.half_dead || self.is_escaped
    }

    pub fn is_alive_for_action(&self) -> bool {
        self.current_hp > 0 && !self.is_dead_or_escaped()
    }

    /// Java `MonsterGroup.getRandomMonster(..., aliveOnly=true, cardRandomRng)`
    /// filters out half-dead, dying, and escaping monsters. It does not check
    /// `currentHealth`, because subsequent actions own their own cancellation.
    pub fn is_random_target_candidate(&self) -> bool {
        !self.half_dead && !self.is_dying && !self.is_escaped
    }

    pub fn turn_plan(&self) -> MonsterTurnPlan {
        let move_id = self.planned_move_id();
        if self.is_dying || self.half_dead {
            return MonsterTurnPlan::unknown(move_id);
        }

        MonsterTurnPlan {
            move_id,
            steps: self.move_state.planned_steps.clone().unwrap_or_default(),
            visible_spec: self.move_state.planned_visible_spec.clone(),
        }
    }

    pub fn planned_move_id(&self) -> u8 {
        self.move_state.planned_move_id
    }

    pub fn set_planned_move_id(&mut self, move_id: u8) {
        self.move_state.planned_move_id = move_id;
    }

    pub fn set_planned_steps(&mut self, steps: crate::runtime::monster_move::MonsterTurnSteps) {
        self.move_state.planned_steps = Some(steps);
    }

    pub fn set_planned_visible_spec(
        &mut self,
        visible_spec: Option<crate::runtime::monster_move::MonsterMoveSpec>,
    ) {
        self.move_state.planned_visible_spec = visible_spec;
    }

    pub fn record_planned_move(&mut self, move_id: u8) {
        self.move_state.planned_move_id = move_id;
        self.move_state.history.push_back(move_id);
    }

    pub fn move_history(&self) -> &VecDeque<u8> {
        &self.move_state.history
    }

    pub fn move_history_mut(&mut self) -> &mut VecDeque<u8> {
        &mut self.move_state.history
    }

    pub fn louse_bite_damage(&self) -> Option<i32> {
        self.louse.bite_damage
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct MonsterMoveState {
    pub planned_move_id: u8,
    pub history: VecDeque<u8>,
    pub planned_steps: Option<crate::runtime::monster_move::MonsterTurnSteps>,
    pub planned_visible_spec: Option<crate::runtime::monster_move::MonsterMoveSpec>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MonsterProtocolIdentity {
    pub instance_id: Option<u64>,
    pub spawn_order: Option<u64>,
    pub draw_x: Option<i32>,
    pub group_index: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MonsterProtocolObservationState {
    pub visible_intent: Intent,
    /// UI / protocol preview damage after monster damage modifiers are applied.
    /// This is not an executable damage base and must not be fed back into
    /// combat resolution.
    pub preview_damage_per_hit: i32,
}

impl Default for MonsterProtocolObservationState {
    fn default() -> Self {
        Self {
            visible_intent: Intent::Unknown,
            preview_damage_per_hit: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MonsterProtocolState {
    pub observation: MonsterProtocolObservationState,
    pub identity: MonsterProtocolIdentity,
}
