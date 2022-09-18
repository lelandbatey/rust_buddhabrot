extern crate image;
extern crate regex;

use std;
use std::cmp::max;
use std::f64::consts;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;

use self::regex::Regex;

#[derive(Clone)]
pub struct Img {
    height: i64,
    width: i64,
    maximum: i64,
    minimum: i64,
    pixels: Vec<i64>,
}

// This fexp scaling function is taken from here: https://www.brodie-tyrrell.org/bbrot/
pub fn fexp(x: f64, factor: f64) -> f64 {
    1.0 - (consts::E.powf(-factor * x))
}

pub fn log(x: f64, factor: f64) -> f64 {
    (factor * x + 1.0).ln()
}

/// The Img struct is the simplest possible implementation of an image: a two dimensional array of
/// pixels, each pixel represented only as a single integer representing the brightness of that
/// pixel. Multiple Img structs together can represent an RGB image, with one Img struct per
/// channel.
impl Img {
    pub fn new(h: i64, w: i64) -> Img {
        Img {
            height: h,
            width: w,
            maximum: 1,
            minimum: 1000,
            pixels: vec![0; (h * w) as usize],
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
    /// Returns the pixel specified scaled to a u8 by passing the raw value of the pixel and the
    /// maximum pixel value within the image to delegate. `delegate` must return as floating point
    /// value between 0.0 and 1.0, inclusive.
    pub fn scaled_pix_delegate<F>(&self, x: i64, y: i64, delegate: F) -> u8
    where
        F: Fn(f64, f64) -> f64,
    {
        let val = self.pixels[((self.height * y as i64) + x as i64) as usize] as f64;
        (delegate(val, self.maximum as f64) * 255.0) as u8
    }
    pub fn scaled_pix_val(&self, x: i64, y: i64) -> u8 {
        self.scaled_pix_delegate(x, y, |val, mx| {
            fexp(val as f64, 0.001) / fexp(mx as f64, 0.001)
        })
    }
}

// write_ppm writes a PPM formated image from a vector of Img structs
pub fn write_ppm(imgs: &Vec<Img>, fname: String) {
    let mut ppm = std::io::BufWriter::new(File::create(fname.as_str()).unwrap());

    write!(ppm, "P3\n# Created by leland batey RustPPM\n").unwrap();
    write!(ppm, "{} {}\n", imgs[0].width, imgs[0].height).unwrap();
    write!(
        ppm,
        "{}\n",
        max(imgs[0].maximum, max(imgs[1].maximum, imgs[2].maximum))
    )
    .unwrap();
    for pidx in 0..imgs[0].pixels.len() {
        write!(
            ppm,
            "{} {} {}\n",
            imgs[0].pixels[pidx], imgs[1].pixels[pidx], imgs[2].pixels[pidx]
        )
        .unwrap();
    }
}

// write_scaled_ppm writes a PPM formated image from a vector of Img structs, but with each pixel
// value scaled in brightness using the `fexp` function.
pub fn write_scaled_ppm(imgs: &Vec<Img>, fname: String) {
    let mut ppm = std::io::BufWriter::new(File::create(fname.as_str()).unwrap());
    let scale_fn = |val, mx| {
        let scaled_val = ((fexp(val as f64, 0.050) / fexp(mx as f64, 0.050)) * 255.0) as u8;
        // If a pixel is below the minimum brightness threshold but does still have a brightness,
        // then scale that pixel to the minimum brightness threshold.
        if val > 0 && val < (mx / 255) {
            return (mx / 255 + 1 as i64) as u8;
        } else {
            return scaled_val;
        }
    };
    let max_brightness = max(imgs[0].maximum, max(imgs[1].maximum, imgs[2].maximum));

    write!(ppm, "P3\n# Created by leland batey RustPPM\n").unwrap();
    write!(ppm, "{} {}\n", imgs[0].width, imgs[0].height).unwrap();
    write!(ppm, "{}\n", scale_fn(max_brightness, max_brightness)).unwrap();
    for pidx in 0..imgs[0].pixels.len() {
        write!(
            ppm,
            "{} {} {}\n",
            scale_fn(imgs[0].pixels[pidx], max_brightness),
            scale_fn(imgs[1].pixels[pidx], max_brightness),
            scale_fn(imgs[2].pixels[pidx], max_brightness),
        )
        .unwrap();
    }
}

pub fn write_scaled_png<F>(imgs: &Vec<Img>, fname: String, scale_func: F)
where
    F: Fn(f64, f64) -> f64,
{
    let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(
        imgs[0].width as u32,
        imgs[0].height as u32,
    );
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = imgs[0].scaled_pix_delegate(x as i64, y as i64, &scale_func);
        let g = imgs[1].scaled_pix_delegate(x as i64, y as i64, &scale_func);
        let b = imgs[2].scaled_pix_delegate(x as i64, y as i64, &scale_func);

        *pixel = image::Rgb([r, g, b]);
    }
    let ref mut fout = File::create(&Path::new(fname.as_str())).unwrap();
    let _ = image::ImageRgb8(imgbuf).save(fout, image::PNG);
}

pub fn write_png(imgs: &Vec<Img>, fname: String) {
    write_scaled_png(imgs, fname, |val, _| val);
}

/// read_ppm reads a plain ppm file into a triplet of Img structs.
pub fn read_ppm(fname: String) -> Vec<Img> {
    let mut f = File::open(fname).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    let re = Regex::new(r"#.*").unwrap();
    let nocomments = re.replace_all(contents.as_str(), "");
    let lines = nocomments.split('\n').collect::<Vec<&str>>();
    // Simple state machine for parsing PPM
    enum State {
        AwaitMagicNum,
        AwaitWidth,
        AwaitHeight,
        AwaitMaxval,
        AwaitRed,
        AwaitGreen,
        AwaitBlue,
    }
    // Our vector of images, each representing a color channel, in order [r, g, b].
    let mut imgs: Vec<Img> = vec![];
    let mut cur: State = State::AwaitMagicNum;
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
                State::AwaitMagicNum => {
                    if token == "P3" {
                        cur = State::AwaitWidth;
                    }
                }
                State::AwaitWidth => {
                    width = token.parse().unwrap();
                    cur = State::AwaitHeight;
                }
                State::AwaitHeight => {
                    height = token.parse().unwrap();
                    cur = State::AwaitMaxval;
                }
                State::AwaitMaxval => {
                    // We actually ignore the maxval and calculate that on a per-channel level
                    // automatically since each channel of RGB is represented as its own Img
                    // structure.
                    cur = State::AwaitRed;
                    // But, let's take the time now to initialize our images
                    imgs.push(Img::new(width, height));
                    imgs.push(Img::new(width, height));
                    imgs.push(Img::new(width, height));
                }
                State::AwaitRed => {
                    imgs[0].set_px(x, y, token.parse().unwrap());
                    cur = State::AwaitGreen;
                }
                State::AwaitGreen => {
                    imgs[1].set_px(x, y, token.parse().unwrap());
                    cur = State::AwaitBlue;
                }
                State::AwaitBlue => {
                    imgs[2].set_px(x, y, token.parse().unwrap());
                    cur = State::AwaitRed;
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

// rescale_ppm accepts the path of a PPM file, reads that ppm file, applies several different
// scaling functions to the values of each pixel in the PPM and saves a new PNG for each scaling
// function.
pub fn rescale_ppm(ppmname: String) {
    let imgs = read_ppm(ppmname.clone());
    println!("{}", imgs.len());
    println!("{}x{}", imgs[1].width, imgs[0].height);
    let mut scaling_funcs: Vec<(&str, Box<dyn Fn(f64, f64) -> f64>)> = Vec::new();
    scaling_funcs.push((
        "fexp0_001",
        Box::new(|val, mx| (fexp(val as f64, 0.001) / fexp(mx as f64, 0.001))),
    ));
    scaling_funcs.push((
        "fexp0_005",
        Box::new(|val, mx| (fexp(val as f64, 0.005) / fexp(mx as f64, 0.005))),
    ));
    scaling_funcs.push((
        "fexp0_010",
        Box::new(|val, mx| (fexp(val as f64, 0.010) / fexp(mx as f64, 0.010))),
    ));
    scaling_funcs.push((
        "fexp0_050",
        Box::new(|val, mx| (fexp(val as f64, 0.050) / fexp(mx as f64, 0.050))),
    ));
    scaling_funcs.push((
        "fexp0_100",
        Box::new(|val, mx| (fexp(val as f64, 0.100) / fexp(mx as f64, 0.100))),
    ));
    scaling_funcs.push((
        "log1_0",
        Box::new(|val, mx| log(val as f64, 1.0) / log(mx as f64, 1.0)),
    ));
    scaling_funcs.push((
        "log0_5",
        Box::new(|val, mx| log(val as f64, 0.5) / log(mx as f64, 0.5)),
    ));
    scaling_funcs.push((
        "log0_1",
        Box::new(|val, mx| log(val as f64, 0.1) / log(mx as f64, 0.1)),
    ));
    scaling_funcs.push((
        "log0_01",
        Box::new(|val, mx| log(val as f64, 0.01) / log(mx as f64, 0.01)),
    ));
    scaling_funcs.push((
        "ceil",
        Box::new(|val, _| {
            if val > 0.0 {
                return 1.0;
            }
            return 0.0;
        }),
    ));

    for func in scaling_funcs {
        let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(
            imgs[0].width as u32,
            imgs[0].height as u32,
        );
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
