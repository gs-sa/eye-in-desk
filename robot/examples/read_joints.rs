use robot::rpc::{robot_service_client::RobotServiceClient, GetRobotInfoRequest};

#[tokio::main]
async fn main() {
    let mut c = RobotServiceClient::connect("http://127.0.0.1:50054").await.unwrap();
    while let Ok(resp) = c.get_robot_info(GetRobotInfoRequest{}).await {
        println!("{:?}", resp.into_inner().joints);
    }
}