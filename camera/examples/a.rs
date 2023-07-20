use camera::run_camera_service;

#[tokio::main]
async fn main() {
    run_camera_service(50053).await
}