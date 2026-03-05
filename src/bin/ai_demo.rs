//! Enemy AI Demo
//!
//! This demonstrates the hardcoded enemy AI system with v5 JSON data:
//! 1. Load monster definitions from verified JSON
//! 2. Create monster states
//! 3. Simulate multiple turns of AI decision-making
//!
//! Run with: cargo run --release --bin ai_demo

use rand::SeedableRng;
use rand_xoshiro::Xoshiro256StarStar;

// Import from the library
use sts_sim::loader::MonsterLibrary;
use sts_sim::enemy::MonsterState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Slay the Spire Simulator - Enemy AI Demo (v5)           ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // === Step 1: Load Monster Library ===
    println!("📚 Loading monster library...");
    let library = MonsterLibrary::load("data/monsters_verified.json")?;
    println!("   Loaded {} monsters\n", library.len());
    
    // Show some stats
    println!("📊 Monster Statistics:");
    println!("   Act 1: {} monsters", library.monsters_in_act(1).len());
    println!("   Act 2: {} monsters", library.monsters_in_act(2).len());
    println!("   Act 3: {} monsters", library.monsters_in_act(3).len());
    println!("   Bosses: {} monsters", library.monsters_of_type("boss").len());
    println!("   Elites: {} monsters", library.monsters_of_type("elite").len());

    // === Step 2: Demo various AI patterns ===
    let mut rng = Xoshiro256StarStar::seed_from_u64(12345);
    
    println!("\n{}", "═".repeat(60));
    println!("🎮 AI BEHAVIOR DEMOS");
    println!("{}\n", "═".repeat(60));

    // Demo 1: Cultist (init_sequence + repeat)
    demo_monster(&library, "Cultist", &mut rng, 5)?;
    
    // Demo 2: Jaw Worm (probabilistic with history constraints)
    demo_monster(&library, "Jaw Worm", &mut rng, 8)?;
    
    // Demo 3: Slime Boss (cycle)
    demo_monster(&library, "Slime Boss", &mut rng, 9)?;
    
    // Demo 4: Mad Gremlin (repeat)
    demo_monster(&library, "Mad Gremlin", &mut rng, 5)?;
    
    // Demo 5: Gremlin Nob (init + probabilistic)
    demo_monster(&library, "Gremlin Nob", &mut rng, 6)?;
    
    // Demo 6: Acid Slime (L) - HP threshold
    demo_monster_with_damage(&library, "Acid Slime (L)", &mut rng, 6)?;

    println!("\n✅ AI Demo Complete!");
    Ok(())
}

fn demo_monster(
    library: &MonsterLibrary,
    name: &str,
    rng: &mut Xoshiro256StarStar,
    turns: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let def = library.get(name)?;
    let mut state = MonsterState::new(def, rng, 0);
    
    println!("🐉 {} (HP: {}/{}) [{}]", name, state.hp, state.max_hp, def.monster_type);
    println!("   Moves: {:?}", def.moves.values().map(|m| &m.name).collect::<Vec<_>>());
    
    print!("   Turns: ");
    for turn in 1..=turns {
        state.plan_next_move(def, rng, &[]);
        print!("[T{}:{}", turn, state.current_move);
        if turn < turns {
            print!("] → ");
        } else {
            println!("]");
        }
    }
    println!();
    Ok(())
}

fn demo_monster_with_damage(
    library: &MonsterLibrary,
    name: &str,
    rng: &mut Xoshiro256StarStar,
    turns: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let def = library.get(name)?;
    let mut state = MonsterState::new(def, rng, 0);
    
    println!("🐉 {} (HP: {}/{}) - Testing HP Threshold", name, state.hp, state.max_hp);
    
    print!("   Turns: ");
    for turn in 1..=turns {
        // Damage the monster mid-fight
        if turn == 4 {
            state.hp = (state.max_hp * 40) / 100; // Drop to 40% HP
            print!("[💥HP→{}] ", state.hp);
        }
        
        state.plan_next_move(def, rng, &[]);
        print!("[T{}:{}", turn, state.current_move);
        if turn < turns {
            print!("] → ");
        } else {
            println!("]");
        }
    }
    println!();
    Ok(())
}
