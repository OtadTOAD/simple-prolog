// Database Conversion Utility
// Converts prolog_database.json to prolog_database.bin for faster loading
// Run with: cargo run --release --bin convert_db

use simple_prolog::app::database::Database;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Database Conversion Utility ===\n");

    let json_path = "prolog_database.json";
    let bin_path = "prolog_database.bin";

    if !Path::new(json_path).exists() {
        eprintln!("Error: {} not found!", json_path);
        eprintln!("Make sure you're running this from the project root directory.");
        return Ok(());
    }

    println!("Loading JSON database from {}...", json_path);
    let start = std::time::Instant::now();
    let db = Database::new(Path::new(json_path))?;
    let load_time = start.elapsed();

    println!("✓ Loaded in {:.2}s", load_time.as_secs_f64());
    println!("  - Words: {}", db.words.len());
    println!("  - Patterns: {}", db.patterns.len());

    println!("\nSaving as binary to {}...", bin_path);
    let start = std::time::Instant::now();
    db.save(bin_path)?;
    let save_time = start.elapsed();

    println!("✓ Saved in {:.2}s", save_time.as_secs_f64());

    let json_size = std::fs::metadata(json_path)?.len();
    let bin_size = std::fs::metadata(bin_path)?.len();

    println!("\n=== Results ===");
    println!("JSON size:   {:.2} MB", json_size as f64 / 1_000_000.0);
    println!("Binary size: {:.2} MB", bin_size as f64 / 1_000_000.0);
    println!(
        "Reduction:   {:.1}%",
        (1.0 - bin_size as f64 / json_size as f64) * 100.0
    );

    println!("\n=== Speed Test ===");
    let start = std::time::Instant::now();
    let _test_db = Database::new(Path::new(bin_path))?;
    let bin_load_time = start.elapsed();

    println!("Binary load: {:.2}s", bin_load_time.as_secs_f64());
    println!(
        "Speedup:     {:.1}x faster",
        load_time.as_secs_f64() / bin_load_time.as_secs_f64()
    );

    println!("\n✓ Conversion complete!");
    println!("Your app will now use {} for fast loading.", bin_path);

    Ok(())
}
