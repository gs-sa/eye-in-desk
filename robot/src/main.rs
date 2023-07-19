use franka::Frame;
use franka::FrankaResult;
use franka::Robot;
use franka::Torques;
use nalgebra::Isometry3;
use nalgebra::Matrix4;
use nalgebra::Rotation3;
use nalgebra::SMatrix;
use nalgebra::SVector;
use nalgebra::UnitQuaternion;
use nalgebra::Vector3;
use nalgebra::{Matrix3, Matrix6, Matrix6x1};

pub fn array_to_isometry(array: &[f64; 16]) -> Isometry3<f64> {
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

#[tokio::main]
async fn main() -> FrankaResult<()> {
    let translational_stiffness = 150.;
    let rotational_stiffness = 10.;
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
    let model = robot.load_model(true)?;

    // Set additional parameters always before the control loop, NEVER in the control loop!
    // Set collision behavior.
    robot.set_collision_behavior(
        [100.; 7], [100.; 7], [100.; 7], [100.; 7], [100.; 6], [100.; 6], [100.; 6], [100.; 6],
    )?;
    let initial_state = robot.read_once()?;
    let initial_transform = array_to_isometry(&initial_state.O_T_EE);
    let position_d = initial_transform.translation.vector;
    let orientation_d = initial_transform.rotation;

    println!(
        "WARNING: Collision thresholds are set to high values. \
             Make sure you have the user stop at hand!"
    );
    println!("After starting try to push the robot and see how it reacts.");
    println!("Press Enter to continue...");
    std::io::stdin().read_line(&mut String::new()).unwrap();
    let result = robot.control_torques(
        |state, _step| -> Torques {
            let coriolis = SVector::<_, 7>::from_column_slice(&model.coriolis_from_state(&state));
            let jacobian_array = model.zero_jacobian_from_state(&Frame::EndEffector, &state);
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

            let orientation: UnitQuaternion<f64> = UnitQuaternion::new_normalize(orientation);
            let error_quaternion: UnitQuaternion<f64> = orientation.inverse() * orientation_d;
            error.view_mut((3, 0), (3, 1)).copy_from(
                &-(transform.rotation.to_rotation_matrix()
                    * Vector3::new(error_quaternion.i, error_quaternion.j, error_quaternion.k)),
            );
            let tau_task = jacobian.transpose() * (-stiffness * error - damping * (jacobian * dq));
            let tau = tau_task + coriolis;
            let torques = tau.into();
            Torques::new(torques)
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
