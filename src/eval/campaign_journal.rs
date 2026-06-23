use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ai::route_planner_v1::{
    MapDecisionPacketV1, MapRouteTargetV1, NeedVectorV1, NodeFeaturesV1,
    RouteCandidatePoolProvenanceV1, RouteEvaluationCalibrationStatusV1, RouteEvaluationSourceV1,
    RouteMapActionV1, RouteMoveCandidateV1, RoutePathSummaryV1, RouteProjectionCoverageV1,
    RouteProjectionSourceV1, RouteSafetyFlagV1, RouteScoreTermsV1, RouteValueFactorsV1,
};
use crate::eval::branch_experiment::{
    BranchExperimentBossRelicCandidateEntryV1, BranchExperimentCampfirePlanCandidateEntryV1,
    BranchExperimentEventCandidateEntryV1, BranchExperimentFirstEliteEvidenceV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
    BranchExperimentRouteCandidateEntryV1,
};

pub const CAMPAIGN_JOURNAL_SCHEMA_NAME: &str = "CampaignJournal";
pub const CAMPAIGN_JOURNAL_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalV1 {
    pub schema_name: String,
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_candidate_pools: Vec<CampaignJournalRouteCandidatePoolRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_paths: Vec<CampaignJournalBranchPathV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_path_nodes: Vec<CampaignJournalBranchPathNodeV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_branch_paths: Vec<CampaignJournalEventBranchPathRefV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<CampaignJournalEventV1>,
}

impl CampaignJournalV1 {
    pub fn new() -> Self {
        Self {
            schema_name: CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string(),
            schema_version: CAMPAIGN_JOURNAL_SCHEMA_VERSION,
            route_candidate_pools: Vec::new(),
            branch_paths: Vec::new(),
            branch_path_nodes: Vec::new(),
            event_branch_paths: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn extend(&mut self, events: impl IntoIterator<Item = CampaignJournalEventV1>) {
        self.events.extend(events);
        if self.schema_name.is_empty() {
            self.schema_name = CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string();
        }
        if self.schema_version == 0 {
            self.schema_version = CAMPAIGN_JOURNAL_SCHEMA_VERSION;
        }
    }

    pub fn compact_for_campaign_artifact_v1(&mut self) {
        self.compact_event_envelopes_for_campaign_artifact_v1();
        self.compact_candidates_for_campaign_artifact_v1();
        self.compact_branch_paths_for_campaign_artifact_v1();
        self.compact_route_candidate_pools_for_campaign_artifact_v1();
    }

    fn compact_event_envelopes_for_campaign_artifact_v1(&mut self) {
        for (event_index, event) in self.events.iter_mut().enumerate() {
            let old_event_id = campaign_journal_event_id_from_payload_v1(&event.payload);
            if let Some(decision_id) = event.payload.decision_id_mut_v1() {
                if !decision_id.is_empty() {
                    *decision_id = format!("d{event_index}");
                }
            }
            let new_event_id = campaign_journal_event_id_from_payload_v1(&event.payload);
            if event.event_id == old_event_id || event.event_id == new_event_id {
                event.event_id.clear();
            }
        }
    }

    fn compact_candidates_for_campaign_artifact_v1(&mut self) {
        for event in &mut self.events {
            event.payload.for_each_candidate_mut_v1(
                CampaignJournalCandidateV1::compact_for_campaign_artifact_v1,
            );
        }
    }

    fn compact_route_candidate_pools_for_campaign_artifact_v1(&mut self) {
        use std::collections::BTreeMap;

        let mut pool_indexes = BTreeMap::<String, String>::new();
        self.route_candidate_pools.clear();
        for event in &mut self.events {
            match &mut event.payload {
                CampaignJournalEventPayloadV1::RouteCandidatePool {
                    map_decision_packet,
                    route_candidates,
                    candidates,
                    candidate_pool_provenance,
                    route_candidate_pool_ref,
                    ..
                } => {
                    if let Some(packet) = map_decision_packet.take() {
                        if route_candidates.is_empty() {
                            *route_candidates = packet
                                .candidates
                                .iter()
                                .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                                .collect();
                        }
                    }
                    for candidate in route_candidates.iter_mut() {
                        candidate.compact_for_campaign_artifact_v1();
                    }
                    if !route_candidates.is_empty() {
                        let key = serde_json::to_string(route_candidates)
                            .unwrap_or_else(|_| format!("{route_candidates:?}"));
                        let pool_id = if let Some(pool_id) = pool_indexes.get(&key).cloned() {
                            pool_id
                        } else {
                            let pool_id = format!(
                                "route_candidate_pool:{}",
                                self.route_candidate_pools.len()
                            );
                            pool_indexes.insert(key, pool_id.clone());
                            self.route_candidate_pools.push(
                                CampaignJournalRouteCandidatePoolRecordV1 {
                                    pool_id: pool_id.clone(),
                                    route_candidates: route_candidates.clone(),
                                },
                            );
                            pool_id
                        };
                        *route_candidate_pool_ref = Some(pool_id);
                        route_candidates.clear();
                    }
                    *candidate_pool_provenance = None;
                    candidates.clear();
                }
                CampaignJournalEventPayloadV1::RouteDecision {
                    selected_route_candidate,
                    selected_target_node,
                    candidate_pool_provenance,
                    first_elite,
                    ..
                } => {
                    *selected_route_candidate = None;
                    *selected_target_node = None;
                    *candidate_pool_provenance = None;
                    *first_elite = BranchExperimentFirstEliteEvidenceV1::default();
                }
                _ => {}
            }
        }
    }

    pub fn hydrate_route_candidate_pools_v1(&mut self) {
        for event in &mut self.events {
            let CampaignJournalEventPayloadV1::RouteCandidatePool {
                route_candidate_pool_ref,
                route_candidates,
                ..
            } = &mut event.payload
            else {
                continue;
            };
            if !route_candidates.is_empty() {
                continue;
            }
            let Some(pool_id) = route_candidate_pool_ref.as_deref() else {
                continue;
            };
            if let Some(record) = self
                .route_candidate_pools
                .iter()
                .find(|record| record.pool_id == pool_id)
            {
                *route_candidates = record.route_candidates.clone();
            }
        }
    }

    pub fn hydrate_event_ids_v1(&mut self) {
        for event in &mut self.events {
            if event.event_id.is_empty() {
                event.event_id = campaign_journal_event_id_from_payload_v1(&event.payload);
            }
        }
    }

    fn compact_branch_paths_for_campaign_artifact_v1(&mut self) {
        use std::collections::BTreeMap;

        let mut path_ids = BTreeMap::<(Vec<String>, Vec<String>), String>::new();
        let mut node_indexes =
            BTreeMap::<(Option<String>, Option<String>, Option<String>), usize>::new();
        self.event_branch_paths.clear();

        for (event_index, event) in self.events.iter_mut().enumerate() {
            if event.branch_choices.is_empty() && event.branch_commands.is_empty() {
                continue;
            }
            let key = (event.branch_choices.clone(), event.branch_commands.clone());
            let branch_path_id = if let Some(branch_path_id) = path_ids.get(&key).cloned() {
                branch_path_id
            } else {
                let branch_path_id = campaign_journal_branch_path_node_id_v1(
                    &mut self.branch_path_nodes,
                    &mut node_indexes,
                    &key.0,
                    &key.1,
                );
                path_ids.insert(key, branch_path_id.clone());
                branch_path_id
            };
            self.event_branch_paths
                .push(CampaignJournalEventBranchPathRefV1 {
                    event_index,
                    branch_path_id,
                });
            event.branch_choices.clear();
            event.branch_commands.clear();
        }
        self.branch_paths.clear();
    }

    pub fn hydrate_branch_paths_v1(&mut self) {
        use std::collections::BTreeMap;

        let path_lookup = self
            .branch_paths
            .iter()
            .map(|path| {
                (
                    path.branch_path_id.as_str(),
                    (path.branch_choices.clone(), path.branch_commands.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let node_lookup = self
            .branch_path_nodes
            .iter()
            .map(|node| (node.branch_path_id.as_str(), node))
            .collect::<BTreeMap<_, _>>();
        let event_path_lookup = self
            .event_branch_paths
            .iter()
            .map(|event_path| (event_path.event_index, event_path.branch_path_id.as_str()))
            .collect::<BTreeMap<_, _>>();

        for (event_index, event) in self.events.iter_mut().enumerate() {
            if !event.branch_choices.is_empty() || !event.branch_commands.is_empty() {
                continue;
            }
            let Some(branch_path_id) = event_path_lookup.get(&event_index).copied() else {
                continue;
            };
            if let Some((choices, commands)) = path_lookup.get(branch_path_id) {
                event.branch_choices = choices.clone();
                event.branch_commands = commands.clone();
                continue;
            }
            if let Some((choices, commands)) =
                campaign_journal_hydrate_branch_path_node_v1(branch_path_id, &node_lookup)
            {
                event.branch_choices = choices;
                event.branch_commands = commands;
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CampaignJournalEventV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub event_id: String,
    #[serde(rename = "r", alias = "round")]
    pub round: usize,
    #[serde(
        default,
        rename = "b",
        alias = "branch_id",
        skip_serializing_if = "String::is_empty"
    )]
    pub branch_id: String,
    #[serde(rename = "i", alias = "branch_index")]
    pub branch_index: usize,
    #[serde(default, rename = "frontier", alias = "branch_frontier_title")]
    pub branch_frontier_title: String,
    #[serde(default, rename = "a", alias = "act")]
    pub act: u8,
    #[serde(default, rename = "f", alias = "floor")]
    pub floor: i32,
    #[serde(
        default,
        rename = "choices",
        alias = "branch_choices",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub branch_choices: Vec<String>,
    #[serde(
        default,
        rename = "commands",
        alias = "branch_commands",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub branch_commands: Vec<String>,
    #[serde(
        default,
        rename = "combat_retry",
        alias = "combat_budget_retry_used",
        skip_serializing_if = "is_false"
    )]
    pub combat_budget_retry_used: bool,
    #[serde(flatten)]
    pub payload: CampaignJournalEventPayloadV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalRouteCandidatePoolRecordV1 {
    pub pool_id: String,
    pub route_candidates: Vec<CampaignJournalRouteCandidateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalBranchPathV1 {
    #[serde(rename = "id", alias = "branch_path_id")]
    pub branch_path_id: String,
    #[serde(
        default,
        rename = "choices",
        alias = "branch_choices",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub branch_choices: Vec<String>,
    #[serde(
        default,
        rename = "commands",
        alias = "branch_commands",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub branch_commands: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignJournalEventBranchPathRefV1 {
    pub event_index: usize,
    pub branch_path_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignJournalBranchPathNodeV1 {
    pub branch_path_id: String,
    pub parent_branch_path_id: Option<String>,
    pub branch_choice: Option<String>,
    pub branch_command: Option<String>,
}

impl Serialize for CampaignJournalEventBranchPathRefV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(path_index) = campaign_journal_compact_path_id_index_v1(&self.branch_path_id) {
            return (self.event_index, path_index).serialize(serializer);
        }
        #[derive(Serialize)]
        struct LegacyRef<'a> {
            #[serde(rename = "e")]
            event_index: usize,
            #[serde(rename = "p")]
            branch_path_id: &'a str,
        }
        LegacyRef {
            event_index: self.event_index,
            branch_path_id: &self.branch_path_id,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CampaignJournalEventBranchPathRefV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Compact(usize, usize),
            Map {
                #[serde(alias = "event_index")]
                e: usize,
                #[serde(alias = "branch_path_id")]
                p: String,
            },
        }
        match Wire::deserialize(deserializer)? {
            Wire::Compact(event_index, path_index) => Ok(Self {
                event_index,
                branch_path_id: campaign_journal_compact_path_id_from_index_v1(path_index),
            }),
            Wire::Map { e, p } => Ok(Self {
                event_index: e,
                branch_path_id: p,
            }),
        }
    }
}

impl Serialize for CampaignJournalBranchPathNodeV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(path_index) = campaign_journal_compact_path_id_index_v1(&self.branch_path_id) {
            let parent_index = self
                .parent_branch_path_id
                .as_deref()
                .and_then(campaign_journal_compact_path_id_index_v1);
            return (
                path_index,
                parent_index,
                &self.branch_choice,
                &self.branch_command,
            )
                .serialize(serializer);
        }

        #[derive(Serialize)]
        struct LegacyNode<'a> {
            #[serde(rename = "id")]
            branch_path_id: &'a str,
            #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
            parent_branch_path_id: Option<&'a str>,
            #[serde(rename = "choice", skip_serializing_if = "Option::is_none")]
            branch_choice: Option<&'a str>,
            #[serde(rename = "cmd", skip_serializing_if = "Option::is_none")]
            branch_command: Option<&'a str>,
        }
        LegacyNode {
            branch_path_id: &self.branch_path_id,
            parent_branch_path_id: self.parent_branch_path_id.as_deref(),
            branch_choice: self.branch_choice.as_deref(),
            branch_command: self.branch_command.as_deref(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CampaignJournalBranchPathNodeV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Compact(usize, Option<usize>, Option<String>, Option<String>),
            Map {
                #[serde(alias = "branch_path_id")]
                id: String,
                #[serde(default, alias = "parent_branch_path_id")]
                p: Option<String>,
                #[serde(default, alias = "branch_choice")]
                choice: Option<String>,
                #[serde(default, alias = "branch_command")]
                cmd: Option<String>,
            },
        }
        match Wire::deserialize(deserializer)? {
            Wire::Compact(path_index, parent_index, branch_choice, branch_command) => Ok(Self {
                branch_path_id: campaign_journal_compact_path_id_from_index_v1(path_index),
                parent_branch_path_id: parent_index
                    .map(campaign_journal_compact_path_id_from_index_v1),
                branch_choice,
                branch_command,
            }),
            Wire::Map { id, p, choice, cmd } => Ok(Self {
                branch_path_id: id,
                parent_branch_path_id: p,
                branch_choice: choice,
                branch_command: cmd,
            }),
        }
    }
}

fn campaign_journal_compact_path_id_index_v1(path_id: &str) -> Option<usize> {
    path_id.strip_prefix('p')?.parse::<usize>().ok()
}

fn campaign_journal_compact_path_id_from_index_v1(path_index: usize) -> String {
    format!("p{path_index}")
}

fn campaign_journal_branch_path_node_id_v1(
    nodes: &mut Vec<CampaignJournalBranchPathNodeV1>,
    node_indexes: &mut std::collections::BTreeMap<
        (Option<String>, Option<String>, Option<String>),
        usize,
    >,
    branch_choices: &[String],
    branch_commands: &[String],
) -> String {
    let mut parent_id = None::<String>;
    let max_len = branch_choices.len().max(branch_commands.len());
    for index in 0..max_len {
        let choice = branch_choices.get(index).cloned();
        let command = branch_commands.get(index).cloned();
        let key = (parent_id.clone(), choice.clone(), command.clone());
        let node_index = if let Some(node_index) = node_indexes.get(&key).copied() {
            node_index
        } else {
            let node_index = nodes.len();
            node_indexes.insert(key, node_index);
            nodes.push(CampaignJournalBranchPathNodeV1 {
                branch_path_id: format!("p{node_index}"),
                parent_branch_path_id: parent_id.clone(),
                branch_choice: choice,
                branch_command: command,
            });
            node_index
        };
        parent_id = nodes
            .get(node_index)
            .map(|node| node.branch_path_id.clone());
    }
    parent_id.unwrap_or_else(|| "p:root".to_string())
}

fn campaign_journal_hydrate_branch_path_node_v1(
    branch_path_id: &str,
    node_lookup: &std::collections::BTreeMap<&str, &CampaignJournalBranchPathNodeV1>,
) -> Option<(Vec<String>, Vec<String>)> {
    let mut choices = Vec::<String>::new();
    let mut commands = Vec::<String>::new();
    let mut current_id = Some(branch_path_id);
    while let Some(branch_path_id) = current_id {
        let node = node_lookup.get(branch_path_id).copied()?;
        if let Some(choice) = node.branch_choice.clone() {
            choices.push(choice);
        }
        if let Some(command) = node.branch_command.clone() {
            commands.push(command);
        }
        current_id = node.parent_branch_path_id.as_deref();
    }
    choices.reverse();
    commands.reverse();
    Some((choices, commands))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "event_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CampaignJournalEventPayloadV1 {
    RewardCandidateSet {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        max_reward_options_per_branch: usize,
        original_count: usize,
        selected_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    ShopBranchCandidateSet {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    ShopCandidatePool {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        branch_frontier_count: usize,
        rollout_head_plan_id: Option<String>,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    CampfireCandidatePool {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        branch_option_count: usize,
        selected_plan_id: Option<String>,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    EventCandidatePool {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        game_event_id: String,
        candidate_count: usize,
        branch_option_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    BossRelicCandidatePool {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        branch_option_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    RouteCandidatePool {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        selected_index: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        candidate_pool_provenance: Option<RouteCandidatePoolProvenanceV1>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        map_decision_packet: Option<MapDecisionPacketV1>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        route_candidates: Vec<CampaignJournalRouteCandidateV1>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        route_candidate_pool_ref: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    RouteDecision {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        decision_id: String,
        route_branch_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_index: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_candidate_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_candidate_rank: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_target_node: Option<MapRouteTargetV1>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_route_candidate: Option<CampaignJournalRouteCandidateV1>,
        target: String,
        move_kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        safety_flag: Option<RouteSafetyFlagV1>,
        safety: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        candidate_pool_provenance: Option<RouteCandidatePoolProvenanceV1>,
        command: String,
        elite_prep_bp: i32,
        #[serde(
            default,
            skip_serializing_if = "branch_experiment_first_elite_evidence_is_default_v1"
        )]
        first_elite: BranchExperimentFirstEliteEvidenceV1,
    },
}

impl CampaignJournalEventPayloadV1 {
    fn decision_id_mut_v1(&mut self) -> Option<&mut String> {
        match self {
            Self::RewardCandidateSet { decision_id, .. }
            | Self::ShopBranchCandidateSet { decision_id, .. }
            | Self::ShopCandidatePool { decision_id, .. }
            | Self::CampfireCandidatePool { decision_id, .. }
            | Self::EventCandidatePool { decision_id, .. }
            | Self::BossRelicCandidatePool { decision_id, .. }
            | Self::RouteCandidatePool { decision_id, .. }
            | Self::RouteDecision { decision_id, .. } => Some(decision_id),
        }
    }

    fn for_each_candidate_mut_v1(
        &mut self,
        mut visit: impl FnMut(&mut CampaignJournalCandidateV1),
    ) {
        match self {
            Self::RewardCandidateSet { candidates, .. }
            | Self::ShopBranchCandidateSet { candidates, .. }
            | Self::ShopCandidatePool { candidates, .. }
            | Self::CampfireCandidatePool { candidates, .. }
            | Self::EventCandidatePool { candidates, .. }
            | Self::BossRelicCandidatePool { candidates, .. }
            | Self::RouteCandidatePool { candidates, .. } => {
                for candidate in candidates {
                    visit(candidate);
                }
            }
            Self::RouteDecision { .. } => {}
        }
    }
}

fn campaign_journal_event_id_from_payload_v1(payload: &CampaignJournalEventPayloadV1) -> String {
    match payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { decision_id, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { decision_id, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { decision_id, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { decision_id, .. } => {
            format!("{decision_id}:candidate_set")
        }
        CampaignJournalEventPayloadV1::RouteDecision { decision_id, .. } => {
            format!("{decision_id}:route")
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalCandidateV1 {
    #[serde(rename = "id", alias = "candidate_id")]
    pub candidate_id: String,
    #[serde(rename = "cmd", alias = "command")]
    pub command: String,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "class", alias = "semantic_class")]
    pub semantic_class: String,
    #[serde(
        default,
        rename = "adm",
        alias = "admission",
        skip_serializing_if = "CampaignJournalCandidateAdmissionTraceV1::is_unknown"
    )]
    pub admission: CampaignJournalCandidateAdmissionTraceV1,
    #[serde(rename = "disp", alias = "disposition")]
    pub disposition: CampaignJournalCandidateDispositionV1,
}

impl CampaignJournalCandidateV1 {
    pub fn compact_for_campaign_artifact_v1(&mut self) {
        self.semantic_class =
            compact_candidate_semantic_class_for_campaign_artifact_v1(&self.semantic_class);
        self.admission.compact_for_campaign_artifact_v1();
    }
}

fn compact_candidate_semantic_class_for_campaign_artifact_v1(semantic_class: &str) -> String {
    if let Some(rest) = semantic_class.strip_prefix("strategic_retention=") {
        let verdict = rest
            .split(":verdict:")
            .nth(1)
            .and_then(|tail| tail.split(":class:").next())
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown");
        let class = rest
            .split(":class:")
            .nth(1)
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown");
        return format!("retention:{verdict}:{class}");
    }

    let keep_prefixes = [
        "role:",
        "kind:",
        "lane:",
        "verdict:",
        "branch:",
        "effect:",
        "projection:",
        "strategy_tag:",
    ];
    let parts = semantic_class
        .split_whitespace()
        .filter(|part| keep_prefixes.iter().any(|prefix| part.starts_with(prefix)))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        semantic_class.to_string()
    } else {
        parts.join(" ")
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalRouteCandidateV1 {
    #[serde(rename = "id", alias = "candidate_id")]
    pub candidate_id: String,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub rank: usize,
    #[serde(
        default,
        rename = "sel",
        alias = "selected",
        skip_serializing_if = "is_false"
    )]
    pub selected: bool,
    #[serde(
        default,
        rename = "node",
        alias = "target_node",
        skip_serializing_if = "Option::is_none"
    )]
    pub target_node: Option<MapRouteTargetV1>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub room_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub move_kind: String,
    #[serde(
        default,
        rename = "act",
        alias = "action",
        skip_serializing_if = "Option::is_none"
    )]
    pub action: Option<RouteMapActionV1>,
    #[serde(
        default,
        rename = "safe",
        alias = "safety_flag",
        skip_serializing_if = "Option::is_none"
    )]
    pub safety_flag: Option<RouteSafetyFlagV1>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub safety: String,
    #[serde(default, rename = "score", skip_serializing_if = "is_zero_f32")]
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_terms: Option<RouteScoreTermsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_factors: Option<RouteValueFactorsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_source: Option<RouteEvaluationSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_calibration_status: Option<RouteEvaluationCalibrationStatusV1>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_features: Option<NodeFeaturesV1>,
    #[serde(
        default,
        rename = "facts",
        alias = "path_facts",
        skip_serializing_if = "Option::is_none"
    )]
    pub path_facts: Option<CampaignJournalRoutePathFactsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_summary: Option<RoutePathSummaryV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<NeedVectorV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_source: Option<RouteProjectionSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_coverage: Option<RouteProjectionCoverageV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_budget: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_path_count: Option<usize>,
    #[serde(
        default,
        rename = "elite_prep",
        alias = "elite_prep_bp",
        skip_serializing_if = "is_zero_i32"
    )]
    pub elite_prep_bp: i32,
    #[serde(
        default,
        skip_serializing_if = "branch_experiment_first_elite_evidence_is_default_v1"
    )]
    pub first_elite: BranchExperimentFirstEliteEvidenceV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cautions: Vec<String>,
}

impl CampaignJournalRouteCandidateV1 {
    pub fn compact_for_campaign_artifact_v1(&mut self) {
        self.score_terms = None;
        self.value_factors = None;
        self.evaluation_source = None;
        self.evaluation_calibration_status = None;
        self.node_features = None;
        if self.path_facts.is_none() {
            self.path_facts = self
                .path_summary
                .as_ref()
                .map(CampaignJournalRoutePathFactsV1::from_route_path_summary_v1);
        }
        self.path_summary = None;
        self.needs = None;
        self.projection_source = None;
        self.projection_coverage = None;
        self.path_budget = None;
        self.observed_path_count = None;
        self.first_elite = BranchExperimentFirstEliteEvidenceV1::default();
        self.reasons.clear();
        self.cautions.clear();
        if self.target_node.is_some() {
            self.target.clear();
            self.room_type.clear();
            self.move_kind.clear();
        }
        if self.action.is_some() {
            self.command.clear();
        }
        if self.safety_flag.is_some() {
            self.safety.clear();
        }
    }

    pub fn from_route_entry_v1(candidate: &BranchExperimentRouteCandidateEntryV1) -> Self {
        Self {
            candidate_id: candidate.candidate_id.clone(),
            rank: candidate.rank,
            selected: candidate.selected,
            target_node: candidate.target_node.clone(),
            target: candidate.target.clone(),
            room_type: candidate.room_type.clone(),
            move_kind: candidate.move_kind.clone(),
            action: candidate.action.clone(),
            safety_flag: candidate.safety_flag,
            safety: candidate.safety.clone(),
            score: candidate.score,
            score_terms: candidate.score_terms.clone(),
            value_factors: candidate.value_factors.clone(),
            evaluation_source: candidate.evaluation_source,
            evaluation_calibration_status: candidate.evaluation_calibration_status,
            command: candidate.command.clone(),
            node_features: candidate.node_features.clone(),
            path_facts: None,
            path_summary: candidate.path_summary.clone(),
            needs: candidate.needs.clone(),
            projection_source: candidate.projection_source,
            projection_coverage: candidate.projection_coverage,
            path_budget: candidate.path_budget,
            observed_path_count: candidate.observed_path_count,
            elite_prep_bp: candidate.elite_prep_bp,
            first_elite: candidate.first_elite.clone(),
            reasons: candidate.reasons.clone(),
            cautions: candidate.cautions.clone(),
        }
    }

    pub fn from_route_move_candidate_v1(candidate: &RouteMoveCandidateV1) -> Self {
        Self::from_route_move_candidate_with_selected_v1(candidate, false)
    }

    pub fn from_route_move_candidate_with_selected_v1(
        candidate: &RouteMoveCandidateV1,
        selected: bool,
    ) -> Self {
        let path = &candidate.projection.path_summary;
        Self {
            candidate_id: candidate.candidate_id.clone(),
            rank: candidate.rank,
            selected,
            target_node: Some(candidate.target.clone()),
            target: route_target_label_v1(&candidate.target),
            room_type: route_room_type_label_v1(candidate.target.room_type),
            move_kind: format!("{:?}", candidate.target.move_kind),
            action: Some(candidate.action.clone()),
            safety_flag: Some(candidate.evaluation.safety),
            safety: format!("{:?}", candidate.evaluation.safety),
            score: candidate.evaluation.total_score,
            score_terms: Some(candidate.evaluation.score_terms.clone()),
            value_factors: Some(candidate.evaluation.value_factors.clone()),
            evaluation_source: Some(candidate.evaluation.value_source),
            evaluation_calibration_status: Some(candidate.evaluation.calibration_status),
            command: candidate.command.clone(),
            node_features: Some(candidate.features.clone()),
            path_facts: None,
            path_summary: Some(path.clone()),
            needs: Some(candidate.needs.clone()),
            projection_source: Some(candidate.projection.metadata.source),
            projection_coverage: Some(candidate.projection.metadata.coverage),
            path_budget: Some(candidate.projection.metadata.path_budget),
            observed_path_count: Some(candidate.projection.metadata.observed_path_count),
            elite_prep_bp: route_score_to_basis_points_v1(
                candidate.evaluation.score_terms.elite_prep,
            ),
            first_elite: BranchExperimentFirstEliteEvidenceV1 {
                paths_with_first_elite: path.first_elite.paths_with_first_elite,
                forced: path.first_elite.forced,
                optional: path.first_elite.optional,
                min_hallway_fights_before: path.first_elite.min_hallway_fights_before,
                max_hallway_fights_before: path.first_elite.max_hallway_fights_before,
                min_unknowns_before: path.first_elite.min_unknowns_before,
                max_unknowns_before: path.first_elite.max_unknowns_before,
                min_fires_before: path.first_elite.min_fires_before,
                max_fires_before: path.first_elite.max_fires_before,
                min_shops_before: path.first_elite.min_shops_before,
                max_shops_before: path.first_elite.max_shops_before,
                can_bail_to_rest_before: path.first_elite.can_bail_to_rest_before,
                can_bail_to_shop_before: path.first_elite.can_bail_to_shop_before,
            },
            reasons: candidate.evaluation.legacy_reasons.clone(),
            cautions: candidate.evaluation.legacy_cautions.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalRoutePathFactsV1 {
    #[serde(
        default,
        rename = "pc",
        alias = "path_count",
        skip_serializing_if = "is_zero_usize"
    )]
    pub path_count: usize,
    #[serde(
        default,
        rename = "cap",
        alias = "path_budget_exhausted",
        skip_serializing_if = "is_false"
    )]
    pub path_budget_exhausted: bool,
    #[serde(
        default,
        rename = "ep_min",
        alias = "min_early_pressure",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_early_pressure: usize,
    #[serde(
        default,
        rename = "ep_max",
        alias = "max_early_pressure",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_early_pressure: usize,
    #[serde(
        default,
        rename = "e_min",
        alias = "min_elites",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_elites: usize,
    #[serde(
        default,
        rename = "e_max",
        alias = "max_elites",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_elites: usize,
    #[serde(
        default,
        rename = "shop_min",
        alias = "min_shops",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_shops: usize,
    #[serde(
        default,
        rename = "shop_max",
        alias = "max_shops",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_shops: usize,
    #[serde(
        default,
        rename = "fire_min",
        alias = "min_fires",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_fires: usize,
    #[serde(
        default,
        rename = "fire_max",
        alias = "max_fires",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_fires: usize,
    #[serde(
        default,
        rename = "q_min",
        alias = "min_unknowns",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_unknowns: usize,
    #[serde(
        default,
        rename = "q_max",
        alias = "max_unknowns",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_unknowns: usize,
    #[serde(
        default,
        rename = "t_min",
        alias = "min_treasures",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_treasures: usize,
    #[serde(
        default,
        rename = "t_max",
        alias = "max_treasures",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_treasures: usize,
    #[serde(
        default,
        rename = "shop_floor",
        alias = "first_shop_floor",
        skip_serializing_if = "Option::is_none"
    )]
    pub first_shop_floor: Option<i32>,
    #[serde(
        default,
        rename = "fire_floor",
        alias = "first_fire_floor",
        skip_serializing_if = "Option::is_none"
    )]
    pub first_fire_floor: Option<i32>,
    #[serde(
        default,
        rename = "dmg_before_rec_min",
        alias = "min_damage_rooms_before_recovery",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_damage_rooms_before_recovery: usize,
    #[serde(
        default,
        rename = "dmg_before_rec_max",
        alias = "max_damage_rooms_before_recovery",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_damage_rooms_before_recovery: usize,
    #[serde(
        default,
        rename = "q_before_rec_min",
        alias = "min_unknowns_before_recovery",
        skip_serializing_if = "is_zero_usize"
    )]
    pub min_unknowns_before_recovery: usize,
    #[serde(
        default,
        rename = "q_before_rec_max",
        alias = "max_unknowns_before_recovery",
        skip_serializing_if = "is_zero_usize"
    )]
    pub max_unknowns_before_recovery: usize,
    #[serde(
        default,
        rename = "rec_before_dmg",
        alias = "paths_with_recovery_before_damage",
        skip_serializing_if = "is_zero_usize"
    )]
    pub paths_with_recovery_before_damage: usize,
}

impl CampaignJournalRoutePathFactsV1 {
    pub fn from_route_path_summary_v1(path: &RoutePathSummaryV1) -> Self {
        Self {
            path_count: path.path_count,
            path_budget_exhausted: path.path_budget_exhausted,
            min_early_pressure: path.min_early_pressure,
            max_early_pressure: path.max_early_pressure,
            min_elites: path.min_elites,
            max_elites: path.max_elites,
            min_shops: path.min_shops,
            max_shops: path.max_shops,
            min_fires: path.min_fires,
            max_fires: path.max_fires,
            min_unknowns: path.min_unknowns,
            max_unknowns: path.max_unknowns,
            min_treasures: path.min_treasures,
            max_treasures: path.max_treasures,
            first_shop_floor: path.first_shop_floor,
            first_fire_floor: path.first_fire_floor,
            min_damage_rooms_before_recovery: path.min_damage_rooms_before_recovery,
            max_damage_rooms_before_recovery: path.max_damage_rooms_before_recovery,
            min_unknowns_before_recovery: path.min_unknowns_before_recovery,
            max_unknowns_before_recovery: path.max_unknowns_before_recovery,
            paths_with_recovery_before_damage: path.paths_with_recovery_before_damage,
        }
    }
}

fn route_target_label_v1(target: &MapRouteTargetV1) -> String {
    format!(
        "x={} y={} {}",
        target.x,
        target.y,
        route_room_type_label_v1(target.room_type)
    )
}

fn route_room_type_label_v1(room_type: Option<crate::state::map::node::RoomType>) -> String {
    match room_type {
        Some(crate::state::map::node::RoomType::EventRoom) => "Event",
        Some(crate::state::map::node::RoomType::MonsterRoom) => "Monster",
        Some(crate::state::map::node::RoomType::MonsterRoomElite) => "Elite",
        Some(crate::state::map::node::RoomType::MonsterRoomBoss) => "Boss",
        Some(crate::state::map::node::RoomType::RestRoom) => "Rest",
        Some(crate::state::map::node::RoomType::ShopRoom) => "Shop",
        Some(crate::state::map::node::RoomType::TreasureRoom) => "Treasure",
        Some(crate::state::map::node::RoomType::TrueVictoryRoom) => "Victory",
        None => "Unknown",
    }
    .to_string()
}

fn route_score_to_basis_points_v1(score: f32) -> i32 {
    (score * 100.0).round() as i32
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalCandidateAdmissionTraceV1 {
    #[serde(rename = "s", alias = "status")]
    pub status: CampaignJournalCandidateAdmissionStatusV1,
    #[serde(
        default,
        rename = "cat",
        alias = "reason_category",
        skip_serializing_if = "CampaignJournalCandidateAdmissionReasonCategoryV1::is_unknown"
    )]
    pub reason_category: CampaignJournalCandidateAdmissionReasonCategoryV1,
    #[serde(
        default,
        rename = "code",
        alias = "reason_code",
        skip_serializing_if = "CampaignJournalCandidateAdmissionReasonCodeV1::is_unknown"
    )]
    pub reason_code: CampaignJournalCandidateAdmissionReasonCodeV1,
    #[serde(
        default,
        rename = "src",
        alias = "source",
        skip_serializing_if = "String::is_empty"
    )]
    pub source: String,
    #[serde(default, rename = "reason", skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, rename = "lane", skip_serializing_if = "String::is_empty")]
    pub lane: String,
    #[serde(
        default,
        rename = "reps",
        alias = "representative_count",
        skip_serializing_if = "is_zero_usize"
    )]
    pub representative_count: usize,
    #[serde(
        default,
        rename = "suppressed",
        alias = "suppressed_count",
        skip_serializing_if = "is_zero_usize"
    )]
    pub suppressed_count: usize,
}

impl CampaignJournalCandidateAdmissionTraceV1 {
    pub fn new(
        status: CampaignJournalCandidateAdmissionStatusV1,
        source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let source = source.into();
        let reason = reason.into();
        Self {
            status,
            reason_category: admission_reason_category_from_source_v1(&source),
            reason_code: admission_reason_code_from_text_v1(&reason),
            source,
            reason,
            lane: String::new(),
            representative_count: 0,
            suppressed_count: 0,
        }
    }

    pub fn from_disposition(
        disposition: CampaignJournalCandidateDispositionV1,
        source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let status = match disposition {
            CampaignJournalCandidateDispositionV1::Kept => {
                CampaignJournalCandidateAdmissionStatusV1::Scheduled
            }
            CampaignJournalCandidateDispositionV1::Pruned => {
                CampaignJournalCandidateAdmissionStatusV1::Deferred
            }
        };
        Self::new(status, source, reason)
    }

    pub fn with_lane(mut self, lane: impl Into<String>) -> Self {
        self.lane = lane.into();
        self
    }

    pub fn with_counts(mut self, representative_count: usize, suppressed_count: usize) -> Self {
        self.representative_count = representative_count;
        self.suppressed_count = suppressed_count;
        self
    }

    pub fn compact_for_campaign_artifact_v1(&mut self) {
        if self.reason_category == CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown {
            self.reason_category = admission_reason_category_from_source_v1(&self.source);
        }
        if self.reason_code == CampaignJournalCandidateAdmissionReasonCodeV1::Unknown {
            self.reason_code = admission_reason_code_from_text_v1(&self.reason);
        }
        self.source.clear();
        self.reason.clear();
    }

    pub fn normalized_reason_category(&self) -> CampaignJournalCandidateAdmissionReasonCategoryV1 {
        if self.reason_category != CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown {
            return self.reason_category;
        }
        admission_reason_category_from_source_v1(&self.source)
    }

    pub fn normalized_reason_code(&self) -> CampaignJournalCandidateAdmissionReasonCodeV1 {
        if self.reason_code != CampaignJournalCandidateAdmissionReasonCodeV1::Unknown {
            return self.reason_code;
        }
        admission_reason_code_from_text_v1(&self.reason)
    }

    pub fn is_unknown(&self) -> bool {
        self.status == CampaignJournalCandidateAdmissionStatusV1::Unknown
            && self.reason_category == CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown
            && self.reason_code == CampaignJournalCandidateAdmissionReasonCodeV1::Unknown
            && self.source.is_empty()
            && self.reason.is_empty()
            && self.lane.is_empty()
            && self.representative_count == 0
            && self.suppressed_count == 0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionReasonCategoryV1 {
    Unknown,
    LegacyDisposition,
    RetentionBucket,
    BranchAdmission,
    EventBoundary,
}

impl Default for CampaignJournalCandidateAdmissionReasonCategoryV1 {
    fn default() -> Self {
        Self::Unknown
    }
}

impl CampaignJournalCandidateAdmissionReasonCategoryV1 {
    pub fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::LegacyDisposition => "legacy_disposition",
            Self::RetentionBucket => "retention_bucket",
            Self::BranchAdmission => "branch_admission",
            Self::EventBoundary => "event_boundary",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionReasonCodeV1 {
    Unknown,
    Admit,
    Blocked,
    CurrentEventBoundaryCandidate,
    Deferred,
    Kept,
    Pruned,
    Reject,
    Scheduled,
    Selected,
}

impl Default for CampaignJournalCandidateAdmissionReasonCodeV1 {
    fn default() -> Self {
        Self::Unknown
    }
}

impl CampaignJournalCandidateAdmissionReasonCodeV1 {
    pub fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Admit => "admit",
            Self::Blocked => "blocked",
            Self::CurrentEventBoundaryCandidate => "current_event_boundary_candidate",
            Self::Deferred => "deferred",
            Self::Kept => "kept",
            Self::Pruned => "pruned",
            Self::Reject => "reject",
            Self::Scheduled => "scheduled",
            Self::Selected => "selected",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionStatusV1 {
    Unknown,
    Scheduled,
    Deferred,
    Rejected,
}

impl Default for CampaignJournalCandidateAdmissionStatusV1 {
    fn default() -> Self {
        Self::Unknown
    }
}

pub fn campaign_journal_candidate_from_campfire_entry_v1(
    candidate: &BranchExperimentCampfirePlanCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    let disposition = if candidate.branch_admission == "selected" {
        CampaignJournalCandidateDispositionV1::Kept
    } else {
        CampaignJournalCandidateDispositionV1::Pruned
    };
    CampaignJournalCandidateV1 {
        candidate_id: candidate.plan_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: campfire_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            campaign_journal_status_from_branch_admission_v1(&candidate.branch_admission),
            "campfire_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.role.clone())
        .with_counts(candidate.representative_count, candidate.suppressed_count),
        disposition,
    }
}

pub fn campaign_journal_candidate_from_event_entry_v1(
    candidate: &BranchExperimentEventCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    let disposition = if candidate.branch_admission == "selected" {
        CampaignJournalCandidateDispositionV1::Kept
    } else {
        CampaignJournalCandidateDispositionV1::Pruned
    };
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: event_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            campaign_journal_status_from_branch_admission_v1(&candidate.branch_admission),
            "event_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.effect_kind.clone())
        .with_counts(candidate.representative_count, candidate.suppressed_count),
        disposition,
    }
}

fn event_candidate_semantic_class_v1(candidate: &BranchExperimentEventCandidateEntryV1) -> String {
    let mut parts = vec![
        format!("effect:{}", candidate.effect_kind),
        format!("branch:{}", candidate.branch_admission),
        format!("representatives:{}", candidate.representative_count),
    ];
    if let Some(class) = &candidate.event_policy_class {
        parts.push(format!("class:{class}"));
    }
    if let Some(tier) = &candidate.event_policy_tier {
        parts.push(format!("tier:{tier}"));
    }
    if let Some(score) = candidate.event_policy_score {
        parts.push(format!("score:{score}"));
    }
    if candidate.suppressed_count > 0 {
        parts.push(format!("suppressed:{}", candidate.suppressed_count));
    }
    parts.join(" ")
}

pub fn campaign_journal_candidate_from_boss_relic_entry_v1(
    candidate: &BranchExperimentBossRelicCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: boss_relic_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            campaign_journal_status_from_branch_admission_v1(&candidate.branch_admission),
            "boss_relic_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.class.clone()),
        disposition: CampaignJournalCandidateDispositionV1::Kept,
    }
}

fn boss_relic_candidate_semantic_class_v1(
    candidate: &BranchExperimentBossRelicCandidateEntryV1,
) -> String {
    let mut parts = vec![
        format!("relic:{}", candidate.relic),
        format!("class:{}", candidate.class),
        format!("support:{}", candidate.support_gate),
        format!("branch:{}", candidate.branch_admission),
    ];
    if !candidate.added_debt.is_empty() {
        parts.push(format!("debt:{}", candidate.added_debt.join("+")));
    }
    if !candidate.compounding_tags.is_empty() {
        parts.push(format!(
            "compounds:{}",
            candidate.compounding_tags.join("+")
        ));
    }
    parts.join(" ")
}

pub fn campaign_journal_candidate_from_route_entry_v1(
    candidate: &BranchExperimentRouteCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    let (status, reason, disposition) = if candidate.selected {
        (
            CampaignJournalCandidateAdmissionStatusV1::Scheduled,
            "selected",
            CampaignJournalCandidateDispositionV1::Kept,
        )
    } else if candidate.resolved_safety_flag() == RouteSafetyFlagV1::RejectUnlessNoAlternative {
        (
            CampaignJournalCandidateAdmissionStatusV1::Rejected,
            "rejected",
            CampaignJournalCandidateDispositionV1::Pruned,
        )
    } else {
        (
            CampaignJournalCandidateAdmissionStatusV1::Deferred,
            "deferred",
            CampaignJournalCandidateDispositionV1::Pruned,
        )
    };
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.target.clone(),
        semantic_class: route_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            status,
            "route_candidate_pool",
            reason,
        )
        .with_lane(candidate.room_type.clone()),
        disposition,
    }
}

fn route_candidate_semantic_class_v1(candidate: &BranchExperimentRouteCandidateEntryV1) -> String {
    let mut parts = vec![
        format!("room:{}", candidate.room_type),
        format!("move:{}", candidate.move_kind),
        format!("safety:{}", candidate.safety),
        format!("rank:{}", candidate.rank),
        format!("score:{}", candidate.score),
        format!("elite_prep_bp:{}", candidate.elite_prep_bp),
    ];
    if candidate.selected {
        parts.push("selected:true".to_string());
    }
    if !candidate.reasons.is_empty() {
        parts.push(format!("reasons:{}", candidate.reasons.join("+")));
    }
    if !candidate.cautions.is_empty() {
        parts.push(format!("cautions:{}", candidate.cautions.join("+")));
    }
    parts.join(" ")
}

fn campfire_candidate_semantic_class_v1(
    candidate: &BranchExperimentCampfirePlanCandidateEntryV1,
) -> String {
    let mut parts = vec![
        format!("role:{}", candidate.role),
        format!("effect:{}", candidate.effect_kind),
        format!("score_hint:{}", candidate.score_hint),
        format!("confidence_milli:{}", candidate.confidence_milli),
        format!("execute:{}", candidate.execute_autopilot),
        format!("branch_active:{}", candidate.branch_active),
        format!("branch:{}", candidate.branch_admission),
        format!("representatives:{}", candidate.representative_count),
    ];
    if let Some(tag) = &candidate.strategy_tag {
        parts.push(format!("strategy_tag:{tag}"));
    }
    if candidate.suppressed_count > 0 {
        parts.push(format!("suppressed:{}", candidate.suppressed_count));
    }
    parts.join(" ")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateDispositionV1 {
    Kept,
    Pruned,
}

fn campaign_journal_status_from_branch_admission_v1(
    admission: &str,
) -> CampaignJournalCandidateAdmissionStatusV1 {
    match admission.to_ascii_lowercase().as_str() {
        "admit" | "selected" | "scheduled" | "kept" => {
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        }
        "reject" | "rejected" | "blocked" | "block" => {
            CampaignJournalCandidateAdmissionStatusV1::Rejected
        }
        _ => CampaignJournalCandidateAdmissionStatusV1::Deferred,
    }
}

fn admission_reason_category_from_source_v1(
    source: &str,
) -> CampaignJournalCandidateAdmissionReasonCategoryV1 {
    match source {
        "legacy_disposition" => {
            CampaignJournalCandidateAdmissionReasonCategoryV1::LegacyDisposition
        }
        "reward_portfolio" => CampaignJournalCandidateAdmissionReasonCategoryV1::RetentionBucket,
        "event_boundary_packet" => CampaignJournalCandidateAdmissionReasonCategoryV1::EventBoundary,
        source if source.ends_with("_candidate_pool") => {
            CampaignJournalCandidateAdmissionReasonCategoryV1::BranchAdmission
        }
        _ => CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown,
    }
}

fn admission_reason_code_from_text_v1(
    reason: &str,
) -> CampaignJournalCandidateAdmissionReasonCodeV1 {
    match reason.to_ascii_lowercase().as_str() {
        "admit" => CampaignJournalCandidateAdmissionReasonCodeV1::Admit,
        "blocked" | "block" => CampaignJournalCandidateAdmissionReasonCodeV1::Blocked,
        "current_event_boundary_candidate" => {
            CampaignJournalCandidateAdmissionReasonCodeV1::CurrentEventBoundaryCandidate
        }
        "deferred" | "defer" => CampaignJournalCandidateAdmissionReasonCodeV1::Deferred,
        "kept" | "keep" => CampaignJournalCandidateAdmissionReasonCodeV1::Kept,
        "pruned" | "prune" => CampaignJournalCandidateAdmissionReasonCodeV1::Pruned,
        "reject" | "rejected" => CampaignJournalCandidateAdmissionReasonCodeV1::Reject,
        "scheduled" => CampaignJournalCandidateAdmissionReasonCodeV1::Scheduled,
        "selected" => CampaignJournalCandidateAdmissionReasonCodeV1::Selected,
        _ => CampaignJournalCandidateAdmissionReasonCodeV1::Unknown,
    }
}

pub fn reward_portfolio_from_journal_event_v1(
    event: &CampaignJournalEventV1,
) -> Option<BranchExperimentRewardOptionPortfolioV1> {
    let CampaignJournalEventPayloadV1::RewardCandidateSet {
        boundary_title,
        frontier_key,
        depth,
        max_reward_options_per_branch,
        original_count,
        selected_count,
        candidates,
        ..
    } = &event.payload
    else {
        return None;
    };

    let mut selected_options = Vec::new();
    let mut pruned_options = Vec::new();
    for candidate in candidates {
        let entry = BranchExperimentRewardOptionPortfolioEntryV1 {
            command: candidate.command.clone(),
            label: candidate.label.clone(),
            semantic_class: candidate.semantic_class.clone(),
        };
        match candidate.disposition {
            CampaignJournalCandidateDispositionV1::Kept => selected_options.push(entry),
            CampaignJournalCandidateDispositionV1::Pruned => pruned_options.push(entry),
        }
    }

    Some(BranchExperimentRewardOptionPortfolioV1 {
        branch_id: event.branch_id.clone(),
        branch_choices: event.branch_choices.clone(),
        branch_commands: event.branch_commands.clone(),
        depth: *depth,
        frontier_key: frontier_key.clone(),
        boundary_title: boundary_title.clone(),
        max_reward_options_per_branch: *max_reward_options_per_branch,
        original_count: *original_count,
        selected_count: *selected_count,
        selected_options,
        pruned_options,
    })
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

fn is_zero_f32(value: &f32) -> bool {
    *value == 0.0
}

fn branch_experiment_first_elite_evidence_is_default_v1(
    value: &BranchExperimentFirstEliteEvidenceV1,
) -> bool {
    value == &BranchExperimentFirstEliteEvidenceV1::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_candidates_record_structured_admission_trace() {
        let candidate = campaign_journal_candidate_from_event_entry_v1(
            &BranchExperimentEventCandidateEntryV1 {
                candidate_id: "event:0".to_string(),
                command: "event 0".to_string(),
                label: "Take event option".to_string(),
                event_index: Some(0),
                effect_kind: "gain_relic".to_string(),
                effect_key: "golden_idol".to_string(),
                event_policy_class: Some("valuable_event".to_string()),
                event_policy_tier: Some("strong".to_string()),
                event_policy_score: Some(100),
                branch_admission: "selected".to_string(),
                representative_count: 2,
                suppressed_count: 1,
                reasons: vec!["event policy kept representative".to_string()],
            },
        );

        assert_eq!(
            candidate.admission.status,
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        );
        assert_eq!(candidate.admission.source, "event_candidate_pool");
        assert_eq!(candidate.admission.reason, "selected");
        assert_eq!(
            candidate.admission.reason_category,
            CampaignJournalCandidateAdmissionReasonCategoryV1::BranchAdmission
        );
        assert_eq!(
            candidate.admission.reason_code,
            CampaignJournalCandidateAdmissionReasonCodeV1::Selected
        );
        assert_eq!(candidate.admission.representative_count, 2);
        assert_eq!(candidate.admission.suppressed_count, 1);
    }

    #[test]
    fn old_admission_trace_normalizes_reason_from_source_text() {
        let admission: CampaignJournalCandidateAdmissionTraceV1 = serde_json::from_str(
            r#"{"status":"scheduled","source":"reward_portfolio","reason":"kept"}"#,
        )
        .expect("old admission trace should deserialize");

        assert_eq!(
            admission.reason_category,
            CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown
        );
        assert_eq!(
            admission.normalized_reason_category(),
            CampaignJournalCandidateAdmissionReasonCategoryV1::RetentionBucket
        );
        assert_eq!(
            admission.normalized_reason_code(),
            CampaignJournalCandidateAdmissionReasonCodeV1::Kept
        );
    }

    #[test]
    fn route_candidates_record_structured_admission_trace() {
        let candidate = campaign_journal_candidate_from_route_entry_v1(
            &crate::eval::branch_experiment::BranchExperimentRouteCandidateEntryV1 {
                candidate_id: "route_move:normal_edge:x1:y1".to_string(),
                rank: 0,
                selected: true,
                target_node: None,
                target: "x=1 y=1 Monster".to_string(),
                room_type: "Monster".to_string(),
                move_kind: "NormalEdge".to_string(),
                action: None,
                safety_flag: None,
                safety: "ok".to_string(),
                score: 1.25,
                score_terms: None,
                value_factors: None,
                evaluation_source: None,
                evaluation_calibration_status: None,
                command: "go 1".to_string(),
                node_features: None,
                path_summary: None,
                needs: None,
                projection_source: None,
                projection_coverage: None,
                path_budget: None,
                observed_path_count: None,
                elite_prep_bp: 42,
                first_elite: BranchExperimentFirstEliteEvidenceV1::default(),
                reasons: vec!["route planner selected".to_string()],
                cautions: Vec::new(),
            },
        );

        assert_eq!(candidate.candidate_id, "route_move:normal_edge:x1:y1");
        assert_eq!(candidate.command, "go 1");
        assert_eq!(
            candidate.admission.status,
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        );
        assert_eq!(candidate.admission.source, "route_candidate_pool");
        assert_eq!(
            candidate.admission.normalized_reason_category(),
            CampaignJournalCandidateAdmissionReasonCategoryV1::BranchAdmission
        );
        assert_eq!(
            candidate.admission.normalized_reason_code(),
            CampaignJournalCandidateAdmissionReasonCodeV1::Selected
        );
    }

    #[test]
    fn journal_compaction_moves_route_map_packets_to_typed_candidates() {
        let mut run = crate::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = crate::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &crate::state::core::EngineState::MapNavigation,
            crate::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet =
            crate::ai::route_planner_v1::MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(!packet.candidates.is_empty());
        let expected_candidate_count = packet.candidates.len();
        let selected_route_candidate =
            CampaignJournalRouteCandidateV1::from_route_move_candidate_v1(&packet.candidates[0]);
        let mut journal = CampaignJournalV1 {
            schema_name: CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string(),
            schema_version: CAMPAIGN_JOURNAL_SCHEMA_VERSION,
            route_candidate_pools: Vec::new(),
            branch_paths: Vec::new(),
            branch_path_nodes: Vec::new(),
            event_branch_paths: Vec::new(),
            events: vec![
                CampaignJournalEventV1 {
                    event_id: "route-pool:candidate_set".to_string(),
                    round: 1,
                    branch_id: "root".to_string(),
                    branch_index: 0,
                    branch_frontier_title: "Map".to_string(),
                    act: 1,
                    floor: 1,
                    branch_choices: Vec::new(),
                    branch_commands: Vec::new(),
                    combat_budget_retry_used: false,
                    payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                        decision_id: "route-pool".to_string(),
                        boundary_title: "Map".to_string(),
                        frontier_key: "map".to_string(),
                        depth: 0,
                        candidate_count: expected_candidate_count,
                        selected_index: Some(0),
                        candidate_pool_provenance: None,
                        map_decision_packet: Some(packet),
                        route_candidates: Vec::new(),
                        route_candidate_pool_ref: None,
                        candidates: Vec::new(),
                    },
                },
                CampaignJournalEventV1 {
                    event_id: "route-decision".to_string(),
                    round: 1,
                    branch_id: "root".to_string(),
                    branch_index: 0,
                    branch_frontier_title: "Map".to_string(),
                    act: 1,
                    floor: 1,
                    branch_choices: Vec::new(),
                    branch_commands: Vec::new(),
                    combat_budget_retry_used: false,
                    payload: CampaignJournalEventPayloadV1::RouteDecision {
                        decision_id: "route-decision".to_string(),
                        route_branch_id: "route-branch".to_string(),
                        selected_index: Some(0),
                        selected_candidate_id: Some(selected_route_candidate.candidate_id.clone()),
                        selected_candidate_rank: Some(selected_route_candidate.rank),
                        selected_target_node: selected_route_candidate.target_node.clone(),
                        selected_route_candidate: Some(selected_route_candidate),
                        target: "x=1 y=0".to_string(),
                        move_kind: "normal_edge".to_string(),
                        safety_flag: None,
                        safety: "ok".to_string(),
                        candidate_pool_provenance: None,
                        command: "go 1".to_string(),
                        elite_prep_bp: 0,
                        first_elite: BranchExperimentFirstEliteEvidenceV1::default(),
                    },
                },
            ],
        };

        journal.compact_for_campaign_artifact_v1();
        assert_eq!(journal.route_candidate_pools.len(), 1);
        let pooled_candidates = &journal.route_candidate_pools[0].route_candidates;
        assert_eq!(pooled_candidates.len(), expected_candidate_count);
        assert!(pooled_candidates[0].path_facts.is_some());
        assert!(pooled_candidates[0].path_summary.is_none());
        assert!(pooled_candidates[0].score_terms.is_none());
        assert!(pooled_candidates[0].value_factors.is_none());
        assert!(pooled_candidates[0].evaluation_source.is_none());
        assert!(pooled_candidates[0].evaluation_calibration_status.is_none());
        assert!(pooled_candidates[0].node_features.is_none());
        assert!(pooled_candidates[0].needs.is_none());
        assert!(pooled_candidates[0].reasons.is_empty());

        match &journal.events[0].payload {
            CampaignJournalEventPayloadV1::RouteCandidatePool {
                map_decision_packet,
                route_candidates,
                route_candidate_pool_ref,
                ..
            } => {
                assert!(map_decision_packet.is_none());
                assert!(route_candidates.is_empty());
                assert_eq!(
                    route_candidate_pool_ref.as_deref(),
                    Some(journal.route_candidate_pools[0].pool_id.as_str())
                );
            }
            _ => panic!("expected route candidate pool"),
        }
        journal.hydrate_route_candidate_pools_v1();
        match &journal.events[0].payload {
            CampaignJournalEventPayloadV1::RouteCandidatePool {
                route_candidates, ..
            } => {
                assert_eq!(route_candidates.len(), expected_candidate_count);
            }
            _ => panic!("expected route candidate pool"),
        }
        match &journal.events[1].payload {
            CampaignJournalEventPayloadV1::RouteDecision {
                selected_route_candidate,
                selected_candidate_id,
                selected_target_node,
                candidate_pool_provenance,
                first_elite,
                ..
            } => {
                assert!(selected_route_candidate.is_none());
                assert!(selected_candidate_id.is_some());
                assert!(selected_target_node.is_none());
                assert!(candidate_pool_provenance.is_none());
                assert!(branch_experiment_first_elite_evidence_is_default_v1(
                    first_elite
                ));
            }
            _ => panic!("expected route decision"),
        }
    }

    #[test]
    fn journal_compaction_compacts_pretyped_route_candidates_without_map_packet() {
        let mut run = crate::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = crate::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &crate::state::core::EngineState::MapNavigation,
            crate::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet =
            crate::ai::route_planner_v1::MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        let expected_candidate_count = packet.candidates.len();
        let route_candidates = packet
            .candidates
            .iter()
            .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
            .collect::<Vec<_>>();
        assert!(route_candidates[0].score_terms.is_some());
        assert!(route_candidates[0].value_factors.is_some());

        let mut journal = CampaignJournalV1 {
            schema_name: CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string(),
            schema_version: CAMPAIGN_JOURNAL_SCHEMA_VERSION,
            route_candidate_pools: Vec::new(),
            branch_paths: Vec::new(),
            branch_path_nodes: Vec::new(),
            event_branch_paths: Vec::new(),
            events: vec![CampaignJournalEventV1 {
                event_id: "route-pool:candidate_set".to_string(),
                round: 1,
                branch_id: "root".to_string(),
                branch_index: 0,
                branch_frontier_title: "Map".to_string(),
                act: 1,
                floor: 1,
                branch_choices: Vec::new(),
                branch_commands: Vec::new(),
                combat_budget_retry_used: false,
                payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                    decision_id: "route-pool".to_string(),
                    boundary_title: "Map".to_string(),
                    frontier_key: "map".to_string(),
                    depth: 0,
                    candidate_count: expected_candidate_count,
                    selected_index: Some(0),
                    candidate_pool_provenance: None,
                    map_decision_packet: None,
                    route_candidates,
                    route_candidate_pool_ref: None,
                    candidates: vec![CampaignJournalCandidateV1 {
                        candidate_id: "generic-route".to_string(),
                        command: "go 1".to_string(),
                        label: "legacy route label".to_string(),
                        semantic_class: "legacy diagnostic".to_string(),
                        admission: CampaignJournalCandidateAdmissionTraceV1::default(),
                        disposition: CampaignJournalCandidateDispositionV1::Kept,
                    }],
                },
            }],
        };

        journal.compact_for_campaign_artifact_v1();
        assert_eq!(journal.route_candidate_pools.len(), 1);
        let pooled_candidates = &journal.route_candidate_pools[0].route_candidates;
        assert_eq!(pooled_candidates.len(), expected_candidate_count);
        assert!(pooled_candidates[0].path_facts.is_some());
        assert!(pooled_candidates[0].path_summary.is_none());
        assert!(pooled_candidates[0].score_terms.is_none());
        assert!(pooled_candidates[0].value_factors.is_none());
        assert!(pooled_candidates[0].evaluation_source.is_none());
        assert!(pooled_candidates[0].evaluation_calibration_status.is_none());
        assert!(pooled_candidates[0].node_features.is_none());
        assert!(pooled_candidates[0].needs.is_none());
        assert!(pooled_candidates[0].reasons.is_empty());
        assert!(pooled_candidates[0].cautions.is_empty());

        match &journal.events[0].payload {
            CampaignJournalEventPayloadV1::RouteCandidatePool {
                route_candidates,
                route_candidate_pool_ref,
                candidates,
                ..
            } => {
                assert!(candidates.is_empty());
                assert!(route_candidates.is_empty());
                assert_eq!(
                    route_candidate_pool_ref.as_deref(),
                    Some(journal.route_candidate_pools[0].pool_id.as_str())
                );
            }
            _ => panic!("expected route candidate pool"),
        }
        journal.hydrate_route_candidate_pools_v1();
        match &journal.events[0].payload {
            CampaignJournalEventPayloadV1::RouteCandidatePool {
                route_candidates, ..
            } => {
                assert_eq!(route_candidates.len(), expected_candidate_count);
            }
            _ => panic!("expected route candidate pool"),
        }
    }
}
