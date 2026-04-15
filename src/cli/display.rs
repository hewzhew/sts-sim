use crate::combat::CombatState;
use crate::content::cards;
use crate::content::cards::CardTarget;
use crate::content::monsters::EnemyId;
use crate::content::potions::PotionId;
use crate::content::powers::store::powers_for;
use crate::content::relics::RelicId;
use crate::map::node::RoomType;
use crate::state::core::EngineState;
use crate::state::run::RunState;
use crate::state::selection::{
    DomainCardSnapshot, DomainEvent, EngineDiagnostic, EngineDiagnosticClass, SelectionScope,
    SelectionTargetRef,
};

pub fn print_state(es: &EngineState, rs: &RunState, cs: &Option<CombatState>) {
    // Use combat HP if in combat, otherwise run-state HP
    let (hp, max_hp) = if let Some(combat) = cs {
        (
            combat.entities.player.current_hp,
            combat.entities.player.max_hp,
        )
    } else {
        (rs.current_hp, rs.max_hp)
    };
    println!(
        "--- Act {} Floor {} | HP: {}/{} | Gold: {} ---",
        rs.act_num, rs.floor_num, hp, max_hp, rs.gold
    );

    match es {
        EngineState::EventRoom => {
            if let Some(event) = &rs.event_state {
                let choices = crate::engine::event_handler::get_event_choices(rs);
                println!("  EVENT: {:?} (screen {})", event.id, event.current_screen);
                for (i, c) in choices.iter().enumerate() {
                    if c.disabled {
                        println!(
                            "    [{}] {} (Disabled: {})",
                            i,
                            c.text,
                            c.disabled_reason.as_deref().unwrap_or("")
                        );
                    } else {
                        println!("    [{}] {}", i, c.text);
                    }
                }
                println!("  → Type a number to choose");
            } else {
                println!("  EVENT ROOM (no event state?)");
            }
        }
        EngineState::MapNavigation => {
            println!("  MAP — Choose next node:");
            let choices = next_map_choices(rs);
            for choice in &choices {
                if choice.boss_transition {
                    println!("    [boss] {:?} (go 0, y={})", choice.room_type, choice.y);
                } else {
                    println!(
                        "    [go {}] {:?} (y={})",
                        choice.input_x, choice.room_type, choice.y
                    );
                }
            }
            if choices.is_empty() {
                println!(
                    "    (no adjacent nodes — floor {} y={})",
                    rs.floor_num, rs.map.current_y
                );
                if is_boss_transition(rs) {
                    println!("    Try: 'go 0' to enter the boss");
                } else {
                    println!("    Try: 'go 0' through 'go 6'");
                }
            }
        }
        EngineState::CombatPlayerTurn => {
            if let Some(cs) = cs {
                print_combat(cs);
                println!("  → 'play <idx> [target]', 'end', 'potion <slot> [target]', 'skip'");
            }
        }
        EngineState::PendingChoice(pc) => {
            if let Some(request) = pc.selection_request() {
                println!(
                    "  {} SELECT ({:?}): {}",
                    selection_scope_label(request.scope),
                    request.reason,
                    request.constraint.describe(request.targets.len())
                );
                for (i, target) in request.targets.iter().enumerate() {
                    if let Some(label) = describe_selection_target(target, request.scope, rs, cs) {
                        println!("    [{i}] {label}");
                    }
                }
            } else {
                println!("  PENDING CHOICE: {:?}", pc);
            }
            println!("  → 'choose <indices...>' or 'cancel'");
        }
        EngineState::RewardScreen(reward) => {
            println!("  REWARDS:");
            for (i, item) in reward.items.iter().enumerate() {
                println!("    [{}] {:?}", i, item);
            }
            if let Some(ref card_options) = reward.pending_card_choice {
                println!("  CARD CHOICE:");
                for (i, reward_card) in card_options.iter().enumerate() {
                    let def = cards::get_card_definition(reward_card.id);
                    println!(
                        "    [pick {}] {}{}",
                        i,
                        def.name,
                        if reward_card.upgrades > 0 {
                            format!("+{}", reward_card.upgrades)
                        } else {
                            String::new()
                        }
                    );
                }
                println!("  → 'pick <idx>' or 'skip'");
            } else {
                println!("  → 'claim <idx>' to take, 'skip'/'proceed' to leave");
            }
        }
        EngineState::Campfire => {
            println!("  CAMPFIRE:");
            println!("    [rest]  Heal {} HP", rs.max_hp * 30 / 100);
            println!("    [smith] Upgrade a card");
            println!("  → 'rest' or 'smith <deck_idx>'");
        }
        EngineState::Shop(shop) => {
            println!("  SHOP:");
            println!("  Cards:");
            for (i, sc) in shop.cards.iter().enumerate() {
                let def = cards::get_card_definition(sc.card_id);
                println!("    [buy card {}] {} — {} gold", i, def.name, sc.price);
            }
            println!("  Relics:");
            for (i, sr) in shop.relics.iter().enumerate() {
                println!(
                    "    [buy relic {}] {:?} — {} gold",
                    i, sr.relic_id, sr.price
                );
            }
            println!("  Potions:");
            for (i, sp) in shop.potions.iter().enumerate() {
                println!(
                    "    [buy potion {}] {:?} — {} gold",
                    i, sp.potion_id, sp.price
                );
            }
            if shop.purge_available {
                println!("  Purge: {} gold", shop.purge_cost);
            } else {
                println!("  Purge: (Sold Out)");
            }
            println!("  → 'buy card/relic/potion <idx>', 'purge <deck_idx>' (or 'purge'), 'leave'");
        }
        EngineState::RunPendingChoice(rpc) => {
            let request = rpc.selection_request(rs);
            println!(
                "  DECK SELECT ({:?}): {}",
                request.reason,
                request.constraint.describe(request.targets.len())
            );
            for (i, target) in request.targets.iter().enumerate() {
                if let Some(label) = describe_selection_target(target, SelectionScope::Deck, rs, cs)
                {
                    println!("    [{i}] {label}");
                }
            }
            println!("  → 'select <idx1> <idx2> ...' or 'cancel'");
        }
        EngineState::GameOver(result) => {
            println!("  GAME OVER: {:?}", result);
        }
        EngineState::BossRelicSelect(bs) => {
            println!("  BOSS RELIC SELECT: choose a reward!");
            for (i, r) in bs.relics.iter().enumerate() {
                println!("    [{}] {:?}", i, r);
            }
            println!("  → 'relic <idx>' or 'skip'");
        }
        EngineState::EventCombat(_) => {
            if let Some(cs) = cs {
                print_combat(cs);
                println!("  → 'play <idx> [target]', 'end', 'potion <slot> [target]', 'skip'");
            } else {
                println!("  EVENT COMBAT (awaiting initialization...)");
            }
        }
        _ => {
            println!("  State: {:?}", std::mem::discriminant(es));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapChoiceEntry {
    pub input_x: usize,
    pub room_type: Option<RoomType>,
    pub y: i32,
    pub boss_transition: bool,
}

pub fn is_boss_transition(rs: &RunState) -> bool {
    rs.map.current_y == 14
}

pub fn normalize_map_choice_x(rs: &RunState, x: usize) -> usize {
    if is_boss_transition(rs) {
        0
    } else {
        x
    }
}

pub fn next_map_choices(rs: &RunState) -> Vec<MapChoiceEntry> {
    let next_y = if rs.map.current_y == -1 {
        0
    } else {
        rs.map.current_y + 1
    };

    if is_boss_transition(rs) {
        return vec![MapChoiceEntry {
            input_x: 0,
            room_type: Some(RoomType::MonsterRoomBoss),
            y: next_y,
            boss_transition: true,
        }];
    }

    let mut choices = Vec::new();
    if next_y <= rs.map.graph.len() as i32 {
        for x in 0..7 {
            if rs.map.can_travel_to(x, next_y, false) {
                let room_type = if next_y < rs.map.graph.len() as i32 {
                    rs.map.graph[next_y as usize][x as usize].class
                } else {
                    Some(RoomType::MonsterRoomBoss)
                };
                choices.push(MapChoiceEntry {
                    input_x: x as usize,
                    room_type,
                    y: next_y,
                    boss_transition: false,
                });
            }
        }
    }
    choices
}

fn find_combat_card_by_uuid(cs: &CombatState, uuid: u32) -> Option<&crate::combat::CombatCard> {
    cs.zones
        .hand
        .iter()
        .chain(cs.zones.draw_pile.iter())
        .chain(cs.zones.discard_pile.iter())
        .chain(cs.zones.exhaust_pile.iter())
        .chain(cs.zones.limbo.iter())
        .find(|card| card.uuid == uuid)
}

fn selection_scope_label(scope: SelectionScope) -> &'static str {
    match scope {
        SelectionScope::Hand => "HAND",
        SelectionScope::Deck => "DECK",
        SelectionScope::Grid => "GRID",
    }
}

fn describe_selection_target(
    target: &SelectionTargetRef,
    scope: SelectionScope,
    rs: &RunState,
    cs: &Option<CombatState>,
) -> Option<String> {
    match target {
        SelectionTargetRef::CardUuid(uuid) => match scope {
            SelectionScope::Deck => {
                rs.master_deck
                    .iter()
                    .find(|card| card.uuid == *uuid)
                    .map(|card| {
                        describe_card_snapshot(&DomainCardSnapshot {
                            id: card.id,
                            upgrades: card.upgrades,
                            uuid: card.uuid,
                        })
                    })
            }
            SelectionScope::Hand | SelectionScope::Grid => cs
                .as_ref()
                .and_then(|combat| find_combat_card_by_uuid(combat, *uuid))
                .map(|card| {
                    describe_card_snapshot(&DomainCardSnapshot {
                        id: card.id,
                        upgrades: card.upgrades,
                        uuid: card.uuid,
                    })
                }),
        },
    }
}

pub fn describe_card_snapshot(card: &DomainCardSnapshot) -> String {
    let def = cards::get_card_definition(card.id);
    format!(
        "{}{} (uuid={})",
        def.name,
        if card.upgrades > 0 {
            format!("+{}", card.upgrades)
        } else {
            String::new()
        },
        card.uuid
    )
}

pub fn describe_monster_target(combat: &CombatState, target_id: usize) -> Option<String> {
    combat
        .entities
        .monsters
        .iter()
        .enumerate()
        .find(|(_, monster)| monster.id == target_id)
        .map(|(slot, monster)| {
            let monster_name = EnemyId::from_id(monster.monster_type)
                .map(|enemy| enemy.get_name())
                .unwrap_or("Unknown Monster");
            format!(
                "{} [slot {}, id {}, intent {:?}]",
                monster_name, slot, monster.id, monster.current_intent
            )
        })
}

pub fn describe_play_card_choice(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
) -> Option<String> {
    let card = combat.zones.hand.get(card_index)?;
    let def = cards::get_card_definition(card.id);
    let mut text = format!("Play {} [hand {}]", def.name, card_index);
    if matches!(def.target, CardTarget::Enemy) {
        if let Some(target_id) = target.and_then(|id| describe_monster_target(combat, id)) {
            text.push_str(&format!(" -> {target_id}"));
        } else {
            text.push_str(" -> <unresolved target>");
        }
    }
    Some(text)
}

pub fn describe_potion_use_choice(
    combat: &CombatState,
    potion_index: usize,
    target: Option<usize>,
) -> Option<String> {
    let potion = combat.entities.potions.get(potion_index)?.as_ref()?;
    let def = crate::content::potions::get_potion_definition(potion.id);
    let mut text = format!("Use {} [slot {}]", def.name, potion_index);
    if def.target_required {
        if let Some(target_id) = target.and_then(|id| describe_monster_target(combat, id)) {
            text.push_str(&format!(" -> {target_id}"));
        } else {
            text.push_str(" -> <unresolved target>");
        }
    }
    Some(text)
}

pub fn describe_bot_map_choice(rs: &RunState, x: usize) -> String {
    if is_boss_transition(rs) {
        "Boss Node".to_string()
    } else {
        format!("Node X={x}")
    }
}

fn signed_delta(delta: i32) -> String {
    if delta >= 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

fn describe_relic_id(relic_id: RelicId) -> String {
    let raw = format!("{relic_id:?}");
    let mut pretty = String::with_capacity(raw.len() + 4);
    let mut prev_lower_or_digit = false;
    for ch in raw.chars() {
        if ch.is_ascii_uppercase() && prev_lower_or_digit {
            pretty.push(' ');
        }
        pretty.push(ch);
        prev_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
    }
    pretty
}

fn describe_potion_id(potion_id: PotionId) -> String {
    crate::content::potions::get_potion_definition(potion_id)
        .name
        .to_string()
}

pub fn render_user_feed_event(event: &DomainEvent) -> String {
    match event {
        DomainEvent::RelicObtained { relic_id, .. } => {
            format!("  [RELIC] Obtained {}", describe_relic_id(*relic_id))
        }
        DomainEvent::RelicLost { relic_id, .. } => {
            format!("  [RELIC] Lost {}", describe_relic_id(*relic_id))
        }
        DomainEvent::GoldChanged {
            delta, new_total, ..
        } => {
            format!("  [GOLD] {} ({})", signed_delta(*delta), new_total)
        }
        DomainEvent::HpChanged {
            delta,
            current_hp,
            max_hp,
            ..
        } => format!(
            "  [HP] {} ({}/{})",
            signed_delta(*delta),
            current_hp,
            max_hp
        ),
        DomainEvent::MaxHpChanged {
            delta,
            current_hp,
            max_hp,
            ..
        } => format!(
            "  [MAX HP] {} ({}/{})",
            signed_delta(*delta),
            current_hp,
            max_hp
        ),
        DomainEvent::PotionObtained {
            potion_id, slot, ..
        } => format!(
            "  [POTION] Obtained {} (slot {})",
            describe_potion_id(*potion_id),
            slot
        ),
        DomainEvent::SelectionResolved {
            scope,
            reason,
            selected,
            ..
        } => format!("  [SELECT] {:?} {:?} x{}", scope, reason, selected.len()),
        DomainEvent::CardObtained { card, .. } => {
            format!(
                "  [OBTAIN] Added card to deck: {}",
                describe_card_snapshot(card)
            )
        }
        DomainEvent::CardRemoved { card, .. } => {
            format!(
                "  [REMOVE] Removed card from deck: {}",
                describe_card_snapshot(card)
            )
        }
        DomainEvent::CardUpgraded { before, after, .. } => format!(
            "  [UPGRADE] {} -> {}",
            describe_card_snapshot(before),
            describe_card_snapshot(after)
        ),
        DomainEvent::CardTransformed { before, after, .. } => format!(
            "  [TRANSFORM] {} -> {}",
            describe_card_snapshot(before),
            describe_card_snapshot(after)
        ),
        DomainEvent::CardsExhausted { cards, .. } => format!(
            "  [EXHAUST] {}",
            cards
                .iter()
                .map(describe_card_snapshot)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub fn render_engine_diagnostic(diagnostic: &EngineDiagnostic) -> String {
    let prefix = match diagnostic.class {
        EngineDiagnosticClass::Normalization => "  [NORM]",
        EngineDiagnosticClass::Suspicious => "  [WARN]",
        EngineDiagnosticClass::Broken => "  [ERROR]",
    };
    format!("{prefix} {}", diagnostic.message)
}

pub fn print_combat(cs: &CombatState) {
    println!(
        "  COMBAT — Turn {} | Energy: {}",
        cs.turn.turn_count, cs.turn.energy
    );

    println!(
        "  Player: HP {}/{} Block {} ",
        cs.entities.player.current_hp, cs.entities.player.max_hp, cs.entities.player.block
    );
    if let Some(powers_list) = powers_for(cs, cs.entities.player.id) {
        if !powers_list.is_empty() {
            let powers: Vec<String> = powers_list
                .iter()
                .map(|p| format!("{:?}({})", p.power_type, p.amount))
                .collect();
            println!("    Powers: {}", powers.join(", "));
        }
    }

    let hide_intents = cs.entities.player.has_relic(RelicId::RunicDome);

    for m in &cs.entities.monsters {
        if m.is_dying {
            continue;
        }

        let name = EnemyId::from_id(m.monster_type)
            .map(|eid| eid.get_name())
            .unwrap_or("Unknown");

        let intent_str = if hide_intents {
            "Hidden".to_string()
        } else {
            use crate::combat::Intent;
            match m.current_intent {
                Intent::Attack { hits, .. } => {
                    format!("Attack {{ damage: {}, hits: {} }}", m.intent_dmg, hits)
                }
                Intent::AttackBuff { hits, .. } => {
                    format!("AttackBuff {{ damage: {}, hits: {} }}", m.intent_dmg, hits)
                }
                Intent::AttackDebuff { hits, .. } => format!(
                    "AttackDebuff {{ damage: {}, hits: {} }}",
                    m.intent_dmg, hits
                ),
                Intent::AttackDefend { hits, .. } => format!(
                    "AttackDefend {{ damage: {}, hits: {} }}",
                    m.intent_dmg, hits
                ),
                _ => format!("{:?}", m.current_intent),
            }
        };

        println!(
            "  Monster[{}] {} (id={}): HP {}/{} Block {} Intent {}",
            m.slot, name, m.id, m.current_hp, m.max_hp, m.block, intent_str
        );
        if let Some(powers_list) = powers_for(cs, m.id) {
            if !powers_list.is_empty() {
                let powers: Vec<String> = powers_list
                    .iter()
                    .map(|p| format!("{:?}({})", p.power_type, p.amount))
                    .collect();
                println!("    Powers: {}", powers.join(", "));
            }
        }
    }

    // Potion readout
    if cs.entities.potions.iter().any(|p| p.is_some()) {
        let potion_strings: Vec<String> = cs
            .entities
            .potions
            .iter()
            .enumerate()
            .filter_map(|(idx, opt_p)| opt_p.as_ref().map(|p| format!("[{}] {:?}", idx, p.id)))
            .collect();
        println!("  Potions: {}", potion_strings.join(", "));
    }

    println!("  Hand ({}):", cs.zones.hand.len());
    for (i, card) in cs.zones.hand.iter().enumerate() {
        let def = cards::get_card_definition(card.id);
        let c_cost = card.get_cost();
        let cost_str = if c_cost >= 0 {
            format!("[{}]", c_cost)
        } else {
            "[X]".to_string()
        };
        println!(
            "    [{}] {} {} {}",
            i,
            cost_str,
            def.name,
            if card.upgrades > 0 { "+" } else { "" }
        );
    }

    println!(
        "  Draw: {} | Discard: {} | Exhaust: {}",
        cs.zones.draw_pile.len(),
        cs.zones.discard_pile.len(),
        cs.zones.exhaust_pile.len()
    );
}

pub fn print_detailed_state(es: &EngineState, rs: &RunState, _cs: &Option<CombatState>) {
    println!("=== Detailed RunState ===");
    println!("  Deck ({} cards):", rs.master_deck.len());
    for (i, card) in rs.master_deck.iter().enumerate() {
        let def = cards::get_card_definition(card.id);
        println!(
            "    [{}] {} {}",
            i,
            def.name,
            if card.upgrades > 0 {
                format!("+{}", card.upgrades)
            } else {
                String::new()
            }
        );
    }
    println!("  Relics ({}):", rs.relics.len());
    for r in &rs.relics {
        println!("    {:?} (counter={})", r.id, r.counter);
    }
    println!("  Potions:");
    for (i, p) in rs.potions.iter().enumerate() {
        match p {
            Some(pot) => println!("    [{}] {:?}", i, pot.id),
            None => println!("    [{}] (empty)", i),
        }
    }
    println!("  Engine: {:?}", std::mem::discriminant(es));
}

#[cfg(test)]
mod tests {
    use super::{
        describe_bot_map_choice, describe_monster_target, describe_play_card_choice,
        next_map_choices, normalize_map_choice_x, render_user_feed_event,
    };
    use crate::combat::Intent;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::PotionId;
    use crate::content::relics::RelicId;
    use crate::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::map::state::MapState;
    use crate::state::events::EventId;
    use crate::state::run::RunState;
    use crate::state::selection::DomainEvent;
    use crate::state::selection::DomainEventSource;
    use crate::testing::support::test_support::{
        basic_combat, combat_with_hand_and_intent, CombatTestExt,
    };

    #[test]
    fn renders_run_level_outcome_events() {
        assert_eq!(
            render_user_feed_event(&DomainEvent::RelicObtained {
                relic_id: RelicId::BlackStar,
                source: DomainEventSource::Event(EventId::Neow),
            }),
            "  [RELIC] Obtained Black Star"
        );
        assert_eq!(
            render_user_feed_event(&DomainEvent::RelicLost {
                relic_id: RelicId::BurningBlood,
                source: DomainEventSource::Event(EventId::Neow),
            }),
            "  [RELIC] Lost Burning Blood"
        );
        assert_eq!(
            render_user_feed_event(&DomainEvent::GoldChanged {
                delta: 14,
                new_total: 113,
                source: DomainEventSource::RewardScreen,
            }),
            "  [GOLD] +14 (113)"
        );
        assert_eq!(
            render_user_feed_event(&DomainEvent::HpChanged {
                delta: -20,
                current_hp: 47,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::GoldenIdol),
            }),
            "  [HP] -20 (47/80)"
        );
        assert_eq!(
            render_user_feed_event(&DomainEvent::PotionObtained {
                potion_id: PotionId::BlockPotion,
                slot: 1,
                source: DomainEventSource::Event(EventId::Neow),
            }),
            "  [POTION] Obtained Block Potion (slot 1)"
        );
    }

    #[test]
    fn boss_transition_collapses_to_single_choice_and_normalized_input() {
        let mut run = RunState::new(31, 0, false, "Ironclad");
        run.map.current_y = 14;
        run.map.current_x = 3;
        let choices = next_map_choices(&run);
        assert_eq!(choices.len(), 1);
        assert!(choices[0].boss_transition);
        assert_eq!(normalize_map_choice_x(&run, 6), 0);
        assert_eq!(describe_bot_map_choice(&run, 4), "Boss Node");
    }

    #[test]
    fn describes_monster_target_with_slot_and_id() {
        let mut run = RunState::new(32, 0, false, "Ironclad");
        let mut row0 = vec![MapRoomNode::new(0, 0)];
        row0[0].class = Some(RoomType::MonsterRoom);
        row0[0].edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut row1 = vec![MapRoomNode::new(0, 1)];
        row1[0].class = Some(RoomType::MonsterRoom);
        run.map = MapState::new(vec![row0, row1]);
        run.map.current_x = 0;
        run.map.current_y = 0;
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::Byrd)
            .with_monster_intent(1, Intent::Buff);
        combat.entities.player = run.build_combat_player(0);
        let label = describe_monster_target(&combat, combat.entities.monsters[0].id).unwrap();
        assert!(label.contains("Byrd"));
        assert!(label.contains("slot 0"));
        assert!(label.contains("id 1"));
    }

    #[test]
    fn describes_targeted_card_play_semantically() {
        let combat = combat_with_hand_and_intent(
            vec![
                crate::combat::CombatCard::new(crate::content::cards::CardId::BloodForBlood, 10),
                crate::combat::CombatCard::new(crate::content::cards::CardId::Strike, 11),
            ],
            Intent::Buff,
        );
        let description = describe_play_card_choice(&combat, 0, Some(1)).unwrap();
        assert!(description.contains("Play Blood for Blood"));
        assert!(description.contains("hand 0"));
        assert!(description.contains("Byrd") || description.contains("Jaw Worm"));
    }
}
