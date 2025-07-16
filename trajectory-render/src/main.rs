//#[macro use]
extern crate crossbeam_channel;

extern crate argparse;
extern crate serde;
extern crate serde_json;

extern crate buddhabrot;

use std::collections::HashMap;
use std::io;
use std::thread;
use std::time::Duration;

use argparse::{ArgumentParser, Store, StoreTrue};
use crossbeam_channel::{unbounded, Receiver, Sender};

use buddhabrot::buddha::{Complex, Trajectory, Waypoint};
use buddhabrot::ppm;

fn main() -> io::Result<()> {
    let mut scale_ppm_many = false;
    let mut thread_count = 3;
    let mut height: i64 = 1024;
    let mut width: i64 = 1024;
    let mut output_fname: String = "image.ppm".to_string();
    {
        let mut argparse = ArgumentParser::new();
        argparse.refer(&mut scale_ppm_many).add_option(
            &["--scale-ppm-many"],
            StoreTrue,
            "Whether to output many different scaled values of the PPM as PNG (default is off)",
        );
        argparse.refer(&mut height).add_option(
            &["--height"],
            Store,
            "Height in pixels of the output image (default 1024)",
        );
        argparse.refer(&mut width).add_option(
            &["--width"],
            Store,
            "Width in pixels of the output image (default 1024)",
        );
        argparse.refer(&mut output_fname).add_option(
            &["-o", "--output"],
            Store,
            "Path of the output image (default 'image.ppm')",
        );
        argparse.refer(&mut thread_count).add_option(
            &["-t", "--threads"],
            Store,
            "Number of threads to use (default 3)",
        );
        argparse.parse_args_or_exit();
    }
    println!("Height: {}", height);
    println!("Width: {}", width);

    let mut trajectories: Vec<Trajectory> = vec![];
    let mut trajectory_count = 0;
    let (s1, r) = unbounded();
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                if input.trim() == "" {
                    continue;
                }
                let traj: Trajectory = serde_json::from_str(input.as_str())?;
                s1.send(traj.clone()).unwrap();
                trajectories.push(traj.clone());
                trajectory_count += 1;
            }
            Err(error) => {
                println!("error: {}", error);
                return Err(error);
            }
        };
    }
    let (wps, wpr) = unbounded();
    for _ in 0..thread_count {
        let _r = r.clone();
        let _wps = wps.clone();
        let _ = thread::spawn(move || {
            calculate_waypoints(_r, _wps);
        });
    }

    // Our vector of images, each representing a color channel, in order [r, g, b].
    let mut imgs: Vec<ppm::Img> = vec![
        ppm::Img::new(width, height),
        ppm::Img::new(width, height),
        ppm::Img::new(width, height),
    ];
    let mut wp_added = 0;
    println!("Trajectory count {}", trajectories.len());
    let max_iterations = trajectories
        .iter()
        .fold(0, |max, x| if x.length > max { x.length } else { max });
    let min_iterations =
        trajectories.iter().fold(
            std::i64::MAX,
            |min, x| if x.length < min { x.length } else { min },
        );
    let timeout = Duration::from_millis(950 + (100 * max_iterations) as u64);
    println!("Max length of trajectory: {}", max_iterations);
    println!("Min length of trajectory: {}", min_iterations);
    for _ in 0..trajectory_count {
        let trajectory: Trajectory = match wpr.recv_timeout(timeout) {
            Ok(t) => t,
            Err(_) => {
                break;
            }
        };
        for p in &trajectory.waypoints {
            let (px, py) = calc_pixel_pos(p.point.re, p.point.im, height, width);
            if px == -1 {
                continue;
            };

            let final_iteration = trajectory.length;
            let iter_span: f64 = (max_iterations - min_iterations) as f64;
            let min_iters: f64 = min_iterations as f64;

            // If we've set a sufficiently high minimum iteration number, then the
            // distribution of discovered orbits will be much more uniform, so make the
            // color distribution uniform. Otherwise, have it be inverse log base 10 to
            // compensate for the large number of small orbits.
            //
            // values have been tweaked from above to give more blue and green with longer
            // trajectories
            let red_factor = if min_iterations > 100 { 0.70 } else { 0.10 };
            let green_factor = if min_iterations > 100 { 0.20 } else { 0.01 };

            let red_min = ((iter_span * red_factor) + min_iters) as i64;
            let green_min = ((iter_span * green_factor) + min_iters) as i64;
            let blue_max = green_min;
            if max_iterations == min_iterations {
                // If there's only one trajectory, make it white so it's very visible.
                imgs[0].incr_px(px, py);
                imgs[1].incr_px(px, py);
                imgs[2].incr_px(px, py);
            } else if final_iteration > red_min {
                imgs[0].incr_px(px, py);
            } else if final_iteration > green_min {
                imgs[1].incr_px(px, py);
            } else if final_iteration < blue_max {
                imgs[2].incr_px(px, py);
            }
            wp_added += 1;
        }
    }
    println!("Waypoints added: {}", wp_added);
    ppm::write_ppm(&imgs, output_fname.clone());

    if scale_ppm_many {
        println!("--scale-ppm-many provided, writing image to disk as PNG but scaled using many different algorithms");
        ppm::rescale_ppm(&imgs, output_fname.clone());
    }

    let parts: Vec<&str> = output_fname.split(".").collect();
    let no_ext = &parts[0..parts.len() - 1].join(".");
    ppm::write_scaled_png(&imgs, "scaled_".to_owned() + no_ext + ".png", |val, mx| {
        ppm::fexp(val as f64, 0.100) / ppm::fexp(mx as f64, 0.100)
    });
    Ok(())
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
fn calculate_waypoints(receive_traj: Receiver<Trajectory>, send_waypoints: Sender<Trajectory>) {
    // centerx : hard coded at -0.75
    // centery : hard coded at 0
    // x span: [-2.5, 1.0]
    // y span: [-1.0, 1.0]

    loop {
        let old_traj: Trajectory = match receive_traj.try_recv() {
            Ok(t) => t,
            Err(_) => {
                break;
            }
        };
        let mut escaped = false;
        let mut z = Complex::new(0.0, 0.0);
        let cn = Complex::new(old_traj.init_c.re, old_traj.init_c.im);
        let mut trajectory: Trajectory = Trajectory {
            init_c: cn,
            waypoints: Vec::new(),
            length: 0,
        };
        if will_loop_forever(cn) {
            continue;
        }
        let mut periods = HashMap::new();
        for itercount in 0..(old_traj.length) {
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
        match send_waypoints.send(trajectory) {
            Ok(_) => (),
            Err(_) => {
                break;
            }
        }
    }
}

fn calc_pixel_pos(x: f64, y: f64, height: i64, width: i64) -> (i64, i64) {
    // original
    //let (startx, stopx): (f64, f64) = (-2.25, 0.75);
    //let (starty, stopy): (f64, f64) = (-1.5, 1.5);

    // cuts off the bottom
    //let (startx, stopx): (f64, f64) = (-1.75, 0.25);
    //let (starty, stopy): (f64, f64) = (-1.0, 1.0);

    // Shrunk by 1/4 from the left, 1/8 top & 1/8 bottom
    // 1/4 = 0.75
    // 1/8 = 0.375
    // This one centers the frame SO much better!
    let (startx, stopx): (f64, f64) = (-1.5, 0.75);
    let (starty, stopy): (f64, f64) = (-1.125, 1.125);

    let xspan = stopx - startx;
    let yspan = stopy - starty;

    let xp = (x - startx) / xspan * width as f64;
    let yp = (y - starty) / yspan * height as f64;
    if xp < 0.0 || xp >= (width as f64) || yp < 0.0 || yp >= (height as f64) {
        return (-1, -1);
    }
    return (xp as i64, yp as i64);
}
