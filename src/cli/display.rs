use crate::state::core::EngineState;
use crate::state::run::RunState;
use crate::combat::CombatState;
use crate::content::cards;
use crate::content::relics::RelicId;
use crate::content::monsters::EnemyId;


pub fn print_state(es: &EngineState, rs: &RunState, cs: &Option<CombatState>) {
    // Use combat HP if in combat, otherwise run-state HP
    let (hp, max_hp) = if let Some(combat) = cs {
        (combat.player.current_hp, combat.player.max_hp)
    } else {
        (rs.current_hp, rs.max_hp)
    };
    println!("--- Act {} Floor {} | HP: {}/{} | Gold: {} ---",
        rs.act_num, rs.floor_num, hp, max_hp, rs.gold);

    match es {
        EngineState::EventRoom => {
            if let Some(event) = &rs.event_state {
                let choices = crate::engine::event_handler::get_event_choices(rs);
                println!("  EVENT: {:?} (screen {})", event.id, event.current_screen);
                for (i, c) in choices.iter().enumerate() {
                    if c.disabled {
                        println!("    [{}] {} (Disabled: {})", i, c.text, c.disabled_reason.as_deref().unwrap_or(""));
                    } else {
                        println!("    [{}] {}", i, c.text);
                    }
                }
                println!("  → Type a number to choose");
            } else {
                println!("  EVENT ROOM (no event state?)");
            }
        },
        EngineState::MapNavigation => {
            println!("  MAP — Choose next node:");
            let next_y = if rs.map.current_y == -1 { 0 } else { rs.map.current_y + 1 };
            let mut available = 0;
            if next_y <= rs.map.graph.len() as i32 {
                for x in 0..7 {
                    if rs.map.can_travel_to(x, next_y, false) {
                        let room_type = if next_y < rs.map.graph.len() as i32 {
                            rs.map.graph[next_y as usize][x as usize].class
                        } else {
                            Some(crate::map::node::RoomType::MonsterRoomBoss)
                        };
                        println!("    [go {}] {:?} (y={})", x, room_type, next_y);
                        available += 1;
                    }
                }
            }
            if available == 0 {
                println!("    (no adjacent nodes — floor {} y={})", rs.floor_num, rs.map.current_y);
                println!("    Try: 'go 0' through 'go 6'");
            }
        },
        EngineState::CombatPlayerTurn => {
            if let Some(cs) = cs {
                print_combat(cs);
                println!("  → 'play <idx> [target]', 'end', 'potion <slot> [target]', 'skip'");
            }
        },
        EngineState::PendingChoice(pc) => {
            println!("  PENDING CHOICE: {:?}", pc);
            println!("  → 'choose <indices...>' or 'cancel'");
        },
        EngineState::RewardScreen(reward) => {
            println!("  REWARDS:");
            for (i, item) in reward.items.iter().enumerate() {
                println!("    [{}] {:?}", i, item);
            }
            if let Some(ref card_options) = reward.pending_card_choice {
                println!("  CARD CHOICE:");
                for (i, &card_id) in card_options.iter().enumerate() {
                    let def = cards::get_card_definition(card_id);
                    println!("    [pick {}] {}", i, def.name);
                }
                println!("  → 'pick <idx>' or 'skip'");
            } else {
                println!("  → 'claim <idx>' to take, 'skip'/'proceed' to leave");
            }
        },
        EngineState::Campfire => {
            println!("  CAMPFIRE:");
            println!("    [rest]  Heal {} HP", rs.max_hp * 30 / 100);
            println!("    [smith] Upgrade a card");
            println!("  → 'rest' or 'smith <deck_idx>'");
        },
        EngineState::Shop(shop) => {
            println!("  SHOP:");
            println!("  Cards:");
            for (i, sc) in shop.cards.iter().enumerate() {
                let def = cards::get_card_definition(sc.card_id);
                println!("    [buy card {}] {} — {} gold", i, def.name, sc.price);
            }
            println!("  Relics:");
            for (i, sr) in shop.relics.iter().enumerate() {
                println!("    [buy relic {}] {:?} — {} gold", i, sr.relic_id, sr.price);
            }
            println!("  Purge: {} gold", shop.purge_cost);
            println!("  → 'buy card/relic/potion <idx>', 'purge <deck_idx>', 'leave'");
        },
        EngineState::RunPendingChoice(rpc) => {
            println!("  DECK SELECT ({:?}): choose {}-{} cards",
                rpc.reason, rpc.min_choices, rpc.max_choices);
            for (i, card) in rs.master_deck.iter().enumerate() {
                let def = cards::get_card_definition(card.id);
                println!("    [{}] {} (uuid={})", i, def.name, card.uuid);
            }
            println!("  → 'select <idx1> <idx2> ...' or 'cancel'");
        },
        EngineState::GameOver(result) => {
            println!("  GAME OVER: {:?}", result);
        },
        EngineState::BossRelicSelect(bs) => {
            println!("  BOSS RELIC SELECT: choose a reward!");
            for (i, r) in bs.relics.iter().enumerate() {
                println!("    [{}] {:?}", i, r);
            }
            println!("  → 'relic <idx>' or 'skip'");
        },
        EngineState::EventCombat(_) => {
            if let Some(cs) = cs {
                print_combat(cs);
                println!("  → 'play <idx> [target]', 'end', 'potion <slot> [target]', 'skip'");
            } else {
                println!("  EVENT COMBAT (awaiting initialization...)");
            }
        },
        _ => {
            println!("  State: {:?}", std::mem::discriminant(es));
        },
    }
}

pub fn print_combat(cs: &CombatState) {
    println!("  COMBAT — Turn {} | Energy: {}", cs.turn_count, cs.energy);

    println!("  Player: HP {}/{} Block {} ", cs.player.current_hp, cs.player.max_hp, cs.player.block);
    if let Some(powers_list) = cs.power_db.get(&cs.player.id) {
        if !powers_list.is_empty() {
            let powers: Vec<String> = powers_list.iter()
                .map(|p| format!("{:?}({})", p.power_type, p.amount))
                .collect();
            println!("    Powers: {}", powers.join(", "));
        }
    }

    let hide_intents = cs.player.has_relic(RelicId::RunicDome);

    for m in &cs.monsters {
        if m.is_dying { continue; }

        let name = EnemyId::from_id(m.monster_type)
            .map(|eid| eid.get_name())
            .unwrap_or("Unknown");

        let intent_str = if hide_intents {
            "Hidden".to_string()
        } else {
            use crate::combat::Intent;
            match m.current_intent {
                Intent::Attack { hits, .. } => format!("Attack {{ damage: {}, hits: {} }}", m.intent_dmg, hits),
                Intent::AttackBuff { hits, .. } => format!("AttackBuff {{ damage: {}, hits: {} }}", m.intent_dmg, hits),
                Intent::AttackDebuff { hits, .. } => format!("AttackDebuff {{ damage: {}, hits: {} }}", m.intent_dmg, hits),
                Intent::AttackDefend { hits, .. } => format!("AttackDefend {{ damage: {}, hits: {} }}", m.intent_dmg, hits),
                _ => format!("{:?}", m.current_intent)
            }
        };

        println!("  Monster[{}] {} (id={}): HP {}/{} Block {} Intent {}",
            m.slot, name, m.id, m.current_hp, m.max_hp, m.block, intent_str);
        if let Some(powers_list) = cs.power_db.get(&m.id) {
            if !powers_list.is_empty() {
                let powers: Vec<String> = powers_list.iter()
                    .map(|p| format!("{:?}({})", p.power_type, p.amount))
                    .collect();
                println!("    Powers: {}", powers.join(", "));
            }
        }
    }
    
    // Potion readout
    if cs.potions.iter().any(|p| p.is_some()) {
        let potion_strings: Vec<String> = cs.potions.iter()
            .enumerate()
            .filter_map(|(idx, opt_p)| opt_p.as_ref().map(|p| format!("[{}] {:?}", idx, p.id)))
            .collect();
        println!("  Potions: {}", potion_strings.join(", "));
    }

    println!("  Hand ({}):", cs.hand.len());
    for (i, card) in cs.hand.iter().enumerate() {
        let def = cards::get_card_definition(card.id);
        let c_cost = card.get_cost();
        let cost_str = if c_cost >= 0 { format!("[{}]", c_cost) } else { "[X]".to_string() };
        println!("    [{}] {} {} {}", i, cost_str, def.name,
            if card.upgrades > 0 { "+" } else { "" });
    }

    println!("  Draw: {} | Discard: {} | Exhaust: {}",
        cs.draw_pile.len(), cs.discard_pile.len(), cs.exhaust_pile.len());
}

pub fn print_detailed_state(es: &EngineState, rs: &RunState, _cs: &Option<CombatState>) {
    println!("=== Detailed RunState ===");
    println!("  Deck ({} cards):", rs.master_deck.len());
    for (i, card) in rs.master_deck.iter().enumerate() {
        let def = cards::get_card_definition(card.id);
        println!("    [{}] {} {}", i, def.name,
            if card.upgrades > 0 { format!("+{}", card.upgrades) } else { String::new() });
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


