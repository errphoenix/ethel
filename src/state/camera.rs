use core::f32;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default)]
pub struct ViewPoint {
    pub orientation: glam::Quat,
    pub position: glam::Vec3,
}

impl std::ops::Mul<glam::Quat> for ViewPoint {
    type Output = ViewPoint;

    fn mul(self, rhs: glam::Quat) -> Self::Output {
        Self::Output {
            orientation: self.orientation * rhs,
            position: self.position,
        }
    }
}

impl std::ops::Mul<glam::Vec3> for ViewPoint {
    type Output = ViewPoint;

    fn mul(self, rhs: glam::Vec3) -> Self::Output {
        Self::Output {
            orientation: self.orientation,
            position: self.position * rhs,
        }
    }
}

impl std::ops::Add<glam::Vec3> for ViewPoint {
    type Output = ViewPoint;

    fn add(self, rhs: glam::Vec3) -> Self::Output {
        Self::Output {
            orientation: self.orientation,
            position: self.position + rhs,
        }
    }
}

impl std::ops::Sub<glam::Vec3> for ViewPoint {
    type Output = ViewPoint;

    fn sub(self, rhs: glam::Vec3) -> Self::Output {
        Self::Output {
            orientation: self.orientation,
            position: self.position - rhs,
        }
    }
}

impl ViewPoint {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn from_position(pos: impl Into<glam::Vec3>) -> Self {
        Self {
            orientation: glam::Quat::IDENTITY,
            position: pos.into(),
        }
    }

    #[inline(always)]
    pub fn forward(&self) -> glam::Vec3 {
        self.orientation * glam::Vec3::NEG_Z
    }

    #[inline(always)]
    pub fn right(&self) -> glam::Vec3 {
        self.orientation * glam::Vec3::X
    }

    #[inline(always)]
    pub fn up(&self) -> glam::Vec3 {
        self.orientation * glam::Vec3::Y
    }

    #[inline(always)]
    pub fn yaw_pitch(&self) -> (f32, f32) {
        let (yaw, pitch, _roll) = self.orientation.to_euler(glam::EulerRot::YXZ);
        (yaw, pitch)
    }

    #[inline(always)]
    pub fn translate(&mut self, translation: impl Into<glam::Vec3>) {
        let vec3 = translation.into();
        self.position += vec3;
    }

    #[inline(always)]
    pub fn rotate_axis(&mut self, axis: impl Into<glam::Vec3>, angle: f32) {
        let quat = glam::Quat::from_axis_angle(axis.into(), angle);
        self.orientation = quat * self.orientation;
    }

    #[inline(always)]
    pub fn rotate(&mut self, rotation: impl Into<glam::Quat>) {
        let quat = rotation.into();
        self.orientation = quat * self.orientation;
    }

    #[inline(always)]
    pub fn rotate_axis_world(&mut self, axis: impl Into<glam::Vec3>, angle: f32) {
        let quat = glam::Quat::from_axis_angle(axis.into(), angle);
        self.orientation *= quat;
    }

    #[inline(always)]
    pub fn rotate_world(&mut self, rotation: impl Into<glam::Quat>) {
        let quat = rotation.into();
        self.orientation *= quat;
    }

    #[inline(always)]
    pub fn translation(&self) -> glam::Vec3 {
        self.position
    }

    #[inline(always)]
    pub fn translation_mut(&mut self) -> &mut glam::Vec3 {
        &mut self.position
    }

    #[inline(always)]
    pub fn orientation(&self) -> glam::Quat {
        self.orientation
    }

    #[inline(always)]
    pub fn orientation_mut(&mut self) -> &mut glam::Quat {
        &mut self.orientation
    }

    #[inline(always)]
    pub fn into_mat4(self) -> glam::Mat4 {
        glam::Mat4::from_rotation_translation(self.orientation, self.position).inverse()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct OrbitalDistance(f32);

impl Default for OrbitalDistance {
    fn default() -> Self {
        Self(Self::DEFAULT_BASE_DISTANCE)
    }
}

impl OrbitalDistance {
    pub const DEFAULT_BASE_DISTANCE: f32 = 5.0;

    pub fn new(distance: f32) -> Self {
        Self(distance)
    }

    pub fn set(&mut self, distance: f32) {
        self.0 = distance.min(0.0);
    }

    pub fn into_inner(&self) -> f32 {
        self.0
    }
}

impl std::ops::Add<f32> for OrbitalDistance {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Self((self.0 + rhs).max(0.0))
    }
}

impl std::ops::AddAssign<f32> for OrbitalDistance {
    fn add_assign(&mut self, rhs: f32) {
        self.0 = (self.0 + rhs).max(0.0);
    }
}

impl std::ops::Sub<f32> for OrbitalDistance {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        Self((self.0 - rhs).max(0.0))
    }
}

impl std::ops::SubAssign<f32> for OrbitalDistance {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 = (self.0 - rhs).max(0.0);
    }
}
impl std::ops::Deref for OrbitalDistance {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct RotationLimits {
    pub yaw: Range<f32>,
    pub pitch: Range<f32>,
}

impl Default for RotationLimits {
    fn default() -> Self {
        Self {
            yaw: Self::DEFAULT_YAW_LIMIT,
            pitch: Self::DEFAULT_PITCH_LIMIT,
        }
    }
}

impl RotationLimits {
    pub const DEFAULT_YAW_LIMIT: Range<f32> = f32::NEG_INFINITY..f32::INFINITY;
    pub const DEFAULT_PITCH_LIMIT: Range<f32> = -Self::PITCH_LIMIT_90_DEG..Self::PITCH_LIMIT_90_DEG;

    const PITCH_LIMIT_90_DEG: f32 = f32::consts::FRAC_PI_2 - 0.5;

    pub fn new(yaw: Range<f32>, pitch: Range<f32>) -> Self {
        Self { yaw, pitch }
    }

    #[inline(always)]
    pub fn clamp_yaw(&self, v: f32) -> f32 {
        v.clamp(self.yaw.start, self.yaw.end)
    }

    #[inline(always)]
    pub fn clamp_pitch(&self, v: f32) -> f32 {
        v.clamp(self.pitch.start, self.pitch.end)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Orbital {
    viewpoint: ViewPoint,
    orbit_distance: OrbitalDistance,
    limits: RotationLimits,
    anchor: glam::Vec3,
}

impl Orbital {
    pub fn new(viewpoint: ViewPoint, distance: OrbitalDistance, limits: RotationLimits) -> Self {
        Self {
            viewpoint,
            orbit_distance: distance,
            limits,
            anchor: glam::Vec3::ZERO,
        }
    }

    pub fn with_anchor(
        viewpoint: ViewPoint,
        orbit_distance: OrbitalDistance,
        anchor: glam::Vec3,
        limits: RotationLimits,
    ) -> Self {
        Self {
            viewpoint,
            orbit_distance,
            limits,
            anchor,
        }
    }

    pub fn update(&mut self, d_yaw: f32, d_pitch: f32) {
        let (yaw, pitch) = self.viewpoint.yaw_pitch();
        let yaw = self.limits.clamp_yaw(yaw - d_yaw);
        let pitch = self.limits.clamp_pitch(pitch + d_pitch);

        self.viewpoint.orientation = glam::Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, 0.0);
        self.viewpoint.position = self.anchor - (self.viewpoint.forward() * *self.orbit_distance);
    }

    pub fn viewpoint(&self) -> &ViewPoint {
        &self.viewpoint
    }

    pub fn viewpoint_mut(&mut self) -> &mut ViewPoint {
        &mut self.viewpoint
    }

    pub fn distance(&self) -> OrbitalDistance {
        self.orbit_distance
    }

    pub fn distance_mut(&mut self) -> &mut OrbitalDistance {
        &mut self.orbit_distance
    }

    pub fn rotation_limits(&self) -> &RotationLimits {
        &self.limits
    }

    pub fn rotation_limits_mut(&mut self) -> &mut RotationLimits {
        &mut self.limits
    }
}
