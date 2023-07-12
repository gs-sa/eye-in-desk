use opencv::{
    aruco::{
        detect_markers, draw_detected_markers, estimate_pose_single_markers, DetectorParameters,
        Dictionary, DICT_4X4_100,
    },
    core::{
        add_weighted, no_array, subtract, Mat, Matx33d, Point2f, Scalar, Size, Vec3d, Vector,
        BORDER_DEFAULT,
    },
    highgui,
    imgproc::{cvt_color, gaussian_blur, COLOR_BGR2GRAY},
    prelude::DetectorParametersTrait,
    videoio::{
        VideoCapture, VideoCaptureTrait, CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH,
    },
};
use std::{fmt::Debug, sync::mpsc::Sender};
use tracing::{info, span, Level};

#[derive(Debug)]
pub struct CornerData {
    pub id: i32,
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub c: (f32, f32),
    pub d: (f32, f32),
}

impl CornerData {
    fn from_vec(v: Vector<Point2f>, id:i32) -> Self {
        assert_eq!(v.len(), 4);
        CornerData {
            id,
            a: (v.get(0).unwrap().x, v.get(0).unwrap().y),
            b: (v.get(1).unwrap().x, v.get(1).unwrap().y),
            c: (v.get(2).unwrap().x, v.get(2).unwrap().y),
            d: (v.get(3).unwrap().x, v.get(3).unwrap().y),
        }
    }
}

pub fn camera_run(sender: Sender<Vec<CornerData>>) {
    info!("opencv run");
    let mut vc = VideoCapture::new_default(0).unwrap();
    vc.set(CAP_PROP_FRAME_WIDTH, 1920.0).unwrap();
    vc.set(CAP_PROP_FRAME_HEIGHT as i32, 1080.0).unwrap();
    vc.set(CAP_PROP_FPS as i32, 30.0).unwrap();

    let mut frame = Mat::default();
    let dict = Dictionary::get(DICT_4X4_100).unwrap();
    let mut corners = Vector::<Vector<Point2f>>::new();
    let mut ids = Vector::<i32>::new();
    let parameters = DetectorParameters::create().unwrap();
    // parameters.set_adaptive_thresh_win_size_max(31);
    // parameters.set_adaptive_thresh_win_size_min(10);
    // parameters.set_adaptive_thresh_win_size_step(3);
    let mut rejected_img_points = no_array();
    loop {
        let Ok(is_read_good) = vc.read(&mut frame) else {
            continue;
        };
        if is_read_good {
            detect_markers(
                &frame,
                &dict,
                &mut corners,
                &mut ids,
                &parameters,
                &mut rejected_img_points,
            )
            .unwrap();
            if corners.len() > 0 {
                draw_detected_markers(
                    &mut frame,
                    &mut corners,
                    &mut ids,
                    Scalar::new(0.0, 255.0, 0.0, 0.0),
                )
                .unwrap();
                // estimate_pose_single_markers(
                //     &corners,
                //     0.03,
                //     &camera_matrix,
                //     &dist_coeffs,
                //     &mut rvecs,
                //     &mut tvecs,
                // )
                // .unwrap();
                // println!("{:?}", rvecs);
                // println!("{:?}", tvecs);
                let mut data = vec![];
                for (corner, id) in corners.iter().zip(ids.iter()) {
                    data.push(CornerData::from_vec(corner, id));
                }
                match sender.send(data) {
                    Err(err) => {
                        eprintln!("{}", err);
                    }
                    _=>{}
                }
            }

            highgui::imshow("webcam", &frame).unwrap();
            let code = highgui::wait_key(1).unwrap();
            if let Some('q') = char::from_u32(code as u32) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
