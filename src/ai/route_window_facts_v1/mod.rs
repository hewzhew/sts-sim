//! Route-window facts are a modal, value-free projection over the visible map.
//!
//! This module deliberately does not score routes and does not answer event
//! decisions. It only states what the currently reachable path family proves,
//! may prove, cannot prove, or leaves unknown.

use serde::{Deserialize, Serialize};

use crate::ai::route_planner_v1::route_targets;
use crate::content::relics::RelicId;
use crate::state::map::node::{MapRoomNode, RoomType};
use crate::state::RunState;

pub const ROUTE_WINDOW_FACTS_SCHEMA_NAME: &str = "RouteWindowFactsV1";
pub const ROUTE_WINDOW_FACTS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowFactsConfigV1 {
    /// Number of future map nodes to inspect after the current decision resolves.
    pub horizon_nodes: usize,
    /// Maximum number of path suffixes to enumerate before downgrading universal
    /// and negative claims to unknown.
    pub path_budget: usize,
}

impl Default for RouteWindowFactsConfigV1 {
    fn default() -> Self {
        Self {
            horizon_nodes: 5,
            path_budget: 2_000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowFactsV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub cursor: RouteWindowCursorV1,
    pub coverage: RouteWindowCoverageV1,
    pub observed_path_count: usize,
    pub facts: Vec<RouteWindowFactV1>,
}

impl RouteWindowFactsV1 {
    pub fn facts_for(&self, predicate: &RouteWindowPredicateV1) -> Vec<&RouteWindowFactV1> {
        self.facts
            .iter()
            .filter(|fact| &fact.predicate == predicate)
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowCursorV1 {
    pub current_x: i32,
    pub current_y: i32,
    pub starts_after_current_decision: bool,
    pub start_targets: Vec<RouteWindowNodeV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowCoverageKindV1 {
    CompleteWithinHorizon,
    PartialPathBudget,
    PartialUnmodeledMobility,
    UnavailableMap,
    NoVisibleContinuation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowCoverageV1 {
    pub kind: RouteWindowCoverageKindV1,
    pub horizon_nodes: usize,
    pub path_budget: usize,
    pub path_budget_exhausted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<RouteWindowLimitationV1>,
}

impl RouteWindowCoverageV1 {
    fn can_prove_universal_and_negative(&self) -> bool {
        self.kind == RouteWindowCoverageKindV1::CompleteWithinHorizon
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowLimitationV1 {
    MapPlaceholder,
    PathBudgetExhausted,
    FutureWingBootsJumpsNotEnumerated,
    NoLegalFutureTargets,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowNodeV1 {
    pub x: i32,
    pub y: i32,
    pub room_type: Option<RoomType>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWindowFactV1 {
    pub window: RouteWindowKindV1,
    pub predicate: RouteWindowPredicateV1,
    pub modality: RouteWindowModalityV1,
    pub scope: RouteWindowScopeV1,
    pub horizon_nodes: usize,
    pub provenance: RouteWindowProvenanceV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowKindV1 {
    Danger,
    Recovery,
    Liquidity,
    Payoff,
    Coverage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouteWindowPredicateV1 {
    ReachableWithin {
        subject: RouteWindowSubjectV1,
        nodes: usize,
    },
    PresentInWindow {
        subject: RouteWindowSubjectV1,
    },
    CountRangeInWindow {
        subject: RouteWindowSubjectV1,
        min: usize,
        max: usize,
    },
    OccursBefore {
        subject: RouteWindowSubjectV1,
        before: RouteWindowSubjectV1,
    },
    CoReachableWithin {
        left: RouteWindowSubjectV1,
        right: RouteWindowSubjectV1,
        nodes: usize,
    },
    BypassExists {
        subject: RouteWindowSubjectV1,
    },
    UnknownOpportunity {
        subject: RouteWindowSubjectV1,
    },
    Coverage {
        coverage_kind: RouteWindowCoverageKindV1,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowSubjectV1 {
    KnownCombat,
    HallwayCombat,
    Elite,
    Boss,
    Campfire,
    Shop,
    Treasure,
    UnknownRoom,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowModalityV1 {
    Must,
    Can,
    Cannot,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowScopeV1 {
    PathFamily,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteWindowProvenanceV1 {
    AllCoveredPaths,
    SomeCoveredPath,
    NoCoveredPathComplete,
    PartialObservation,
    MapUnavailable,
}

#[derive(Clone, Debug)]
struct ObservedPath {
    nodes: Vec<RouteWindowNodeV1>,
}

pub fn build_route_window_facts_v1(
    run_state: &RunState,
    config: RouteWindowFactsConfigV1,
) -> RouteWindowFactsV1 {
    let cursor = route_window_cursor_v1(run_state);
    let mut facts = Vec::new();

    if run_state.map.is_checkpoint_externalized_placeholder() || run_state.map.graph.is_empty() {
        let coverage = RouteWindowCoverageV1 {
            kind: RouteWindowCoverageKindV1::UnavailableMap,
            horizon_nodes: config.horizon_nodes,
            path_budget: config.path_budget,
            path_budget_exhausted: false,
            limitations: vec![RouteWindowLimitationV1::MapPlaceholder],
        };
        facts.push(coverage_fact(&coverage));
        return RouteWindowFactsV1 {
            schema_name: ROUTE_WINDOW_FACTS_SCHEMA_NAME.to_string(),
            schema_version: ROUTE_WINDOW_FACTS_SCHEMA_VERSION,
            cursor,
            coverage,
            observed_path_count: 0,
            facts,
        };
    }

    let mut paths = Vec::new();
    let mut path_budget_exhausted = false;
    for target in route_targets(run_state) {
        collect_path_suffixes(
            run_state,
            target.x,
            target.y,
            Vec::new(),
            config.horizon_nodes,
            config.path_budget,
            &mut paths,
            &mut path_budget_exhausted,
        );
        if path_budget_exhausted {
            break;
        }
    }

    let has_unmodeled_future_mobility = wing_boots_charges(run_state) > 0;
    let mut limitations = Vec::new();
    if path_budget_exhausted {
        limitations.push(RouteWindowLimitationV1::PathBudgetExhausted);
    }
    if has_unmodeled_future_mobility {
        limitations.push(RouteWindowLimitationV1::FutureWingBootsJumpsNotEnumerated);
    }
    if paths.is_empty() {
        limitations.push(RouteWindowLimitationV1::NoLegalFutureTargets);
    }

    let coverage_kind = if path_budget_exhausted {
        RouteWindowCoverageKindV1::PartialPathBudget
    } else if paths.is_empty() {
        RouteWindowCoverageKindV1::NoVisibleContinuation
    } else if has_unmodeled_future_mobility {
        RouteWindowCoverageKindV1::PartialUnmodeledMobility
    } else {
        RouteWindowCoverageKindV1::CompleteWithinHorizon
    };
    let coverage = RouteWindowCoverageV1 {
        kind: coverage_kind,
        horizon_nodes: config.horizon_nodes,
        path_budget: config.path_budget,
        path_budget_exhausted,
        limitations,
    };

    facts.push(coverage_fact(&coverage));
    facts.extend(derive_subject_facts(
        &paths,
        &coverage,
        config.horizon_nodes,
    ));
    facts.extend(derive_before_facts(&paths, &coverage, config.horizon_nodes));
    facts.extend(derive_coreachability_facts(
        &paths,
        &coverage,
        config.horizon_nodes,
    ));
    facts.extend(derive_unknown_opportunity_facts(
        &paths,
        &coverage,
        config.horizon_nodes,
    ));
    facts.sort_by_key(fact_sort_key);
    facts.dedup();

    RouteWindowFactsV1 {
        schema_name: ROUTE_WINDOW_FACTS_SCHEMA_NAME.to_string(),
        schema_version: ROUTE_WINDOW_FACTS_SCHEMA_VERSION,
        cursor,
        coverage,
        observed_path_count: paths.len(),
        facts,
    }
}

fn route_window_cursor_v1(run_state: &RunState) -> RouteWindowCursorV1 {
    let start_targets = if run_state.map.is_checkpoint_externalized_placeholder()
        || run_state.map.graph.is_empty()
    {
        Vec::new()
    } else {
        route_targets(run_state)
            .into_iter()
            .map(|target| RouteWindowNodeV1 {
                x: target.x,
                y: target.y,
                room_type: target.room_type,
            })
            .collect()
    };
    RouteWindowCursorV1 {
        current_x: run_state.map.current_x,
        current_y: run_state.map.current_y,
        starts_after_current_decision: true,
        start_targets,
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_path_suffixes(
    run_state: &RunState,
    x: i32,
    y: i32,
    mut prefix: Vec<RouteWindowNodeV1>,
    horizon_nodes: usize,
    path_budget: usize,
    paths: &mut Vec<ObservedPath>,
    path_budget_exhausted: &mut bool,
) {
    if paths.len() >= path_budget {
        *path_budget_exhausted = true;
        return;
    }
    if prefix.len() >= horizon_nodes {
        paths.push(ObservedPath { nodes: prefix });
        return;
    }
    let Some(node) = node_at(run_state, x, y) else {
        paths.push(ObservedPath { nodes: prefix });
        return;
    };
    prefix.push(RouteWindowNodeV1 {
        x: node.x,
        y: node.y,
        room_type: node.class,
    });
    if prefix.len() >= horizon_nodes || node.edges.is_empty() || node.y >= 15 {
        paths.push(ObservedPath { nodes: prefix });
        return;
    }
    for edge in &node.edges {
        collect_path_suffixes(
            run_state,
            edge.dst_x,
            edge.dst_y,
            prefix.clone(),
            horizon_nodes,
            path_budget,
            paths,
            path_budget_exhausted,
        );
        if *path_budget_exhausted {
            return;
        }
    }
}

fn node_at(run_state: &RunState, x: i32, y: i32) -> Option<&MapRoomNode> {
    run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))
}

fn wing_boots_charges(run_state: &RunState) -> i32 {
    run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::WingBoots && !relic.used_up)
        .map(|relic| relic.counter.max(0))
        .unwrap_or_default()
}

fn coverage_fact(coverage: &RouteWindowCoverageV1) -> RouteWindowFactV1 {
    RouteWindowFactV1 {
        window: RouteWindowKindV1::Coverage,
        predicate: RouteWindowPredicateV1::Coverage {
            coverage_kind: coverage.kind,
        },
        modality: match coverage.kind {
            RouteWindowCoverageKindV1::CompleteWithinHorizon => RouteWindowModalityV1::Must,
            RouteWindowCoverageKindV1::UnavailableMap
            | RouteWindowCoverageKindV1::NoVisibleContinuation => RouteWindowModalityV1::Unknown,
            RouteWindowCoverageKindV1::PartialPathBudget
            | RouteWindowCoverageKindV1::PartialUnmodeledMobility => RouteWindowModalityV1::Unknown,
        },
        scope: RouteWindowScopeV1::PathFamily,
        horizon_nodes: coverage.horizon_nodes,
        provenance: match coverage.kind {
            RouteWindowCoverageKindV1::CompleteWithinHorizon => {
                RouteWindowProvenanceV1::AllCoveredPaths
            }
            RouteWindowCoverageKindV1::UnavailableMap => RouteWindowProvenanceV1::MapUnavailable,
            RouteWindowCoverageKindV1::PartialPathBudget
            | RouteWindowCoverageKindV1::PartialUnmodeledMobility
            | RouteWindowCoverageKindV1::NoVisibleContinuation => {
                RouteWindowProvenanceV1::PartialObservation
            }
        },
    }
}

fn derive_subject_facts(
    paths: &[ObservedPath],
    coverage: &RouteWindowCoverageV1,
    horizon_nodes: usize,
) -> Vec<RouteWindowFactV1> {
    let mut facts = Vec::new();
    for subject in standard_subjects() {
        let counts = paths
            .iter()
            .map(|path| {
                path.nodes
                    .iter()
                    .filter(|node| subject_matches(*subject, node))
                    .count()
            })
            .collect::<Vec<_>>();
        let min = counts.iter().copied().min().unwrap_or(0);
        let max = counts.iter().copied().max().unwrap_or(0);
        facts.push(RouteWindowFactV1 {
            window: window_for_subject(*subject),
            predicate: RouteWindowPredicateV1::CountRangeInWindow {
                subject: *subject,
                min,
                max,
            },
            modality: if paths.is_empty() {
                RouteWindowModalityV1::Unknown
            } else if min > 0 && coverage.can_prove_universal_and_negative() {
                RouteWindowModalityV1::Must
            } else if max > 0 {
                RouteWindowModalityV1::Can
            } else if coverage.can_prove_universal_and_negative() {
                RouteWindowModalityV1::Cannot
            } else {
                RouteWindowModalityV1::Unknown
            },
            scope: RouteWindowScopeV1::PathFamily,
            horizon_nodes,
            provenance: provenance_for_counts(min, max, coverage),
        });

        facts.push(presence_fact(
            RouteWindowPredicateV1::PresentInWindow { subject: *subject },
            *subject,
            min,
            max,
            coverage,
            horizon_nodes,
        ));

        for nodes in 1..=horizon_nodes {
            let within = paths
                .iter()
                .map(|path| first_subject_index(path, *subject).is_some_and(|idx| idx < nodes))
                .collect::<Vec<_>>();
            let can = within.iter().any(|value| *value);
            let must = !within.is_empty() && within.iter().all(|value| *value);
            facts.push(RouteWindowFactV1 {
                window: window_for_subject(*subject),
                predicate: RouteWindowPredicateV1::ReachableWithin {
                    subject: *subject,
                    nodes,
                },
                modality: modality_from_can_must(can, must, coverage),
                scope: RouteWindowScopeV1::PathFamily,
                horizon_nodes,
                provenance: provenance_from_can_must(can, must, coverage),
            });
        }

        let bypass_count = paths
            .iter()
            .filter(|path| first_subject_index(path, *subject).is_none())
            .count();
        if bypass_count > 0 {
            facts.push(RouteWindowFactV1 {
                window: window_for_subject(*subject),
                predicate: RouteWindowPredicateV1::BypassExists { subject: *subject },
                modality: RouteWindowModalityV1::Can,
                scope: RouteWindowScopeV1::PathFamily,
                horizon_nodes,
                provenance: RouteWindowProvenanceV1::SomeCoveredPath,
            });
        } else {
            facts.push(RouteWindowFactV1 {
                window: window_for_subject(*subject),
                predicate: RouteWindowPredicateV1::BypassExists { subject: *subject },
                modality: if paths.is_empty() || !coverage.can_prove_universal_and_negative() {
                    RouteWindowModalityV1::Unknown
                } else {
                    RouteWindowModalityV1::Cannot
                },
                scope: RouteWindowScopeV1::PathFamily,
                horizon_nodes,
                provenance: if paths.is_empty() || !coverage.can_prove_universal_and_negative() {
                    RouteWindowProvenanceV1::PartialObservation
                } else {
                    RouteWindowProvenanceV1::NoCoveredPathComplete
                },
            });
        }
    }
    facts
}

fn derive_before_facts(
    paths: &[ObservedPath],
    coverage: &RouteWindowCoverageV1,
    horizon_nodes: usize,
) -> Vec<RouteWindowFactV1> {
    let pairs = [
        (
            RouteWindowSubjectV1::KnownCombat,
            RouteWindowSubjectV1::Campfire,
        ),
        (
            RouteWindowSubjectV1::KnownCombat,
            RouteWindowSubjectV1::Shop,
        ),
        (RouteWindowSubjectV1::Elite, RouteWindowSubjectV1::Campfire),
        (RouteWindowSubjectV1::Campfire, RouteWindowSubjectV1::Elite),
        (
            RouteWindowSubjectV1::Shop,
            RouteWindowSubjectV1::KnownCombat,
        ),
        (RouteWindowSubjectV1::Boss, RouteWindowSubjectV1::Campfire),
    ];
    pairs
        .into_iter()
        .map(|(subject, before)| {
            let observations = paths
                .iter()
                .map(|path| occurs_before(path, subject, before))
                .collect::<Vec<_>>();
            let can = observations.iter().any(|value| *value);
            let must = !observations.is_empty() && observations.iter().all(|value| *value);
            RouteWindowFactV1 {
                window: window_for_subject(subject),
                predicate: RouteWindowPredicateV1::OccursBefore { subject, before },
                modality: modality_from_can_must(can, must, coverage),
                scope: RouteWindowScopeV1::PathFamily,
                horizon_nodes,
                provenance: provenance_from_can_must(can, must, coverage),
            }
        })
        .collect()
}

fn derive_coreachability_facts(
    paths: &[ObservedPath],
    coverage: &RouteWindowCoverageV1,
    horizon_nodes: usize,
) -> Vec<RouteWindowFactV1> {
    let pairs = [
        (RouteWindowSubjectV1::Shop, RouteWindowSubjectV1::Campfire),
        (RouteWindowSubjectV1::Elite, RouteWindowSubjectV1::Campfire),
        (RouteWindowSubjectV1::Shop, RouteWindowSubjectV1::Elite),
        (RouteWindowSubjectV1::Treasure, RouteWindowSubjectV1::Shop),
    ];
    pairs
        .into_iter()
        .map(|(left, right)| {
            let observations = paths
                .iter()
                .map(|path| {
                    first_subject_index(path, left).is_some()
                        && first_subject_index(path, right).is_some()
                })
                .collect::<Vec<_>>();
            let can = observations.iter().any(|value| *value);
            let must = !observations.is_empty() && observations.iter().all(|value| *value);
            RouteWindowFactV1 {
                window: window_for_subject(left),
                predicate: RouteWindowPredicateV1::CoReachableWithin {
                    left,
                    right,
                    nodes: horizon_nodes,
                },
                modality: modality_from_can_must(can, must, coverage),
                scope: RouteWindowScopeV1::PathFamily,
                horizon_nodes,
                provenance: provenance_from_can_must(can, must, coverage),
            }
        })
        .collect()
}

fn derive_unknown_opportunity_facts(
    paths: &[ObservedPath],
    coverage: &RouteWindowCoverageV1,
    horizon_nodes: usize,
) -> Vec<RouteWindowFactV1> {
    let has_unknown = paths
        .iter()
        .any(|path| first_subject_index(path, RouteWindowSubjectV1::UnknownRoom).is_some());
    [
        RouteWindowSubjectV1::Shop,
        RouteWindowSubjectV1::Treasure,
        RouteWindowSubjectV1::KnownCombat,
    ]
    .into_iter()
    .map(|subject| RouteWindowFactV1 {
        window: window_for_subject(subject),
        predicate: RouteWindowPredicateV1::UnknownOpportunity { subject },
        modality: if has_unknown {
            RouteWindowModalityV1::Unknown
        } else if coverage.can_prove_universal_and_negative() {
            RouteWindowModalityV1::Cannot
        } else {
            RouteWindowModalityV1::Unknown
        },
        scope: RouteWindowScopeV1::PathFamily,
        horizon_nodes,
        provenance: if has_unknown || !coverage.can_prove_universal_and_negative() {
            RouteWindowProvenanceV1::PartialObservation
        } else {
            RouteWindowProvenanceV1::NoCoveredPathComplete
        },
    })
    .collect()
}

fn presence_fact(
    predicate: RouteWindowPredicateV1,
    subject: RouteWindowSubjectV1,
    min: usize,
    max: usize,
    coverage: &RouteWindowCoverageV1,
    horizon_nodes: usize,
) -> RouteWindowFactV1 {
    RouteWindowFactV1 {
        window: window_for_subject(subject),
        predicate,
        modality: if min > 0 && coverage.can_prove_universal_and_negative() {
            RouteWindowModalityV1::Must
        } else if max > 0 {
            RouteWindowModalityV1::Can
        } else if coverage.can_prove_universal_and_negative() {
            RouteWindowModalityV1::Cannot
        } else {
            RouteWindowModalityV1::Unknown
        },
        scope: RouteWindowScopeV1::PathFamily,
        horizon_nodes,
        provenance: provenance_for_counts(min, max, coverage),
    }
}

fn standard_subjects() -> &'static [RouteWindowSubjectV1] {
    &[
        RouteWindowSubjectV1::KnownCombat,
        RouteWindowSubjectV1::HallwayCombat,
        RouteWindowSubjectV1::Elite,
        RouteWindowSubjectV1::Boss,
        RouteWindowSubjectV1::Campfire,
        RouteWindowSubjectV1::Shop,
        RouteWindowSubjectV1::Treasure,
        RouteWindowSubjectV1::UnknownRoom,
    ]
}

fn window_for_subject(subject: RouteWindowSubjectV1) -> RouteWindowKindV1 {
    match subject {
        RouteWindowSubjectV1::KnownCombat
        | RouteWindowSubjectV1::HallwayCombat
        | RouteWindowSubjectV1::Elite
        | RouteWindowSubjectV1::Boss => RouteWindowKindV1::Danger,
        RouteWindowSubjectV1::Campfire => RouteWindowKindV1::Recovery,
        RouteWindowSubjectV1::Shop => RouteWindowKindV1::Liquidity,
        RouteWindowSubjectV1::Treasure | RouteWindowSubjectV1::UnknownRoom => {
            RouteWindowKindV1::Payoff
        }
    }
}

fn subject_matches(subject: RouteWindowSubjectV1, node: &RouteWindowNodeV1) -> bool {
    match subject {
        RouteWindowSubjectV1::KnownCombat => matches!(
            node.room_type,
            Some(RoomType::MonsterRoom)
                | Some(RoomType::MonsterRoomElite)
                | Some(RoomType::MonsterRoomBoss)
        ),
        RouteWindowSubjectV1::HallwayCombat => node.room_type == Some(RoomType::MonsterRoom),
        RouteWindowSubjectV1::Elite => node.room_type == Some(RoomType::MonsterRoomElite),
        RouteWindowSubjectV1::Boss => node.room_type == Some(RoomType::MonsterRoomBoss),
        RouteWindowSubjectV1::Campfire => node.room_type == Some(RoomType::RestRoom),
        RouteWindowSubjectV1::Shop => node.room_type == Some(RoomType::ShopRoom),
        RouteWindowSubjectV1::Treasure => node.room_type == Some(RoomType::TreasureRoom),
        RouteWindowSubjectV1::UnknownRoom => node.room_type == Some(RoomType::EventRoom),
    }
}

fn first_subject_index(path: &ObservedPath, subject: RouteWindowSubjectV1) -> Option<usize> {
    path.nodes
        .iter()
        .position(|node| subject_matches(subject, node))
}

fn occurs_before(
    path: &ObservedPath,
    subject: RouteWindowSubjectV1,
    before: RouteWindowSubjectV1,
) -> bool {
    let Some(subject_idx) = first_subject_index(path, subject) else {
        return false;
    };
    match first_subject_index(path, before) {
        Some(before_idx) => subject_idx < before_idx,
        None => true,
    }
}

fn modality_from_can_must(
    can: bool,
    must: bool,
    coverage: &RouteWindowCoverageV1,
) -> RouteWindowModalityV1 {
    if must && coverage.can_prove_universal_and_negative() {
        RouteWindowModalityV1::Must
    } else if can {
        RouteWindowModalityV1::Can
    } else if coverage.can_prove_universal_and_negative() {
        RouteWindowModalityV1::Cannot
    } else {
        RouteWindowModalityV1::Unknown
    }
}

fn provenance_from_can_must(
    can: bool,
    must: bool,
    coverage: &RouteWindowCoverageV1,
) -> RouteWindowProvenanceV1 {
    if must && coverage.can_prove_universal_and_negative() {
        RouteWindowProvenanceV1::AllCoveredPaths
    } else if can {
        RouteWindowProvenanceV1::SomeCoveredPath
    } else if coverage.can_prove_universal_and_negative() {
        RouteWindowProvenanceV1::NoCoveredPathComplete
    } else {
        RouteWindowProvenanceV1::PartialObservation
    }
}

fn provenance_for_counts(
    min: usize,
    max: usize,
    coverage: &RouteWindowCoverageV1,
) -> RouteWindowProvenanceV1 {
    if min > 0 && coverage.can_prove_universal_and_negative() {
        RouteWindowProvenanceV1::AllCoveredPaths
    } else if max > 0 {
        RouteWindowProvenanceV1::SomeCoveredPath
    } else if coverage.can_prove_universal_and_negative() {
        RouteWindowProvenanceV1::NoCoveredPathComplete
    } else {
        RouteWindowProvenanceV1::PartialObservation
    }
}

fn fact_sort_key(fact: &RouteWindowFactV1) -> (u8, String, u8, u8, usize, u8) {
    (
        window_sort_key(fact.window),
        format!("{:?}", fact.predicate),
        modality_sort_key(fact.modality),
        scope_sort_key(fact.scope),
        fact.horizon_nodes,
        provenance_sort_key(fact.provenance),
    )
}

fn window_sort_key(window: RouteWindowKindV1) -> u8 {
    match window {
        RouteWindowKindV1::Danger => 0,
        RouteWindowKindV1::Recovery => 1,
        RouteWindowKindV1::Liquidity => 2,
        RouteWindowKindV1::Payoff => 3,
        RouteWindowKindV1::Coverage => 4,
    }
}

fn modality_sort_key(modality: RouteWindowModalityV1) -> u8 {
    match modality {
        RouteWindowModalityV1::Must => 0,
        RouteWindowModalityV1::Can => 1,
        RouteWindowModalityV1::Cannot => 2,
        RouteWindowModalityV1::Unknown => 3,
    }
}

fn scope_sort_key(scope: RouteWindowScopeV1) -> u8 {
    match scope {
        RouteWindowScopeV1::PathFamily => 0,
    }
}

fn provenance_sort_key(provenance: RouteWindowProvenanceV1) -> u8 {
    match provenance {
        RouteWindowProvenanceV1::AllCoveredPaths => 0,
        RouteWindowProvenanceV1::SomeCoveredPath => 1,
        RouteWindowProvenanceV1::NoCoveredPathComplete => 2,
        RouteWindowProvenanceV1::PartialObservation => 3,
        RouteWindowProvenanceV1::MapUnavailable => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::map::node::{MapEdge, MapRoomNode};

    fn node(x: i32, y: i32, class: RoomType) -> MapRoomNode {
        let mut node = MapRoomNode::new(x, y);
        node.class = Some(class);
        node
    }

    fn run_with_graph(graph: Vec<Vec<MapRoomNode>>, current_x: i32, current_y: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.map = crate::state::map::state::MapState::new(graph);
        run_state.map.current_x = current_x;
        run_state.map.current_y = current_y;
        run_state
    }

    fn has_fact(
        facts: &RouteWindowFactsV1,
        predicate: RouteWindowPredicateV1,
        modality: RouteWindowModalityV1,
        provenance: RouteWindowProvenanceV1,
    ) -> bool {
        facts.facts.iter().any(|fact| {
            fact.predicate == predicate
                && fact.modality == modality
                && fact.provenance == provenance
        })
    }

    #[test]
    fn current_node_is_not_counted_as_future_window() {
        let mut event = node(0, 0, RoomType::EventRoom);
        event.edges.insert(MapEdge::new(0, 0, 0, 1));
        let fire = node(0, 1, RoomType::RestRoom);
        let run_state = run_with_graph(vec![vec![event], vec![fire]], 0, 0);

        let facts = build_route_window_facts_v1(
            &run_state,
            RouteWindowFactsConfigV1 {
                horizon_nodes: 2,
                path_budget: 16,
            },
        );

        assert!(has_fact(
            &facts,
            RouteWindowPredicateV1::PresentInWindow {
                subject: RouteWindowSubjectV1::UnknownRoom
            },
            RouteWindowModalityV1::Cannot,
            RouteWindowProvenanceV1::NoCoveredPathComplete,
        ));
        assert!(has_fact(
            &facts,
            RouteWindowPredicateV1::PresentInWindow {
                subject: RouteWindowSubjectV1::Campfire
            },
            RouteWindowModalityV1::Must,
            RouteWindowProvenanceV1::AllCoveredPaths,
        ));
    }

    #[test]
    fn can_is_not_upgraded_to_must_for_optional_shop() {
        let mut start = node(0, 0, RoomType::MonsterRoom);
        start.edges.insert(MapEdge::new(0, 0, 0, 1));
        start.edges.insert(MapEdge::new(0, 0, 1, 1));
        let shop = node(0, 1, RoomType::ShopRoom);
        let fire = node(1, 1, RoomType::RestRoom);
        let run_state = run_with_graph(vec![vec![start], vec![shop, fire]], 0, 0);

        let facts = build_route_window_facts_v1(
            &run_state,
            RouteWindowFactsConfigV1 {
                horizon_nodes: 1,
                path_budget: 16,
            },
        );

        assert!(has_fact(
            &facts,
            RouteWindowPredicateV1::ReachableWithin {
                subject: RouteWindowSubjectV1::Shop,
                nodes: 1,
            },
            RouteWindowModalityV1::Can,
            RouteWindowProvenanceV1::SomeCoveredPath,
        ));
        assert!(has_fact(
            &facts,
            RouteWindowPredicateV1::BypassExists {
                subject: RouteWindowSubjectV1::Shop,
            },
            RouteWindowModalityV1::Can,
            RouteWindowProvenanceV1::SomeCoveredPath,
        ));
    }

    #[test]
    fn partial_coverage_does_not_claim_cannot() {
        let mut start = node(0, 0, RoomType::MonsterRoom);
        start.edges.insert(MapEdge::new(0, 0, 0, 1));
        start.edges.insert(MapEdge::new(0, 0, 1, 1));
        let combat = node(0, 1, RoomType::MonsterRoom);
        let shop = node(1, 1, RoomType::ShopRoom);
        let run_state = run_with_graph(vec![vec![start], vec![combat, shop]], 0, 0);

        let facts = build_route_window_facts_v1(
            &run_state,
            RouteWindowFactsConfigV1 {
                horizon_nodes: 1,
                path_budget: 1,
            },
        );

        assert_eq!(
            facts.coverage.kind,
            RouteWindowCoverageKindV1::PartialPathBudget
        );
        assert!(!has_fact(
            &facts,
            RouteWindowPredicateV1::PresentInWindow {
                subject: RouteWindowSubjectV1::Shop
            },
            RouteWindowModalityV1::Cannot,
            RouteWindowProvenanceV1::NoCoveredPathComplete,
        ));
    }
}
