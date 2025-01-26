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
        }

        if let Some(cluster) = floor.get_cluster('N') {
            if let Some(first_aisle) = cluster.get_first_aisle() {
                println!("{:#?}", first_aisle);
            }
            if let Some(last_aisle) = cluster.get_last_aisle() {
                println!("{:#?}", last_aisle);
            }
        } else {
            println!("Cluster 'N' does not exist.");
        }
    } else {
        println!("error reading csv");
        process::exit(1);
    }
}
