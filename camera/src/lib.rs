use opencv::{
    aruco::{
        detect_markers, draw_detected_markers, get_predefined_dictionary, DetectorParameters,
        Dictionary, PREDEFINED_DICTIONARY_NAME::DICT_4X4_50,
    },
    core::{no_array, Mat, Mat_AUTO_STEP, Point2f, Ptr, Scalar, Vector, CV_8UC3},
    highgui, Result, prelude::*,
};

use nokhwa::{
    native_api_backend,
    pixel_format::RgbFormat,
    query,
    utils::{RequestedFormat, RequestedFormatType},
    Camera as NativeCam,
};

use std::net::SocketAddr;
use std::sync::Arc;

use rpc::camera_service_server::{CameraService, CameraServiceServer};
use rpc::{ArucoPosition, GetArucosPositionRequest, GetArucosPositionResponse};

use tokio::sync::RwLock;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod rpc {
    tonic::include_proto!("camera");
}

pub trait Camera {
    type Aruco;
    type CameraIter<'a>: Iterator<Item = Vec<Aruco>>
    where
        Self: 'a;
    fn new(index: u32) -> Self;
    fn debug(&mut self, debug: bool);
    fn iter(&mut self) -> Self::CameraIter<'_>;
}

#[derive(Debug)]
pub struct Aruco {
    pub id: i32,
    // [ left_top, right_top, right_bottom, left_bottom ]
    pub corners: [(f32, f32); 4],
}

pub struct CvCamera {
    debug: bool,
    cam: NativeCam,
}

impl Camera for CvCamera {
    type Aruco = Aruco;

    type CameraIter<'a> = CvCameraIter<'a>;

    fn new(index: u32) -> Self {
        let api = native_api_backend().unwrap();
        let cams = query(api).unwrap();
        if cams.is_empty() {
            panic!("no avilible camera");
        }
        let cam_info = cams
            .iter()
            .find(|c| c.index().as_index().unwrap() == index)
            .unwrap();
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let cam = NativeCam::with_backend(cam_info.index().clone(), format, api).unwrap();
        Self { debug: false, cam }
    }

    fn debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    fn iter(&mut self) -> Self::CameraIter<'_> {
        self.cam.open_stream().unwrap();
        let dictionary = get_predefined_dictionary(DICT_4X4_50).unwrap();
        let mut parameters = DetectorParameters::create().unwrap();
        // parameters.set_adaptive_thresh_constant(val)
        parameters.set_adaptive_thresh_win_size_max(40);
        parameters.set_adaptive_thresh_win_size_min(10);
        // parameters.set_adaptive_thresh_win_size_step(val)
        let len = (self.cam.resolution().width() as usize)
            * (self.cam.resolution().height() as usize)
            * 3;
        let buf = vec![0; len];
        // buf.resize(len, 0);
        CvCameraIter {
            cam: &mut self.cam,
            buf,
            corners: Vector::new(),
            ids: Vector::new(),
            debug: self.debug,
            dictionary,
            parameters,
        }
    }
}

pub struct CvCameraIter<'a> {
    cam: &'a mut NativeCam,
    buf: Vec<u8>,
    dictionary: Ptr<Dictionary>,
    parameters: Ptr<DetectorParameters>,
    corners: Vector<Vector<Point2f>>,
    ids: Vector<i32>,
    debug: bool,
}

// impl CvCameraIter {
//     fn new(index: u32) -> Self {
//         let api = native_api_backend().unwrap();
//         let cams = query(api).unwrap();
//         if cams.is_empty() {
//             panic!("no avilible camera");
//         }
//         let cam_info = cams
//             .iter()
//             .find(|c| c.index().as_index().unwrap() == index)
//             .unwrap();
//         let format =
//             RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
//         let cam = NativeCam::with_backend(cam_info.index().clone(), format, api).unwrap();
//         Self { debug: false, cam }
//     }

//     fn debug(&mut self, debug: bool) {
//         self.debug = debug;
//     }
// }

impl<'a> Iterator for CvCameraIter<'a> {
    type Item = Vec<Aruco>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cam
            .write_frame_to_buffer::<RgbFormat>(&mut self.buf)
            .unwrap();
        let mut img = unsafe {
            Mat::new_rows_cols_with_data(
                self.cam.resolution().height_y as i32,
                self.cam.resolution().width_x as i32,
                CV_8UC3,
                self.buf.as_ptr() as *mut _,
                Mat_AUTO_STEP,
            )
        }
        .unwrap();

        detect_markers(
            &img,
            &self.dictionary,
            &mut self.corners,
            &mut self.ids,
            &self.parameters,
            &mut no_array(),
            &no_array(),
            &no_array(),
        )
        .unwrap();

        if self.debug && !self.corners.is_empty() {
            // let mut resized_mat = Mat::zeros(540, 960, CV_8UC3).unwrap().a();
            // let size = resized_mat.size().unwrap();
            // resize(&img, &mut resized_mat, size, 0., 0., INTER_LINEAR).unwrap();
            draw_detected_markers(
                &mut img,
                &self.corners,
                &self.ids,
                Scalar::new(0.0, 255.0, 0.0, 0.0),
            )
            .unwrap();
        }

        if self.debug {
            highgui::imshow("webcam", &img).unwrap();
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
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("camera services start at {}", addr.to_string());
    let arc_arucos = Arc::new(RwLock::new(vec![]));

    let cam = CameraServ {
        arucos: arc_arucos.clone(),
    };

    let service = CameraServiceServer::new(cam);

    let job1 = async move {
        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .unwrap();
    };
    let job2 = async move {
        let mut cam = CvCamera::new(0);
        cam.debug(true);
        for arucos in cam.iter() {
            *arc_arucos.write().await = from_corners_to_position(arucos);
        }
    };
    futures::future::join(job1, job2).await;
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
