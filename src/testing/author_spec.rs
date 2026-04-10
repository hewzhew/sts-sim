use serde::Deserialize;
use serde_json::{json, Value};

use crate::content::cards::get_card_definition;
use crate::diff::mapper::card_id_from_java;

use super::scenario::{
    ScenarioAssertion, ScenarioCardSelector, ScenarioFixture, ScenarioKind, ScenarioOracleKind,
    ScenarioProvenance, ScenarioStep, StructuredScenarioStep,
};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CombatAuthorSpec {
    pub name: String,
    #[serde(default = "default_player_class")]
    pub player_class: String,
    #[serde(default = "default_room_type")]
    pub room_type: String,
    #[serde(default = "default_turn")]
    pub turn: u32,
    #[serde(default)]
    pub player: AuthorPlayerSpec,
    #[serde(default)]
    pub monsters: Vec<AuthorMonsterSpec>,
    #[serde(default)]
    pub hand: Vec<AuthorCardSpec>,
    #[serde(default)]
    pub draw_pile: Vec<AuthorCardSpec>,
    #[serde(default)]
    pub discard_pile: Vec<AuthorCardSpec>,
    #[serde(default)]
    pub exhaust_pile: Vec<AuthorCardSpec>,
    #[serde(default)]
    pub relics: Vec<AuthorRelicSpec>,
    #[serde(default)]
    pub steps: Vec<AuthorStepSpec>,
    #[serde(default, alias = "expect")]
    pub assertions: Vec<AuthorAssertionSpec>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub provenance: Option<ScenarioProvenance>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthorPlayerSpec {
    #[serde(default)]
    pub current_hp: Option<i32>,
    #[serde(default)]
    pub max_hp: Option<i32>,
    #[serde(default)]
    pub block: Option<i32>,
    #[serde(default)]
    pub energy: Option<u8>,
    #[serde(default)]
    pub gold: Option<i32>,
    #[serde(default)]
    pub powers: Vec<AuthorPowerSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorMonsterSpec {
    pub id: String,
    pub current_hp: i32,
    #[serde(default)]
    pub max_hp: Option<i32>,
    #[serde(default)]
    pub block: Option<i32>,
    #[serde(default)]
    pub powers: Vec<AuthorPowerSpec>,
    #[serde(default = "default_intent")]
    pub intent: String,
    #[serde(default = "default_move_base_damage")]
    pub move_base_damage: i32,
    #[serde(default = "default_move_adjusted_damage")]
    pub move_adjusted_damage: i32,
    #[serde(default = "default_move_hits")]
    pub move_hits: i32,
    #[serde(default)]
    pub move_id: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPowerSpec {
    pub id: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AuthorCardSpec {
    Simple(String),
    Detailed(AuthorCardEntry),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorCardEntry {
    pub id: String,
    #[serde(default)]
    pub upgrades: u8,
    #[serde(default)]
    pub cost: Option<i32>,
    #[serde(default)]
    pub misc: Option<i32>,
    #[serde(default = "default_count")]
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AuthorRelicSpec {
    Simple(String),
    Detailed(AuthorRelicEntry),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorRelicEntry {
    pub id: String,
    #[serde(default = "default_relic_counter")]
    pub counter: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AuthorStepSpec {
    Raw(String),
    Command {
        command: String,
    },
    Play {
        play: AuthorPlayStep,
    },
    End {
        end: bool,
    },
    PotionUse {
        potion_use: AuthorPotionUseStep,
    },
    HumanCardReward {
        human_card_reward: AuthorHumanCardRewardStep,
    },
    HandSelect {
        hand_select: AuthorSelectCardsStep,
    },
    GridSelect {
        grid_select: AuthorSelectCardsStep,
    },
    Cancel {
        cancel: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPlayStep {
    pub card: AuthorCardSelectorSpec,
    #[serde(default)]
    pub target: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPotionUseStep {
    pub slot: usize,
    #[serde(default)]
    pub target: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorHumanCardRewardStep {
    #[serde(default)]
    pub choice: Option<usize>,
    #[serde(default)]
    pub skip: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorSelectCardsStep {
    pub cards: Vec<AuthorCardSelectorSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AuthorCardSelectorSpec {
    Index(usize),
    SimpleId(String),
    Detailed(AuthorCardSelectorEntry),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorCardSelectorEntry {
    pub id: String,
    #[serde(default = "default_occurrence")]
    pub occurrence: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AuthorAssertionSpec {
    Field(AuthorFieldAssertionSpec),
    MonsterCount {
        monster_count: i64,
        #[serde(default)]
        note: Option<String>,
    },
    PileContains {
        pile_contains: AuthorPileContainsSpec,
        #[serde(default)]
        note: Option<String>,
    },
    PileCount {
        pile_count: AuthorPileCountSpec,
        #[serde(default)]
        note: Option<String>,
    },
    PileSize {
        pile_size: AuthorPileSizeSpec,
        #[serde(default)]
        note: Option<String>,
    },
    PlayerStat {
        player_stat: AuthorPlayerStatAssertionSpec,
        #[serde(default)]
        note: Option<String>,
    },
    PlayerPower {
        player_power: AuthorPowerAssertionTargetSpec,
        #[serde(default)]
        note: Option<String>,
    },
    MonsterStat {
        monster_stat: AuthorMonsterStatAssertionSpec,
        #[serde(default)]
        note: Option<String>,
    },
    MonsterPower {
        monster_power: AuthorMonsterPowerAssertionSpec,
        #[serde(default)]
        note: Option<String>,
    },
    MonsterMissing {
        monster_missing: usize,
        #[serde(default)]
        note: Option<String>,
    },
    HasRelic {
        has_relic: String,
        #[serde(default)]
        note: Option<String>,
    },
    RelicCount {
        relic_count: AuthorRelicCountSpec,
        #[serde(default)]
        note: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorFieldAssertionSpec {
    pub field: String,
    #[serde(default)]
    pub number: Option<i64>,
    #[serde(default)]
    pub string: Option<String>,
    #[serde(default)]
    pub bool: Option<bool>,
    #[serde(default)]
    pub missing: Option<bool>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPileContainsSpec {
    pub pile: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPileCountSpec {
    pub pile: String,
    pub id: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPileSizeSpec {
    pub pile: String,
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPlayerStatAssertionSpec {
    pub stat: String,
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorPowerAssertionTargetSpec {
    pub id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorMonsterStatAssertionSpec {
    pub monster: usize,
    pub stat: String,
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorMonsterPowerAssertionSpec {
    pub monster: usize,
    pub id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorRelicCountSpec {
    pub id: String,
    pub count: i64,
}

pub fn compile_combat_author_spec(spec: &CombatAuthorSpec) -> Result<ScenarioFixture, String> {
    let player_max_hp = spec
        .player
        .max_hp
        .unwrap_or_else(|| base_max_hp_for_class(&spec.player_class));
    let player_current_hp = spec.player.current_hp.unwrap_or(player_max_hp);

    let hand = expand_cards(&spec.hand, "hand", false)?;
    let draw_pile = expand_cards(&spec.draw_pile, "draw", true)?;
    let discard_pile = expand_cards(&spec.discard_pile, "discard", false)?;
    let exhaust_pile = expand_cards(&spec.exhaust_pile, "exhaust", false)?;

    let monsters = spec
        .monsters
        .iter()
        .map(|monster| {
            Ok(json!({
                "id": monster.id,
                "current_hp": monster.current_hp,
                "max_hp": monster.max_hp.unwrap_or(monster.current_hp),
                "block": monster.block.unwrap_or(0),
                "powers": compile_powers(&monster.powers),
                "intent": monster.intent,
                "move_base_damage": monster.move_base_damage,
                "move_adjusted_damage": monster.move_adjusted_damage,
                "move_hits": monster.move_hits,
                "move_id": monster.move_id,
                "is_gone": false,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let initial_game_state = json!({
        "class": spec.player_class,
        "room_type": spec.room_type,
        "relics": compile_relics(&spec.relics),
        "potions": [],
        "combat_state": {
            "turn": spec.turn,
            "room_type": spec.room_type,
            "player": {
                "current_hp": player_current_hp,
                "max_hp": player_max_hp,
                "block": spec.player.block.unwrap_or(0),
                "energy": spec.player.energy.unwrap_or(3),
                "gold": spec.player.gold.unwrap_or(99),
                "powers": compile_powers(&spec.player.powers),
            },
            "monsters": monsters,
            "hand": hand,
            "draw_pile": draw_pile,
            "discard_pile": discard_pile,
            "exhaust_pile": exhaust_pile,
            "potions": [],
        }
    });

    Ok(ScenarioFixture {
        name: spec.name.clone(),
        kind: ScenarioKind::Combat,
        oracle_kind: ScenarioOracleKind::Synthetic,
        initial_game_state,
        initial_protocol_meta: None,
        steps: spec
            .steps
            .iter()
            .map(compile_step)
            .collect::<Result<Vec<_>, _>>()?,
        assertions: spec
            .assertions
            .iter()
            .map(compile_assertion)
            .collect::<Result<Vec<_>, _>>()?,
        provenance: spec.provenance.clone(),
        tags: spec.tags.clone(),
    })
}

fn expand_cards(
    specs: &[AuthorCardSpec],
    zone_prefix: &str,
    reverse_output: bool,
) -> Result<Vec<Value>, String> {
    let mut cards = Vec::new();
    let mut zone_index = 0usize;
    for spec in specs {
        let entry = match spec {
            AuthorCardSpec::Simple(id) => AuthorCardEntry {
                id: id.clone(),
                upgrades: 0,
                cost: None,
                misc: None,
                count: 1,
            },
            AuthorCardSpec::Detailed(entry) => entry.clone(),
        };
        if entry.count == 0 {
            continue;
        }
        let card_id = card_id_from_java(&entry.id)
            .ok_or_else(|| format!("unknown Java card id '{}'", entry.id))?;
        let def = get_card_definition(card_id);
        for local_index in 0..entry.count {
            cards.push(json!({
                "id": entry.id,
                "uuid": format!("{zone_prefix}-{zone_index}-{local_index}"),
                "upgrades": entry.upgrades,
                "cost": entry.cost.unwrap_or(def.cost as i32),
                "misc": entry.misc.unwrap_or(0),
            }));
        }
        zone_index += 1;
    }
    if reverse_output {
        cards.reverse();
    }
    Ok(cards)
}

fn compile_powers(powers: &[AuthorPowerSpec]) -> Vec<Value> {
    powers
        .iter()
        .map(|power| {
            json!({
                "id": power.id,
                "amount": power.amount,
            })
        })
        .collect()
}

fn compile_relics(relics: &[AuthorRelicSpec]) -> Vec<Value> {
    relics
        .iter()
        .map(|relic| match relic {
            AuthorRelicSpec::Simple(id) => json!({
                "id": id,
                "counter": -1,
            }),
            AuthorRelicSpec::Detailed(entry) => json!({
                "id": entry.id,
                "counter": entry.counter,
            }),
        })
        .collect()
}

fn compile_step(step: &AuthorStepSpec) -> Result<ScenarioStep, String> {
    let command = match step {
        AuthorStepSpec::Raw(command) => ScenarioStep {
            command: command.clone(),
            ..Default::default()
        },
        AuthorStepSpec::Command { command } => ScenarioStep {
            command: command.clone(),
            ..Default::default()
        },
        AuthorStepSpec::Play { play } => {
            let selector = compile_card_selector(&play.card)?;
            let command = describe_play_step(&selector, play.target);
            ScenarioStep {
                command,
                structured: Some(StructuredScenarioStep::Play {
                    selector,
                    target: play.target,
                }),
                ..Default::default()
            }
        }
        AuthorStepSpec::End { end } => {
            if !end {
                return Err("structured end step must be {\"end\": true}".to_string());
            }
            ScenarioStep {
                command: "END".to_string(),
                structured: Some(StructuredScenarioStep::End),
                ..Default::default()
            }
        }
        AuthorStepSpec::PotionUse { potion_use } => {
            let mut command = format!("POTION USE {}", potion_use.slot);
            if let Some(target) = potion_use.target {
                command.push(' ');
                command.push_str(&target.to_string());
            }
            ScenarioStep {
                command,
                structured: Some(StructuredScenarioStep::PotionUse {
                    slot: potion_use.slot,
                    target: potion_use.target,
                }),
                ..Default::default()
            }
        }
        AuthorStepSpec::HumanCardReward { human_card_reward } => {
            let command = if human_card_reward.skip {
                "HUMAN_CARD_REWARD SKIP".to_string()
            } else if let Some(choice) = human_card_reward.choice {
                format!("HUMAN_CARD_REWARD {}", choice)
            } else {
                return Err(
                    "human_card_reward step must set either skip=true or choice=<index>"
                        .to_string(),
                );
            };
            ScenarioStep {
                command,
                structured: Some(if human_card_reward.skip {
                    StructuredScenarioStep::Cancel
                } else {
                    StructuredScenarioStep::Choose {
                        index: human_card_reward.choice.expect("choice already checked"),
                    }
                }),
                ..Default::default()
            }
        }
        AuthorStepSpec::HandSelect { hand_select } => {
            let selectors = compile_card_selectors(&hand_select.cards)?;
            ScenarioStep {
                command: format!("HAND_SELECT {}", describe_selectors(&selectors)),
                structured: Some(StructuredScenarioStep::HandSelect { selectors }),
                ..Default::default()
            }
        }
        AuthorStepSpec::GridSelect { grid_select } => {
            let selectors = compile_card_selectors(&grid_select.cards)?;
            ScenarioStep {
                command: format!("GRID_SELECT {}", describe_selectors(&selectors)),
                structured: Some(StructuredScenarioStep::GridSelect { selectors }),
                ..Default::default()
            }
        }
        AuthorStepSpec::Cancel { cancel } => {
            if !cancel {
                return Err("structured cancel step must be {\"cancel\": true}".to_string());
            }
            ScenarioStep {
                command: "CANCEL".to_string(),
                structured: Some(StructuredScenarioStep::Cancel),
                ..Default::default()
            }
        }
    };
    Ok(command)
}

fn compile_card_selectors(
    selectors: &[AuthorCardSelectorSpec],
) -> Result<Vec<ScenarioCardSelector>, String> {
    selectors.iter().map(compile_card_selector).collect()
}

fn compile_card_selector(
    selector: &AuthorCardSelectorSpec,
) -> Result<ScenarioCardSelector, String> {
    match selector {
        AuthorCardSelectorSpec::Index(index) => {
            if *index == 0 {
                Err("structured card selector uses 1-based index; got 0".to_string())
            } else {
                Ok(ScenarioCardSelector::Index { index: *index })
            }
        }
        AuthorCardSelectorSpec::SimpleId(id) => Ok(ScenarioCardSelector::JavaId {
            id: id.clone(),
            occurrence: 1,
        }),
        AuthorCardSelectorSpec::Detailed(entry) => {
            if entry.occurrence == 0 {
                return Err("structured card selector occurrence must be >= 1".to_string());
            }
            Ok(ScenarioCardSelector::JavaId {
                id: entry.id.clone(),
                occurrence: entry.occurrence,
            })
        }
    }
}

fn describe_play_step(selector: &ScenarioCardSelector, target: Option<usize>) -> String {
    let mut base = match selector {
        ScenarioCardSelector::Index { index } => format!("PLAY {}", index),
        ScenarioCardSelector::JavaId { id, occurrence } => {
            if *occurrence == 1 {
                format!("PLAY_ID {}", id)
            } else {
                format!("PLAY_ID {} #{}", id, occurrence)
            }
        }
    };
    if let Some(target) = target {
        base.push_str(&format!(" -> {}", target));
    }
    base
}

fn describe_selectors(selectors: &[ScenarioCardSelector]) -> String {
    selectors
        .iter()
        .map(|selector| match selector {
            ScenarioCardSelector::Index { index } => format!("#{index}"),
            ScenarioCardSelector::JavaId { id, occurrence } => {
                if *occurrence == 1 {
                    id.clone()
                } else {
                    format!("{id}#{occurrence}")
                }
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn compile_assertion(assertion: &AuthorAssertionSpec) -> Result<ScenarioAssertion, String> {
    match assertion {
        AuthorAssertionSpec::Field(assertion) => {
            let (expected_kind, expected_value) = if assertion.missing.unwrap_or(false) {
                ("missing".to_string(), None)
            } else if let Some(number) = assertion.number {
                ("number".to_string(), Some(json!(number)))
            } else if let Some(string) = &assertion.string {
                ("string".to_string(), Some(json!(string)))
            } else if let Some(value) = assertion.bool {
                ("bool".to_string(), Some(json!(value)))
            } else {
                return Err(format!(
                    "assertion for '{}' must specify one of number/string/bool/missing",
                    assertion.field
                ));
            };

            Ok(ScenarioAssertion {
                field: assertion.field.clone(),
                expected_kind,
                expected_value,
                note: assertion.note.clone(),
                ..Default::default()
            })
        }
        AuthorAssertionSpec::MonsterCount {
            monster_count,
            note,
        } => Ok(ScenarioAssertion {
            field: "monster_count".to_string(),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(monster_count)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::PileContains {
            pile_contains,
            note,
        } => Ok(ScenarioAssertion {
            field: format!(
                "{}.contains[{}]",
                normalize_pile_name(&pile_contains.pile)?,
                pile_contains.id
            ),
            expected_kind: "bool".to_string(),
            expected_value: Some(json!(true)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::PileCount { pile_count, note } => Ok(ScenarioAssertion {
            field: format!(
                "{}.count[{}]",
                normalize_pile_name(&pile_count.pile)?,
                pile_count.id
            ),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(pile_count.count)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::PileSize { pile_size, note } => Ok(ScenarioAssertion {
            field: format!("{}_size", normalize_pile_name(&pile_size.pile)?),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(pile_size.count)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::PlayerStat { player_stat, note } => Ok(ScenarioAssertion {
            field: format!("player.{}", normalize_player_stat_name(&player_stat.stat)?),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(player_stat.value)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::PlayerPower { player_power, note } => Ok(ScenarioAssertion {
            field: format!("player.power[{}].amount", player_power.id),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(player_power.amount)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::MonsterStat { monster_stat, note } => Ok(ScenarioAssertion {
            field: format!(
                "monster[{}].{}",
                monster_stat.monster,
                normalize_monster_stat_name(&monster_stat.stat)?
            ),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(monster_stat.value)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::MonsterPower {
            monster_power,
            note,
        } => Ok(ScenarioAssertion {
            field: format!(
                "monster[{}].power[{}].amount",
                monster_power.monster, monster_power.id
            ),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(monster_power.amount)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::MonsterMissing {
            monster_missing,
            note,
        } => Ok(ScenarioAssertion {
            field: format!("monster[{monster_missing}].hp"),
            expected_kind: "missing".to_string(),
            expected_value: None,
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::HasRelic { has_relic, note } => Ok(ScenarioAssertion {
            field: format!("relics.contains[{has_relic}]"),
            expected_kind: "bool".to_string(),
            expected_value: Some(json!(true)),
            note: note.clone(),
            ..Default::default()
        }),
        AuthorAssertionSpec::RelicCount { relic_count, note } => Ok(ScenarioAssertion {
            field: format!("relics.count[{}]", relic_count.id),
            expected_kind: "number".to_string(),
            expected_value: Some(json!(relic_count.count)),
            note: note.clone(),
            ..Default::default()
        }),
    }
}

fn normalize_pile_name(pile: &str) -> Result<&'static str, String> {
    match pile {
        "hand" => Ok("hand"),
        "draw" | "draw_pile" => Ok("draw_pile"),
        "discard" | "discard_pile" => Ok("discard_pile"),
        "exhaust" | "exhaust_pile" => Ok("exhaust_pile"),
        "limbo" => Ok("limbo"),
        other => Err(format!(
            "unsupported pile '{}' in structured assertion; expected hand/draw/discard/exhaust/limbo",
            other
        )),
    }
}

fn normalize_player_stat_name(stat: &str) -> Result<&'static str, String> {
    match stat {
        "hp" | "current_hp" => Ok("hp"),
        "block" => Ok("block"),
        "energy" => Ok("energy"),
        other => Err(format!(
            "unsupported player stat '{}' in structured assertion; expected hp/current_hp/block/energy",
            other
        )),
    }
}

fn normalize_monster_stat_name(stat: &str) -> Result<&'static str, String> {
    match stat {
        "hp" | "current_hp" => Ok("hp"),
        "block" => Ok("block"),
        other => Err(format!(
            "unsupported monster stat '{}' in structured assertion; expected hp/current_hp/block",
            other
        )),
    }
}

fn base_max_hp_for_class(player_class: &str) -> i32 {
    match player_class {
        "Silent" => 70,
        "Defect" => 75,
        "Watcher" => 72,
        _ => 80,
    }
}

fn default_player_class() -> String {
    "Ironclad".to_string()
}

fn default_room_type() -> String {
    "MonsterRoom".to_string()
}

fn default_turn() -> u32 {
    1
}

fn default_intent() -> String {
    "UNKNOWN".to_string()
}

fn default_move_base_damage() -> i32 {
    -1
}

fn default_move_adjusted_damage() -> i32 {
    -1
}

fn default_move_hits() -> i32 {
    1
}

fn default_count() -> usize {
    1
}

fn default_relic_counter() -> i32 {
    -1
}

fn default_occurrence() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::scenario::{assert_fixture, replay_fixture};

    #[test]
    fn compile_minimal_silent_spec_into_synthetic_fixture() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "silent_neutralize",
            "player_class": "Silent",
            "player": {
                "energy": 3
            },
            "monsters": [
                {"id": "JawWorm", "current_hp": 40}
            ],
            "hand": ["Neutralize"],
            "steps": ["PLAY 1 0"],
            "expect": [
                {"field": "monster[0].hp", "number": 37},
                {"field": "monster[0].power[Weakened].amount", "number": 1}
            ],
            "tags": ["silent", "starter"]
        }))
        .expect("author spec should parse");

        let fixture = compile_combat_author_spec(&spec).expect("spec should compile");
        assert_eq!(fixture.oracle_kind, ScenarioOracleKind::Synthetic);
        assert_eq!(fixture.tags, vec!["silent", "starter"]);
        assert_fixture(&fixture).expect("compiled fixture should pass");
    }

    #[test]
    fn draw_pile_author_order_is_top_first() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "draw_order",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "hand": ["Pommel Strike"],
            "draw_pile": ["Strike_R", "Defend_R"],
            "steps": ["PLAY 1 0"],
            "expect": [
                {"field": "hand_size", "number": 1}
            ]
        }))
        .expect("author spec should parse");

        let fixture = compile_combat_author_spec(&spec).expect("spec should compile");
        let replay = replay_fixture(&fixture).expect("fixture should replay");
        assert_eq!(
            replay
                .combat
                .zones
                .hand
                .iter()
                .map(|card| crate::content::cards::java_id(card.id))
                .collect::<Vec<_>>(),
            vec!["Strike_R"]
        );
    }

    #[test]
    fn assertion_requires_one_expected_variant() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "bad_assertion",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "assertions": [{"field": "hand_size"}]
        }))
        .expect("author spec should parse");

        let err = compile_combat_author_spec(&spec).expect_err("assertion should fail");
        assert!(err.contains("must specify one of number/string/bool/missing"));
    }

    #[test]
    fn structured_steps_compile_into_commands() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "structured_steps",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "hand": ["Neutralize"],
            "steps": [
                { "play": { "card": 1, "target": 0 } },
                { "end": true }
            ],
            "expect": [
                { "field": "monster[0].hp", "number": 37 }
            ]
        }))
        .expect("author spec should parse");

        let fixture = compile_combat_author_spec(&spec).expect("spec should compile");
        assert_eq!(fixture.steps[0].command, "PLAY 1 -> 0");
        assert_eq!(fixture.steps[1].command, "END");
        assert!(fixture.steps[0].structured.is_some());
    }

    #[test]
    fn structured_play_rejects_zero_card_index() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "bad_play",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "steps": [
                { "play": { "card": 0, "target": 0 } }
            ],
            "expect": []
        }))
        .expect("author spec should parse");

        let err = compile_combat_author_spec(&spec).expect_err("spec should fail");
        assert!(err.contains("1-based index"));
    }

    #[test]
    fn play_by_java_id_and_hand_select_by_java_id_compile_and_replay() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "play_by_id",
            "player_class": "Silent",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "hand": ["Acrobatics", "Survivor"],
            "draw_pile": ["Strike_R", "Defend_R", "Bash"],
            "steps": [
                { "play": { "card": "Acrobatics" } },
                { "hand_select": { "cards": ["Survivor"] } }
            ],
            "expect": [
                { "field": "discard_pile_size", "number": 2 }
            ]
        }))
        .expect("author spec should parse");

        let fixture = compile_combat_author_spec(&spec).expect("spec should compile");
        assert!(fixture.steps[0].command.starts_with("PLAY_ID Acrobatics"));
        assert!(fixture.steps[1].command.starts_with("HAND_SELECT Survivor"));
        assert_fixture(&fixture).expect("compiled fixture should pass");
    }

    #[test]
    fn structured_assertions_compile_into_low_level_fields() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "structured_assertions",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "relics": ["Anchor"],
            "assertions": [
                { "monster_count": 1 },
                { "pile_contains": { "pile": "hand", "id": "Strike_R" } },
                { "pile_count": { "pile": "draw", "id": "Defend_R", "count": 2 } },
                { "pile_size": { "pile": "draw", "count": 2 } },
                { "player_stat": { "stat": "energy", "value": 3 } },
                { "player_power": { "id": "Strength", "amount": 2 } },
                { "monster_stat": { "monster": 0, "stat": "hp", "value": 40 } },
                { "monster_power": { "monster": 0, "id": "Poison", "amount": 5 } },
                { "monster_missing": 1 },
                { "has_relic": "Anchor" },
                { "relic_count": { "id": "Anchor", "count": 1 } }
            ],
            "hand": ["Strike_R"],
            "draw_pile": [
                "Defend_R",
                "Defend_R"
            ],
            "player": {
                "powers": [{ "id": "Strength", "amount": 2 }]
            }
        }))
        .expect("author spec should parse");

        let fixture = compile_combat_author_spec(&spec).expect("spec should compile");
        assert_eq!(fixture.assertions[0].field, "monster_count");
        assert_eq!(fixture.assertions[0].expected_kind, "number");
        assert_eq!(fixture.assertions[1].field, "hand.contains[Strike_R]");
        assert_eq!(fixture.assertions[1].expected_kind, "bool");
        assert_eq!(fixture.assertions[2].field, "draw_pile.count[Defend_R]");
        assert_eq!(fixture.assertions[3].field, "draw_pile_size");
        assert_eq!(fixture.assertions[4].field, "player.energy");
        assert_eq!(fixture.assertions[5].field, "player.power[Strength].amount");
        assert_eq!(fixture.assertions[6].field, "monster[0].hp");
        assert_eq!(
            fixture.assertions[7].field,
            "monster[0].power[Poison].amount"
        );
        assert_eq!(fixture.assertions[8].field, "monster[1].hp");
        assert_eq!(fixture.assertions[8].expected_kind, "missing");
        assert_eq!(fixture.assertions[9].field, "relics.contains[Anchor]");
        assert_eq!(fixture.assertions[10].field, "relics.count[Anchor]");
    }

    #[test]
    fn structured_assertions_replay_with_piles_powers_and_relics() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "structured_assertions_replay",
            "player_class": "Silent",
            "player": {
                "powers": [{ "id": "Strength", "amount": 2 }]
            },
            "monsters": [{
                "id": "JawWorm",
                "current_hp": 40,
                "powers": [{ "id": "Poison", "amount": 5 }]
            }],
            "relics": ["Anchor"],
            "hand": ["Strike_G"],
            "draw_pile": ["Defend_G", "Defend_G"],
            "expect": [
                { "monster_count": 1 },
                { "pile_contains": { "pile": "hand", "id": "Strike_G" } },
                { "pile_count": { "pile": "draw_pile", "id": "Defend_G", "count": 2 } },
                { "pile_size": { "pile": "draw_pile", "count": 2 } },
                { "player_stat": { "stat": "hp", "value": 70 } },
                { "player_stat": { "stat": "energy", "value": 3 } },
                { "player_power": { "id": "Strength", "amount": 2 } },
                { "monster_stat": { "monster": 0, "stat": "hp", "value": 40 } },
                { "monster_stat": { "monster": 0, "stat": "block", "value": 0 } },
                { "monster_power": { "monster": 0, "id": "Poison", "amount": 5 } },
                { "monster_missing": 1 },
                { "has_relic": "Anchor" },
                { "relic_count": { "id": "Anchor", "count": 1 } }
            ]
        }))
        .expect("author spec should parse");

        let fixture = compile_combat_author_spec(&spec).expect("spec should compile");
        assert_fixture(&fixture).expect("compiled fixture should pass");
    }

    #[test]
    fn structured_assertion_rejects_unknown_pile_alias() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "bad_structured_assertion",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "assertions": [
                { "pile_contains": { "pile": "stash", "id": "Strike_R" } }
            ]
        }))
        .expect("author spec should parse");

        let err = compile_combat_author_spec(&spec).expect_err("assertion should fail");
        assert!(err.contains("unsupported pile"));
    }

    #[test]
    fn structured_stat_assertion_rejects_unknown_stat() {
        let spec: CombatAuthorSpec = serde_json::from_value(json!({
            "name": "bad_structured_stat",
            "monsters": [{"id": "JawWorm", "current_hp": 40}],
            "assertions": [
                { "player_stat": { "stat": "gold", "value": 99 } }
            ]
        }))
        .expect("author spec should parse");

        let err = compile_combat_author_spec(&spec).expect_err("assertion should fail");
        assert!(err.contains("unsupported player stat"));
    }
}
