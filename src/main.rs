

//extern crate argparse;
extern crate image;
extern crate rand;
extern crate time;
extern crate num;

use std::sync::mpsc::channel;
use std::time::Duration;
use std::str::FromStr;
use std::cmp::max;
use std::thread;

use image::Pixel;
use std::fs::File;
use std::path::Path;

use num::complex::Complex;
use rand::Rng;


struct Img {
    height: i64,
    width: i64,
    maximum: i64,
    minimum: i64,
    pixels: Vec<i64>,
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
}


fn main() {
    let thread_count = 3;
    let max_iterations = 256u16;

    let imgx = 800;
    let imgy = 800;

    let scalex = 4.0 / imgx as f32;
    let scaley = 4.0 / imgy as f32;

    let (mut centerx, mut centery) = (-0.74, 0.0);
    //let startzoom = 1.26;
    let startzoom = 2.0;

    let mut zoomlevel = 1.0;

    let (startx, stopx) = (centerx - (startzoom / (2.0 as f64).powf(zoomlevel)),
                           centerx + (startzoom / (2.0 as f64).powf(zoomlevel)));
    let (starty, stopy) = (centery - (startzoom / (2.0 as f64).powf(zoomlevel)),
                           centery + (startzoom / (2.0 as f64).powf(zoomlevel)));

    //let MAX_TRAJECTORIES: usize = 12000000;
    let MAX_TRAJECTORIES: usize = 480000000;
    let MAX_ITERATIONS = 64;

    let mut children = vec![];

    let (tx, rx) = channel();
    for c in 0..thread_count {
        let child_tx = tx.clone();
        // Spin up threads to calculate trajectories
        let child = thread::spawn(move || {
            println!("Thread {} started", c);
            for traj in 0..(MAX_TRAJECTORIES / thread_count) {
                let mut reststops: Vec<[u32; 2]> = Vec::new();
                let mut escaped = false;
                let mut z = Complex::new(0.0, 0.0);
                let samplescale = 1.5;
                let c = Complex::new((startx * samplescale) +
                                     rand::random::<f64>() *
                                     ((stopx * samplescale) - (startx * samplescale)),
                                     (starty * samplescale) +
                                     rand::random::<f64>() *
                                     ((stopy * samplescale) - (starty * samplescale)));
                for _ in 0..MAX_ITERATIONS {
                    if escaped {
                        break;
                    }
                    z = z * z + c;

                    let length = stopx - startx;
                    // May want to swap x and y for upward facing buddha
                    let x = (z.re - startx) / length * imgx as f64;
                    let y = (z.im - starty) / (stopy - starty) * imgy as f64;

                    if !(x < 0.0 || x >= (imgx as f64) || y < 0.0 || y >= (imgy as f64)) {
                        reststops.push([x as u32, y as u32]);
                    }
                    if z.norm() > 2.0 {
                        escaped = true;
                    }
                }
                if escaped {
                    //child_tx.send(reststops.clone()).unwrap();
                    match child_tx.send(reststops.clone()) {
                        Ok(_) => (),
                        Err(_) => break,
                    }
                }
            }
            println!("Thread {} finished", c);
            drop(child_tx);
        });
        children.push(child);
    }

    let mut img = Img::new(imgx, imgy);

    let mut done = false;
    println!("Begun recieving reststops");
    let timeout = Duration::from_millis(250);
    for _ in 0..MAX_TRAJECTORIES {
        match rx.recv_timeout(timeout) {
            Ok(reststops) => {
                for p in reststops {
                    img.incr_px(p[0] as i64, p[1] as i64);
                }
            }
            Err(_) => {
                println!("Timed out!");
                break;
            }
        }
    }
    println!("Finished coming up with pixel values");
    // Create a new ImgBuf with width: imgx and height: imgy
    let mut imgbuf = image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::new(imgx as u32, imgy as u32);

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let val = img.pixels[((img.height * y as i64) + x as i64) as usize];
        *pixel = pixel.map(|v| ((val as f64 / img.maximum as f64) * 255.0) as u8)
    }

    // Save the image as “fractal.png”
    let rightnow = time::strftime("%Y-%m-%d_%H:%M:%S", &time::now()).unwrap();
    let ref mut fout = File::create(&Path::new((String::from_str("fractal").unwrap() +
                                                rightnow.as_str() +
                                                ".png")
            .as_str()))
        .unwrap();
    // We must indicate the image’s color type and what format to save as
    let _ = image::ImageLuma8(imgbuf).save(fout, image::PNG);
    //println!("Hello, world!");
}
