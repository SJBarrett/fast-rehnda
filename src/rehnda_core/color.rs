use bytemuck_derive::{Pod, Zeroable};

#[repr(C)]
#[derive(Pod, Zeroable, Debug, PartialEq, Copy, Clone)]
pub struct ColorRgbaF {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgbaF {

}

impl ColorRgbaF {
    pub const WHITE: Self = ColorRgbaF::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r,
            g,
            b,
            a
        }
    }

    pub const fn new_from_array(p: [f32; 4]) -> Self {
        Self {
            r: p[0],
            g: p[1],
            b: p[2],
            a: p[3],
        }
    }

    pub fn to_rgba8(&self) -> [u8; 4] {
        [f_to_8bit(self.r), f_to_8bit(self.g), f_to_8bit(self.b), f_to_8bit(self.a)]
    }
}

fn f_to_8bit(f: f32) -> u8 {
    if f >= 1.0 {
        255u8
    } else {
        (f * 255.0).round().clamp(0.0, 255.0) as u8
    }
}