use robot::run_robot_service;

#[tokio::main]
async fn main() {
    run_robot_service(50054).await;
}