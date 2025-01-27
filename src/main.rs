mod models;
mod utils;

use models::{Floor, StowSlotBuilder};
use std::env;
use std::io::{self, Write};
use std::process;
use utils::read_csv;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];

    if let Ok(records) = read_csv(file_path) {
        let floor = Floor::new(records);
        let aisle_count = floor.clusters.iter().map(|c| c.aisles.len()).sum::<usize>();
        println!("{:?} aisles", aisle_count);
        println!("PPH: {}", floor.packages_per_hour());
        println!("Total packages: {}", floor.get_total_packages());

        let mut stow_slot_builder = StowSlotBuilder::new(&floor);
        stow_slot_builder.start_algorithm(250);
        stow_slot_builder.display_stow_slots();
        println!("Total stow slots: {}", stow_slot_builder.total_stow_slots());
        stow_slot_builder.stow_slots_per_cluster();
    } else {
        println!("Error reading CSV file");
        process::exit(1);
    }

    println!("Press Enter to exit...");
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());
}
