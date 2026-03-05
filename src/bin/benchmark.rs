//! Benchmark for Rayon parallel simulation
//!
//! Run with: cargo run --release --bin benchmark

use std::time::Instant;

// Import from the library
use sts_sim::interop::run_parallel_simulations;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Slay the Spire Simulator - Rayon Parallel Benchmark     ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let cards_path = "data/cards.json";
    let max_turns = 50;

    // Warmup
    println!("🔥 Warming up...");
    let _ = run_parallel_simulations(100, 0, cards_path, max_turns)?;

    // Benchmark different batch sizes
    let batch_sizes = [1_000, 10_000, 100_000];

    println!("\n📊 Running benchmarks...\n");
    println!("{:>12} | {:>12} | {:>15} | {:>12}", 
        "Simulations", "Time (s)", "Sims/sec", "µs/sim");
    println!("{:-<12}-+-{:-<12}-+-{:-<15}-+-{:-<12}", "", "", "", "");

    for &num_sims in &batch_sizes {
        let start = Instant::now();
        let results = run_parallel_simulations(num_sims, 0, cards_path, max_turns)?;
        let elapsed = start.elapsed();

        let sims_per_sec = num_sims as f64 / elapsed.as_secs_f64();
        let us_per_sim = elapsed.as_micros() as f64 / num_sims as f64;

        // Calculate win rate
        let wins = results.iter().filter(|r| r.won).count();
        let win_rate = wins as f64 / results.len() as f64 * 100.0;

        println!("{:>12} | {:>12.3} | {:>15.0} | {:>12.1}", 
            num_sims, elapsed.as_secs_f64(), sims_per_sec, us_per_sim);

        // Show stats for largest batch
        if num_sims >= 10_000 {
            let avg_turns: f64 = results.iter()
                .map(|r| r.turns_survived as f64)
                .sum::<f64>() / results.len() as f64;
            let avg_damage: f64 = results.iter()
                .map(|r| r.damage_dealt as f64)
                .sum::<f64>() / results.len() as f64;

            println!("\n  📈 Stats for {} simulations:", num_sims);
            println!("     Win rate: {:.1}%", win_rate);
            println!("     Avg turns survived: {:.1}", avg_turns);
            println!("     Avg damage dealt: {:.1}", avg_damage);
            println!();
        }
    }

    // CPU info
    println!("\n💻 System Info:");
    println!("   Rayon threads: {}", rayon::current_num_threads());

    println!("\n✅ Benchmark complete!");
    println!("\n💡 To use from Python:");
    println!("   maturin develop --release");
    println!("   python demo.py");

    Ok(())
}
