use bevy::prelude::*;


pub fn setup_lighting(mut commands: Commands) {
        commands.spawn((
            PointLight {
                shadows_enabled: false,
                intensity: 10_000_000.,
                range: 1_000.0,
                shadow_depth_bias: 0.2,
                ..default()
            },
            Transform::from_xyz(8.0, 16.0, 20.0),
        ));
}


pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 20.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        bevy_panorbit_camera::PanOrbitCamera::default(),
    ));

    commands.spawn((
        Text::new("Looking at Neural Circuit"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/* I might need a way in the future to move up-down and side-side not jsut rotate around axes

pub fn alter_camera_posn_up_dn(mut query: Query<&mut Transform, With<Camera>>, keyboard: Res<ButtonInput<KeyCode>>) {
    let mut camera_transform = query.single_mut();
    if keyboard.pressed(KeyCode::ControlLeft) {
        if  keyboard.just_pressed(KeyCode::ArrowUp) {
            let rotation = camera_transform.rotation;
            let new_rotation = Quat::from_rotation_x(0.5) * rotation; // Rotate by 0.5 radians (~30 degrees) around the x-axis
            //camera_transform.with_rotation(new_rotation);
            camera_transform.rotate_around(Vec3::ZERO, new_rotation); 
                                            
        } 
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            let rotation = camera_transform.rotation;
            let new_rotation = Quat::from_rotation_x(-0.5) * rotation; // Rotate by 0.5 radians (~30 degrees) around the x-axis
            //camera_transform.with_rotation(new_rotation);
            camera_transform.rotate_around(Vec3::ZERO, new_rotation);
}
    }
}

pub fn zoom_perspective(
    mut query_camera: Query<&mut Projection, With<Camera>>,keyboard: Res<ButtonInput<KeyCode>>) {
    // assume perspective. do nothing if orthographic.
    let Projection::Perspective(persp) = query_camera.single_mut().into_inner() else {
        return;
    };
    if keyboard.pressed(KeyCode::ShiftLeft) {
        if  keyboard.just_pressed(KeyCode::ArrowUp) {
            // zoom in
            persp.fov /= 1.05;
        } 

        if keyboard.just_pressed(KeyCode::ArrowDown) {
            // zoom out
            persp.fov *= 1.05;
        }
    }
}*/

