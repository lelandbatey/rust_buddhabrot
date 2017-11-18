
#[macro_use]
extern crate serde_derive;

extern crate argparse;
extern crate image;
extern crate rand;
extern crate time;
//extern crate num;

use std::path::Path;
use std::fs::File;

use argparse::{ArgumentParser, Store};


mod ppm;
mod buddha;

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
        argparse.refer(&mut thread_count).add_option(
            &["-t", "--threads"],
            Store,
            "Number of threads to use (default 4)",
        );
        argparse.refer(&mut imgx).add_option(
            &["--width"],
            Store,
            "Width of the output image",
        );
        argparse.refer(&mut imgy).add_option(
            &["--height"],
            Store,
            "Height of the output image",
        );
        argparse.refer(&mut max_iterations).add_option(
            &["--max_iters"],
            Store,
            "Maximum number of allowed iterations.",
        );
        argparse.refer(&mut min_iterations).add_option(
            &["--min_iters"],
            Store,
            "Minimum required number of iterations.",
        );
        argparse.refer(&mut centerx).add_option(
            &["-x"],
            Store,
            "The center X coordinate",
        );
        argparse.refer(&mut centery).add_option(
            &["-y"],
            Store,
            "The center Y coordinate",
        );
        argparse.refer(&mut zoomlevel).add_option(
            &["-z", "--zoom"],
            Store,
            "Amount of zoom in render",
        );
        argparse.refer(&mut sample_multiplier).add_option(
            &["-s", "--samples"],
            Store,
            "Number of samples per pixel (default 200)",
        );
        argparse.refer(&mut trajectory_count).add_option(
            &["--trajectory-count"],
            Store,
            "Absolute number of trajectories to find",
        );
        argparse.refer(&mut samplescale).add_option(
            &["--sample_scale"],
            Store,
            "Size of sampling area compared to viewing area (default 5)",
        );
        argparse.refer(&mut ppmname).add_option(
            &["--rescale-ppm"],
            Store,
            "Name of ppm to rescale with different algorithms",
        );
        argparse.parse_args_or_exit();
    }

    if ppmname != "" {
        ppm::rescale_ppm(ppmname);
        return;
    }

    if trajectory_count == 0 {
        trajectory_count = (imgx as f64 * imgy as f64 * sample_multiplier) as usize;
    }

    // Create the template name for all files to be created of this session.
    let file_out_tmpl = time::strftime("fractal%Y-%m-%d__%H-%M-%S", &time::now()).unwrap();
    // Create file for logging JSON values of trajectories.
    let jsonname = file_out_tmpl.clone() + ".json";

    let conf = buddha::Conf {
        json_file: jsonname,
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

    println!("Starting with these values: {:?}", conf);

    let imgs: Vec<ppm::Img> = buddha::render_buddhabort(conf);

    println!("Finished coming up with pixel values");
    // Create a new ImgBuf with width: imgx and height: imgy
    let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(imgx as u32, imgy as u32);

    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = imgs[0].scaled_pix_val(x as i64, y as i64);
        let g = imgs[1].scaled_pix_val(x as i64, y as i64);
        let b = imgs[2].scaled_pix_val(x as i64, y as i64);

        *pixel = image::Rgb([r, g, b]);
    }

    let rightnow = time::strftime("%Y-%m-%d__%H-%M-%S", &time::now()).unwrap();
    println!("Completed at {}", rightnow);

    // Save as a plain ppm
    ppm::write_ppm(imgs, file_out_tmpl.clone() + ".ppm");

    // Save the image as “fractal.png”
    let pngname = file_out_tmpl + ".png";
    let ref mut fout = File::create(&Path::new(pngname.as_str())).unwrap();
    // We must indicate the image’s color type and what format to save as
    let _ = image::ImageRgb8(imgbuf).save(fout, image::PNG);
}
