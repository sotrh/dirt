pub trait Camera {
    fn view(&self) -> glam::Mat4;
    fn proj(&self) -> glam::Mat4;
    fn view_proj(&self) -> glam::Mat4 {
        self.proj() * self.view()
    }
}

pub struct Camera2d {
    width: f32,
    height: f32,
}

impl Camera2d {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
    
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.width = width as f32;
        self.height = height as f32;
    }
}

impl Camera for Camera2d {
    fn view(&self) -> glam::Mat4 {
        glam::Mat4::IDENTITY
    }

    fn proj(&self) -> glam::Mat4 {
        glam::Mat4::orthographic_rh(0.0, self.width, 0.0, self.height, 0.0, 1.0)
    }
}

const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct PerspectiveCamera {
    pub position: glam::Vec3,
    yaw: f32,
    pitch: f32,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl PerspectiveCamera {
    pub fn new<V: Into<glam::Vec3>>(
        position: V,
        yaw: f32,
        pitch: f32,
        width: u32,
        height: u32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            position: position.into(),
            yaw,
            pitch,
            aspect: width as f32 / height as f32,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
}

impl Camera for PerspectiveCamera {
    fn view(&self) -> glam::Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        glam::Mat4::look_to_rh(
            self.position,
            glam::Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            glam::Vec3::Y,
        )
    }

    fn proj(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}
