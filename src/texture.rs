use crate::color::Color;

#[derive(Debug, Clone)]  // Derivamos Debug para poder imprimir texturas
pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Color>,  // Datos de la imagen
}

impl Texture {
    pub fn new(width: usize, height: usize, data: Vec<Color>) -> Self {
        Texture { width, height, data }
    }

    pub fn get_color(&self, x: usize, y: usize) -> Color {
        self.data[y * self.width + x]
    }
}
