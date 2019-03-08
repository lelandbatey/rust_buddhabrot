
extern crate argparse;
extern crate serde_json;
extern crate serde;
extern crate rand;
extern crate time;

extern crate buddhabrot;

use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;
use std::time::Duration;
use std::thread;

use buddhabrot::buddha::{Trajectory, Complex, Waypoint};
use argparse::{ArgumentParser, Store};
use rand::Rng;

fn main() {
    let mut thread_count = 3;
    let mut trajectory_count = 1000;
    let mut max_iterations: i64 = 1024;
    let mut min_iterations: i64 = 64;
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
    coordinate_search(thread_count, trajectory_count, max_iterations, min_iterations)
}


// Function to coordinate other functions
// Function to search for candidates and write them to a channel
// Function to recieve from the channel and write them to the output of choice
fn coordinate_search(thread_count: usize, trajectory_count: usize, max_iterations: i64, min_iterations: i64) {
    // Choose an output file based on the current time. This file name is a good candidate for a
    // user-providable CLI parameter in the future.
    //let filename = time::strftime("trajectory_candidates_%Y-%m-%d__%H-%M-%S.json", &time::now()).unwrap();

    // Set up the variables necessary for candidate searching
    //
    // Calculate the number of trajectories each thread should search for. We do integer division
    // to find the number of trajectories each thread should search for, so the total count may be
    // less than the count specified on the CLI
    let per_thread_traj_count: usize = trajectory_count / thread_count;
    let total_trajectory_count: usize = per_thread_traj_count * thread_count;
    let thread_trajectories: Vec<usize> = (0..thread_count).map(|_| per_thread_traj_count).collect();

    // Start the threads that do the searching
    let (sender, reciever) = channel();
    for trajs in thread_trajectories.iter() {
        let child_sender = sender.clone();
        let t = trajs.clone();
        let _ = thread::spawn(move || {
            search_and_transmit(t, max_iterations, min_iterations, child_sender);
        });
    }

    // Start the function for recieving and writing to the file

    // If the program is failing to find *anything* for long enough, we want it to time out and
    // just print what we've got. The timeout is thus based on a bare minimum, 1/4 of a second,
    // plus a number of milliseconds equal to 100 * the minimum number of iterations. It's a pretty
    // usable heuristic.
    let timeout = Duration::from_millis(250 + (100 * min_iterations) as u64);

    // Recieve all the trajectories and print them (for now)
    for _ in 0..total_trajectory_count {
        match reciever.recv_timeout(timeout) {
            Ok(trajectory) => {
                println!("{}", serde_json::to_string(&trajectory).unwrap());
            }
            Err(_) => {
                println!("\n\nTimed Out!\n\n");
                break;
            }
        }
    }
}

// tells us if a point in the complex plane will loop forever by telling us if it's within the main
// cardiod or within the second-order bulb. This returning false doesn't guarantee that there's a
// finite number of loops, as this is just a quick special case to speed things up.
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
fn search_and_transmit(trajectory_count: usize, max_iterations: i64, min_iterations: i64, sender: Sender<Trajectory>) {
    let mut rng = rand::thread_rng();
    let mut valid_trajectory_count = 0;
    // centerx : hard coded at -0.75
    // centery : hard coded at 0
    // x span: [-2.5, 1.0]
    // y span: [-1.0, 1.0]
    //
    let (startx, stopx): (f64, f64) = (-2.5, 1.0);
    let (starty, stopy): (f64, f64) = (-1.0, 1.0);
    let xspan = stopx - startx;
    let yspan = stopy - starty;

    while valid_trajectory_count < trajectory_count {
        let mut escaped = false;
        let mut z = Complex::new(0.0, 0.0);
        let cn = Complex::new(startx + rng.gen::<f64>() * xspan, starty + rng.gen::<f64>() * yspan);
        let mut trajectory: Trajectory = Trajectory {
            init_c: cn,
            waypoints: Vec::new(),
            length: 0,
        };
        if will_loop_forever(cn) {
            continue;
        }
        let mut periods = HashMap::new();
        for itercount in 0..max_iterations {
            trajectory.length = itercount as i64;
            if escaped {
                break;
            }
            z = z * z + cn;
            let waypoint = Waypoint {
                // Ignore the image coordinates as they'll never be used in this program.
                img_x: 0,
                img_y: 0,
                point: z.clone(),
            };
            trajectory.waypoints.push(waypoint);
            if z.norm() > 2.0 {
                escaped = true;
            }
            // Check if we've encountered this point before (useful for avoiding cyclical
            // but never ending z's). This bit of math is a fancy way of checking if
            // itercount is a power of 2. This algorithm is called "Brent's Algorithm" and
            // I originally found it here: https://softologyblog.wordpress.com/2011/06/26/buddhabrot-fractals/
            if itercount & (itercount - 1) == 0 {
                let k = format!("{:?}", z);
                if periods.contains_key(&k) {
                    break;
                }
                periods.insert(k, itercount);
            }
        }
        if escaped && !(trajectory.length < min_iterations) {
            match sender.send(trajectory) {
                Ok(_) => (),
                Err(_) => break,
            }
            valid_trajectory_count += 1;
        }
    }
}
