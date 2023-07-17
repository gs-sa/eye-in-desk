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
use std::{fmt::Display, net::SocketAddr, sync::Arc};
use tokio::sync::{broadcast::Sender, mpsc};

#[derive(Debug, Clone, Serialize)]
pub enum DrawObject {
    Aruco {
        x: f32,
        y: f32,
        size: f32,
    },

    Text {
        text: String,
        x: f32,
        y: f32,
        size: f32,
    },

    Circle {
        x: f32,
        y: f32,
        radius: f32,
    },
    // Rectangle {
    //     x: f32,
    //     y: f32,
    //     width: f32,
    //     height: f32,
    // },
}

#[derive(Debug, Clone)]
pub enum Command {
    GetDrawableSize,
}

impl Command {
    fn to_i32(&self) -> i32 {
        match self {
            Command::GetDrawableSize => 0,
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_i32())
    }
}

#[derive(Debug, Clone)]
pub enum CommandResult {
    DrawableSizeResult([f64; 2]),
}

// 8001
pub async fn run_ws_server(
    draw_objects_receicer: Sender<Vec<DrawObject>>,
    command_sender: Sender<Command>,
    command_result_sender: mpsc::Sender<CommandResult>,
    port: u16,
) {
    let router = Router::new()
        .route("/DrawWs", get(handle_draw_ws_upgrade))
        .with_state(Arc::new((
            draw_objects_receicer,
            command_sender,
            command_result_sender,
        )));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

type DrawState = Arc<(
    Sender<Vec<DrawObject>>,
    Sender<Command>,
    mpsc::Sender<CommandResult>,
)>;

async fn handle_draw_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<DrawState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_draw_ws(socket, state))
}

async fn handle_draw_ws(
    ws: WebSocket,
    txs: DrawState,
) {
    let (mut sink, mut stream) = ws.split();
    let mut draw_rx = txs.0.subscribe();
    let mut command_rx = txs.1.subscribe();

    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(objects) = (draw_rx.recv()) => {
                    let json = serde_json::to_string(&objects).unwrap();
                    if sink.send(Message::Text(json)).await.is_err() {
                        return;
                    }
                }
                Ok(command) = (command_rx.recv()) => {
                    if sink.send(Message::Text(command.to_string())).await.is_err() {
                        return;
                    }
                }
                else => {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(res) = stream.next().await {
            match res {
                Ok(Message::Binary(bin)) => {
                    if bin.len() != 8 * 2 {
                        continue;
                    }
                    let mut buf = [0; 8 * 2];
                    buf.copy_from_slice(&bin);
                    let x = f64::from_le_bytes(buf[0..8].try_into().unwrap());
                    let y = f64::from_le_bytes(buf[8..16].try_into().unwrap());
                    if (txs.2.send(CommandResult::DrawableSizeResult([x, y])).await).is_err() {
                        continue;
                    }
                }
                Err(_) => continue,
                _ => {}
            }
        }
        // while let Some(Ok(Message::Close(_))) = stream.next().await {
        //     break;
        // }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

#[test]
fn draw_json() {
    let a = DrawObject::Aruco {
        x: 1.0,
        y: 2.0,
        size: 3.0,
    };
    let b = DrawObject::Text {
        text: "Hello".to_string(),
        x: 10.,
        y: 10.,
        size: 1.,
    };
    let c = DrawObject::Circle {
        x: 1.0,
        y: 2.0,
        radius: 3.0,
    };
    let vec = vec![a, b, c];
    let json = serde_json::to_string(&vec).unwrap();
    println!("{}", json);
}
