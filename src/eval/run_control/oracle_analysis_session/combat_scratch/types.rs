use serde::{Deserialize, Serialize};

use crate::content::potions::Potion;
use crate::content::relics::RelicState;
use crate::eval::fingerprint::StateFingerprintV2;
use crate::runtime::combat::{
    CombatCard, CombatPhase, EphemeralCounters, OrbEntity, Power, StanceId, ThiefRuntimeState,
};
use crate::runtime::monster_move::{MonsterMoveSpec, MonsterTurnSteps};
use crate::sim::combat::CombatTerminal;
use crate::sim::combat_action_surface::CombatSelectionActionFamilyV2;
use crate::state::core::ClientInput;

pub const ORACLE_ANALYSIS_COMBAT_SCRATCH_SCHEMA_NAME: &str = "OracleAnalysisCombatScratch";
pub const ORACLE_ANALYSIS_COMBAT_SCRATCH_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchNodeCheckpointV1 {
    pub scratch_node_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_scratch_node_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ClientInput>,
    pub exact_state_hash: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchCheckpointV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub run_node_id: usize,
    pub root_exact_state_hash: String,
    pub max_engine_steps_per_transition: usize,
    pub cursor_scratch_node_id: u64,
    pub next_scratch_node_id: u64,
    pub nodes: Vec<OracleAnalysisCombatScratchNodeCheckpointV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchContextV1 {
    pub act: u8,
    pub floor: i32,
    pub gold: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchCardV1 {
    #[serde(flatten)]
    pub card: CombatCard,
    pub effective_cost: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OracleAnalysisCombatScratchActionSelectorV1 {
    Atomic {
        scratch_node_id: u64,
        action_index: usize,
    },
    Selection {
        scratch_node_id: u64,
        family_index: usize,
        input_index: usize,
    },
    Card {
        scratch_node_id: u64,
        card_uuid: u32,
        target: Option<usize>,
    },
    Potion {
        scratch_node_id: u64,
        potion_uuid: u32,
        target: Option<usize>,
    },
    EndTurn {
        scratch_node_id: u64,
    },
}

impl OracleAnalysisCombatScratchActionSelectorV1 {
    pub(super) fn scratch_node_id(self) -> u64 {
        match self {
            Self::Atomic {
                scratch_node_id, ..
            }
            | Self::Selection {
                scratch_node_id, ..
            }
            | Self::Card {
                scratch_node_id, ..
            }
            | Self::Potion {
                scratch_node_id, ..
            }
            | Self::EndTurn { scratch_node_id } => scratch_node_id,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchActionV1 {
    pub selector: OracleAnalysisCombatScratchActionSelectorV1,
    pub action_ref: String,
    pub action_key: String,
    pub input: ClientInput,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchSelectionFamilyV1 {
    pub family_index: usize,
    pub family: CombatSelectionActionFamilyV2,
    pub total_input_count: usize,
    pub page_offset: usize,
    pub page_limit: usize,
    pub next_page_offset: Option<usize>,
    pub actions: Vec<OracleAnalysisCombatScratchActionV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchActionSurfaceV1 {
    pub exact_state_hash: String,
    pub atomic_actions: Vec<OracleAnalysisCombatScratchActionV1>,
    pub selection_families: Vec<OracleAnalysisCombatScratchSelectionFamilyV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchPlayerV1 {
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub energy: u8,
    pub stance: StanceId,
    pub orbs: Vec<OrbEntity>,
    pub relics: Vec<RelicState>,
    pub powers: Vec<Power>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchMonsterV1 {
    pub entity_id: usize,
    pub slot: u8,
    pub label: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub is_dying: bool,
    pub is_escaped: bool,
    pub half_dead: bool,
    pub planned_move_id: u8,
    pub planned_steps: MonsterTurnSteps,
    pub intent: Option<MonsterMoveSpec>,
    pub thief: ThiefRuntimeState,
    pub powers: Vec<Power>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchPositionV1 {
    pub fingerprint: StateFingerprintV2,
    pub terminal: CombatTerminal,
    pub turn: u32,
    pub phase: CombatPhase,
    pub counters: EphemeralCounters,
    pub player: OracleAnalysisCombatScratchPlayerV1,
    pub hand: Vec<OracleAnalysisCombatScratchCardV1>,
    pub draw_pile_top_first: Vec<OracleAnalysisCombatScratchCardV1>,
    pub discard_pile: Vec<OracleAnalysisCombatScratchCardV1>,
    pub exhaust_pile: Vec<OracleAnalysisCombatScratchCardV1>,
    pub limbo: Vec<OracleAnalysisCombatScratchCardV1>,
    pub potions: Vec<Option<Potion>>,
    pub monsters: Vec<OracleAnalysisCombatScratchMonsterV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchViewV1 {
    pub run_node_id: usize,
    pub context: OracleAnalysisCombatScratchContextV1,
    pub root_exact_state_hash: String,
    pub max_engine_steps_per_transition: usize,
    pub cursor_scratch_node_id: u64,
    pub scratch_node_count: usize,
    pub parent_scratch_node_id: Option<u64>,
    pub input_from_parent: Option<ClientInput>,
    pub position: OracleAnalysisCombatScratchPositionV1,
    pub legal_actions: OracleAnalysisCombatScratchActionSurfaceV1,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchDecisionActionV1 {
    pub index: usize,
    pub input: ClientInput,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchDecisionSelectionFamilyV1 {
    pub family_index: usize,
    pub family: CombatSelectionActionFamilyV2,
    pub total_input_count: usize,
    pub page_offset: usize,
    pub page_limit: usize,
    pub next_page_offset: Option<usize>,
    pub actions: Vec<OracleAnalysisCombatScratchDecisionActionV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchDecisionViewV1 {
    pub run_node_id: usize,
    pub context: OracleAnalysisCombatScratchContextV1,
    pub cursor_scratch_node_id: u64,
    pub scratch_node_count: usize,
    pub parent_scratch_node_id: Option<u64>,
    pub input_from_parent: Option<ClientInput>,
    pub terminal: CombatTerminal,
    pub turn: u32,
    pub phase: CombatPhase,
    pub counters: EphemeralCounters,
    pub player: OracleAnalysisCombatScratchPlayerV1,
    pub hand: Vec<OracleAnalysisCombatScratchCardV1>,
    pub draw_pile_top_first: Vec<OracleAnalysisCombatScratchCardV1>,
    pub discard_pile: Vec<OracleAnalysisCombatScratchCardV1>,
    pub exhaust_pile: Vec<OracleAnalysisCombatScratchCardV1>,
    pub potions: Vec<Option<Potion>>,
    pub monsters: Vec<OracleAnalysisCombatScratchMonsterV1>,
    pub atomic_actions: Vec<OracleAnalysisCombatScratchDecisionActionV1>,
    pub selection_families: Vec<OracleAnalysisCombatScratchDecisionSelectionFamilyV1>,
}

impl From<OracleAnalysisCombatScratchViewV1> for OracleAnalysisCombatScratchDecisionViewV1 {
    fn from(view: OracleAnalysisCombatScratchViewV1) -> Self {
        let position = view.position;
        Self {
            run_node_id: view.run_node_id,
            context: view.context,
            cursor_scratch_node_id: view.cursor_scratch_node_id,
            scratch_node_count: view.scratch_node_count,
            parent_scratch_node_id: view.parent_scratch_node_id,
            input_from_parent: view.input_from_parent,
            terminal: position.terminal,
            turn: position.turn,
            phase: position.phase,
            counters: position.counters,
            player: position.player,
            hand: position.hand,
            draw_pile_top_first: position.draw_pile_top_first,
            discard_pile: position.discard_pile,
            exhaust_pile: position.exhaust_pile,
            potions: position.potions,
            monsters: position.monsters,
            atomic_actions: view
                .legal_actions
                .atomic_actions
                .into_iter()
                .filter(|action| {
                    !matches!(
                        &action.input,
                        ClientInput::EndTurn
                            | ClientInput::PlayCard { .. }
                            | ClientInput::UsePotion { .. }
                    )
                })
                .map(|action| {
                    let OracleAnalysisCombatScratchActionSelectorV1::Atomic {
                        action_index, ..
                    } = action.selector
                    else {
                        unreachable!("atomic action carries an atomic selector")
                    };
                    OracleAnalysisCombatScratchDecisionActionV1 {
                        index: action_index,
                        input: action.input,
                    }
                })
                .collect(),
            selection_families: view
                .legal_actions
                .selection_families
                .into_iter()
                .map(
                    |family| OracleAnalysisCombatScratchDecisionSelectionFamilyV1 {
                        family_index: family.family_index,
                        family: family.family,
                        total_input_count: family.total_input_count,
                        page_offset: family.page_offset,
                        page_limit: family.page_limit,
                        next_page_offset: family.next_page_offset,
                        actions: family
                            .actions
                            .into_iter()
                            .map(|action| {
                                let OracleAnalysisCombatScratchActionSelectorV1::Selection {
                                    input_index,
                                    ..
                                } = action.selector
                                else {
                                    unreachable!("structured action carries a selection selector")
                                };
                                OracleAnalysisCombatScratchDecisionActionV1 {
                                    index: input_index,
                                    input: action.input,
                                }
                            })
                            .collect(),
                    },
                )
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchTreeNodeV1 {
    pub scratch_node_id: u64,
    pub parent_scratch_node_id: Option<u64>,
    pub is_cursor: bool,
    pub input_from_parent: Option<ClientInput>,
    pub action_key_from_parent: Option<String>,
    pub exact_state_hash: String,
    pub terminal: CombatTerminal,
    pub turn: u32,
    pub player_hp: i32,
    pub player_block: i32,
    pub enemy_hp_total: i32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchTreeV1 {
    pub run_node_id: usize,
    pub root_exact_state_hash: String,
    pub cursor_scratch_node_id: u64,
    pub nodes: Vec<OracleAnalysisCombatScratchTreeNodeV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchSearchRequestV1 {
    pub max_quanta: usize,
    pub quantum_nodes: usize,
    pub quantum_ms: u64,
    pub wall_ms: u64,
}

impl Default for OracleAnalysisCombatScratchSearchRequestV1 {
    fn default() -> Self {
        Self {
            max_quanta: 4,
            quantum_nodes: 1_024,
            quantum_ms: 100,
            wall_ms: 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleAnalysisCombatScratchSearchExitV1 {
    WitnessAdded,
    PortfolioCompleteWithoutWitness,
    AllowanceExhausted,
    DeadlineReached,
    QuantumLimitReached,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatScratchSearchReportV1 {
    pub run_node_id: usize,
    pub source_scratch_node_id: u64,
    pub search_root_exact_state_hash: String,
    pub exit: OracleAnalysisCombatScratchSearchExitV1,
    pub quanta_served: usize,
    pub elapsed_ms: u64,
    pub generation_work: u64,
    pub exact_states: usize,
    pub completed_turn_options: usize,
    pub max_player_turn: u32,
    pub last_status: Option<&'static str>,
    pub additional_potions_allowed: u32,
    pub appended_action_count: usize,
    pub first_appended_scratch_node_id: Option<u64>,
    pub terminal_scratch_node_id: Option<u64>,
}
