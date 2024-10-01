use nalgebra_glm::Vec3;
use crate::color::Color;

pub struct Light {
    pub position: Vec3,  // Posición de la luz en el espacio
    pub color: Color,    // Color de la luz (normalmente blanco)
    pub intensity: f32,  // Intensidad de la luz
}

impl Light {
    pub fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
        }
    }
}
