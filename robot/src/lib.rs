pub mod rpc {
    tonic::include_proto!("robot");
}

use rpc::{
    robot_service_server::{RobotService, RobotServiceServer},
    GetRobotInfoRequest, GetRobotInfoResponse, SetRobotModeRequest, SetRobotModeResponse,
    SetRobotTargetRequest, SetRobotTargetResponse,
};
use tokio::sync::broadcast::{channel, error::TryRecvError, Sender};

use std::{net::SocketAddr, thread};

use franka::FrankaResult;
use franka::Robot;
use franka::Torques;
use franka::{Finishable, Frame};
use nalgebra::Isometry3;
use nalgebra::Matrix4;
use nalgebra::Rotation3;
use nalgebra::SMatrix;
use nalgebra::SVector;
use nalgebra::UnitQuaternion;
use nalgebra::Vector3;
use nalgebra::{Matrix3, Matrix6, Matrix6x1};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Clone)]
struct RobotInfo {
    joints: Vec<f64>,
    t: Vec<f64>,
}

struct RobotServ {
    info_channel: Sender<RobotInfo>,
    cmd_channel: Sender<Vec<f64>>,
    mode_channel: Sender<RobotMode>,
}

#[derive(Clone)]
enum RobotMode {
    Target,
    Drag,
}

#[tonic::async_trait]
impl RobotService for RobotServ {
    async fn get_robot_info(
        &self,
        _request: Request<GetRobotInfoRequest>,
    ) -> Result<Response<GetRobotInfoResponse>, Status> {
        let mut ic = self.info_channel.subscribe();
        if let Ok(info) = ic.recv().await {
            Ok(Response::new(GetRobotInfoResponse {
                joints: info.joints,
                t: info.t,
            }))
        } else {
            Err(Status::internal("Failed to recv joints"))
        }
    }

    async fn set_robot_target(
        &self,
        request: Request<SetRobotTargetRequest>,
    ) -> Result<Response<SetRobotTargetResponse>, Status> {
        if self.cmd_channel.send(request.into_inner().t).is_err() {
            Err(Status::internal("Failed to send target"))
        } else {
            Ok(Response::new(SetRobotTargetResponse {}))
        }
    }

    async fn set_robot_mode(
        &self,
        request: Request<SetRobotModeRequest>,
    ) -> Result<Response<SetRobotModeResponse>, Status> {
        if self
            .mode_channel
            .send(match request.into_inner().mode {
                0 => RobotMode::Target,
                1 => RobotMode::Drag,
                _ => RobotMode::Drag,
            })
            .is_err()
        {
            Err(Status::internal("Failed to send target"))
        } else {
            Ok(Response::new(SetRobotModeResponse {}))
        }
    }
}

pub async fn run_robot_service(port: u16) {
    let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], port));
    let (info_sender, _) = channel(10);
    let (cmd_sender, _) = channel(10);
    let (mode_sender, _) = channel(10);

    let service = RobotServiceServer::new(RobotServ {
        info_channel: info_sender.clone(),
        cmd_channel: cmd_sender.clone(),
        mode_channel: mode_sender.clone(),
    });

    thread::spawn(move || run_robot(info_sender, cmd_sender, mode_sender));

    Server::builder()
        .add_service(service)
        .serve(addr)
        .await
        .unwrap();
}

pub fn array_to_isometry(array: &[f64]) -> Isometry3<f64> {
    let rot = Rotation3::from_matrix(
        &Matrix4::from_column_slice(array)
            .remove_column(3)
            .remove_row(3),
    );
    Isometry3::from_parts(
        Vector3::new(array[12], array[13], array[14]).into(),
        rot.into(),
    )
}

fn run_robot(
    info_sender: Sender<RobotInfo>,
    cmd_sender: Sender<Vec<f64>>,
    mode_sender: Sender<RobotMode>,
) -> FrankaResult<()> {
    let mut cmd_recv = cmd_sender.subscribe();
    let mut mode_recv = mode_sender.subscribe();

    let translational_stiffness = 150.;
    let rotational_stiffness = 20.;
    let mut stiffness: Matrix6<f64> = Matrix6::zeros();
    let mut damping: Matrix6<f64> = Matrix6::zeros();
    stiffness
        .view_mut((0, 0), (3, 3))
        .copy_from(&(Matrix3::identity() * translational_stiffness));
    stiffness
        .view_mut((3, 3), (3, 3))
        .copy_from(&(Matrix3::identity() * rotational_stiffness));
    damping
        .view_mut((0, 0), (3, 3))
        .copy_from(&(Matrix3::identity() * translational_stiffness.sqrt()));
    damping
        .view_mut((3, 3), (3, 3))
        .copy_from(&(Matrix3::identity() * rotational_stiffness.sqrt()));

    let mut robot = Robot::new("192.168.1.101", Some(franka::RealtimeConfig::Ignore), None)?;
    robot.automatic_error_recovery().unwrap();
    let model = robot.load_model(true)?;

    // Set additional parameters always before the control loop, NEVER in the control loop!
    // Set collision behavior.
    robot.set_collision_behavior(
        [100.; 7], [100.; 7], [100.; 7], [100.; 7], [100.; 6], [100.; 6], [100.; 6], [100.; 6],
    )?;
    let initial_state = robot.read_once()?;
    let initial_transform = array_to_isometry(&initial_state.O_T_EE);
    let mut position_d = initial_transform.translation.vector;
    let mut orientation_d = initial_transform.rotation;

    let mut mode = RobotMode::Target;
    println!("robot start");
    // println!("After starting try to push the robot and see how it reacts.");
    // println!("Press Enter to continue...");
    // std::io::stdin().read_line(&mut String::new()).unwrap();
    let result = robot.control_torques(
        |state, _step| -> Torques {
            if info_sender.receiver_count() != 0 {
                let info = RobotInfo {
                    joints: state.q.to_vec(),
                    t: state.O_T_EE.to_vec(),
                };
                if info_sender.send(info).is_err() {
                    println!("joint sender fail");
                }
            }

            match cmd_recv.try_recv() {
                Ok(t) => {
                    let target_transform = array_to_isometry(&t);
                    position_d = target_transform.translation.vector;
                    orientation_d = target_transform.rotation;
                }
                Err(TryRecvError::Closed) => {
                    println!("no cmd senders");
                    let mut t = Torques::new([0.; 7]);
                    t.set_motion_finished(true);
                    return t;
                }
                _ => {}
            }

            match mode_recv.try_recv() {
                Ok(m) => {
                    mode = m;
                }
                _ => {}
            }

            match mode {
                RobotMode::Target => {
                    let coriolis =
                        SVector::<_, 7>::from_column_slice(&model.coriolis_from_state(&state));
                    let jacobian_array =
                        model.zero_jacobian_from_state(&Frame::EndEffector, &state);
                    let jacobian = SMatrix::<_, 6, 7>::from_column_slice(&jacobian_array);

                    let dq = SVector::<_, 7>::from_column_slice(&state.dq);
                    let transform = array_to_isometry(&state.O_T_EE);
                    let position = transform.translation.vector;
                    let mut orientation = transform.rotation.quaternion().clone();

                    let mut error = Matrix6x1::<f64>::zeros();
                    let pos_error = position - position_d;
                    error.view_mut((0, 0), (3, 1)).copy_from(&pos_error);

                    if orientation_d.coords.dot(&orientation.coords) < 0. {
                        orientation.coords = -orientation.coords;
                    }

                    let orientation: UnitQuaternion<f64> =
                        UnitQuaternion::new_normalize(orientation);
                    let error_quaternion: UnitQuaternion<f64> =
                        orientation.inverse() * orientation_d;
                    error.view_mut((3, 0), (3, 1)).copy_from(
                        &-(transform.rotation.to_rotation_matrix()
                            * Vector3::new(
                                error_quaternion.i,
                                error_quaternion.j,
                                error_quaternion.k,
                            )),
                    );
                    let tau_task =
                        jacobian.transpose() * (-stiffness * error - damping * (jacobian * dq));
                    let tau = tau_task + coriolis;
                    let mut torques: [f64; 7] = tau.into();
                    for i in 0..7 {
                        if torques[i] > 12. {
                            torques[i] = 12.
                        }
                        if torques[i] < -12. {
                            torques[i] = -12.
                        }
                    }
                    Torques::new(torques)
                }
                RobotMode::Drag => Torques::new([0.; 7]),
            }
        },
        None,
        None,
    );

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{}", e);
            Ok(())
        }
    }
}
