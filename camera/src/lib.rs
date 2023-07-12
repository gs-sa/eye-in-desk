// use opencv::{
//     core::{no_array, Mat, Point2f, Scalar, Vector},
//     highgui,
//     objdetect::{
//         draw_detected_markers, get_predefined_dictionary_i32, ArucoDetector, DetectorParameters,
//         RefineParameters, DICT_4X4_100,
//     },
//     prelude::{ArucoDetectorTraitConst, DetectorParametersTrait},
//     videoio::{
//         VideoCapture, VideoCaptureTrait, CAP_ANY, CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT,
//         CAP_PROP_FRAME_WIDTH,
//     },
//     Result,
// };

use std::net::SocketAddr;
use std::sync::Arc;

use camera::camera_service_server::{CameraService, CameraServiceServer};
use camera::{GetArucosPositionRequest, GetArucosPositionResponse, ArucoPosition};

use tokio::sync::RwLock;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod camera {
    tonic::include_proto!("camera");
}

pub trait Camera {
    type Aruco;
    type CameraIter: Iterator<Item = Vec<Aruco>>;
    fn new(index: i32) -> Self;
    fn debug(&mut self, debug: bool);
    fn iter(&mut self) -> Self::CameraIter;
}

// pub struct CvCamera {
//     debug: bool,
//     video_capture: VideoCapture,
//     frame: Mat,
//     aruco_detector: ArucoDetector,
//     corners: Vector<Vector<Point2f>>,
//     ids: Vector<i32>,
// }

#[derive(Debug)]
pub struct Aruco {
    pub id: i32,
    // [ left_top, right_top, right_bottom, left_bottom ]
    pub corners: [(f32, f32); 4],
}

// impl CvCamera {
//     pub fn new(index: i32) -> Result<CvCamera> {
//         let mut video_capture = VideoCapture::new(index, CAP_ANY)?;
//         video_capture.set(CAP_PROP_FRAME_WIDTH, 1920.0).unwrap();
//         video_capture.set(CAP_PROP_FRAME_HEIGHT, 1080.0).unwrap();
//         video_capture.set(CAP_PROP_FPS, 30.0).unwrap();
//         let mut dp = DetectorParameters::default()?;
//         dp.set_adaptive_thresh_win_size_max(31);
//         dp.set_adaptive_thresh_win_size_step(7);
//         Ok(CvCamera {
//             debug: false,
//             video_capture,
//             frame: Mat::default(),
//             aruco_detector: ArucoDetector::new(
//                 &get_predefined_dictionary_i32(DICT_4X4_100)?,
//                 &dp,
//                 RefineParameters::new(10., 3., true)?,
//             )?,
//             corners: Vector::new(),
//             ids: Vector::new(),
//         })
//     }

//     pub fn debug(&mut self, debug: bool) {
//         self.debug = debug;
//     }
// }

// impl Iterator for CvCamera {
//     type Item = Vec<Aruco>;

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.video_capture.read(&mut self.frame) {
//             Ok(success) if !success => return None,
//             Err(_) => return None,
//             _ => {}
//         }
//         if self
//             .aruco_detector
//             .detect_markers(
//                 &self.frame,
//                 &mut self.corners,
//                 &mut self.ids,
//                 &mut no_array(),
//             )
//             .is_err()
//         {
//             return None;
//         }

//         if self.debug && !self.corners.is_empty() {
//             draw_detected_markers(
//                 &mut self.frame,
//                 &self.corners,
//                 &self.ids,
//                 Scalar::new(0.0, 255.0, 0.0, 0.0),
//             )
//             .unwrap();
//         }

//         if self.debug {
//             highgui::imshow("webcam", &self.frame).unwrap();
//             let code = highgui::wait_key(1).unwrap();
//             if let Some('q') = char::from_u32(code as u32) {
//                 return None;
//             }
//         }

//         let arucos = self
//             .corners
//             .iter()
//             .zip(self.ids.iter())
//             .map(|(c, id)| Aruco {
//                 id,
//                 corners: [
//                     (c.get(0).unwrap().x, c.get(0).unwrap().y),
//                     (c.get(1).unwrap().x, c.get(1).unwrap().y),
//                     (c.get(2).unwrap().x, c.get(2).unwrap().y),
//                     (c.get(3).unwrap().x, c.get(3).unwrap().y),
//                 ],
//             })
//             .collect::<Vec<_>>();
//         Some(arucos)
//     }
// }

// #[test]
// fn cv_camera_new() {
//     // select camera
//     let _cv_cam = CvCamera::new(0).unwrap();
// }

// #[test]
// fn cv_camera_iter() {
//     let mut cv_cam = CvCamera::new(0).unwrap();
//     cv_cam.debug(true);
//     for arucos in cv_cam {
//         println!("{:?}", arucos);
//     }
// }

struct CvCamera {}

impl Camera for CvCamera {
    type Aruco = Aruco;

    type CameraIter = CvCameraIter;

    fn new(index: i32) -> Self {
        todo!()
    }

    fn debug(&mut self, debug: bool) {
        todo!()
    }

    fn iter(&mut self) -> Self::CameraIter {
        todo!()
    }
}

struct CvCameraIter {}

impl Iterator for CvCameraIter {
    type Item = Vec<Aruco>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

struct CameraServ {
    arucos: Arc<RwLock<Vec<ArucoPosition>>>,
}

#[tonic::async_trait]
impl CameraService for CameraServ {
    async fn get_arucos_position(
        &self,
        _request: Request<GetArucosPositionRequest>,
    ) -> Result<Response<GetArucosPositionResponse>, Status> {
        Ok(Response::new(GetArucosPositionResponse {
            arucos: self.arucos.read().await.clone(),
        }))
    }
}

pub fn run_camera_service(port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let arc_arucos = Arc::new(RwLock::new(vec![]));

    let cam = CameraServ {
        arucos: arc_arucos.clone(),
    };

    tokio::spawn(async move {
        let service = CameraServiceServer::new(cam);
        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .unwrap();
    });

    tokio::spawn(async move {
        let mut cam = CvCamera::new(0);
        cam.debug(false);
        for arucos in cam.iter() {
            *arc_arucos.write().await = from_corners_to_position(arucos);
        }
    });
}

fn from_corners_to_position(arucos: Vec<Aruco>) -> Vec<ArucoPosition> {
    arucos
        .into_iter()
        .map(|aruco| {
            let xy = aruco
                .corners
                .iter()
                .fold((0., 0.), |init, c| (init.0 + c.0, init.1 + c.1));

            let x = aruco.corners[1].0 - aruco.corners[0].0;
            let y = aruco.corners[1].1 - aruco.corners[0].1;
            let rot = y.atan2(x);
            let mut sizes = vec![];
            for s in 0..4 {
                let x = aruco.corners[s].0 - aruco.corners[(s + 1) % 4].0;
                let y = aruco.corners[s].1 - aruco.corners[(s + 1) % 4].1;
                let s = (x * x + y * y).sqrt();
                sizes.push(s);
            }
            let size = sizes.iter().sum::<f32>() / 4.;
            ArucoPosition {
                id: aruco.id,
                x: xy.0 / 4.,
                y: xy.1 / 4.,
                rot,
                size,
            }
        })
        .collect::<Vec<_>>()
}
