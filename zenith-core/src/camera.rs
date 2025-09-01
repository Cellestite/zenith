use glam::{EulerRot, Mat4, Quat, Vec3};
use log::{warn};
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::window::{CursorGrabMode, Window};
use crate::input::InputActionMapper;
use crate::math::{Degree, Radians};
use crate::system_event::SystemEventCollector;

// Zenith world space coordinate system (right-hand side, z up)
//
//                z
//                ^    y
//                |   /
//                |  /
//                | /
//                ----------> x
//

pub const NEAR_PLANE: f32 = 0.1;
pub const WORLD_SPACE_UP: Vec3 = Vec3::new(0., 0., 1.);
pub const WORLD_SPACE_FORWARD: Vec3 = Vec3::new(0., 1., 0.);
pub const WORLD_SPACE_RIGHT: Vec3 = Vec3::new(1., 0., 0.);

#[derive(Debug)]
pub struct Camera {
    position: Vec3,
    rotation: Quat,
    pitch: Radians,
    yaw: Radians,

    // cached values
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    view: Mat4,
    proj: Mat4,
}

impl Default for Camera {
    fn default() -> Self {
        let mut cam = Self {
            position: Default::default(),
            rotation: Quat::IDENTITY,
            pitch: Default::default(),
            yaw: Default::default(),

            forward: WORLD_SPACE_FORWARD,
            right: WORLD_SPACE_RIGHT,
            up: WORLD_SPACE_UP,

            view: Default::default(),
            proj: Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_6, 1.77777, NEAR_PLANE),
        };
        cam.update_view();
        cam
    }
}

impl Camera {
    pub fn new(fov_y: Radians, aspect_ratio: f32, z_near: f32) -> Self {
        let mut cam = Self {
            proj: Mat4::perspective_infinite_reverse_rh(fov_y.into(), aspect_ratio, z_near.max(0.0001)),
            ..Default::default()
        };
        cam.update_view();
        cam
    }

    #[inline]
    pub fn location(&self) -> Vec3 {
        self.position
    }

    #[inline]
    pub fn view(&self) -> Mat4 { self.view }

    #[inline]
    pub fn projection(&self) -> Mat4 {
        self.proj
    }

    #[inline]
    pub fn view_projection(&self) -> Mat4 {
        self.proj * self.view
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        self.forward
    }

    #[inline]
    pub fn right(&self) -> Vec3 {
        self.right
    }

    #[inline]
    pub fn up(&self) -> Vec3 {
        self.up
    }

    fn translate(&mut self, delta_position: Vec3) {
        let r = self.right();
        let f = self.forward();
        let u = self.up();

        self.position += r * delta_position.x + f * delta_position.y + u * delta_position.z;
    }

    fn rotate(&mut self, delta_yaw: Radians, delta_pitch: Radians, max_pitch: Radians) {
        self.yaw += delta_yaw;
        self.pitch += delta_pitch;
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);
        // eliminate roll and avoid gimbal lock
        self.rotation = Quat::from_euler(EulerRot::ZXY, self.yaw.into(), self.pitch.into(), 0.);
    }

    fn update_view(&mut self) {
        let forward = self.forward();
        self.view = Mat4::look_to_rh(self.position, forward, WORLD_SPACE_UP);
    }

    fn update_local_basis(&mut self) {
        self.forward = self.rotation * WORLD_SPACE_FORWARD;
        self.right = self.rotation * WORLD_SPACE_RIGHT;
        self.up = self.rotation * WORLD_SPACE_UP;
    }
}

pub struct CameraController {
    accum_local_pitch: Radians,
    max_pitch_angle: Radians,
    accum_local_yaw: Radians,

    move_speed: f32,
    mouse_sensitivity: f32,
    // Higher the value, server the lagging. Zero means no smoothing
    smoothing_factor: f32,

    accum_dx: f32,
    accum_dy: f32,
    is_grabbed: bool,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            accum_local_pitch: Default::default(),
            max_pitch_angle: Degree::from(89.99).into(),
            accum_local_yaw: Default::default(),

            move_speed: 70.,
            mouse_sensitivity: 1.,
            smoothing_factor: 0.85,

            accum_dx: 0.0,
            accum_dy: 0.0,
            is_grabbed: false,
        }
    }
}

impl CameraController {
    pub fn new(mouse_sensitivity: f32) -> Self {
        Self {
            mouse_sensitivity,
            ..Default::default()
        }
    }

    pub fn process_event(&mut self, event: &SystemEventCollector, window: &Window) {
        for event in event.window_events() {
            match event {
                WindowEvent::MouseInput { button, state, .. } => {
                    if *button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                self.grab_cursor(window);
                            }
                            ElementState::Released => {
                                self.release_cursor(window);
                            }
                        }
                    }
                }
                WindowEvent::Focused(false) => {
                    // release cursor when window loses focus
                    self.release_cursor(window);
                }
                _ => {}
            }
        }

        for event in event.device_events() {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    if self.is_grabbed {
                        self.accum_dx += delta.0 as f32;
                        self.accum_dy += delta.1 as f32;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update_cameras<'a>(&mut self, delta_time: f32, mapper: &InputActionMapper, to_update_cameras: impl IntoIterator<Item = &'a mut Camera>) {
        let d_local_yaw: Radians = Radians::from(-self.accum_dx * self.mouse_sensitivity * delta_time);
        let d_local_pitch: Radians = Radians::from(-self.accum_dy * self.mouse_sensitivity * delta_time);

        let blend_factor = 1.0 - self.smoothing_factor.powf(delta_time * 60.0);

        self.accum_local_yaw += d_local_yaw;
        self.accum_local_pitch += d_local_pitch;

        let delta_yaw = self.accum_local_yaw * blend_factor;
        let delta_pitch = self.accum_local_pitch * blend_factor;

        self.accum_local_yaw -= delta_yaw;
        self.accum_local_pitch -= delta_pitch;

        let axis_dir = Vec3::new(
            mapper.get_axis("strafe"),
            mapper.get_axis("walk"),
            mapper.get_axis("lift"),
        );
        let delta_pos = axis_dir * self.move_speed * delta_time;

        for camera in to_update_cameras {
            camera.rotate(delta_yaw, delta_pitch, self.max_pitch_angle);
            camera.translate(delta_pos);
            camera.update_local_basis();
            camera.update_view();
        }

        self.accum_dx = 0.0;
        self.accum_dy = 0.0;
    }

    fn grab_cursor(&mut self, window: &Window) {
        self.is_grabbed = true;

        window.set_cursor_visible(false);

        if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            if window.set_cursor_grab(CursorGrabMode::Confined).is_err() {
                warn!("Failed to grab cursor.")
            }
        }
    }

    fn release_cursor(&mut self, window: &Window) {
        self.is_grabbed = false;
        window.set_cursor_visible(true);

        if window.set_cursor_grab(CursorGrabMode::None).is_err() {
            warn!("Failed to release cursor.")
        }
    }
}
