use opencv::{
    aruco::{detect_markers, draw_detected_markers, DetectorParameters, Dictionary, DICT_4X4_100},
    core::{no_array, Mat, Point2f, Ptr, Scalar, Vector},
    highgui,
    videoio::{
        VideoCapture, VideoCaptureTrait, CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH,
    },
    Result,
};

// use opencv::aruco::
use std::net::SocketAddr;
use std::sync::Arc;

use camera::camera_service_server::{CameraService, CameraServiceServer};
use camera::{ArucoPosition, GetArucosPositionRequest, GetArucosPositionResponse};

use tokio::sync::RwLock;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod camera {
    tonic::include_proto!("camera");
}

pub trait Camera {
    type Aruco;
    // type CameraIter<'a> : Iterator<Item = Vec<Aruco>>;
    // type CameraIter<'a>:Iterator<Item = Vec<Aruco>>;
    type CameraIter<'a>: Iterator<Item = Vec<Self::Aruco>>
    where
        Self: 'a;
    fn new(index: i32) -> Self;
    fn debug(&mut self, debug: bool);
    fn iter<'a>(&'a mut self) -> Self::CameraIter<'a>;
}

#[derive(Debug)]
pub struct Aruco {
    pub id: i32,
    // [ left_top, right_top, right_bottom, left_bottom ]
    pub corners: [(f32, f32); 4],
}

pub struct CvCamera {
    debug: bool,
    video_capture: VideoCapture,
}

impl Camera for CvCamera {
    type Aruco = Aruco;

    type CameraIter<'a> = CvCameraIter<'a>;

    fn new(index: i32) -> Self {
        let mut vc = VideoCapture::new_default(index).unwrap();
        vc.set(CAP_PROP_FRAME_WIDTH, 1920.0).unwrap();
        vc.set(CAP_PROP_FRAME_HEIGHT as i32, 1080.0).unwrap();
        vc.set(CAP_PROP_FPS as i32, 30.0).unwrap();

        Self {
            debug: false,
            video_capture: vc,
        }
    }

    fn debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    fn iter<'a>(&'a mut self) -> Self::CameraIter<'a> {
        let frame = Mat::default();
        let dict = Dictionary::get(DICT_4X4_100).unwrap();
        let corners = Vector::<Vector<Point2f>>::new();
        let ids = Vector::<i32>::new();
        let parameters = DetectorParameters::create().unwrap();
        CvCameraIter {
            vc: &mut self.video_capture,
            debug: self.debug,
            frame,
            corners,
            ids,
            parameters,
            dict,
        }
    }
}

pub struct CvCameraIter<'a> {
    vc: &'a mut VideoCapture,
    debug: bool,
    frame: Mat,
    dict: Ptr<Dictionary>,
    corners: Vector<Vector<Point2f>>,
    ids: Vector<i32>,
    parameters: Ptr<DetectorParameters>,
}

impl Iterator for CvCameraIter<'_> {
    type Item = Vec<Aruco>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.vc.read(&mut self.frame) {
            Ok(success) if !success => return None,
            Err(_) => return None,
            _ => {}
        }
        if detect_markers(
            &self.frame,
            &self.dict,
            &mut self.corners,
            &mut self.ids,
            &self.parameters,
            &mut no_array(),
        )
        .is_err()
        {
            return None;
        }

        if self.debug && !self.corners.is_empty() {
            draw_detected_markers(
                &mut self.frame,
                &self.corners,
                &self.ids,
                Scalar::new(0.0, 255.0, 0.0, 0.0),
            )
            .unwrap();
        }

        if self.debug {
            highgui::imshow("webcam", &self.frame).unwrap();
            let code = highgui::wait_key(1).unwrap();
            if let Some('q') = char::from_u32(code as u32) {
                return None;
            }
        }

        let arucos = self
            .corners
            .iter()
            .zip(self.ids.iter())
            .map(|(c, id)| Aruco {
                id,
                corners: [
                    (c.get(0).unwrap().x, c.get(0).unwrap().y),
                    (c.get(1).unwrap().x, c.get(1).unwrap().y),
                    (c.get(2).unwrap().x, c.get(2).unwrap().y),
                    (c.get(3).unwrap().x, c.get(3).unwrap().y),
                ],
            })
            .collect::<Vec<_>>();
        Some(arucos)
    }
}

#[test]
fn cam_test() {
    let mut cam = CvCamera::new(0);
    cam.debug(false);
    for arucos in cam.iter() {
        if !arucos.is_empty() {
            println!("{:?}", arucos);
        }
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

pub async fn run_camera_service(port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let arc_arucos = Arc::new(RwLock::new(vec![]));

    let cam = CameraServ {
        arucos: arc_arucos.clone(),
    };

    let job1 = async move {
        let service = CameraServiceServer::new(cam);
        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .unwrap();
    };

    let job2 = async move {
        let mut cam = CvCamera::new(0);
        cam.debug(false);
        for arucos in cam.iter() {
            *arc_arucos.write().await = from_corners_to_position(arucos);
        }
    };

    tokio::join!(job1, job2);
    // let jobs = futures::future::join(job1, job2);
    // jobs.await;
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
