use camera::rpc::camera_service_client::CameraServiceClient;

#[tokio::main]
async fn main() {
    let c = CameraServiceClient::connect(format!("http://127.0.0.1:50053")).await.unwrap();
}