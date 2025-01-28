mod config;
mod models;
mod utils;

use models::{Floor, StowSlotBuilder};
use std::env;
use std::io::{self, Write};
use std::process;
use utils::{read_config, read_csv};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <csv_file_path>", args[0]);
        println!("ensure that you drag your csv file onto the executable");
        println!("Press Enter to exit...");
        let _ = io::stdout().flush();
        let _ = io::stdin().read_line(&mut String::new());
        process::exit(0);
    }

    let csv_file_path = &args[1];

    // Determine the directory of the executable
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path
        .parent()
        .expect("Failed to get executable directory");

    // Construct the path to the config file
    let config_file_path = exe_dir.join("config.toml");

    // Read the config file
    let config = match read_config(config_file_path.to_str().unwrap()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error reading config file: {}", e);
            process::exit(1);
        }
    };

    if let Ok(records) = read_csv(csv_file_path) {
        let floor = Floor::new(records);
        let aisle_count = floor.clusters.iter().map(|c| c.aisles.len()).sum::<usize>();
        println!("{:?} aisles", aisle_count);
        println!("PPH: {}", floor.packages_per_hour());
        println!("Total packages: {}", floor.get_total_packages());

        let mut stow_slot_builder = StowSlotBuilder::new(&floor);
        stow_slot_builder.start_algorithm(config.target_pph);
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
