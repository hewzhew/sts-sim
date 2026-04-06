
use std::fs;
fn main() {
    let content = fs::read_to_string("tools/replays/2026-03-28-20-22-57--Q70XI43RHP1P.jsonl").unwrap();
    for (i, line) in content.lines().enumerate() {
        if line.contains("\"combat_idx\": 6") && line.contains("\"type\": \"play\"") {
            println!("Line {}: {}", i, &line[..std::cmp::min(1000, line.len())]);
        }
    }
}

