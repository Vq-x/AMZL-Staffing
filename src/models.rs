use crate::utils::Config;
use once_cell::sync::Lazy;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::ops::DerefMut;
use std::path::Path;
use std::rc::Rc;

pub static TOTAL_HOURS: Lazy<f32> = Lazy::new(|| Config::load().unwrap().total_hours);

#[derive(Default, Serialize, Deserialize, Debug)]
pub enum Algorithm {
    TargetHC,
    #[default]
    TargetPPH,
}
#[derive(Serialize, Deserialize)]
pub struct AlgorithmConfig {
    pub algorithm: Algorithm,
    pub target_pph: i32,
    pub target_hc: i32,
    pub max_aisle_count: i32,
    pub min_aisle_count: i32,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::TargetPPH,
            target_pph: 1000,
            target_hc: 10,
            max_aisle_count: 3,
            min_aisle_count: 2,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SortZone {
    pub cluster: char,
    pub aisle: u32,
    pub level: u32,
    pub column: char,
}

impl SortZone {
    pub fn display(&self) -> String {
        format!(
            "{}-{}.{}{}",
            self.cluster, self.aisle, self.level, self.column
        )
    }
}

impl<'de> Deserialize<'de> for SortZone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SortZoneVisitor;

        impl<'de> Visitor<'de> for SortZoneVisitor {
            type Value = SortZone;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in the format 'A-1.1A'")
            }

            fn visit_str<E>(self, value: &str) -> Result<SortZone, E>
            where
                E: de::Error,
            {
                let parts: Vec<&str> = value.split('-').collect();
                if parts.len() != 2 {
                    return Err(de::Error::custom("expected format 'A-1.1A'"));
                }

                let cluster = parts[0]
                    .chars()
                    .next()
                    .ok_or_else(|| de::Error::custom("missing cluster"))?;
                let aisle_level_column: Vec<&str> = parts[1].split('.').collect();
                if aisle_level_column.len() != 2 {
                    return Err(de::Error::custom("expected format '1.1A'"));
                }

                let aisle = aisle_level_column[0]
                    .parse::<u32>()
                    .map_err(de::Error::custom)?;
                let level_column: Vec<char> = aisle_level_column[1].chars().collect();
                if level_column.len() != 2 {
                    return Err(de::Error::custom("expected format '1A'"));
                }

                let level = level_column[0]
                    .to_digit(10)
                    .ok_or_else(|| de::Error::custom("invalid level"))?;
                let column = level_column[1];

                Ok(SortZone {
                    cluster,
                    aisle,
                    level,
                    column,
                })
            }
        }

        deserializer.deserialize_str(SortZoneVisitor)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct BagRecord {
    #[serde(rename = "Sort Zone")]
    pub sort_zone: SortZone,
    #[serde(rename = "Planned Bag Count")]
    pub planned_bag_count: i32,
    #[serde(rename = "Planned Package Count")]
    pub planned_package_count: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Aisle {
    pub cluster: char,
    pub aisle_num: u32,
    pub bag_records: Vec<BagRecord>,
    pub locked: bool,
}

impl Aisle {
    pub fn total_packages(&self) -> i32 {
        self.bag_records
            .iter()
            .map(|b| b.planned_package_count)
            .sum()
    }

    pub fn get_aisle_pph(&self) -> f32 {
        self.total_packages() as f32 / *TOTAL_HOURS
    }

    pub fn display_aisle(&self) -> String {
        format!("{}-{}", self.cluster, self.aisle_num)
    }
    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
    }
}

#[derive(Debug)]
pub struct Cluster {
    pub cluster: char,
    pub aisles: Vec<Rc<Aisle>>,
    pub aisle_pairs: Vec<AislePair>,
}

impl Cluster {
    pub fn get_aisle(&self, aisle: u32) -> Option<&Rc<Aisle>> {
        self.aisles.iter().find(|a| a.aisle_num == aisle)
    }

    pub fn get_first_aisle(&self) -> Option<&Rc<Aisle>> {
        self.aisles.iter().min_by_key(|a| a.aisle_num)
    }

    pub fn get_last_aisle(&self) -> Option<&Rc<Aisle>> {
        self.aisles.iter().max_by_key(|a| a.aisle_num)
    }

    pub fn get_total_packages(&self) -> i32 {
        self.aisles.iter().map(|a| a.total_packages()).sum::<i32>()
    }

    pub fn get_next_aisle(&self, aisle: u32) -> Option<&Rc<Aisle>> {
        self.aisles.iter().find(|a| a.aisle_num == aisle + 1)
    }

    pub fn get_previous_aisle(&self, aisle: u32) -> Option<&Rc<Aisle>> {
        self.aisles.iter().find(|a| a.aisle_num == aisle - 1)
    }

    pub fn generate_aisle_pairs(&mut self) {
        self.aisle_pairs.clear();

        // Sort aisles by aisle number to ensure proper pairing
        self.aisles.sort_by_key(|a| a.aisle_num);

        // Group aisles into pairs (odd with even)
        for (i, aisle) in self.aisles.iter().enumerate() {
            // Skip even aisles as starting points (we want odd-even pairs)
            if aisle.aisle_num % 2 == 0 {
                continue;
            }

            // Find the next aisle (which should be even)
            if let Some(next_idx) = self
                .aisles
                .iter()
                .position(|a| a.aisle_num == aisle.aisle_num + 1)
            {
                // Create a pair with the current (odd) aisle and the next (even) aisle
                let pair = AislePair {
                    aisle1: Some(Rc::clone(&self.aisles[i])),
                    aisle2: Some(Rc::clone(&self.aisles[next_idx])),
                };
                self.aisle_pairs.push(pair);
            } else {
                // If no matching even aisle, create a pair with just the odd aisle
                let pair = AislePair {
                    aisle1: Some(Rc::clone(&self.aisles[i])),
                    aisle2: None,
                };
                self.aisle_pairs.push(pair);
            }
        }
    }
    pub fn aisle_pairs_len(&mut self) -> usize {
        if self.aisle_pairs.is_empty() {
            self.generate_aisle_pairs();
        }
        self.aisle_pairs.len()
    }

    // Get aisles from a pair, sharing references instead of cloning
    pub fn get_aisles_from_pair(&self, pair: &AislePair) -> Vec<Rc<Aisle>> {
        let mut result = Vec::new();
        if let Some(aisle) = &pair.aisle1 {
            result.push(Rc::clone(aisle));
        }
        if let Some(aisle) = &pair.aisle2 {
            result.push(Rc::clone(aisle));
        }
        result
    }

    pub fn get_pair_from_aisle(&self, aisle: &Rc<Aisle>) -> Option<(usize, &AislePair)> {
        self.aisle_pairs.iter().enumerate().find(|(i, p)| {
            p.aisle1
                .as_ref()
                .is_some_and(|a| a.aisle_num == aisle.aisle_num)
                || p.aisle2
                    .as_ref()
                    .is_some_and(|a| a.aisle_num == aisle.aisle_num)
        })
    }

    // get lowest pph from a set amount of aisle pairs
    // pub fn get_lowest_pph(
    //     &self,
    //     aisle_pair_range: usize,
    //     max_aisle_count: usize,
    // ) -> Vec<(StowSlot, f32)> {
    //     let mut stow_slots = Vec::new();
    //     // iterate through the aisle pairs and get the next n aisles and calculate the pph, return the lowest pph range.
    //     for i in 0..self.aisle_pairs.len() {
    //         if i + aisle_pair_range > self.aisle_pairs.len() {
    //             break;
    //         }
    //         if self.aisle_pairs[i..i + aisle_pair_range]
    //             .iter()
    //             .any(|pair| pair.aisle1.as_ref().is_some_and(|aisle| aisle.locked))
    //         {
    //             continue;
    //         }
    //         let pph = self.aisle_pairs[i..i + aisle_pair_range]
    //             .iter()
    //             .map(|pair| {
    //                 let aisles = self.get_aisles_from_pair(pair);
    //                 aisles
    //                     .iter()
    //                     .map(|a| a.total_packages() as f32 / *TOTAL_HOURS)
    //                     .sum::<f32>()
    //             })
    //             .sum::<f32>();
    //         stow_slots.push((
    //             StowSlot::new(
    //                 self.cluster,
    //                 self.aisle_pairs[i..i + aisle_pair_range]
    //                     .iter()
    //                     .flat_map(|pair| self.get_aisles_from_pair(pair))
    //                     .collect(),
    //             ),
    //             pph,
    //         ));
    //     }
    //     stow_slots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    //     // Take only the first five elements (or fewer if there aren't five)
    //     // let count = std::cmp::min(max_aisle_count, stow_slots.len());
    //     // stow_slots.into_iter().take(count).collect()
    //     stow_slots
    // }
}

#[derive(Debug)]
pub struct Floor {
    pub clusters: Vec<Cluster>,
}

impl Floor {
    pub fn new(bags: Vec<BagRecord>) -> Self {
        let mut clusters: Vec<Cluster> = Vec::new();
        for bag in bags {
            let cluster_char = bag.sort_zone.cluster;
            let aisle_number = bag.sort_zone.aisle;

            let cluster = clusters.iter_mut().find(|c| c.cluster == cluster_char);
            if let Some(cluster) = cluster {
                let aisle = cluster
                    .aisles
                    .iter_mut()
                    .find(|a| a.aisle_num == aisle_number);
                if let Some(aisle) = aisle {
                    // Clone the Rc to avoid the borrow checker error
                    let aisle_clone = Rc::clone(aisle);
                    // Check if we can get a mutable reference
                    if let Some(aisle_mut) = Rc::get_mut(aisle) {
                        aisle_mut.bag_records.push(bag);
                    } else {
                        // If we can't get a mutable reference, create a new Aisle with the updated bag_records
                        let mut new_bag_records = aisle_clone.bag_records.clone();
                        new_bag_records.push(bag);
                        *aisle = Rc::new(Aisle {
                            cluster: cluster_char,
                            aisle_num: aisle_number,
                            bag_records: new_bag_records,
                            ..Default::default()
                        });
                    }
                } else {
                    cluster.aisles.push(Rc::new(Aisle {
                        cluster: cluster_char,
                        aisle_num: aisle_number,
                        bag_records: vec![bag],
                        ..Default::default()
                    }));
                }
            } else {
                clusters.push(Cluster {
                    cluster: cluster_char,
                    aisles: vec![Rc::new(Aisle {
                        cluster: cluster_char,
                        aisle_num: aisle_number,
                        bag_records: vec![bag],
                        ..Default::default()
                    })],
                    aisle_pairs: Vec::new(),
                });
            }
        }

        // Sort aisles by aisle number
        for cluster in &mut clusters {
            cluster.aisles.sort_by_key(|a| a.aisle_num);
        }

        let mut floor = Self { clusters };
        floor.generate_aisle_pairs();
        floor
    }

    pub fn packages_per_hour(&self) -> f32 {
        self.clusters
            .iter()
            .map(|c| c.aisles.iter().map(|a| a.total_packages()).sum::<i32>())
            .sum::<i32>() as f32
            / *TOTAL_HOURS
    }

    pub fn get_aisle_in_cluster(&self, cluster: char, aisle: u32) -> Option<&Rc<Aisle>> {
        self.clusters
            .iter()
            .find(|c| c.cluster == cluster)
            .and_then(|c| c.aisles.iter().find(|a| a.aisle_num == aisle))
    }

    pub fn get_cluster(&self, cluster: char) -> Option<&Cluster> {
        self.clusters.iter().find(|c| c.cluster == cluster)
    }

    pub fn get_total_packages(&self) -> i32 {
        self.clusters
            .iter()
            .map(|c| c.get_total_packages())
            .sum::<i32>()
    }

    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let records = crate::utils::read_csv(path.as_ref().to_str().unwrap())?;
        Ok(Self::new(records))
    }

    pub fn cluster(&self, cluster: char) -> Option<&Cluster> {
        self.clusters.iter().find(|c| c.cluster == cluster)
    }

    pub fn get_total_stow_slots(&self) -> i32 {
        self.clusters
            .iter()
            .map(|c| c.aisles.iter().map(|a| a.total_packages()).sum::<i32>())
            .sum::<i32>()
    }

    pub fn generate_aisle_pairs(&mut self) {
        for cluster in &mut self.clusters {
            cluster.generate_aisle_pairs();
        }
    }

    pub fn get_all_aisle_pairs(&self) -> Vec<&AislePair> {
        self.clusters
            .iter()
            .flat_map(|c| c.aisle_pairs.iter())
            .collect()
    }

    pub fn create_stow_slot_builder(self) -> StowSlotBuilder {
        let floor_rc = Rc::new(RefCell::new(self));
        StowSlotBuilder::new(floor_rc)
    }

    pub fn to_rc(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }
}

#[derive(Debug, Clone)]
pub struct StowSlot {
    pub cluster: char,
    pub aisles: Vec<Rc<Aisle>>,
    pub is_floater: bool,
    pub pph: f32,
    pub locked: bool,
}

impl StowSlot {
    pub fn new(cluster: char, aisles: Vec<Rc<Aisle>>) -> Self {
        let mut obj = Self {
            cluster,
            aisles,
            is_floater: false,
            pph: 0.0,
            locked: false,
        };
        obj.update_pph();
        obj
    }

    pub fn add_aisle(&mut self, aisle: Rc<Aisle>) {
        self.aisles.push(aisle);
        self.update_pph();
    }

    pub fn add_aisles(&mut self, aisles: Vec<Rc<Aisle>>) {
        self.aisles.extend(aisles);
        self.update_pph();
    }

    fn update_pph(&mut self) {
        self.pph =
            self.aisles.iter().map(|a| a.total_packages()).sum::<i32>() as f32 / *TOTAL_HOURS;
        self.is_floater = self.pph <= 150.0;
    }

    pub fn display_aisles(&self) {
        for aisle in &self.aisles {
            println!("{}", aisle.display_aisle());
        }
    }

    pub fn display_aisle_range(&self) {
        println!(
            "{} - {}: {} PPH, is floater: {}",
            self.aisles.first().unwrap().display_aisle(),
            self.aisles.last().unwrap().display_aisle(),
            self.pph as i32,
            self.is_floater
        );
    }

    pub fn is_consecutive(&self) -> bool {
        self.aisles
            .iter()
            .zip(self.aisles.iter().skip(1))
            .all(|(a, b)| a.aisle_num + 1 == b.aisle_num)
    }
    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
        for aisle in &mut self.aisles {
            if let Some(aisle_mut) = Rc::get_mut(aisle) {
                aisle_mut.locked = self.locked;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AislePair {
    pub aisle1: Option<Rc<Aisle>>,
    pub aisle2: Option<Rc<Aisle>>,
}

impl AislePair {
    pub fn is_complete(&self) -> bool {
        self.aisle1.is_some() && self.aisle2.is_some()
    }

    pub fn total_packages(&self) -> i32 {
        let mut total = 0;
        if let Some(aisle) = &self.aisle1 {
            total += aisle.total_packages();
        }
        if let Some(aisle) = &self.aisle2 {
            total += aisle.total_packages();
        }
        total
    }

    pub fn get_aisles(&self) -> Vec<Rc<Aisle>> {
        let mut aisles = Vec::new();
        if let Some(aisle) = &self.aisle1 {
            aisles.push(Rc::clone(aisle));
        }
        aisles
    }

    pub fn display(&self) -> String {
        match (&self.aisle1, &self.aisle2) {
            (Some(a1), Some(a2)) => format!("{} & {}", a1.display_aisle(), a2.display_aisle()),
            (Some(a1), None) => format!("{} (unpaired)", a1.display_aisle()),
            (None, Some(a2)) => format!("{} (unpaired)", a2.display_aisle()),
            (None, None) => "Empty pair".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct StowSlotBuilder {
    floor: Rc<RefCell<Floor>>,
    pub stow_slots: Vec<StowSlot>,
}

impl StowSlotBuilder {
    pub fn new(floor: Rc<RefCell<Floor>>) -> Self {
        Self {
            floor,
            stow_slots: Vec::new(),
        }
    }

    pub fn get_stow_slot_from_aisle(&mut self, aisle: &Rc<Aisle>) -> Option<&mut StowSlot> {
        self.stow_slots.iter_mut().find(|s| {
            s.aisles
                .iter()
                .any(|a| a.aisle_num == aisle.aisle_num && a.cluster == aisle.cluster)
        })
    }

    pub fn display_stow_slots(&self) {
        for slot in &self.stow_slots {
            slot.display_aisle_range();
        }
    }

    pub fn total_stow_slots(&self) -> i32 {
        self.stow_slots.len() as i32
    }

    pub fn get_lowest_pph(
        &self,
        cluster: &Cluster,
        aisle_pair_range: usize,
        max_aisle_count: usize,
    ) -> Vec<(StowSlot, f32)> {
        let mut stow_slots = Vec::new();
        // iterate through the aisle pairs and get the next n aisles and calculate the pph, return the lowest pph range.
        for i in 0..cluster.aisle_pairs.len() {
            if i + aisle_pair_range > cluster.aisle_pairs.len() {
                break;
            }
            if cluster.aisle_pairs[i..i + aisle_pair_range]
                .iter()
                .any(|pair| pair.aisle1.as_ref().is_some_and(|aisle| aisle.locked))
            {
                continue;
            }
            let pph = cluster.aisle_pairs[i..i + aisle_pair_range]
                .iter()
                .map(|pair| {
                    let aisles = cluster.get_aisles_from_pair(pair);
                    aisles
                        .iter()
                        .map(|a| a.total_packages() as f32 / *TOTAL_HOURS)
                        .sum::<f32>()
                })
                .sum::<f32>();
            stow_slots.push((
                StowSlot::new(
                    cluster.cluster,
                    cluster.aisle_pairs[i..i + aisle_pair_range]
                        .iter()
                        .flat_map(|pair| cluster.get_aisles_from_pair(pair))
                        .collect(),
                ),
                pph,
            ));
        }
        stow_slots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take only the first five elements (or fewer if there aren't five)
        // let count = std::cmp::min(max_aisle_count, stow_slots.len());
        // stow_slots.into_iter().take(count).collect()
        stow_slots
    }

    pub fn stow_slots_per_cluster(&self) {
        // Create a local copy of the data we need to avoid borrowing issues
        let clusters: Vec<_> = self
            .floor
            .borrow()
            .clusters
            .iter()
            .map(|c| c.cluster)
            .collect();

        for cluster_char in clusters {
            println!(
                "stow slots in cluster {}: {}",
                cluster_char,
                self.stow_slots
                    .iter()
                    .filter(|s| s.cluster == cluster_char)
                    .count()
            );
        }
    }

    pub fn fill_stow_slots_around_locked(&mut self, max_aisle_count: usize) {
        let mut new_stow_slots: Vec<StowSlot> = Vec::new();
        for cluster in &mut self.floor.borrow_mut().clusters {
            let mut count = 0;
            let mut current_slot: StowSlot = StowSlot::new(cluster.cluster, Vec::new());
            for aisle_pair in &mut cluster.aisle_pairs {
                let aisles = aisle_pair.get_aisles();
                // if the count is greater than the max_aisle_count or the aisles have a locked aisle, create a new stow slot and return
                if count >= max_aisle_count || aisles.iter().any(|a| a.locked) {
                    new_stow_slots.push(current_slot);
                    current_slot = StowSlot::new(cluster.cluster, Vec::new());
                    count = 0;
                } else {
                    count += 1;
                    current_slot.add_aisles(aisles);
                }
            }
            new_stow_slots.push(current_slot);
        }
        self.stow_slots = new_stow_slots;
    }

    pub fn add_stow_slot(
        &mut self,
        mut stow_slot: StowSlot,
        is_locked: bool,
    ) -> Result<(), String> {
        if stow_slot.aisles.iter().any(|a| a.locked) {
            return Err("Stow slot contains locked aisles".to_string());
        }

        // TODO: Check if the current stow slot is surrounded by other stow slots with only 1 aisle pair, if so this stow slot should not be made. from self.stow_slots.
        let floor = self.floor.borrow();
        let cluster = floor.get_cluster(stow_slot.cluster).unwrap();
        stow_slot.aisles.sort_by_key(|a| a.aisle_num);
        let (i1, _pair1) = cluster
            .get_pair_from_aisle(stow_slot.aisles.first().unwrap())
            .unwrap();
        let (i2, _pair2) = cluster
            .get_pair_from_aisle(stow_slot.aisles.last().unwrap())
            .unwrap();

        let mut aisle_pairs = cluster.aisle_pairs.clone();
        aisle_pairs.sort_by_key(|p| p.aisle1.as_ref().unwrap().aisle_num);

        match (aisle_pairs.get(i1 - 2), aisle_pairs.get(i1 - 1)) {
            (Some(pair2), Some(pair1)) => {
                // if pair2 has no aisles or has a locked aisle and pair1 has an aisle that is not locked, return true.
                if pair2.get_aisles().is_empty()
                    || pair2.get_aisles().iter().any(|a| a.locked)
                        && pair1.get_aisles().iter().any(|a| !a.locked)
                {
                    return Err("Stow slot is not allowed to be created".to_string());
                }
            }
            (None, Some(pair1)) => {
                // if pair2 doesn't exist and pair1 has no aisles or pair1 doesn't have a locked aisle, return true.
                if pair1.get_aisles().is_empty() || pair1.get_aisles().iter().any(|a| !a.locked) {
                    return Err("Stow slot is not allowed to be created".to_string());
                }
            }
            (Some(_), None) => return Err("unreachable".to_string()),
            (_, _) => {}
        };

        match (aisle_pairs.get(i2 + 1), aisle_pairs.get(i2 + 2)) {
            (Some(pair1), Some(pair2)) => {
                // if pair1 has no aisles or has a locked aisle and pair2 has an aisle that is not locked, return true.
                if pair1.get_aisles().is_empty()
                    || pair1.get_aisles().iter().any(|a| a.locked)
                        && pair2.get_aisles().iter().any(|a| !a.locked)
                {
                    return Err("Stow slot is not allowed to be created".to_string());
                }
            }
            (None, Some(pair2)) => {
                // if pair1 doesn't exist and pair2 has no aisles or pair2 doesn't have a locked aisle, return true.
                if pair2.get_aisles().is_empty() || pair2.get_aisles().iter().any(|a| !a.locked) {
                    return Err("Stow slot is not allowed to be created".to_string());
                }
            }
            (Some(_), None) => return Err("unreachable".to_string()),
            (_, _) => {}
        };

        if is_locked {
            if stow_slot.locked {
                return Err("Stow slot is already locked".to_string());
            }
            stow_slot.toggle_lock();
        }

        self.stow_slots.push(stow_slot);
        Ok(())
    }

    pub fn start_algorithm(&mut self, algorithm: AlgorithmConfig) {
        match algorithm.algorithm {
            Algorithm::TargetPPH => self.start_algorithm_target_pph(algorithm),
            Algorithm::TargetHC => self.start_algorithm_target_hc(algorithm),
        }
    }

    pub fn start_algorithm_target_pph(&mut self, algorithm: AlgorithmConfig) {
        // First collect all the aisles we need to process
        let mut aisle_assignments: Vec<(char, Rc<Aisle>, Option<Rc<Aisle>>)> = Vec::new();

        // Collect all the data we need in a separate scope to limit the borrow
        {
            // Borrow the floor immutably to collect data
            let floor = self.floor.borrow();
            for cluster in &floor.clusters {
                for aisle in &cluster.aisles {
                    let previous = cluster.get_previous_aisle(aisle.aisle_num).cloned();
                    aisle_assignments.push((cluster.cluster, Rc::clone(aisle), previous));
                }
            }
        }

        // Now process the assignments without borrowing self.floor
        for (cluster_char, aisle, previous_aisle) in aisle_assignments {
            match previous_aisle {
                Some(previous) => {
                    if let Some(existing_slot) = self.get_stow_slot_from_aisle(&previous) {
                        if existing_slot.pph <= algorithm.target_pph as f32 {
                            existing_slot.add_aisle(Rc::clone(&aisle));
                            continue;
                        }
                    }
                    // Borrow floor only when needed and in a limited scope
                    let new_slot = { StowSlot::new(cluster_char, vec![Rc::clone(&aisle)]) };
                    self.stow_slots.push(new_slot);
                }
                None => {
                    // Borrow floor only when needed and in a limited scope
                    let new_slot = { StowSlot::new(cluster_char, vec![Rc::clone(&aisle)]) };
                    self.stow_slots.push(new_slot);
                }
            }
        }
    }

    pub fn start_algorithm_target_hc(&mut self, algorithm: AlgorithmConfig) {
        // First generate the aisle pairs if aisle pairs are not generated.
        if self
            .floor
            .borrow()
            .clusters
            .iter()
            .all(|c| c.aisle_pairs.is_empty())
        {
            // Use a block to limit the scope of the mutable borrow
            let mut floor = self.floor.borrow_mut();
            floor.generate_aisle_pairs();
        }

        // Now collect all the aisle pairs we need to process
        let mut lowest_pph: Vec<(StowSlot, f32)> = Vec::new();
        self.floor.borrow().clusters.iter().for_each(|c| {
            let lowest = self.get_lowest_pph(c, 3, 3);
            lowest_pph.extend(lowest);
        });
        lowest_pph.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        lowest_pph
            .iter()
            .for_each(|slot| slot.0.display_aisle_range());
        self.fill_stow_slots_around_locked(2);
        while self.stow_slots.len() >= algorithm.target_hc as usize {
            let diff = self.stow_slots.len() - algorithm.target_hc as usize;

            if self
                .floor
                .borrow()
                .clusters
                .iter()
                .all(|c| c.aisle_pairs.len() % 2 == 0)
            {}
        }
    }
}
