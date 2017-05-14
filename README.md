
Rust Buddhabrot Renderer
========================

A multi-threaded buddhabrot renderer written in Rust generating color PNG images.


Here's an example of building and running this code and the output it produces:

```
# Commands to install and run
cargo build --release 
/usr/bin/time -f "Wall time: %e, Max resident size: %M" ./target/release/buddhabrot -x -0.5 --width 1024 --height 1024 --threads 3 --samples 0.001 --sample_scale 5 --max_iters 100000 --min_iters 10000
```

Produced the following image after 5-10 minutes of processing on my computer:

![](http://lelandbatey.com/projects/buddhabrot/1k--intense-fractal2017-05-13_20:36:35.png)
