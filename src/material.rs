use crate::color::Color;
use crate::texture::Texture;

#[derive(Debug, Clone)]  // Quitamos Copy, mantenemos Debug y Clone
pub struct Material {
    pub diffuse: Color,
    pub specular: f32,
    pub albedo: [f32; 4],
    pub refractive_index: f32,
    pub has_texture: bool,
    pub texture: Option<Texture>,  // Textura opcional
}

impl Material {
    pub fn get_diffuse_color(&self, u: f32, v: f32) -> Color {
        if let Some(texture) = &self.texture {
            let tex_x = (u * (texture.width as f32)) as usize % texture.width;
            let tex_y = ((1.0 - v) * (texture.height as f32)) as usize % texture.height;
            let pixel = texture.data[tex_y * texture.width + tex_x];
            Color::new(pixel.r, pixel.g, pixel.b)
        } else {
            self.diffuse.clone()
        }
    }

    pub fn black() -> Self {
        Material {
            diffuse: Color::new(0, 0, 0),
            specular: 0.0,
            albedo: [0.0, 0.0, 0.0, 0.0],
            refractive_index: 1.0,
            has_texture: false,
            texture: None,
        }
    }
}
