
Rust Buddhabrot Renderer
========================

A multi-threaded [buddhabrot](https://en.wikipedia.org/wiki/Buddhabrot) renderer written in Rust generating color PNG images.


Here's an example of building and running this code and the output it produces:

```
# Commands to install and run
cargo build --release
/usr/bin/time -f "Wall time: %e, Max resident size: %M" ./target/release/trajectory-gen --max-iters 80000 --min-iters 50000 -t 3 --trajectory-count 50 > test_trajs.json
./target/release/trajectory-render -o image.png < test_trajs.json
```

Produced the following image after 60 seconds of processing on my computer:

![](https://user-images.githubusercontent.com/1964720/95031604-c1f10e80-066b-11eb-95a2-f30bb09092bf.png)

### What are Buddhabrot fractals?

Buddhabrot fractals are 2-d histograms (a.k.a. probability distributions) of
the `z` values from iterating the Mandelbrot equation, but only using *certain
special* complex numbers as the inputs. These "special" complex numbers are
those complex numbers which are "almost" in the Mandelbrot set but which do
escape to infinity eventually. The complex numbers which we graph the
histograms of are the complex numbers which take a very long time (many
iterations) through the Mandelbrot equation before their `z` value becomes
large enough that it escapes to infinity. We refer to the combination of an
initial complex number and the various `z` values that it spits as a
**"trajectory"**. A trajectory will have an initial starting complex number
(`cn` in the Mandelbrot equation) and then a sequnce of complex numbers which
are the `z` values produced after each iteration step of the Mandelbrot
equation. Most trajectories that we randomly select will quickly display one of
a few behaviors:

1. If the initial point is outside the Mandelbrot set, then `z` values will usually immediately start to grow off to infinity
2. If the initial point is inside the Mandelbrot set, then it will never escape to infinity because its `z` values will form a cycle

The trajectories we're looking for are those which are *just barely* outside of
the Mandelbrot set; the trajectories which do grow to infinity but only after
many, many iterations (hundreds of thousands to millions+ iterations).

### What am I actually looking at in these pictures?

You're looking at a grid with color squares (pixels) which visualizes where the
`z` values of long-running trajectories landed in their many iterations. The
brighter a square, then the more times a `z` value landed somewhere in the
complex plane covered by that square pixel. The colors don't have a lot of
significance (in my renderings). I create the colors by sorting trajectories
into three piles, one for each channel of red-green-blue, then effectively
overlaying those three separate "black-and-white" pictures to make the colors.
There's no significance to the colors other than looking nice.
