// Mechanical split from combat_case.rs. Test module only.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixtures::author_spec::CombatAuthorSpec;
    use std::fs;

    #[test]
    fn protocol_snapshot_case_round_trips() {
        let case = CombatCase {
            id: "roundtrip".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::ProtocolSnapshot(CombatCaseProtocolSnapshotBasis {
                combat_truth: serde_json::json!({"turn": 1, "player": {"current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": []}, "monsters": [], "hand": [], "draw_pile": [], "discard_pile": [], "exhaust_pile": [], "limbo": [], "card_queue": [], "potions": []}),
                combat_observation: serde_json::json!({"player": {"current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": []}, "monsters": [], "hand": [], "discard_pile": [], "exhaust_pile": [], "limbo": [], "draw_pile_count": 0}),
                relics: serde_json::json!([]),
                protocol_meta: Some(serde_json::json!({"response_id": 1, "state_frame_id": 1})),
                root_meta: CombatCaseRootMeta {
                    player_class: Some("Ironclad".to_string()),
                    ascension_level: Some(0),
                    seed_hint: Some(1),
                    screen_type: Some("NONE".to_string()),
                    screen_state: Some(serde_json::json!({})),
                },
            }),
            delta: CombatCaseDelta::default(),
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec!["roundtrip".to_string()],
        };

        let payload = serde_json::to_string(&case).expect("serialize case");
        let decoded: CombatCase = serde_json::from_str(&payload).expect("deserialize case");
        assert_eq!(decoded.id, case.id);
        match decoded.basis {
            CombatCaseBasis::ProtocolSnapshot(_) => {}
            other => panic!("expected protocol_snapshot basis, got {other:?}"),
        }
    }

    #[test]
    fn compile_author_case_replays() {
        let spec: CombatAuthorSpec = serde_json::from_value(serde_json::json!({
            "name": "jaw_worm_strike",
            "player_class": "Ironclad",
            "room_type": "MonsterRoom",
            "turn": 1,
            "player": { "energy": 3 },
            "monsters": [{ "id": "JawWorm", "current_hp": 40, "intent": "ATTACK", "move_adjusted_damage": 11, "move_base_damage": 11, "move_hits": 1 }],
            "hand": ["Strike_R"],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "steps": [{ "play": { "card": 1, "target": 0 } }],
            "assertions": [{ "monster_stat": { "monster": 0, "stat": "hp", "value": 34 } }]
        }))
        .expect("author spec");

        let case = compile_combat_author_case(&spec).expect("compile case");
        assert_case(&case).expect("author case should replay");
    }

    #[test]
    fn encounter_template_case_lowers() {
        let case = CombatCase {
            id: "lagavulin_start".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::EncounterTemplate(CombatCaseEncounterTemplateBasis {
                player_class: "Ironclad".to_string(),
                ascension_level: 0,
                encounter_id: "lagavulin".to_string(),
                room_type: "elite".to_string(),
                seed_hint: 7,
                player_current_hp: Some(80),
                player_max_hp: Some(80),
                relics: vec![],
                potions: vec![],
                master_deck: vec![
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Bash".to_string()),
                ],
            }),
            delta: CombatCaseDelta::default(),
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };

        let lowered = lower_case(&case).expect("lower encounter template");
        assert_eq!(lowered.combat.meta.player_class, "Ironclad");
        assert!(!lowered.combat.entities.monsters.is_empty());
    }

    #[test]
    fn protocol_snapshot_case_reduces_to_encounter_template() {
        let spec: CombatAuthorSpec = serde_json::from_value(serde_json::json!({
            "name": "jaw_worm_reduce",
            "player_class": "Ironclad",
            "room_type": "MonsterRoom",
            "turn": 1,
            "player": { "energy": 3, "block": 0 },
            "monsters": [{ "id": "JawWorm", "current_hp": 40, "intent": "ATTACK", "move_adjusted_damage": 11, "move_base_damage": 11, "move_hits": 1 }],
            "hand": ["Strike_R", "Defend_R"],
            "draw_pile": ["Strike_R", "Defend_R", "Bash"],
            "discard_pile": [],
            "exhaust_pile": [],
            "steps": [{ "play": { "card": 1, "target": 0 } }],
            "assertions": [{ "monster_stat": { "monster": 0, "stat": "hp", "value": 34 } }]
        }))
        .expect("author spec");

        let case = compile_combat_author_case(&spec).expect("compile case");
        let protocol = match &case.basis {
            CombatCaseBasis::ProtocolSnapshot(protocol) => protocol.clone(),
            other => panic!("expected protocol snapshot basis, got {other:?}"),
        };
        let template = infer_encounter_template_basis(&protocol)
            .expect("infer template basis")
            .expect("template basis should be inferable");
        let candidate = CombatCase {
            basis: CombatCaseBasis::EncounterTemplate(template),
            delta: build_encounter_template_delta(&protocol).expect("build delta"),
            ..case.clone()
        };
        if let Err(err) = assert_case(&candidate) {
            panic!("candidate encounter_template case should replay: {err}");
        }
        let reduced = CombatCaseReducer::reduce(&case).expect("reduce case");
        match reduced.basis {
            CombatCaseBasis::EncounterTemplate(_) => {}
            other => panic!("expected encounter_template reduction, got {other:?}"),
        }
        assert_case(&reduced).expect("reduced case should replay");
    }

    #[test]
    fn protocol_snapshot_reduction_preserves_runtime_rng_and_truth_relics() {
        let protocol = CombatCaseProtocolSnapshotBasis {
            combat_truth: serde_json::json!({
                "turn": 1,
                "room_type": "MonsterRoom",
                "player": {
                    "current_hp": 85,
                    "max_hp": 87,
                    "block": 5,
                    "energy": 0,
                    "powers": []
                },
                "monsters": [{
                    "id": "SlaverBlue",
                    "current_hp": 33,
                    "max_hp": 50,
                    "block": 0,
                    "move_id": 1,
                    "powers": []
                }],
                "hand": [{
                    "id": "Strike_R",
                    "uuid": "00000000-0000-0000-0000-000000000001",
                    "upgrades": 0,
                    "cost": 1
                }],
                "draw_pile": [{
                    "id": "Bash",
                    "uuid": "00000000-0000-0000-0000-000000000002",
                    "upgrades": 0,
                    "cost": 2
                }],
                "discard_pile": [],
                "exhaust_pile": [],
                "limbo": [],
                "card_queue": [],
                "potions": [
                    { "id": "ColorlessPotion" },
                    { "id": "Potion Slot" },
                    { "id": "Potion Slot" }
                ],
                "relics": [
                    { "id": "Toy Ornithopter" }
                ],
                "colorless_combat_pool": [
                    { "id": "Madness" },
                    { "id": "Discovery" }
                ],
                "rng_state": {
                    "card_rng": {
                        "seed0": 11,
                        "seed1": 29,
                        "counter": 7
                    }
                }
            }),
            combat_observation: serde_json::json!({
                "player": {
                    "current_hp": 85,
                    "max_hp": 87,
                    "block": 5,
                    "energy": 0,
                    "powers": []
                },
                "monsters": [{
                    "id": "SlaverBlue",
                    "current_hp": 33,
                    "max_hp": 50,
                    "block": 0,
                    "powers": []
                }],
                "hand": [{
                    "id": "Strike_R",
                    "uuid": "00000000-0000-0000-0000-000000000001",
                    "upgrades": 0,
                    "cost": 1
                }],
                "discard_pile": [],
                "exhaust_pile": [],
                "limbo": [],
                "draw_pile_count": 1
            }),
            relics: serde_json::json!([]),
            protocol_meta: Some(serde_json::json!({
                "response_id": 158,
                "state_frame_id": 158
            })),
            root_meta: CombatCaseRootMeta {
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                seed_hint: Some(42),
                screen_type: Some("NONE".to_string()),
                screen_state: Some(serde_json::json!({})),
            },
        };

        let template = infer_encounter_template_basis(&protocol)
            .expect("infer template basis")
            .expect("template basis should be inferable");
        assert_eq!(template.encounter_id, "blue_slaver");
        assert!(template.relics.iter().any(|spec| match spec {
            AuthorRelicSpec::Simple(id) => id == "Toy Ornithopter",
            AuthorRelicSpec::Detailed(entry) => entry.id == "Toy Ornithopter",
        }));

        let delta = build_encounter_template_delta(&protocol).expect("build delta");
        assert_eq!(
            delta
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.colorless_combat_pool.as_ref())
                .expect("colorless pool delta"),
            &vec!["Madness".to_string(), "Discovery".to_string()]
        );
        assert_eq!(
            delta
                .rng
                .as_ref()
                .and_then(|rng| rng.card_rng.as_ref())
                .and_then(|channel| channel.counter),
            Some(7)
        );

        let case = CombatCase {
            id: "blue_slaver_runtime_rng".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::EncounterTemplate(template),
            delta,
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };
        let lowered = lower_case(&case).expect("lower reduced case");
        assert_eq!(
            lowered.combat.runtime.colorless_combat_pool,
            vec![CardId::Madness, CardId::Discovery]
        );
        assert_eq!(lowered.combat.rng.card_random_rng.seed0, 11);
        assert_eq!(lowered.combat.rng.card_random_rng.seed1, 29);
        assert_eq!(lowered.combat.rng.card_random_rng.counter, 7);
        assert!(lowered
            .combat
            .entities
            .player
            .relics
            .iter()
            .any(|relic| relic.id == relic_id_from_java("Toy Ornithopter").unwrap()));
    }

    #[test]
    fn encounter_template_reduction_minimizes_redundant_delta_sections() {
        let mut case = CombatCase {
            id: "redundant_delta_minimize".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::EncounterTemplate(CombatCaseEncounterTemplateBasis {
                player_class: "IRONCLAD".to_string(),
                ascension_level: 0,
                encounter_id: "jaw_worm".to_string(),
                room_type: "MonsterRoom".to_string(),
                seed_hint: 7,
                player_current_hp: Some(80),
                player_max_hp: Some(80),
                relics: vec![AuthorRelicSpec::Simple("Burning Blood".to_string())],
                potions: vec![],
                master_deck: vec![
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Bash".to_string()),
                ],
            }),
            delta: CombatCaseDelta {
                relics: vec![CombatCaseRelicDelta {
                    id: "Burning Blood".to_string(),
                    counter: None,
                    used_up: None,
                    amount: None,
                }],
                zones: Some(CombatCaseZonesDelta {
                    hand: Some(vec![CombatCaseCardEntry {
                        id: "Strike_R".to_string(),
                        uuid: Some("00000000-0000-0000-0000-000000000001".to_string()),
                        upgrades: 0,
                        cost: None,
                        misc: None,
                        count: 1,
                    }]),
                    draw_pile: Some(vec![CombatCaseCardEntry {
                        id: "Bash".to_string(),
                        uuid: Some("00000000-0000-0000-0000-000000000002".to_string()),
                        upgrades: 0,
                        cost: None,
                        misc: None,
                        count: 1,
                    }]),
                    discard_pile: None,
                    exhaust_pile: None,
                    limbo: None,
                }),
                runtime: Some(CombatCaseRuntimeDelta {
                    colorless_combat_pool: Some(vec!["Madness".to_string()]),
                }),
                rng: Some(CombatCaseRngDelta {
                    card_rng: Some(CombatCaseRngChannel {
                        seed0: Some(11),
                        seed1: Some(29),
                        counter: Some(7),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };

        let baseline = lower_case(&case).expect("lower redundant case");
        let monster_hp = baseline.combat.entities.monsters[0].current_hp as i64;
        case.expectations.push(CombatCaseExpectation {
            check: CombatCaseCheck::MonsterStat {
                monster: 0,
                stat: "hp".to_string(),
                value: monster_hp,
            },
            response_id: None,
            frame_id: None,
            note: None,
        });

        let reduced = CombatCaseReducer::reduce(&case).expect("reduce redundant delta case");
        assert!(reduced.delta.relics.is_empty());
        assert!(reduced.delta.runtime.is_none());
        assert!(reduced.delta.rng.is_none());
        assert!(reduced.delta.zones.is_none());
        assert!(reduced
            .provenance
            .notes
            .iter()
            .any(|note| note == "minimized_encounter_template_delta"));
        assert_case(&reduced).expect("minimized case should still verify");
    }

    #[test]
    fn live_window_case_materializes_to_protocol_snapshot() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let raw_path = std::env::temp_dir().join(format!(
            "combat_case_live_window_{}_{}.jsonl",
            std::process::id(),
            nonce
        ));
        let raw_record = serde_json::json!({
            "game_state": {
                "class": "Ironclad",
                "ascension_level": 0,
                "seed": 42,
                "screen_type": "NONE",
                "screen_state": {},
                "combat_truth": {
                    "turn": 1,
                    "player": { "current_hp": 80, "max_hp": 80, "block": 0, "powers": [] },
                    "monsters": [],
                    "hand": [],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "card_queue": [],
                    "potions": [],
                    "relics": []
                },
                "combat_observation": {
                    "player": { "current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": [] },
                    "monsters": [],
                    "hand": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "draw_pile_count": 0
                },
                "relics": [],
                "potions": []
            },
            "protocol_meta": {
                "response_id": 10,
                "state_frame_id": 10
            }
        });
        fs::write(
            &raw_path,
            format!(
                "{}\n",
                serde_json::to_string(&raw_record).expect("serialize raw record")
            ),
        )
        .expect("write raw log");

        let case = CombatCase {
            id: "live_window".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::LiveWindow(CombatCaseLiveWindowBasis {
                raw_path: raw_path.display().to_string(),
                debug_path: None,
                from_response_id: 10,
                to_response_id: 10,
                failure_frame: Some(10),
                run_id: Some("test".to_string()),
                target_field: Some("player.current_hp".to_string()),
            }),
            delta: CombatCaseDelta::default(),
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Differential,
                evidence: vec![CombatCaseOracleKind::Differential],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };

        let materialized = CombatCaseReducer::materialize(&case).expect("materialize live window");
        match &materialized.basis {
            CombatCaseBasis::ProtocolSnapshot(protocol) => {
                assert_eq!(protocol.root_meta.player_class.as_deref(), Some("Ironclad"));
                assert_eq!(protocol.root_meta.seed_hint, Some(42));
            }
            other => panic!("expected protocol_snapshot basis, got {other:?}"),
        }
        let lowered = lower_case(&materialized).expect("lower materialized case");
        assert_eq!(lowered.player_class.as_deref(), Some("Ironclad"));

        let _ = fs::remove_file(&raw_path);
    }
}
