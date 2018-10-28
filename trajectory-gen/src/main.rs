
extern crate argparse;

use argparse::{ArgumentParser, Store};

fn main() {
    println!("Hello, world!");
    let mut thread_count = 3;
    let mut trajectory_count = 1000;
    let mut max_iterations: i64 = 1024;
    let mut min_iterations: i64 = 0;
    {
        let mut argparse = ArgumentParser::new();
        argparse.refer(&mut thread_count).add_option(
            &["-t", "--threads"],
            Store,
            "Number of threads to use (default 3)",
        );
        argparse.refer(&mut trajectory_count).add_option(
            &["--trajectory-count"],
            Store,
            "Absolute number of trajectories to find",
        );
        argparse.refer(&mut max_iterations).add_option(
            &["--max-iters"],
            Store,
            "Maximum number of allowed iterations.",
        );
        argparse.refer(&mut min_iterations).add_option(
            &["--min-iters"],
            Store,
            "Minimum required number of iterations.",
        );
        argparse.parse_args_or_exit();
    }
    println!("Number of threads to use: {}", thread_count);
}
