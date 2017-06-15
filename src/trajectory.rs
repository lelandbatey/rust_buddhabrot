
use num::complex::Complex;

/// `trajectory` is a module containing the definitions and utility functions for buddhabrot
/// trajectories. It includes the `trajectory` struct, as well as functions for reading and writing
/// collections of trajectories to files.

pub struct BrotPoint {
    img_x: i64,
    img_y: i64,
    location: Complex<f64>,
}

pub struct Trajectory {
    initial_c: Complex<f64>,
    /// length of a trajectory may not be the same length of the vector of waypoints, as some
    /// waypoints may be excluded due to falling outside the viewing area.
    length: i64,
    waypoints: Vec<BrotPoint>,
}
