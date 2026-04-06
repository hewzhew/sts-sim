use sts_simulator::state::RunState;
use sts_simulator::content::cards::java_id;

fn main() {
    let seed = 1775454153169;
    let mut run_state = RunState::new(seed, 0, false, "Ironclad");
    
    // Fast forward to floor 2 (which could be a shop) or just generate a shop right now.
    // generate_shop doesn't depend on actual floor, just rng states!
    // But let's advance RNG to simulate arriving at a shop at floor 2.
    // In Java, seed generation is static, we can just call generate_shop.
    let shop = run_state.generate_shop();
    
    println!("Shop Cards:");
    for c in &shop.cards {
        println!(" - {} (cost {})", java_id(c.card_id), c.price);
    }
    
    println!("Shop Relics:");
    for r in &shop.relics {
        println!(" - {:?} (cost {})", r.relic_id, r.price);
    }
    
    println!("Shop Potions:");
    for p in &shop.potions {
        println!(" - {:?} (cost {})", p.potion_id, p.price);
    }
    
    println!("Purge Cost: {}", shop.purge_cost);
}
