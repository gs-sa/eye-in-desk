use camera_rpc::{
    camera_service_client::CameraServiceClient, ArucoPosition, GetArucosPositionRequest,
};
use projector_rpc::{
    projector_service_client::ProjectorServiceClient, DrawArucosRequest, DrawCirclesRequest,
    DrawRequest, DrawTextsRequest, GetDrawableSizeRequest,
};
use tonic::{codegen::StdError, transport::Channel, Status};
use web_rpc::{
    web_service_client::WebServiceClient, Object, ShowObjectsRequest, UpdateRobotRequest,
};

mod web_rpc {
    tonic::include_proto!("web");
}

mod camera_rpc {
    tonic::include_proto!("camera");
}

mod projector_rpc {
    tonic::include_proto!("projector");
}

use anyhow::Ok;
use camera::run_camera_service;
use projector_server::run_projector_back_end;
use sim_server::run_sim_back_end;
use tokio::process::Command;
// use wry::{
//     application::{
//         event::{Event, StartCause, WindowEvent},
//         event_loop::{ControlFlow, EventLoop},
//         window::WindowBuilder,
//     },
//     webview::WebViewBuilder,
// };

use projector_rpc::{Aruco, Circle, Text};
static PROJ_PORT: u16 = 50051;
static SIM_PORT: u16 = 50052;
static CAM_PORT: u16 = 50053;

pub struct EyeInDesk {
    cam_client: CameraServiceClient<Channel>,
    proj_client: ProjectorServiceClient<Channel>,
    sim_client: WebServiceClient<Channel>,
}

impl EyeInDesk {
    pub async fn default_connect() -> Self {
        use tonic::transport::Endpoint;
        let cam_addr = Endpoint::from_shared(format!("http://[::1]:{}", CAM_PORT)).unwrap();
        let proj_addr = Endpoint::from_shared(format!("http://[::1]:{}", PROJ_PORT)).unwrap();
        let sim_addr = Endpoint::from_shared(format!("http://[::1]:{}", SIM_PORT)).unwrap();
        EyeInDesk::connect(cam_addr, proj_addr, sim_addr).await
    }

    pub async fn connect<A>(cam_addr: A, proj_addr: A, sim_addr: A) -> Self
    where
        A: TryInto<tonic::transport::Endpoint>,
        A::Error: Into<StdError>,
    {
        let cam_client: CameraServiceClient<Channel> =
            CameraServiceClient::connect(cam_addr).await.unwrap();
        let proj_client = ProjectorServiceClient::connect(proj_addr).await.unwrap();
        let sim_client = WebServiceClient::connect(sim_addr).await.unwrap();

        EyeInDesk {
            cam_client,
            proj_client,
            sim_client,
        }
    }

    pub async fn get_arucos(&mut self) -> Result<Vec<ArucoPosition>, Status> {
        self.cam_client
            .get_arucos_position(GetArucosPositionRequest {})
            .await
            .map(|resp| resp.into_inner().arucos)
    }

    pub async fn get_drawable_size(&mut self) -> Result<(f64, f64), Status> {
        self.proj_client
            .get_drawable_size(GetDrawableSizeRequest {})
            .await
            .map(|resp| {
                let resp = resp.into_inner();
                (resp.width, resp.height)
            })
    }

    pub async fn place_arucos(&mut self, arucos: Vec<Aruco>) -> Result<(), Status> {
        self.proj_client
            .draw_arucos(DrawArucosRequest { markers: arucos })
            .await
            .map(|_| ())
    }

    pub async fn place_texts(&mut self, texts: Vec<Text>) -> Result<(), Status> {
        self.proj_client
            .draw_texts(DrawTextsRequest { texts })
            .await
            .map(|_| ())
    }

    pub async fn place_circles(&mut self, circles: Vec<Circle>) -> Result<(), Status> {
        self.proj_client
            .draw_circles(DrawCirclesRequest { circles })
            .await
            .map(|_| ())
    }

    pub async fn clear_and_draw(&mut self) -> Result<(), Status> {
        self.proj_client.draw(DrawRequest {}).await.map(|_| ())
    }

    pub async fn update_virtual_objects(&mut self, objects: Vec<Object>) -> Result<(), Status> {
        self.sim_client
            .show_objects(ShowObjectsRequest { objects })
            .await
            .map(|_| ())
    }

    pub async fn update_virtual_robot(&mut self, joints: &[f64; 7]) -> Result<(), Status> {
        self.sim_client
            .update_robot(UpdateRobotRequest {
                robot: joints.to_vec(),
            })
            .await
            .map(|_| ())
    }
}

#[tokio::test]
async fn eye_in_desk_get_aruco() {
    use std::result::Result::Ok;
    let mut eid = EyeInDesk::default_connect().await;
    while let Ok(arucos) = eid.get_arucos().await {
        if !arucos.is_empty() {
            println!("{:?}", arucos);
            break;
        }
    }
}

#[tokio::test]
async fn eye_in_desk_get_drawable_size() {
    let mut eid = EyeInDesk::default_connect().await;
    let size = eid.get_drawable_size().await.unwrap();
    println!("{:?}", size);
}

#[tokio::test]
async fn eye_in_desk_draw() {
    let mut eid = EyeInDesk::default_connect().await;
    eid.place_arucos(vec![Aruco {
        x: 100.,
        y: 100.,
        size: 200.,
    }])
    .await
    .unwrap();
    // eid.place_texts(vec![Text {
    //     x: 960.0,
    //     y: 200.0,
    //     text: "Hello World".to_string(),
    //     size: 5.0,
    // }])
    // .await
    // .unwrap();
    // eid.place_circles(vec![Circle {
    //     x: 0.0,
    //     y: 0.0,
    //     radius: 200.0,
    // }])
    // .await
    // .unwrap();
    eid.clear_and_draw().await.unwrap();
}

#[tokio::test]
async fn eye_in_desk_update_virtaul_objects() {
    let mut eid = EyeInDesk::default_connect().await;
    let objects = vec![Object {
        x: 100.0,
        y: 0.0,
        id: 0,
        z: 0.0,
        rot: 0.0,
    }];
    eid.update_virtual_objects(objects).await.unwrap();
}

#[tokio::test]
async fn eye_in_desk_update_virtaul_robot() {
    use std::f64::consts::PI;
    let mut eid = EyeInDesk::default_connect().await;
    let joints = [0., -PI / 4., 0., -3. * PI / 4., 0., PI / 2., PI / 4.];
    eid.update_virtual_robot(&joints).await.unwrap();
}

static PROJ_FILE_PORT: u16 = 8002;
static SIM_FILE_PORT: u16 = 8003;

pub async fn run() -> anyhow::Result<()> {
    // run front end servers
    println!("running front end servers");

    tokio::spawn(run_front_end_server(PROJ_FILE_PORT, "./projector"));
    tokio::spawn(run_front_end_server(SIM_FILE_PORT, "./sim"));
    // run grpc server
    println!("running grpc servers");
    tokio::spawn(run_projector_back_end(PROJ_PORT));
    tokio::spawn(run_sim_back_end(SIM_PORT));
    run_camera_service(CAM_PORT).await;
    Ok(())
}

async fn run_front_end_server(static_file_port: u16, dir: &str) -> anyhow::Result<()> {
    let mut front_end_server = Command::new("npm")
        .current_dir(dir)
        .args(["run", "dev", "--", "--port", &static_file_port.to_string()])
        .spawn()?;
    front_end_server.wait().await?;
    Ok(())
}

// fn run_windows() -> anyhow::Result<()> {
//     let event_loop = EventLoop::new();
//     // build_window(&event_loop, "Projector", PROJ_FILE_PORT)?;
//     // build_window(&event_loop, "Simulation", SIM_FILE_PORT)?;
//     let proj_window = WindowBuilder::new()
//         .with_title("Projector")
//         .build(&event_loop)
//         .unwrap();

//     let _webview = WebViewBuilder::new(proj_window)
//         .unwrap()
//         .with_url(&format!("http://localhost:{}/", PROJ_FILE_PORT))?
//         .build()?;

//     let sim_window = WindowBuilder::new()
//         .with_title("Simulation")
//         .build(&event_loop)
//         .unwrap();

//     let _webview = WebViewBuilder::new(sim_window)
//         .unwrap()
//         .with_url(&format!("http://localhost:{}/", SIM_FILE_PORT))?
//         .build()?;
//     event_loop.run(move |event, _, control_flow| {
//         *control_flow = ControlFlow::Wait;

//         match event {
//             Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
//             Event::WindowEvent {
//                 event: WindowEvent::CloseRequested,
//                 ..
//             } => *control_flow = ControlFlow::Exit,
//             _ => {}
//         }
//     });
// }
