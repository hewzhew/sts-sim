use super::*;

#[test]
fn compact_campaign_report_renders_strategy_prompt() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 3,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "event_strategy".to_string(),
            boundary_title: "Falling".to_string(),
            branch_count: 2,
            act: 0,
            floor: 0,
            stop_reasons: vec!["event policy gap".to_string()],
            examples: vec!["Strike -> Defend".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "provide Falling policy".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains(
        "BranchCampaignV1 seed=521 ascension=A0 domain=debug_a0 role=debug_domain class=Ironclad rounds=2 stop=needs_intervention"
    ));
    assert!(rendered.contains(
        "Active 0 | Frozen 0 | Dead 0 | Abandoned 0 | Victories 0 | Stuck 0 | Discarded 3"
    ));
    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("event_strategy | Falling | branches=2"));
    assert!(rendered.contains(
        "next: write a narrow event rule or choose one branch manually, then rerun the campaign"
    ));
    assert!(!rendered.contains("Top active:"));
}

#[test]
fn compact_campaign_report_renders_actionable_intervention_details() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 6,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![test_campaign_branch("a", 16, 70)],
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 2,
            act: 0,
            floor: 0,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["Clash -> Disarm".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "raise combat search budget".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: vec![BranchCampaignRoundSummaryV1 {
            round: 5,
            started_active: 2,
            produced_branches: 2,
            active_after: 0,
            frozen_added: 0,
            dead_added: 0,
            abandoned_added: 2,
            victories_added: 0,
            stuck_added: 0,
            discarded_added: 0,
            explored_branch_points: 0,
            wall_limit_hit: false,
            branch_limit_hit: false,
            combat_budget_retries: 2,
            ..BranchCampaignRoundSummaryV1::default()
        }],
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("kind: combat_unresolved_after_retry"));
    assert!(rendered.contains("stop: combat search did not find an executable complete win"));
    assert!(rendered.contains("tried: campaign search budget; combat budget retry x2"));
    assert!(rendered.contains("possible inputs: switch macro branch | provide combat tactic | add upstream route/reward rule | raise retry budget only if under-spent"));
}

#[test]
fn compact_campaign_report_suppresses_deferred_strategy_notes_while_active_continues() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 330404105,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 6, 80)],
        frozen: vec![test_campaign_branch("f", 5, 75)],
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![test_campaign_branch("s", 6, 70)],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 1,
            act: 0,
            floor: 0,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["Flex".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "raise combat search budget".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(!rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("stop: combat search did not find an executable complete win"));
}

#[test]
fn compact_campaign_report_suppresses_stale_strategy_notes_after_victory() {
    let mut report = test_campaign_report_with_active("winner", 48, 42);
    report.stop_reason = "victory_found".to_string();
    report.active.clear();
    report.victories = vec![test_campaign_branch("winner", 48, 42)];
    report.stuck = vec![test_campaign_branch("old-stuck", 45, 35)];
    report.strategy_requests = vec![test_campaign_request("event_strategy", "MatchAndKeep")];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(!rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("MatchAndKeep"));
}

#[test]
fn compact_campaign_report_renders_first_and_best_victory_quality() {
    let mut report = test_campaign_report_with_active("winner", 48, 42);
    report.active.clear();
    let mut first = test_campaign_branch("first-win", 48, 6);
    first.status = BranchCampaignBranchStatusV1::TerminalVictory;
    first.choice_labels = vec!["risky line".to_string()];
    let mut best = test_campaign_branch("best-win", 48, 32);
    best.status = BranchCampaignBranchStatusV1::TerminalVictory;
    best.choice_labels = vec!["stable line".to_string()];
    report.victories = vec![first, best];

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("First victory: A1F48 HP 6/80"));
    assert!(rendered.contains("choices: risky line"));
    assert!(rendered.contains("Best victory: A1F48 HP 32/80"));
    assert!(rendered.contains("choices: stable line"));
}

#[test]
fn compact_campaign_report_renders_needs_intervention_even_with_frozen_branches() {
    let mut report = test_campaign_report_with_active("a", 16, 70);
    report.stop_reason = "needs_intervention".to_string();
    report.active.clear();
    report.frozen = vec![test_campaign_branch("old-frozen", 4, 80)];
    report.strategy_requests = vec![test_campaign_request("combat_manual_or_budget", "Combat")];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("Deferred strategy notes:"));
}

#[test]
fn campaign_builds_intervention_when_abandoned_branches_exhaust_routes() {
    let mut a = test_campaign_branch("a", 16, 70);
    a.stop_reason = "combat search did not find an executable complete win".to_string();
    let mut b = test_campaign_branch("b", 16, 62);
    b.stop_reason = "wall-clock deadline hit".to_string();
    let request = abandoned_branches_intervention_request_v1(&[a, b]).expect("request");

    assert_eq!(request.kind, "combat_manual_or_budget");
    assert_eq!(request.boundary_title, "Combat");
    assert_eq!(request.branch_count, 2);
    assert_eq!(request.examples.len(), 2);
    assert_eq!(
        request.stop_reasons,
        vec![
            "combat search did not find an executable complete win".to_string(),
            "wall-clock deadline hit".to_string()
        ]
    );
}

#[test]
fn campaign_records_deferred_note_for_leading_abandoned_combat_while_frozen_exists() {
    let mut abandoned = test_campaign_branch("abandoned-combat", 48, 78);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    abandoned.summary.as_mut().expect("summary").act = 3;
    let frozen = vec![test_campaign_branch("old-frozen", 11, 80)];

    let request = leading_abandoned_combat_intervention_request_v1(&frozen, &[abandoned])
        .expect("leading abandoned combat should request intervention");

    assert_eq!(request.kind, "combat_manual_or_budget");
    assert_eq!(request.boundary_title, "Combat");
    assert_eq!(request.act, 3);
    assert_eq!(request.floor, 48);
    assert_eq!(
        request.stop_reasons,
        vec!["combat search did not find an executable complete win".to_string()]
    );
}

#[test]
fn campaign_does_not_request_leading_abandoned_intervention_when_frozen_is_not_behind() {
    let mut abandoned = test_campaign_branch("abandoned-combat", 12, 78);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    let frozen = vec![test_campaign_branch("same-progress-frozen", 12, 80)];

    assert!(leading_abandoned_combat_intervention_request_v1(&frozen, &[abandoned]).is_none());
}

#[test]
fn campaign_strategy_requests_are_fatal_only_without_continuable_branches() {
    let active = vec![test_campaign_branch("a", 6, 80)];
    let frozen = vec![test_campaign_branch("f", 5, 75)];
    let request = vec![test_campaign_request("combat_manual_or_budget", "Combat")];

    assert!(!campaign_strategy_requests_are_fatal_v1(
        &active,
        &[],
        &request
    ));
    assert!(!campaign_strategy_requests_are_fatal_v1(
        &[],
        &frozen,
        &request
    ));
    assert!(campaign_strategy_requests_are_fatal_v1(&[], &[], &request));
}

#[test]
fn campaign_selection_treats_combat_stuck_as_abandoned_route_branch() {
    let mut combat = test_campaign_branch_with_boundary(
        "combat-stuck",
        "Combat",
        "combat search did not find an executable complete win",
        12,
        55,
    );
    combat.status = BranchCampaignBranchStatusV1::Stuck;

    let selected = select_campaign_branches_v1(vec![combat], 2, 4);

    assert_eq!(selected.abandoned.len(), 1);
    assert_eq!(selected.abandoned[0].branch_id, "combat-stuck");
    assert!(selected.stuck.is_empty());
}

#[test]
fn campaign_selection_keeps_noncombat_stuck_for_strategy_intervention() {
    let mut event = test_campaign_branch_with_boundary(
        "event-stuck",
        "Falling",
        "event option requires human choice",
        36,
        70,
    );
    event.status = BranchCampaignBranchStatusV1::Stuck;

    let selected = select_campaign_branches_v1(vec![event], 2, 4);

    assert_eq!(selected.stuck.len(), 1);
    assert_eq!(selected.stuck[0].branch_id, "event-stuck");
    assert!(selected.abandoned.is_empty());
}

#[test]
fn campaign_strategy_request_prune_drops_requests_without_matching_blocked_branch() {
    let active = vec![test_campaign_branch("a", 10, 80)];
    let stuck = vec![test_campaign_branch_with_boundary(
        "s",
        "Map Preview",
        "route planner declined automatic map selection",
        16,
        60,
    )];
    let requests = vec![
        {
            let mut request = test_campaign_request("event_strategy", "Beggar");
            request.act = 2;
            request.floor = 21;
            request.stop_reasons = vec!["event option requires human choice".to_string()];
            request
        },
        {
            let mut request = test_campaign_request("route_policy_gap", "Map Preview");
            request.act = 1;
            request.floor = 16;
            request.stop_reasons =
                vec!["route planner declined automatic map selection".to_string()];
            request
        },
    ];

    let pruned = prune_resolved_campaign_strategy_requests_v1(requests, &active, &[], &stuck, &[]);

    assert_eq!(pruned.len(), 1);
    assert_eq!(pruned[0].kind, "route_policy_gap");
    assert_eq!(pruned[0].boundary_title, "Map Preview");
}

#[test]
fn campaign_recovers_stuck_branch_if_one_auto_step_leaves_frontier() {
    let mut active = Vec::new();
    let mut frozen = Vec::new();
    let mut stuck = vec![test_campaign_branch_with_boundary(
        "s",
        "Beggar",
        "event option requires human choice",
        21,
        53,
    )];
    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState {
        id: EventId::Beggar,
        current_screen: 2,
        internal_state: 0,
        completed: false,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    session.engine_state = EngineState::EventRoom;
    state_store.insert_session(stuck[0].commands.clone(), session);

    let recovered = recover_auto_advanceable_stuck_branches_v1(
        &mut active,
        &mut frozen,
        &mut stuck,
        &mut state_store,
        1,
        0,
    );

    assert_eq!(recovered, 1);
    assert_eq!(stuck.len(), 0);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].frontier_title, "Map");
    assert!(matches!(
        state_store
            .get_session(&active[0].commands)
            .expect("recovered snapshot should be retained")
            .engine_state,
        EngineState::MapNavigation
    ));
}

#[test]
fn campaign_recovers_stale_empty_campfire_portfolio_when_boundary_is_now_available() {
    let mut active = Vec::new();
    let mut frozen = Vec::new();
    let mut stuck = vec![test_campaign_branch_with_boundary(
        "campfire-stale",
        "Campfire",
        "campfire option portfolio is empty",
        38,
        80,
    )];
    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;
    state_store.insert_session(stuck[0].commands.clone(), session);

    let recovered = recover_auto_advanceable_stuck_branches_v1(
        &mut active,
        &mut frozen,
        &mut stuck,
        &mut state_store,
        1,
        4,
    );

    assert_eq!(recovered, 1);
    assert!(stuck.is_empty());
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].branch_id, "campfire-stale");
    assert_eq!(active[0].frontier_title, "Campfire");
    assert!(active[0]
        .stop_reason
        .contains("recovered from current branch boundary"));
}

#[test]
fn campaign_strategy_request_merge_keeps_stop_reasons() {
    let merged = merge_campaign_strategy_requests_v1(vec![test_experiment_request(
        "combat_manual_or_budget",
        "Combat",
        "combat search did not find an executable complete win",
    )]);

    assert_eq!(
        merged[0].stop_reasons,
        vec!["combat search did not find an executable complete win".to_string()]
    );
}

#[test]
fn campaign_strategy_request_merge_preserves_context_for_human_research() {
    let mut request = test_experiment_request("event_strategy", "GoldenIdol", "event policy gap");
    request.act = 1;
    request.floor = 5;
    request.next_card_reward_offer = Some(vec![
        "Pommel Strike".to_string(),
        "Iron Wave".to_string(),
        "Searing Blow".to_string(),
    ]);
    request.boundary_details = vec![
        "option: [Take] Obtain Golden Idol.".to_string(),
        "option: [Leave] Leave.".to_string(),
    ];

    let merged = merge_campaign_strategy_requests_v1(vec![request]);

    assert_eq!(merged[0].act, 1);
    assert_eq!(merged[0].floor, 5);
    assert_eq!(
        merged[0].next_card_reward_offer,
        Some(vec![
            "Pommel Strike".to_string(),
            "Iron Wave".to_string(),
            "Searing Blow".to_string(),
        ])
    );
    assert_eq!(merged[0].boundary_details.len(), 2);
}

#[test]
fn campaign_strategy_request_merge_keeps_different_floors_separate() {
    let mut early = test_experiment_request(
        "route_policy_gap",
        "Map",
        "route planner declined automatic map selection",
    );
    early.act = 1;
    early.floor = 5;
    early.boundary_details = vec!["Map: current=(5, 4)".to_string()];
    let mut late = test_experiment_request(
        "route_policy_gap",
        "Map",
        "route planner declined automatic map selection",
    );
    late.act = 1;
    late.floor = 10;
    late.boundary_details = vec!["Map: current=(3, 9)".to_string()];

    let merged = merge_campaign_strategy_requests_v1(vec![early, late]);

    assert_eq!(merged.len(), 2);
    assert!(merged.iter().any(|request| {
        request.floor == 5
            && request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(5, 4)"))
            && !request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(3, 9)"))
    }));
    assert!(merged.iter().any(|request| {
        request.floor == 10
            && request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(3, 9)"))
            && !request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(5, 4)"))
    }));
}

#[test]
fn campaign_strategy_request_merge_sanitizes_combat_suggestion() {
    let mut request = test_experiment_request(
        "combat_manual_or_budget",
        "Combat",
        "combat search did not find an executable complete win",
    );
    request.suggested_action =
        "raise combat search budget, relax hp-loss gate, or provide a manual combat line"
            .to_string();

    let merged = merge_campaign_strategy_requests_v1(vec![request]);

    assert!(!merged[0].suggested_action.contains("hp-loss"));
    assert!(!merged[0].suggested_action.contains("relax"));
}

#[test]
fn combat_campaign_next_step_no_longer_mentions_hp_loss_gate() {
    let next = campaign_strategy_next_step_v1("combat_manual_or_budget").expect("next step");

    assert!(!next.contains("hp-loss"));
    assert!(!next.contains("relax"));
}

#[test]
fn compact_campaign_report_renders_budget_stop_hint() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 7, 80)],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Next: budget ended; use .\\tools\\campaign.ps1 -More"));
}

#[test]
fn compact_campaign_report_labels_nonfatal_requests_as_deferred_notes() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 7, 80)],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "route_policy_gap".to_string(),
            boundary_title: "Map".to_string(),
            branch_count: 2,
            act: 0,
            floor: 0,
            stop_reasons: vec!["route planner declined automatic map selection".to_string()],
            examples: vec!["Golden Idol -> Leave".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "adjust route planner policy".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(!rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("Queued interventions:"));
}

#[test]
fn compact_campaign_report_renders_context_only_strategy_packet() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 4,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "event_strategy".to_string(),
            boundary_title: "GoldenIdol".to_string(),
            branch_count: 2,
            act: 1,
            floor: 5,
            stop_reasons: vec!["event policy gap".to_string()],
            examples: vec!["Shockwave -> Clash".to_string()],
            next_card_reward_offer: Some(vec![
                "Pommel Strike".to_string(),
                "Iron Wave".to_string(),
                "Searing Blow".to_string(),
            ]),
            boundary_details: vec![
                "option: [Take] Obtain Golden Idol.".to_string(),
                "option: [Leave] Leave.".to_string(),
            ],
            suggested_action: "provide event policy".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("context: A1F5"));
    assert!(rendered.contains("next reward offer: Pommel Strike | Iron Wave | Searing Blow"));
    assert!(rendered.contains("detail: option: [Take] Obtain Golden Idol."));
    assert!(!rendered.contains("pick Golden Idol"));
}
