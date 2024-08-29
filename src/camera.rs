use macroquad::prelude::*;

pub fn setup_camera() -> Camera3D {
    Camera3D {
        position: vec3(2.0, -3.0, 2.0),
        up: vec3(0.0, 0.0, 1.0),
        target: vec3(0.0, 0.0, 0.0),
        ..Default::default()
    }
}

pub fn forward(cam: &Camera3D) -> Vec3 {
    (cam.target - cam.position).normalize()
}

pub fn right(cam: &Camera3D) -> Vec3 {
    forward(cam).cross(cam.up).normalize()
}

pub fn update_camera(camera: &mut Camera3D, last_mouse_position: &mut (f32, f32)) {
    let (mouse_x, mouse_y) = mouse_position();
    let (dx, dy) = (
        mouse_x - last_mouse_position.0,
        mouse_y - last_mouse_position.1,
    );
    *last_mouse_position = (mouse_x, mouse_y);

    if is_mouse_button_down(MouseButton::Left) {
        let sensitivity = 0.01;
        let right = right(camera);

        camera.position += right * -dx * sensitivity;
        camera.position.y += dy * sensitivity;

        if camera.up.z < 0.0 {
            camera.up.z = 1.0;
        }
    }

    camera.position += forward(camera) * mouse_wheel().1 * 0.1;
}
