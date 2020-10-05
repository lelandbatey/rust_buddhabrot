
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
