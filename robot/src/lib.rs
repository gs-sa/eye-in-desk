pub async fn run_robot_service(port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let job1 = async move {
        let service = CameraServiceServer::new(cam);
        Server::builder()
            .add_service(service)
            .serve(addr)
            .await
            .unwrap();
    };
}