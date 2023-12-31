pub mod rpc {
    tonic::include_proto!("web");
}

pub mod backend;

use std::net::SocketAddr;

use rpc::CameraControlRequest;
use tokio::sync::broadcast::{channel, Sender};
use tonic::{transport::Server, Request, Response, Status};
use {
    backend::{run_server, Control, Object, RobotState},
    rpc::{
        web_service_server::{WebService, WebServiceServer},
        ShowObjectsRequest, UpdateRobotRequest, WebResponse,
    },
};

struct WebRpcServer {
    state_sender: Sender<RobotState>,
    objects_sender: Sender<Vec<Object>>,
    control_sender: Sender<Control>,
}

#[tonic::async_trait]
impl WebService for WebRpcServer {
    async fn show_objects(
        &self,
        request: Request<ShowObjectsRequest>,
    ) -> Result<Response<WebResponse>, Status> {
        let objs = request
            .into_inner()
            .objects
            .into_iter()
            .map(|obj| Object {
                x: obj.x,
                y: obj.y,
                z: obj.z,
                rot: obj.rot,
                id: obj.id,
            })
            .collect::<Vec<_>>();
        self.objects_sender
            .send(objs)
            .map_err(|_| Status::internal("Failed to send objects"))?;
        Ok(Response::new(WebResponse { success: true }))
    }

    async fn update_robot(
        &self,
        request: Request<UpdateRobotRequest>,
    ) -> Result<Response<WebResponse>, Status> {
        let mut joints = [0.; 7];
        for (i, obj) in request.into_inner().robot.into_iter().enumerate() {
            joints[i] = obj
        }
        self.state_sender
            .send(RobotState { joints })
            .map_err(|_| Status::internal("Failed to send robot state"))?;
        Ok(Response::new(WebResponse { success: true }))
    }

    async fn camera_control(
        &self,
        request: Request<CameraControlRequest>,
    ) -> Result<Response<WebResponse>, Status> {
        let req = request.into_inner();
        self.control_sender
            .send(Control {
                rotate_left: req.rotate_left,
                rotate_up: req.rotate_up,
            })
            .map_err(|_| Status::internal("Failed to send robot state"))?;
        Ok(Response::new(WebResponse { success: true }))
    }
}

pub async fn run_sim_back_end(grpc_port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], grpc_port));
    println!("simulation services start at {}", addr);
    let (state_sender, _state_rx) = channel(10);
    let (objects_sender, _objects_rx) = channel(10);
    let (control_sender, _objects_rx) = channel(10);

    let service = WebServiceServer::new(WebRpcServer {
        state_sender: state_sender.clone(),
        objects_sender: objects_sender.clone(),
        control_sender: control_sender.clone(),
    });
    let future1 = run_server(state_sender, objects_sender, control_sender);
    let future2 = async move {
        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .unwrap();
    };
    futures::future::join(future1, future2).await;
}
