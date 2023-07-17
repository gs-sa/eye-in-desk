use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router, Server,
};
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast::Sender;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Clone)]
pub struct RobotState {
    pub joints: [f64; 7],
}

#[derive(Debug, Clone, Serialize)]
pub struct Object {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rot: f32,
    pub id: i32,
}

pub async fn run_server(
    robot_state_receicer: Sender<RobotState>,
    objects_receicer: Sender<Vec<Object>>,
) {
    let router = Router::new()
        .nest_service(
            "/",
            ServeFile::new(
                "/Users/shengge/projetcs/grad_projects/eye_in_desk/web_server/index.html",
            ),
        )
        .nest_service(
            "/assets",
            ServeDir::new("/Users/shengge/projetcs/grad_projects/eye_in_desk/web_server/assets"),
        )
        .nest_service(
            "/Panda",
            ServeDir::new("/Users/shengge/projetcs/grad_projects/eye_in_desk/web_server/Panda"),
        )
        .route("/jointsWs", get(handle_joints_ws_upgrade))
        .with_state(Arc::new(robot_state_receicer))
        .route("/primitiveWs", get(handle_primitive_ws_upgrade))
        .with_state(Arc::new(objects_receicer));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

async fn handle_joints_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Sender<RobotState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_joints_ws(socket, state))
}

async fn handle_primitive_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Sender<Vec<Object>>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_primitive_ws(socket, state))
}

async fn handle_joints_ws(ws: WebSocket, state: Arc<Sender<RobotState>>) {
    let (mut sink, mut stream) = ws.split();
    let mut rx = state.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(robot_state) = rx.recv().await {
            let joints =
                unsafe { std::mem::transmute::<[f64; 7], [u8; 56]>(robot_state.joints) }.to_vec();
            if sink.send(Message::Binary(joints)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

async fn handle_primitive_ws(ws: WebSocket, state: Arc<Sender<Vec<Object>>>) {
    let (mut sink, mut stream) = ws.split();
    let mut rx = state.subscribe();
    let mut send_task = tokio::spawn(async move {
        while let Ok(objects) = rx.recv().await {
            let json = serde_json::to_string(&objects).unwrap();
            if sink.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
