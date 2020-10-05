
extern crate rand;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::ops::{Add, Mul};
use std::cmp::{min, max};
use std::time::Duration;
use std::io::Write;
use std::fs::File;
use std::thread;
use rand::Rng;
use std::fmt;
use std::io;
use std;

extern crate serde;
extern crate serde_json;

use ppm;

/// An implementation of Complex numbers. I could use the `num` crate which has an existing generic
/// implementation of Complex, and in fact that is what I used to use. However, I couldn't get it
/// to work with Serde, so I wrote my own implementation with concrete floats that works with
/// Serde out of the box.
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Mul<Complex> for Complex {
    type Output = Complex;
    #[inline]
    fn mul(self, _rhs: Complex) -> Complex {
        Complex {
            re: (self.re * _rhs.re) - (self.im * _rhs.im),
            im: (self.re * _rhs.im) + (self.im * _rhs.re),
        }
    }
}

impl Add<Complex> for Complex {
    type Output = Complex;
    fn add(self, _rhs: Complex) -> Complex {
        Complex {
            re: self.re + _rhs.re,
            im: self.im + _rhs.im,
        }
    }
}

impl fmt::Display for Complex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:5.1}+{:5.1}j)", self.re, self.im)
    }
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Complex {
        return Complex { re: re, im: im };
    }
    pub fn norm(&self) -> f64 {
        self.re.hypot(self.im)
    }
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Waypoint {
    pub img_x: i32,
    pub img_y: i32,
    pub point: Complex,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trajectory {
    pub init_c: Complex,
    // We won't serialize the "waypoints" field when creating a JSON string, since that would take
    // up way too much space for long trajectories. However, the 'length' field will be serialized
    // which for my purposes is all that'll be needed.
    #[serde(skip)]
    pub waypoints: Vec<Waypoint>,
    /// Length is the number of valid waypoints within the
    pub length: i64,
}

#[derive(Clone)]
pub struct Conf {
    pub json_file: String,
    pub thread_count: usize,
    pub max_iterations: i64,
    pub min_iterations: i64,
    pub width: i64,
    pub height: i64,
    pub samplescale: f64,
    pub centerx: f64,
    pub centery: f64,
    pub zoomlevel: f64,
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
    width: {},
    height: {},
    samplescale: {},
    centerx: {},
    centery: {},
    zoomlevel: {},
    trajectory_count: {}
}}",
            self.json_file,
            self.thread_count,
            self.max_iterations,
            self.min_iterations,
            self.width,
            self.height,
            self.samplescale,
            self.centerx,
            self.centery,
            self.zoomlevel,
            self.trajectory_count
        )
    }
}

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


pub fn render_buddhabort(c: Conf) -> Vec<ppm::Img> {
    let startzoom = 2.0;
    let (startx, stopx) = (
        c.centerx - (startzoom / (2.0 as f64).powf(c.zoomlevel)),
        c.centerx + (startzoom / (2.0 as f64).powf(c.zoomlevel)),
    );
    let (starty, stopy) = (
        c.centery - (startzoom / (2.0 as f64).powf(c.zoomlevel)),
        c.centery + (startzoom / (2.0 as f64).powf(c.zoomlevel)),
    );
    let max_trajectories: usize = c.trajectory_count;

    let mut children = vec![];

    let max_thread_traj = max(1, max_trajectories / c.thread_count);
    let to_recieve: usize = min(c.trajectory_count, max_thread_traj * c.thread_count);
    println!(
        "Spawning {} threads, each producing {} trajectories, for a total of {} \
              trajectories being produced",
        c.thread_count,
        max_thread_traj,
        to_recieve
    );
    let (tx, rx) = channel();
    for idx in 0..c.thread_count {
        let child_tx = tx.clone();
        let tconf = c.clone();
        // Spin up threads to calculate trajectories
        let child = thread::spawn(move || {
            println!("Thread {} started", idx);
            let mut rng = rand::thread_rng();

            let mut valid_traj = 0;

            while valid_traj < max_thread_traj {
                let mut escaped = false;
                let mut z = Complex::new(0.0, 0.0);
                let cn = Complex::new(
                    (startx * tconf.samplescale) +
                        rng.gen::<f64>() *
                            ((stopx * tconf.samplescale) - (startx * tconf.samplescale)),
                    (starty * tconf.samplescale) +
                        rng.gen::<f64>() *
                            ((stopy * tconf.samplescale) - (starty * tconf.samplescale)),
                );
                let mut trajectory: Trajectory = Trajectory {
                    init_c: cn,
                    waypoints: Vec::new(),
                    length: 0,
                };
                if will_loop_forever(cn) {
                    continue;
                }
                let mut periods = HashMap::new();
                for itercount in 0..tconf.max_iterations {
                    trajectory.length = itercount;
                    if escaped {
                        break;
                    }
                    z = z * z + cn;

                    // May want to swap x and y for upward facing buddha
                    let x = (z.re - startx) / (stopx - startx) * tconf.width as f64;
                    let y = (z.im - starty) / (stopy - starty) * tconf.height as f64;

                    if !(x < 0.0 || x >= (tconf.width as f64) || y < 0.0 ||
                             y >= (tconf.height as f64))
                    {
                        let waypoint = Waypoint {
                            img_x: x as i32,
                            img_y: y as i32,
                            point: z.clone(),
                        };
                        trajectory.waypoints.push(waypoint);
                    }
                    if z.norm() > 2.0 {
                        escaped = true;
                    }
                    // Check if we've encountered this point before (useful for avoiding cyclical
                    // but never ending z's). This bit of math is a fancy way of checking if
                    // itercount is a power of 2
                    if itercount & (itercount - 1) == 0 {
                        let k = format!("{:?}", z);
                        if periods.contains_key(&k) {
                            break;
                        }
                        periods.insert(k, itercount);
                    }
                }
                if escaped {
                    if !(trajectory.length < tconf.min_iterations) {
                        match child_tx.send(trajectory) {
                            Ok(_) => (),
                            Err(_) => break,
                        }
                        valid_traj += 1;
                    }
                }
            }
            println!("Thread {} finished", idx);
            drop(child_tx);
        });
        children.push(child);
    }

    // Our vector of images, each representing a color channel, in order [r, g, b].
    let mut imgs: Vec<ppm::Img> = vec![
        ppm::Img::new(c.width, c.height),
        ppm::Img::new(c.width, c.height),
        ppm::Img::new(c.width, c.height),
    ];

    println!("Write to json file: {}", c.json_file);
    let mut json_file = File::create(&std::path::Path::new(c.json_file.as_str())).unwrap();
    let mut logfile = File::create("itercounts.txt").unwrap();
    let mut iter_freq: HashMap<i64, i64> = HashMap::new();

    println!("Begun recieving trajectories");

    // If the program is failing to find *anything* for long enough, we want it to time out and
    // just print what we've got. The timeout is thus based on a bare minimum, 1/4 of a second,
    // plus 100 * the minimum number of iterations. It's a pretty usable heuristic.
    let timeout = Duration::from_millis(250 + (100 * c.min_iterations) as u64);

    // Receive each trajectory found by the workers, using the waypoints of that trajectory to
    // increment brightness values of the output images.
    for traj in 0..to_recieve {
        match rx.recv_timeout(timeout) {
            Ok(trajectory) => {
                if (traj % max(max_trajectories / 100, 1)) == 0 {
                    print!(
                        "{}%\r",
                        ((traj as f64 / max_trajectories as f64) * 100.0) as u32
                    );
                    io::stdout().flush().unwrap();
                }
                write!(
                    json_file,
                    "{}\n",
                    serde_json::to_string(&trajectory).unwrap()
                ).unwrap();
                let final_iteration = trajectory.length;
                let freq = iter_freq.entry(final_iteration).or_insert(0);
                *freq += 1;
                for p in trajectory.waypoints {
                    let iter_span: f64 = (c.max_iterations - c.min_iterations) as f64;
                    let min_iters: f64 = c.min_iterations as f64;

                    // If we've set a sufficiently high minimum iteration number, then the
                    // distribution of discovered orbits will be much more uniform, so make the
                    // color distribution uniform. Otherwise, have it be inverse log base 10 to
                    // compensate for the large number of small orbits.
                    let red_factor = if c.min_iterations > 100 { 0.40 } else { 0.10 };
                    let green_factor = if c.min_iterations > 100 { 0.10 } else { 0.01 };

                    let red_min = ((iter_span * red_factor) + min_iters) as i64;
                    let green_min = ((iter_span * green_factor) + min_iters) as i64;
                    let blue_max = green_min;
                    if final_iteration > red_min {
                        imgs[0].incr_px(p.img_x as i64, p.img_y as i64);
                    } else if final_iteration > green_min {
                        imgs[1].incr_px(p.img_x as i64, p.img_y as i64);
                    } else if final_iteration < blue_max {
                        imgs[2].incr_px(p.img_x as i64, p.img_y as i64);
                    }
                }
            }
            Err(_) => {
                println!("\n\nTimed out!\n\n");
                break;
            }
        }
    }
    for (key, val) in iter_freq.iter() {
        write!(logfile, "{} {}\n", key, val).unwrap();
    }
    println!("Finished coming up with pixel values");

    return imgs;
}
