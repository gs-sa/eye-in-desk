pub mod rpc {
    tonic::include_proto!("projector");
}

pub mod backend;

use backend::{Command, CommandResult, DrawObject};
use rpc::projector_service_server::{ProjectorService, ProjectorServiceServer};
use rpc::{
    DrawCirclesRequest, DrawRequest, DrawTextsRequest, GetDrawableSizeRequest, SizeResponse,
};
use std::net::SocketAddr;
use tokio::sync::broadcast::{channel, Sender};
use tokio::sync::{mpsc, Mutex};
use tonic::{transport::Server, Request, Response, Status};
use {
    backend::run_ws_server,
    rpc::{DrawArucosRequest, DrawResponse},
};

struct DrawService {
    objects: Mutex<Vec<DrawObject>>,
    draw_sender: Sender<Vec<DrawObject>>,
    command_sender: Sender<Command>,
    command_receiver: Mutex<mpsc::Receiver<CommandResult>>,
}

#[tonic::async_trait]
impl ProjectorService for DrawService {
    async fn get_drawable_size(
        &self,
        _request: Request<GetDrawableSizeRequest>,
    ) -> Result<Response<SizeResponse>, Status> {
        if self.command_sender.send(Command::GetDrawableSize).is_err() {
            return Err(Status::internal("Failed to send draw objects"));
        }
        let result = self.command_receiver.lock().await.recv().await.unwrap();
        let response = match result {
            CommandResult::DrawableSizeResult([width, height]) => {
                Response::new(SizeResponse { width, height })
            }
        };
        Ok(response)
    }
    async fn draw(&self, _request: Request<DrawRequest>) -> Result<Response<DrawResponse>, Status> {
        let mut objs = self.objects.lock().await;
        match self.draw_sender.send(objs.to_vec()) {
            Ok(_) => {}
            Err(_) => {
                return Err(Status::internal("Failed to send draw objects"));
            }
        }
        objs.clear();
        let response = Response::new(DrawResponse { success: true });
        Ok(response)
    }

    async fn draw_arucos(
        &self,
        request: Request<DrawArucosRequest>,
    ) -> Result<Response<DrawResponse>, Status> {
        let mut objs = self.objects.lock().await;
        request.into_inner().markers.into_iter().for_each(|marker| {
            objs.push(DrawObject::Aruco {
                x: marker.x,
                y: marker.y,
                size: marker.size,
            });
        });
        let response = Response::new(DrawResponse { success: true });
        Ok(response)
    }

    async fn draw_texts(
        &self,
        request: Request<DrawTextsRequest>,
    ) -> Result<Response<DrawResponse>, Status> {
        let mut objs = self.objects.lock().await;
        request.into_inner().texts.into_iter().for_each(|text| {
            objs.push(DrawObject::Text {
                text: text.text,
                x: text.x,
                y: text.y,
                size: text.size,
            });
        });
        let response = Response::new(DrawResponse { success: true });
        Ok(response)
    }

    async fn draw_circles(
        &self,
        request: Request<DrawCirclesRequest>,
    ) -> Result<Response<DrawResponse>, Status> {
        let mut objs = self.objects.lock().await;
        request.into_inner().circles.into_iter().for_each(|circle| {
            objs.push(DrawObject::Circle {
                x: circle.x,
                y: circle.y,
                radius: circle.radius,
            });
        });
        let response = Response::new(DrawResponse { success: true });
        Ok(response)
    }
}

// grpc: 50053
// ws: 8001
pub async fn run_projector_back_end(grpc_port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], grpc_port));
    println!("projector services start at {}", addr);
    let (draw_sender, _) = channel(10);
    let (command_sender, _) = channel(10);
    let (command_result_sender, command_receiver) = mpsc::channel(10);
    let service = ProjectorServiceServer::new(DrawService {
        objects: Mutex::new(Vec::new()),
        draw_sender: draw_sender.clone(),
        command_sender: command_sender.clone(),
        command_receiver: Mutex::new(command_receiver),
    });
    let future1 = run_ws_server(draw_sender, command_sender, command_result_sender, 8001);
    let future2 = async move {
        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .unwrap()
    };
    
    futures::future::join(future1, future2).await;
}
