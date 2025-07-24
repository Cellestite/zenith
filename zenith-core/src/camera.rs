use std::cell::RefCell;
use glam::{Mat4, Quat, Vec3};
use crate::math::Radians;

#[derive(Debug)]
pub struct Camera {
    location: Vec3,
    pitch: Radians,
    yaw: Radians,
    view: RefCell<Mat4>,
    proj: RefCell<Mat4>,
    view_dirty: RefCell<bool>,
}

// Zenith world space coordinate system (right-hand side, z up)
//
//                z
//                ^    y
//                |   /
//                |  /
//                | /
//                ----------> x
//

impl Default for Camera {
    fn default() -> Self {
        let cam = Self {
            location: Default::default(),
            pitch: Default::default(),
            yaw: Default::default(),
            view: Default::default(),
            proj: RefCell::new(Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_6, 1.77777, Self::Z_NEAR)),
            view_dirty: RefCell::new(true),
        };
        cam.update_view();
        cam
    }
}

impl Camera {
    pub const Z_NEAR: f32 = 0.1;
    pub const UP: Vec3 = Vec3::new(0., 0., 1.);

    pub fn new(fov_y: Radians, aspect_ratio: f32, z_near: f32) -> Self {
        let cam = Self {
            location: Default::default(),
            pitch: Default::default(),
            yaw: Default::default(),
            view: Default::default(),
            proj: RefCell::new(Mat4::perspective_infinite_reverse_rh(fov_y.into(), aspect_ratio, z_near.max(0.0001))),
            view_dirty: RefCell::new(true),
        };
        cam.update_view();
        cam
    }

    pub fn translation(&mut self, unit: Vec3) {
        self.location += unit;
        self.view.borrow_mut().x_axis.w -= unit.x;
        self.view.borrow_mut().y_axis.w -= unit.y;
        self.view.borrow_mut().z_axis.w -= unit.z;
    }

    pub fn rotate_pitch(&mut self, angle: Radians) {
        self.pitch += angle;
        self.pitch = self.pitch.clamp(-std::f32::consts::PI + 0.00001, std::f32::consts::PI - 0.00001).into();
        *self.view_dirty.borrow_mut() = true;
    }

    pub fn rotate_yaw(&mut self, angle: Radians) {
        self.yaw += angle;
        self.yaw %= 2.0 * std::f32::consts::PI;
        *self.view_dirty.borrow_mut() = true;
    }

    pub fn location(&self) -> Vec3 {
        self.location
    }

    pub fn view(&self) -> Mat4 {
        self.update_view();
        self.view.borrow().clone()
    }

    pub fn projection(&self) -> Mat4 {
        self.proj.borrow().clone()
    }

    pub fn view_projection(&self) -> Mat4 {
        self.update_view();
        self.view.borrow().clone() * self.proj.borrow().clone()
    }

    fn update_view(&self) {
        if *self.view_dirty.borrow() {
            let dir = Vec3::new(0., 1., 0.);
            let rotator = Quat::from_rotation_x(self.pitch.into()) * Quat::from_rotation_z(self.yaw.into());
            let dir = rotator * dir;

            *self.view.borrow_mut() = Mat4::look_to_rh(self.location, dir, Self::UP);
            *self.view_dirty.borrow_mut() = false;
        }
    }
}


