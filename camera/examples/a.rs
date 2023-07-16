use camera::{CvCamera, Camera};

fn main() {
    let mut cam = CvCamera::new(0);
    cam.debug(true);
    for arucos in cam.iter() {
        if !arucos.is_empty() {
            println!("{:?}", arucos);
        }
    }
}