use crate::math::{mat4x4::Mat4x4, traits::Vector, vec3::Vec3};

#[must_use]
pub fn perspercive_projection(
    fov: f32,
    screen_aspect: f32,
    screen_near: f32,
    screen_far: f32,
) -> Mat4x4 {
    let (sin_fov, cos_fov) = f32::sin_cos(0.5 * fov);
    let h = cos_fov / sin_fov;
    let w = h / screen_aspect;
    let r = screen_far / (screen_near - screen_far);
    Mat4x4::new(
        w,
        0.0,
        0.0,
        0.0,
        0.0,
        h,
        0.0,
        0.0,
        0.0,
        0.0,
        r,
        -1.0,
        0.0,
        0.0,
        r * screen_near,
        0.0,
    )
    // let y_scale = (fov / 2.0).cos() / (fov / 2.0).sin();
    // let x_scale = y_scale / screen_aspect;

    // let c = screen_far / (screen_near - screen_far);
    // let d = screen_near * c;

    // Mat4x4 {
    //     m00: x_scale,
    //     m11: y_scale,
    //     m22: c,
    //     m23: -1.0,
    //     m32: d,
    //     m33: 0.0,
    //     ..Default::default()
    // }
}

#[must_use]
pub fn scale_matrix(scale: &Vec3) -> Mat4x4 {
    Mat4x4 {
        m00: scale.x,
        m11: scale.y,
        m22: scale.z,
        ..Default::default()
    }
}

#[must_use]
pub fn translation_matrix(translation: &Vec3) -> Mat4x4 {
    Mat4x4 {
        m03: translation.x,
        m13: translation.y,
        m23: translation.z,
        ..Default::default()
    }
}

#[must_use]
pub fn rotation_matrix_euler(rotation: &Vec3) -> Mat4x4 {
    let sin_x = rotation.x.sin();
    let cos_x = rotation.x.cos();

    let sin_y = rotation.y.sin();
    let cos_y = rotation.y.cos();

    let sin_z = rotation.z.sin();
    let cos_z = rotation.z.cos();

    Mat4x4 {
        m00: cos_y * cos_z,
        m01: (sin_x * sin_y).mul_add(cos_z, -cos_x * sin_z),
        m02: (cos_x * sin_y).mul_add(cos_z, sin_x * sin_z),
        m10: cos_y * sin_z,
        m11: (sin_x * sin_y).mul_add(sin_z, cos_x * cos_z),
        m12: (cos_x * sin_y).mul_add(sin_z, -sin_x * cos_z),
        m20: -sin_y,
        m21: sin_x * cos_y,
        m22: cos_x * cos_y,
        ..Default::default()
    }
}

#[must_use]
pub fn transform_matrix_euler(translation: &Vec3, scale: &Vec3, rotation: &Vec3) -> Mat4x4 {
    translation_matrix(translation) * scale_matrix(scale) * rotation_matrix_euler(rotation)
}

#[must_use]
pub fn look_at_matrix(camera_position: Vec3, camera_up: Vec3, camera_forward: Vec3) -> Mat4x4 {
    let z_axis = (camera_forward - camera_position).normalized();
    let x_axis = (&camera_up).normalized();
    let y_axis = z_axis.cross(&x_axis).normalized();
    Mat4x4 {
        m00: y_axis.x,
        m10: y_axis.y,
        m20: y_axis.z,
        m01: x_axis.x,
        m11: x_axis.y,
        m21: x_axis.z,
        m12: -z_axis.y,
        m02: -z_axis.x,
        m22: -z_axis.z,
        m30: -(y_axis.dot_product(&camera_position)),
        m31: -(x_axis.dot_product(&camera_position)),
        m32: (z_axis.dot_product(&camera_position)),
        ..Default::default()
    }
}

#[test]
fn test_rotation_matrix() {
    let input = Vec3::new(0.0, 0.0, 0.0);
    let mat = rotation_matrix_euler(&input);
    let expected = Mat4x4::default();

    assert_eq!(mat, expected);

    let input = Vec3::new(0.0, 0.0, std::f32::consts::PI);
    let mat = rotation_matrix_euler(&input);
    let expected = Mat4x4 {
        m00: -1.0,
        m22: -1.0,
        ..Default::default()
    };

    assert_eq!(mat, expected);
}
