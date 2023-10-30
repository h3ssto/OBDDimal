//! Utilities for BDD testing
use bitvec::prelude::*;
use obddimal::misc::hash_select::HashMap;
use rand::Rng;

/// This prints the one-columns of a random truth table with 8 variables,
/// for testcase generation. The result is to be used in src/core/test.rs.
#[allow(unused)]
pub fn main() {
    let mut truthtable: HashMap<u8, bool> = Default::default();

    let mut rng = rand::thread_rng();

    for i in 0u8..=255u8 {
        let f: bool = rng.gen();
        truthtable.insert(i, f);
    }

    for (k, v) in truthtable {
        if v {
            println!("{},", k.view_bits::<Lsb0>())
        }
    }
}
