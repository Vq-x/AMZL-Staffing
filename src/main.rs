mod models;
mod utils;

use models::{Floor, StowSlot};
use std::process;
use utils::read_csv;

fn main() {
    const TOTAL_HOURS: i32 = 6;
    if let Ok(records) = read_csv() {
        let floor = Floor::new(records);
        let aisle_count = floor.clusters.iter().map(|c| c.aisles.len()).sum::<usize>();
        println!("{:?} aisles", aisle_count);
        println!("PPH: {}", floor.packages_per_hour(TOTAL_HOURS));

        if let Some(aisle) = floor.get_aisle_in_cluster('A', 10) {
            println!("PPH: {}", aisle.get_aisle_pph(TOTAL_HOURS));

            let stow_slot = StowSlot::new(
                vec![
                    aisle.clone(),
                    floor.get_aisle_in_cluster('A', 11).unwrap().clone(),
                    floor.get_aisle_in_cluster('A', 12).unwrap().clone(),
                    floor.get_aisle_in_cluster('A', 13).unwrap().clone(),
                ],
                false,
                TOTAL_HOURS,
            );
            println!("StowSlot PPH: {}", stow_slot.pph);
            stow_slot.display_aisle_range();
        }
    } else {
        println!("error reading csv");
        process::exit(1);
    }
}
