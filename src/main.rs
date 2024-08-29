mod camera;
mod helper;
mod micromouse;
mod physics;

use micromouse::Micromouse;
use physics::{create_maze_floor, create_walls};

use macroquad::prelude::*;
use rapier3d::prelude::*;

#[macroquad::main("Micromouse Simulator")]
async fn main() {
    let mut physics = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let gravity = vector![0.0, 0.0, -9.81];
    let integration_parameters = IntegrationParameters::default();
    let mut physics_pipeline = PhysicsPipeline::new();
    let mut island_manager = IslandManager::new();
    let mut broad_phase = DefaultBroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joint_set = ImpulseJointSet::new();
    let mut multibody_joint_set = MultibodyJointSet::new();
    let mut ccd_solver = CCDSolver::new();
    let mut query_pipeline = QueryPipeline::new();
    let mut physics_hooks = ();
    let mut event_handler = ();

    let floor_friction = 0.9;
    create_maze_floor(&mut physics, &mut colliders, floor_friction);
    create_walls(&mut physics, &mut colliders);

    let wheel_friction = 0.8;
    let mut micromouse = Micromouse::new(&mut physics, &mut colliders, wheel_friction);

    let mut camera = camera::setup_camera();

    let mut last_mouse_position = mouse_position();

    loop {
        micromouse.handle_input();
        micromouse.apply_motor_forces(&mut physics);

        physics_pipeline.step(
            &gravity,
            &integration_parameters,
            &mut island_manager,
            &mut broad_phase,
            &mut narrow_phase,
            &mut physics,
            &mut colliders,
            &mut impulse_joint_set,
            &mut multibody_joint_set,
            &mut ccd_solver,
            Some(&mut query_pipeline),
            &mut physics_hooks,
            &mut event_handler,
        );

        camera::update_camera(&mut camera, &mut last_mouse_position);

        clear_background(WHITE);

        set_camera(&camera);

        micromouse.draw(&physics, &query_pipeline, &colliders);

        set_default_camera();

        draw_text(
            "Use arrow keys to move, mouse to control camera",
            10.0,
            20.0,
            20.0,
            BLACK,
        );

        next_frame().await;
    }
}
