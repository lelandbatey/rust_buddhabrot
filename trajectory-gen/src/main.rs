
use std::sync::mpsc::channel;
use std::cmp::{min, max};
use std::time::Duration;
use std::io::Write;
use std::fs::File;
use std::thread;
use rand::Rng;
use std::fmt;

extern crate serde_json;
extern crate argparse;
extern crate time;
extern crate rand;

extern crate buddhabrot;

use argparse::{ArgumentParser, Store};
use buddhabrot::buddha::{Trajectory, Complex};

fn main() {
    println!("Hello, world!");
    let mut thread_count = 3;
    let mut trajectory_count = 1000;
    let mut max_iterations: i64 = 1024;
    let mut min_iterations: i64 = 0;

    let mut output_template = "%Y-%m-%d__%H-%M-%S_trajectories.json".to_string();

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
        argparse.refer(&mut output_template).add_option(
            &["--output-template"],
            Store,
            "The template name for the output file. Uses `strftime` formating.",
        );
        argparse.parse_args_or_exit();
    }
    let file_out_tmpl = time::strftime(&output_template, &time::now()).unwrap();
    // Create file for logging JSON values of trajectories.
    let json_path = file_out_tmpl.clone() + ".json";
    println!("Number of threads to use: {}", thread_count);
    println!("results of the json path: {}", json_path);
}

#[derive(Clone)]
pub struct Conf {
    pub json_file: String,
    pub thread_count: usize,
    pub max_iterations: i64,
    pub min_iterations: i64,
    pub trajectory_count: usize,
}

impl fmt::Display for Conf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Conf{{
    json_file: {},
    thread_count: {},
    max_iterations: {},
    min_iterations: {},
    trajectory_count: {}
}}",
            self.json_file,
            self.thread_count,
            self.max_iterations,
            self.min_iterations,
            self.trajectory_count
        )
    }
}

//#[derive(Serialize, Deserialize, Debug)]
//pub struct Waypoint {
//img_x: i32,
//img_y: i32,
//point: Complex,
//}

//#[derive(Serialize, Deserialize, Debug)]
//pub struct Trajectory {
//init_c: Complex,
//// We won't serialize the "waypoints" field when creating a JSON string, since that would take
//// up way too much space for long trajectories. However, the 'length' field will be serialized
//// which for my purposes is all that'll be needed.
//#[serde(skip_serializing)]
//waypoints: Vec<Waypoint>,
///// Length is the number of valid waypoints within the
//length: i64,
//}

// tells us if a point in the complex plane will loop forever by telling us if it's within the main
// cardiod or within the second-order bulb.
fn will_loop_forever(z: Complex) -> bool {
    let x = z.re;
    let y = z.im;
    let p: f64 = ((x - 0.25).powi(2) + y.powi(2)).sqrt();
    if x < (p - (2.0 * p.powi(2)) + 0.25) {
        return true;
    }
    if ((x + 1.0).powi(2) + y.powi(2)) < 0.0625 {
        return true;
    }
    return false;
}

pub fn render_buddhabort(c: Conf) {

    let max_trajectories: usize = c.trajectory_count;
    let max_thread_traj = max(1, (max_trajectories / c.thread_count));
    let to_recieve: usize = min(c.trajectory_count, (max_thread_traj * c.thread_count));

    let (tx, rx) = channel();
    for idx in 0..c.thread_count {
        let child_tx = tx.clone();
        let tconf = c.clone();
        // Spin up threads to calculate trajectories
        let child = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut valid_traj = 0;
            while valid_traj < max_thread_traj {
                let mut escaped = false;
                let mut z = Complex::new(0.0, 0.0);
                let samplescale = 1.5;
                let cn = Complex::new(
                    rng.gen::<f64>() * samplescale,
                    rng.gen::<f64>() * samplescale,
                );
                let mut trajectory: Trajectory = Trajectory {
                    init_c: cn,
                    waypoints: Vec::new(),
                    length: 0,
                };
                match child_tx.send(trajectory) {
                    Ok(_) => (),
                    Err(_) => break,
                }
            }
        });
    }

    let mut json_file = File::create(&std::path::Path::new(c.json_file.as_str())).unwrap();
    // If the program is failing to find *anything* for long enough, we want it to time out and
    // just print what we've got. The timeout is thus based on a bare minimum, 1/4 of a second,
    // plus 100 * the minimum number of iterations. It's a pretty usable heuristic.
    let timeout = Duration::from_millis(250 + (100 * c.min_iterations) as u64);
    for traj in 0..to_recieve {
        match rx.recv_timeout(timeout) {
            Ok(trajectory) => {
                write!(
                    json_file,
                    "{}\n",
                    serde_json::to_string(&trajectory).unwrap()
                ).unwrap();
            }
            Err(_) => {
                println!("\n\nTimed out!\n\n");
                break;
            }
        }

    }
}
