mod models;
mod utils;

use std::cell::RefCell;
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <csv_file_path>", args[0]);
        println!("Drag CSV file onto executable");
        wait_for_enter()?;
        process::exit(1);
    }

    let config = utils::Config::load()?;
    let records = utils::read_csv(&args[1])?;
    let floor = models::Floor::new(records);
    print_summary(&floor);

    let floor_rc = Rc::new(RefCell::new(floor));
    let mut stow_slot_builder = models::StowSlotBuilder::new(Rc::clone(&floor_rc));

    stow_slot_builder.start_algorithm(models::AlgorithmConfig {
        algorithm: config.algorithm,
        target_pph: config.target_pph,
        target_hc: config.target_hc,
        max_aisle_count: config.max_aisle_count,
        min_aisle_count: config.min_aisle_count,
    });
    print_results(&stow_slot_builder);

    wait_for_enter()?;
    Ok(())
}

fn print_summary(floor: &models::Floor) {
    println!(
        "Aisles: {}",
        floor.clusters.iter().map(|c| c.aisles.len()).sum::<usize>()
    );
    println!("PPH: {}", floor.packages_per_hour());
    println!("Total Packages: {}", floor.get_total_packages());
}

fn print_results(builder: &models::StowSlotBuilder) {
    builder.display_stow_slots();
    println!("Total Stow Slots: {}", builder.total_stow_slots());
    builder.stow_slots_per_cluster();
}

fn wait_for_enter() -> io::Result<()> {
    println!("Press Enter to exit...");
    io::stdout().flush()?;
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(())
}
