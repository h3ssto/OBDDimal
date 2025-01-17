//! Some Benchmarks to evaluate some usecases of Views.
use std::{
    cmp, env, fs,
    path::Path,
    process,
    time::{Instant, SystemTime},
};

use csv::Writer;
use humantime::format_rfc3339_seconds;
use obddimal::{
    core::{bdd_node::VarID, order::var2level_to_ordered_varids},
    misc::hash_select::HashSet,
    views::bdd_view::BddView,
};
use rand::{seq::SliceRandom, thread_rng};

pub fn main() {
    // Create place to store the results:
    let folder_path = str::replace(
        &format!(
            "eval_views/results-{}",
            format_rfc3339_seconds(SystemTime::now()),
        )
        .to_string(),
        ":",
        "-",
    );
    if Path::new(&folder_path).exists() {
        println!("Results folder already exists?!");
        process::exit(1);
    }
    fs::create_dir_all(&folder_path).unwrap();

    // Run selected tests:
    let cmd_args: HashSet<String> = env::args().collect();
    let all = cmd_args.contains("--all") || cmd_args.len() == 1;

    if all || cmd_args.contains("--slicing") {
        evaluate_slicing(&folder_path, cmd_args.contains("--simple"));
    }

    if all || cmd_args.contains("--atomic-sets") {
        evaluate_atomic_sets(&folder_path);
    }

    if all || cmd_args.contains("--var_nodecount") {
        evaluate_var_nodecount(&folder_path);
    }
}

#[derive(serde::Serialize)]
struct SlicingMeasurement {
    sliced_variable: VarID,
    time_in_seconds: f64,
    size_before: usize,
    size_after: usize,
    nodes_in_manager_before: usize,
    nodes_in_manager_after: usize,
}

fn evaluate_slicing(folder_path: &str, simple: bool) {
    for example in [
        "sandwich",
        "berkeleydb",
        "embtoolkit",
        "busybox_1.18.0",
        "financialservices01",
        "automotive01",
        "automotive02_v1",
        "automotive02_v2",
        "automotive02_v3",
        "automotive02_v4",
    ]
    .iter()
    {
        let iteration_count: usize = if simple { 1 } else { 100 };
        //const MAX_BDDS_TO_KEEP: usize = isize::MAX as usize;
        const MAX_BDDS_TO_KEEP: usize = 2usize;
        for n in 0..iteration_count {
            println!(
                "Slicing {} (iteration {}/{}).",
                example,
                n + 1,
                iteration_count
            );
            // Prepare
            let mut bdds = vec![Some(
                BddView::load_from_dddmp_file(format!("examples/{}.dimacs.dddmp", example))
                    .unwrap()[0]
                    .clone(),
            )];
            bdds.resize(
                cmp::min(
                    bdds[0]
                        .as_ref()
                        .unwrap()
                        .get_manager()
                        .read()
                        .unwrap()
                        .var2level
                        .len(),
                    MAX_BDDS_TO_KEEP,
                ),
                None,
            );
            let mut last_bdd_pos = 0usize;
            let mut varids = var2level_to_ordered_varids(
                &bdds[0]
                    .as_ref()
                    .unwrap()
                    .get_manager()
                    .read()
                    .unwrap()
                    .var2level,
            );
            if !simple {
                varids.shuffle(&mut thread_rng());
            }
            let mut result_writer = Writer::from_path(if simple {
                format!("{}/slicing-{}-simple.csv", folder_path, example)
            } else {
                format!("{}/slicing-{}-{:02}.csv", folder_path, example, n)
            })
            .unwrap();
            // Measure
            for var_id in varids.iter() {
                if *var_id == VarID(0) {
                    continue;
                }

                // Calculate new bdd pos
                let new_bdd_pos = (last_bdd_pos + 1) % bdds.len();

                // Clean up potential removed bdds
                bdds[last_bdd_pos]
                    .as_ref()
                    .unwrap()
                    .get_manager()
                    .write()
                    .unwrap()
                    .clean();
                // Do measurements before
                let size_before = bdds[last_bdd_pos].as_ref().unwrap().count_nodes();
                let nodes_in_manager_before = bdds[last_bdd_pos]
                    .as_ref()
                    .unwrap()
                    .get_manager()
                    .read()
                    .unwrap()
                    .nodes
                    .len();
                // Do slicing
                let remove_vars = [*var_id].into_iter().collect::<HashSet<_>>();
                let time = Instant::now();
                bdds[new_bdd_pos] = Some(
                    bdds[last_bdd_pos]
                        .as_ref()
                        .unwrap()
                        .create_slice_without_vars(&remove_vars),
                );
                let elapsed = time.elapsed();
                // Do measurements after
                let size_after = bdds[new_bdd_pos].as_ref().unwrap().count_nodes();
                let nodes_in_manager_after = bdds[new_bdd_pos]
                    .as_ref()
                    .unwrap()
                    .get_manager()
                    .read()
                    .unwrap()
                    .nodes
                    .len();
                // Store result
                result_writer
                    .serialize(SlicingMeasurement {
                        sliced_variable: *var_id,
                        time_in_seconds: elapsed.as_secs_f64(),
                        size_before,
                        size_after,
                        nodes_in_manager_before,
                        nodes_in_manager_after,
                    })
                    .unwrap();
                result_writer.flush().unwrap();

                if simple {
                    // Remove new BDD
                    bdds[new_bdd_pos] = None;
                } else {
                    // Update old bdd pos
                    last_bdd_pos = new_bdd_pos;
                }
            }
        }
    }
}

#[derive(serde::Serialize)]
struct AtomicSetsMeasurement {
    bdd: String,
    time_in_seconds: f64,
    size_before: usize,
    size_after: usize,
}

fn evaluate_atomic_sets(folder_path: &str) {
    let mut result_writer = Writer::from_path(format!("{}/atomic_sets.csv", folder_path)).unwrap();
    for example in [
        "sandwich",
        "berkeleydb",
        "embtoolkit",
        "busybox_1.18.0",
        "financialservices01",
        "automotive01",
        //"automotive02_v1",
        //"automotive02_v2",
        //"automotive02_v3",
        //"automotive02_v4",
    ]
    .iter()
    {
        println!("Optimizing {}.", example);
        let bdds =
            BddView::load_from_dddmp_file(format!("examples/{}.dimacs.dddmp", example)).unwrap();
        for i in 0..bdds.len() {
            let bdd = bdds[i].clone();
            // Measure
            let size_before = bdd.count_nodes();
            let time = Instant::now();
            let bdd = bdd.optimize_through_atomic_sets().unwrap();
            let elapsed = time.elapsed();
            let size_after = bdd.count_nodes();
            // Store result
            let bdd_name = if bdds.len() == 1 {
                example.to_string()
            } else {
                format!("{}[{}]", example, i)
            };
            result_writer
                .serialize(AtomicSetsMeasurement {
                    bdd: bdd_name,
                    time_in_seconds: elapsed.as_secs_f64(),
                    size_before,
                    size_after,
                })
                .unwrap();
            result_writer.flush().unwrap();
        }
    }
}

#[derive(serde::Serialize)]
struct Var2NodeCount {
    variable: VarID,
    nodes_count: usize,
}

fn evaluate_var_nodecount(folder_path: &str) {
    for example in [
        "sandwich",
        "berkeleydb",
        "embtoolkit",
        "busybox_1.18.0",
        "financialservices01",
        "automotive01",
        "automotive02_v1",
        "automotive02_v2",
        "automotive02_v3",
        "automotive02_v4",
    ]
    .iter()
    {
        let bdd = BddView::load_from_dddmp_file(format!("examples/{}.dimacs.dddmp", example))
            .unwrap()[0]
            .clone();
        let varids = var2level_to_ordered_varids(&bdd.get_manager().read().unwrap().var2level);
        let mut result_writer =
            Writer::from_path(format!("{}/var2nodecount-{}.csv", folder_path, example)).unwrap();
        for var_id in varids.iter() {
            if *var_id == VarID(0) {
                continue;
            }

            let man = bdd.get_manager().clone();
            let man = man.read().unwrap();
            let nodes_count = man.level2nodes[man.var2level[var_id.0]].len();

            result_writer
                .serialize(Var2NodeCount {
                    variable: *var_id,
                    nodes_count,
                })
                .unwrap();
            result_writer.flush().unwrap();
        }
    }
}
