mod models;
mod utils;

use models::{Floor, StowSlot, StowSlotBuilder};
use std::process;
use utils::read_csv;

fn main() {
    if let Ok(records) = read_csv() {
        let floor = Floor::new(records);
        let aisle_count = floor.clusters.iter().map(|c| c.aisles.len()).sum::<usize>();
        println!("{:?} aisles", aisle_count);
        println!("PPH: {}", floor.packages_per_hour());

        if let Some(aisle) = floor.get_aisle_in_cluster('A', 10) {
            println!("PPH: {}", aisle.get_aisle_pph());

            let stow_slot = StowSlot::new(
                'A',
                vec![
                    &aisle,
                    floor.get_aisle_in_cluster('A', 11).unwrap(),
                    floor.get_aisle_in_cluster('A', 12).unwrap(),
                    floor.get_aisle_in_cluster('A', 13).unwrap(),
                ],
                false,
                &floor,
            );
            println!("StowSlot PPH: {}", stow_slot.pph);
            stow_slot.display_aisle_range();
            println!("Is consecutive: {}", stow_slot.is_consecutive());
        }
        let mut stow_slot_builder = StowSlotBuilder::new(&floor);
        stow_slot_builder.start_algorithm(250);
        stow_slot_builder.display_stow_slots();
    } else {
        println!("error reading csv");
        process::exit(1);
    }
}
