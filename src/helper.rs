use rapier3d::na::Vector3;

#[inline(always)]
pub const fn vector3(x: f32, y: f32, z: f32) -> Vector3<f32> {
    Vector3::<f32>::new(x, y, z)
}
