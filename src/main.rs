
extern crate argparse;
extern crate image;
extern crate rand;
extern crate time;
extern crate num;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use num::complex::Complex;
use std::time::Duration;
use std::str::FromStr;
use std::f64::consts;
use std::path::Path;
use std::io::Write;
use std::cmp::max;
use std::fs::File;
use std::thread;
use rand::Rng;

use image::Pixel;

use argparse::{ArgumentParser, Store};
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
    pub fn scaled_pix_val(&self, x: i64, y: i64) -> u8 {
        let val = self.pixels[((self.height * y as i64) + x as i64) as usize];
        (fexp(val as f64, 0.001) / fexp(self.maximum as f64, 0.001) * 255.0) as u8
        // These other scaling methods lead to somewhat less "nice looking" intensity scaling
        //(log(val as f64, 0.01) / log(self.maximum as f64, 0.01) * 255.0) as u8
        //((val as f64 / self.maximum as f64) * 255.0) as u8
    }
}

// write_ppm writes a PPM formated image from a vector of Img structs
fn write_ppm(imgs: Vec<Img>, fname: String) {
    let mut ppm = File::create(fname.as_str()).unwrap();

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
    sample_multiplier: f64,
}

// tells us if a point in the complex plane will loop forever by telling us if it's within the main
// cardiod or within a second-order bulb.
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
    let MAX_TRAJECTORIES: usize = (c.width as f64 * c.height as f64 * c.sample_multiplier) as usize;

    let mut children = vec![];

    let max_thread_traj = (MAX_TRAJECTORIES / c.thread_count);
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
                let mut reststops: (i64, Vec<[u32; 2]>) = (0, Vec::new());
                let mut escaped = false;
                let mut z = Complex::new(0.0, 0.0);
                let cn = Complex::new((startx * tconf.samplescale) +
                                      rng.gen::<f64>() *
                                      ((stopx * tconf.samplescale) - (startx * tconf.samplescale)),
                                      (starty * tconf.samplescale) +
                                      rng.gen::<f64>() *
                                      ((stopy * tconf.samplescale) - (starty * tconf.samplescale)));
                if will_loop_forever(cn) {
                    continue;
                }
                let mut final_iteration = 0;
                for itercount in 0..tconf.max_iterations {
                    if escaped {
                        final_iteration = itercount;
                        break;
                    }
                    z = z * z + cn;

                    // May want to swap x and y for upward facing buddha
                    let x = (z.re - startx) / (stopx - startx) * tconf.width as f64;
                    let y = (z.im - starty) / (stopy - starty) * tconf.height as f64;

                    if !(x < 0.0 || x >= (tconf.width as f64) || y < 0.0 ||
                         y >= (tconf.height as f64)) {
                        reststops.1.push([x as u32, y as u32]);
                    }
                    if z.norm() > 2.0 {
                        escaped = true;
                    }
                }
                if escaped {
                    reststops.0 = final_iteration;
                    if !(final_iteration < tconf.min_iterations) {
                        match child_tx.send(reststops.clone()) {
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
    let mut imgs: Vec<Img> =
        vec![Img::new(c.width, c.height), Img::new(c.width, c.height), Img::new(c.width, c.height)];

    let mut logfile = File::create("itercounts.txt").unwrap();
    let mut iter_freq: HashMap<i64, i64> = HashMap::new();

    println!("Begun recieving reststops");
    let timeout = Duration::from_millis(250 + (100 * c.min_iterations) as u64);
    for traj in 0..(max_thread_traj * c.thread_count) {
        match rx.recv_timeout(timeout) {
            Ok(reststops) => {
                if (traj % max((MAX_TRAJECTORIES / 100), 1)) == 0 {
                    print!("{}%\r",
                           ((traj as f64 / MAX_TRAJECTORIES as f64) * 100.0) as u32);
                    io::stdout().flush().unwrap();
                }
                let final_iteration = reststops.0;
                let freq = iter_freq.entry(final_iteration).or_insert(0);
                *freq += 1;
                for p in reststops.1 {
                    let iter_span: f64 = (c.max_iterations - c.min_iterations) as f64;
                    let min_iters: f64 = c.min_iterations as f64;

                    // If we've set a sufficiently hight minimum iteration number, then the
                    // distribution of discovered orbits will be much more uniform, so make the
                    // color distribution uniform. Otherwise, have it be inverse exponential to
                    // compensate for the large number of small orbits.
                    let red_factor = if c.min_iterations > 100 { 0.40 } else { 0.10 };
                    let green_factor = if c.min_iterations > 100 { 0.10 } else { 0.01 };

                    let red_min = ((iter_span * red_factor) + min_iters) as i64;
                    let green_min = ((iter_span * green_factor) + min_iters) as i64;
                    let blue_max = green_min;
                    if final_iteration > red_min {
                        imgs[0].incr_px(p[0] as i64, p[1] as i64);
                    } else if final_iteration > green_min {
                        imgs[1].incr_px(p[0] as i64, p[1] as i64);
                    } else if final_iteration < blue_max {
                        imgs[2].incr_px(p[0] as i64, p[1] as i64);
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


fn main() {
    let mut thread_count = 3;
    let mut max_iterations: i64 = 1024;
    let mut min_iterations: i64 = 0;

    let mut imgx: i64 = 4096;
    let mut imgy: i64 = 4096;

    let mut samplescale = 5.0;

    let (mut centerx, mut centery) = (-0.74, 0.0);
    let mut zoomlevel = 1.0;

    let mut sample_multiplier: f64 = 200.0;

    {
        let mut argparse = ArgumentParser::new();
        argparse.set_description("Render a buddhabrot set as PNG");
        argparse.refer(&mut thread_count)
            .add_option(&["-t", "--threads"],
                        Store,
                        "Number of threads to use (default 4)");
        argparse.refer(&mut imgx)
            .add_option(&["--width"], Store, "Width of the output image");
        argparse.refer(&mut imgy)
            .add_option(&["--height"], Store, "Height of the output image");
        argparse.refer(&mut max_iterations)
            .add_option(&["--max_iters"],
                        Store,
                        "Maximum number of allowed iterations.");
        argparse.refer(&mut min_iterations)
            .add_option(&["--min_iters"],
                        Store,
                        "Minimum required number of iterations.");
        argparse.refer(&mut centerx)
            .add_option(&["-x"], Store, "The center X coordinate");
        argparse.refer(&mut centery)
            .add_option(&["-y"], Store, "The center Y coordinate");
        argparse.refer(&mut zoomlevel)
            .add_option(&["-z", "--zoom"], Store, "Amount of zoom in render");
        argparse.refer(&mut sample_multiplier)
            .add_option(&["-s", "--samples"],
                        Store,
                        "Number of samples per pixel (default 200)");
        argparse.refer(&mut samplescale)
            .add_option(&["--sample_scale"],
                        Store,
                        "Size of sampling area compared to viewing area (default 5)");
        argparse.parse_args_or_exit();
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
        sample_multiplier: sample_multiplier,
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
