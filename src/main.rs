
extern crate argparse;
extern crate image;
extern crate regex;
extern crate rand;
extern crate time;
extern crate num;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use num::complex::Complex;
use std::time::Duration;
use std::str::FromStr;
use std::f64::consts;
use std::ops::Deref;
use std::path::Path;
use std::io::Write;
use std::cmp::{min, max};
use std::fs::File;
use std::io::Read;
use std::thread;
use rand::Rng;

use image::Pixel;

use argparse::{ArgumentParser, Store};
use regex::Regex;
use std::io;


struct Img {
    height: i64,
    width: i64,
    maximum: i64,
    minimum: i64,
    pixels: Vec<i64>,
}

fn fexp(x: f64, factor: f64) -> f64 {
    1.0 - (consts::E.powf(-factor * x))
}

fn log(x: f64, factor: f64) -> f64 {
    (factor * x + 1.0).ln()
}

impl Img {
    pub fn new(h: i64, w: i64) -> Img {
        Img {
            height: h,
            width: w,
            maximum: 1,
            minimum: 1000,
            pixels: vec![0; (h*w) as usize],
        }
    }
    pub fn set_px(&mut self, x: i64, y: i64, val: i64) {
        if x < self.width && x >= 0 {
            if y < self.height && y >= 0 {
                if val > self.maximum {
                    self.maximum = val
                }
                if val < self.minimum {
                    self.minimum = val
                }
                self.pixels[((self.height * y) + x) as usize] = val
            }
        }
    }
    pub fn incr_px(&mut self, x: i64, y: i64) {
        if x < self.width && x >= 0 {
            if y < self.height && y >= 0 {
                let mut px = self.pixels[((self.height * y) + x) as usize];
                px = px + 1;
                if px > self.maximum {
                    self.maximum = px
                }
                if px < self.minimum {
                    self.minimum = px
                }
                self.pixels[((self.height * y) + x) as usize] = px;
            }
        }
    }
    pub fn pix_val(&mut self, x: i64, y: i64) -> i64 {
        self.pixels[((self.height * y as i64) + x as i64) as usize]
    }
    /// Returns the pixel specified scaled to a u8 by passing the raw value of the pixel and the
    /// maximum pixel value within the image to delegate. `delegate` must return as floating point
    /// value between 0.0 and 1.0, inclusive.
    pub fn scaled_pix_delegate<F>(&self, x: i64, y: i64, delegate: F) -> u8
        where F: Fn(f64, f64) -> f64
    {
        let val = self.pixels[((self.height * y as i64) + x as i64) as usize] as f64;
        (delegate(val, self.maximum as f64) * 255.0) as u8
    }
    pub fn scaled_pix_val(&self, x: i64, y: i64) -> u8 {
        self.scaled_pix_delegate(x,
                                 y,
                                 |val, mx| (fexp(val as f64, 0.001) / fexp(mx as f64, 0.001)))
    }
}

// write_ppm writes a PPM formated image from a vector of Img structs
fn write_ppm(imgs: Vec<Img>, fname: String) {
    let mut ppm = std::io::BufWriter::new(File::create(fname.as_str()).unwrap());

    write!(ppm, "P3\n# Created by leland batey RustPPM\n").unwrap();
    write!(ppm, "{} {}\n", imgs[0].width, imgs[0].height).unwrap();
    write!(ppm,
           "{}\n",
           max(imgs[0].maximum, max(imgs[1].maximum, imgs[2].maximum)))
            .unwrap();
    for pidx in 0..imgs[0].pixels.len() {
        write!(ppm,
               "{} {} {}\n",
               imgs[0].pixels[pidx],
               imgs[1].pixels[pidx],
               imgs[2].pixels[pidx])
                .unwrap();
    }
}

/// read_ppm reads a plain ppm file into a triplet of Img structs.
fn read_ppm(fname: String) -> Vec<Img> {
    let mut f = File::open(fname).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    let re = Regex::new(r"#.*").unwrap();
    let nocomments = re.replace_all(contents.as_str(), "");
    let lines = nocomments.split('\n').collect::<Vec<&str>>();
    // Simple state machine for parsing PPM
    enum State {
        awaitMagicNum,
        awaitWidth,
        awaitHeight,
        awaitMaxval,
        awaitRed,
        awaitGreen,
        awaitBlue,
    }
    // Our vector of images, each representing a color channel, in order [r, g, b].
    let mut imgs: Vec<Img> = vec![];
    let mut cur: State = State::awaitMagicNum;
    let mut height: i64 = 0;
    let mut width: i64 = 0;

    let mut x = 0;
    let mut y = 0;
    for line in lines {
        if line == "" {
            continue;
        }
        let tokens = line.split(char::is_whitespace).collect::<Vec<&str>>();
        for token in tokens {
            if token == "" {
                continue;
            }
            match cur {
                State::awaitMagicNum => {
                    if token == "P3" {
                        cur = State::awaitWidth;
                    }
                }
                State::awaitWidth => {
                    width = token.parse().unwrap();
                    cur = State::awaitHeight;
                }
                State::awaitHeight => {
                    height = token.parse().unwrap();
                    cur = State::awaitMaxval;
                }
                State::awaitMaxval => {
                    // We actually ignore the maxval and calculate that on a per-channel level
                    // automatically since each channel of RGB is represented as its own Img
                    // structure.
                    cur = State::awaitRed;
                    // But, let's take the time now to initialize our images
                    imgs.push(Img::new(width, height));
                    imgs.push(Img::new(width, height));
                    imgs.push(Img::new(width, height));

                }
                State::awaitRed => {
                    imgs[0].set_px(x, y, token.parse().unwrap());
                    cur = State::awaitGreen;
                }
                State::awaitGreen => {
                    imgs[1].set_px(x, y, token.parse().unwrap());
                    cur = State::awaitBlue;
                }
                State::awaitBlue => {
                    imgs[2].set_px(x, y, token.parse().unwrap());
                    cur = State::awaitRed;
                    x += 1;
                }
            }
            if x == width {
                y += 1;
                x = 0;
            }
        }
    }
    return imgs;
}

struct Waypoint {
    img_x: i32,
    img_y: i32,
    point: Complex<f64>,
}

struct Trajectory {
    init_c: Complex<f64>,
    waypoints: Vec<Waypoint>,
    /// Length is the number of valid waypoints within the
    length: i64,
}

#[derive(Copy, Clone)]
struct BuddhaConf {
    thread_count: usize,
    max_iterations: i64,
    min_iterations: i64,
    width: i64,
    height: i64,
    samplescale: f64,
    centerx: f64,
    centery: f64,
    zoomlevel: f64,
    trajectory_count: usize,
}

// tells us if a point in the complex plane will loop forever by telling us if it's within the main
// cardiod or within the second-order bulb.
fn will_loop_forever(z: Complex<f64>) -> bool {
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


fn render_buddhabort(c: BuddhaConf) -> Vec<Img> {
    let startzoom = 2.0;
    let (startx, stopx) = (c.centerx - (startzoom / (2.0 as f64).powf(c.zoomlevel)),
                           c.centerx + (startzoom / (2.0 as f64).powf(c.zoomlevel)));
    let (starty, stopy) = (c.centery - (startzoom / (2.0 as f64).powf(c.zoomlevel)),
                           c.centery + (startzoom / (2.0 as f64).powf(c.zoomlevel)));
    let MAX_TRAJECTORIES: usize = c.trajectory_count;

    let mut children = vec![];

    let max_thread_traj = max(1, (MAX_TRAJECTORIES / c.thread_count));
    let to_recieve: usize = min(c.trajectory_count, (max_thread_traj * c.thread_count));
    println!("Spawning {} threads, each producing {} trajectories, for a total of {} trajectories being produced",
             c.thread_count,
             max_thread_traj,
             to_recieve);
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
                let cn = Complex::new((startx * tconf.samplescale) +
                                      rng.gen::<f64>() *
                                      ((stopx * tconf.samplescale) - (startx * tconf.samplescale)),
                                      (starty * tconf.samplescale) +
                                      rng.gen::<f64>() *
                                      ((stopy * tconf.samplescale) - (starty * tconf.samplescale)));
                let mut trajectory: Trajectory = Trajectory {
                    init_c: cn,
                    waypoints: Vec::new(),
                    length: 0,
                };
                if will_loop_forever(cn) {
                    continue;
                }
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
                         y >= (tconf.height as f64)) {
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
                }
                if escaped {
                    if !(trajectory.length < tconf.min_iterations) {
                        //println!("length: {}, dist: {}, waypoints: {}, did escape: '{}'",
                        //trajectory.length,
                        //trajectory.init_c.norm(),
                        //trajectory.waypoints.len(),
                        //trajectory.init_c);
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
    let mut imgs: Vec<Img> = vec![Img::new(c.width, c.height),
                                  Img::new(c.width, c.height),
                                  Img::new(c.width, c.height)];

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
                if (traj % max((MAX_TRAJECTORIES / 100), 1)) == 0 {
                    print!("{}%\r",
                           ((traj as f64 / MAX_TRAJECTORIES as f64) * 100.0) as u32);
                    io::stdout().flush().unwrap();
                }
                let final_iteration = trajectory.length;
                let freq = iter_freq.entry(final_iteration).or_insert(0);
                *freq += 1;
                for p in trajectory.waypoints {
                    let iter_span: f64 = (c.max_iterations - c.min_iterations) as f64;
                    let min_iters: f64 = c.min_iterations as f64;

                    // If we've set a sufficiently height minimum iteration number, then the
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

// rescale_ppm accepts the path of a PPM file, reads that ppm file, applies several different
// scaling functions to the values of each pixel in the PPM and saves a new PNG for each scaling
// function.
fn rescale_ppm(ppmname: String) {

    let imgs = read_ppm(ppmname.clone());
    println!("{}", imgs.len());
    println!("{}x{}", imgs[1].width, imgs[0].height);
    let mut scaling_funcs: Vec<(&str, Box<Fn(f64, f64) -> f64>)> = Vec::new();
    scaling_funcs.push(("fexp0_001",
                        Box::new(|val, mx| (fexp(val as f64, 0.001) / fexp(mx as f64, 0.001)))));
    scaling_funcs.push(("fexp0_005",
                        Box::new(|val, mx| (fexp(val as f64, 0.005) / fexp(mx as f64, 0.005)))));
    scaling_funcs.push(("fexp0_010",
                        Box::new(|val, mx| (fexp(val as f64, 0.010) / fexp(mx as f64, 0.010)))));
    scaling_funcs.push(("fexp0_050",
                        Box::new(|val, mx| (fexp(val as f64, 0.050) / fexp(mx as f64, 0.050)))));
    scaling_funcs.push(("fexp0_100",
                        Box::new(|val, mx| (fexp(val as f64, 0.100) / fexp(mx as f64, 0.100)))));
    scaling_funcs.push(("log1_0", Box::new(|val, mx| log(val as f64, 1.0) / log(mx as f64, 1.0))));
    scaling_funcs.push(("log0_5", Box::new(|val, mx| log(val as f64, 0.5) / log(mx as f64, 0.5))));
    scaling_funcs.push(("log0_1", Box::new(|val, mx| log(val as f64, 0.1) / log(mx as f64, 0.1))));
    scaling_funcs
        .push(("log0_01", Box::new(|val, mx| log(val as f64, 0.01) / log(mx as f64, 0.01))));

    for func in scaling_funcs {
        let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(imgs[0].width as u32,
                                                                            imgs[0].height as u32);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = imgs[0].scaled_pix_delegate(x as i64, y as i64, func.1.deref());
            let g = imgs[1].scaled_pix_delegate(x as i64, y as i64, func.1.deref());
            let b = imgs[2].scaled_pix_delegate(x as i64, y as i64, func.1.deref());

            *pixel = image::Rgb([r, g, b]);
        }
        let pngname = ppmname.clone() + func.0 + ".png";
        let ref mut fout = File::create(&Path::new(pngname.as_str())).unwrap();
        let _ = image::ImageRgb8(imgbuf).save(fout, image::PNG);
    }
}


fn main() {
    let mut ppmname = "".to_owned();

    let mut thread_count = 3;
    let mut max_iterations: i64 = 1024;
    let mut min_iterations: i64 = 0;

    let mut imgx: i64 = 4096;
    let mut imgy: i64 = 4096;

    let mut samplescale = 1.5;

    let (mut centerx, mut centery) = (-0.74, 0.0);
    let mut zoomlevel = 1.0;

    let mut sample_multiplier: f64 = 200.0;

    let mut trajectory_count: usize = 0;

    {
        let mut argparse = ArgumentParser::new();
        argparse.set_description("Render a buddhabrot set as PNG");
        argparse
            .refer(&mut thread_count)
            .add_option(&["-t", "--threads"],
                        Store,
                        "Number of threads to use (default 4)");
        argparse
            .refer(&mut imgx)
            .add_option(&["--width"], Store, "Width of the output image");
        argparse
            .refer(&mut imgy)
            .add_option(&["--height"], Store, "Height of the output image");
        argparse
            .refer(&mut max_iterations)
            .add_option(&["--max_iters"],
                        Store,
                        "Maximum number of allowed iterations.");
        argparse
            .refer(&mut min_iterations)
            .add_option(&["--min_iters"],
                        Store,
                        "Minimum required number of iterations.");
        argparse
            .refer(&mut centerx)
            .add_option(&["-x"], Store, "The center X coordinate");
        argparse
            .refer(&mut centery)
            .add_option(&["-y"], Store, "The center Y coordinate");
        argparse
            .refer(&mut zoomlevel)
            .add_option(&["-z", "--zoom"], Store, "Amount of zoom in render");
        argparse
            .refer(&mut sample_multiplier)
            .add_option(&["-s", "--samples"],
                        Store,
                        "Number of samples per pixel (default 200)");
        argparse
            .refer(&mut trajectory_count)
            .add_option(&["--trajectory-count"],
                        Store,
                        "Absolute number of trajectories to find");
        argparse
            .refer(&mut samplescale)
            .add_option(&["--sample_scale"],
                        Store,
                        "Size of sampling area compared to viewing area (default 5)");
        argparse
            .refer(&mut ppmname)
            .add_option(&["--rescale-ppm"],
                        Store,
                        "Name of ppm to rescale with different algorithms");
        argparse.parse_args_or_exit();
    }

    if ppmname != "" {
        rescale_ppm(ppmname);
        return;
    }

    if trajectory_count == 0 {
        trajectory_count = (imgx as f64 * imgy as f64 * sample_multiplier) as usize;
    }

    let conf = BuddhaConf {
        thread_count: thread_count,
        max_iterations: max_iterations as i64,
        min_iterations: min_iterations,
        width: imgx,
        height: imgy,
        samplescale: samplescale,
        centerx: centerx,
        centery: centery,
        zoomlevel: zoomlevel,
        trajectory_count: trajectory_count,
    };


    let imgs: Vec<Img> = render_buddhabort(conf);

    println!("Finished coming up with pixel values");
    // Create a new ImgBuf with width: imgx and height: imgy
    let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(imgx as u32, imgy as u32);

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = imgs[0].scaled_pix_val(x as i64, y as i64);
        let g = imgs[1].scaled_pix_val(x as i64, y as i64);
        let b = imgs[2].scaled_pix_val(x as i64, y as i64);

        *pixel = image::Rgb([r, g, b]);
    }

    let rightnow = time::strftime("fractal%Y-%m-%d_%H:%M:%S", &time::now()).unwrap();
    println!("Completed at {}", rightnow);

    // Save as a plain ppm
    write_ppm(imgs, rightnow.clone() + ".ppm");

    // Save the image as “fractal.png”
    let pngname = rightnow + ".png";
    let ref mut fout = File::create(&Path::new(pngname.as_str())).unwrap();
    // We must indicate the image’s color type and what format to save as
    let _ = image::ImageRgb8(imgbuf).save(fout, image::PNG);
}
