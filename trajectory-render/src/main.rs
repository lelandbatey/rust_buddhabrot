
//#[macro use]
extern crate crossbeam_channel;

extern crate serde_json;
extern crate argparse;
extern crate serde;

extern crate buddhabrot;

use std::collections::HashMap;
use std::time::Duration;
use std::thread;
use std::io;

use crossbeam_channel::{unbounded, Sender, Receiver};
use argparse::{ArgumentParser, Store};

use buddhabrot::buddha::{Trajectory, Complex, Waypoint};
use buddhabrot::ppm;

fn main() -> io::Result<()> {
    let mut thread_count = 3;
    let mut height: i64 = 1024;
    let mut width: i64 = 1024;
    {
        let mut argparse = ArgumentParser::new();
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
        argparse.parse_args_or_exit();
    }
    println!("Height: {}", height);
    println!("Width: {}", width);

    let mut trajectory_count = 0;
    let (s1, r) = unbounded();
    while true {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                let traj: Trajectory = serde_json::from_str(input.as_str())?;
                s1.send(traj).unwrap();
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

    let mut imgs: Vec<ppm::Img> = vec![
        ppm::Img::new(width, height),
        ppm::Img::new(width, height),
        ppm::Img::new(width, height),
    ];
    let timeout = Duration::from_millis(950);
    let mut wp_added = 0;
    println!("Trajectory count {}", trajectory_count);
    for progress in 0..trajectory_count {
        let trajectory: Trajectory = match wpr.recv_timeout(timeout) {
            Ok(t) => t,
            Err(_) => {break;}
        };
        println!("Progress {}", progress);
        for p in trajectory.waypoints {
            let (px, py) = calc_pixel_pos(p.point.re, p.point.im, height, width);
            if px == -1 {continue;};
            imgs[0].incr_px(px, py);
            imgs[1].incr_px(px, py);
            imgs[2].incr_px(px, py);
            wp_added += 1;
        }
    }
    println!("Waypoints added: {}", wp_added);
    ppm::write_ppm(imgs, "image.ppm".to_string());
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
    let (startx, stopx): (f64, f64) = (-2.5, 1.0);
    let (starty, stopy): (f64, f64) = (-1.0, 1.0);
    let xspan = stopx - startx;
    let yspan = stopy - starty;

    while true {
        let old_traj: Trajectory = match receive_traj.try_recv() {
            Ok(t) => t,
            Err(_) => {break;}
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
        for itercount in 0..old_traj.length {
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
        if escaped {
            match send_waypoints.send(trajectory) {
                Ok(_) => (),
                Err(_) => break,
            }
        }
    }

}

fn calc_pixel_pos(x: f64, y: f64, height: i64, width: i64) -> (i64, i64) {
    let (startx, stopx): (f64, f64) = (-2.5, 1.0);
    let (starty, stopy): (f64, f64) = (-1.75, 1.75);
    let xspan = stopx - startx;
    let yspan = stopy - starty;

    let xp = (x - startx) / xspan * width as f64;
    let yp = (y - starty) / yspan * height as f64;
    if xp < 0.0 || xp >= (width as f64) || yp < 0.0 || yp >= (height as f64) {
        return (-1, -1);
    }
    return (xp as i64, yp as i64);
}
