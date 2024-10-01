use nalgebra_glm::Vec3;
use crate::intersect::{Intersect, RayIntersect}; // Cambiado de ray_intersect a intersect
use crate::material::Material; // Cambiado de ray_intersect a material
use crate::color::Color;
use crate::light::Light;
use std::any::Any;

pub struct Cube {
    pub center: Vec3,
    pub size: f32,
    pub materials: [Material; 6], 
}

impl Cube {
    pub fn new(center: Vec3, size: f32, materials: [Material; 6]) -> Self {
        Cube { center, size, materials }
    }

    fn get_uv(&self, punto_encuentro: &Vec3) -> (f32, f32) {
        let mitad = self.size / 2.0;
        let min = self.center - Vec3::new(mitad, mitad, mitad);
        let max = self.center + Vec3::new(mitad, mitad, mitad);

        let mut u = 0.0;
        let mut v = 0.0;

        if (punto_encuentro.x - min.x).abs() < 0.001 { 
            u = (punto_encuentro.z - min.z) / (max.z - min.z);
            v = (punto_encuentro.y - min.y) / (max.y - min.y);
        } else if (punto_encuentro.x - max.x).abs() < 0.001 { 
            u = (punto_encuentro.z - min.z) / (max.z - min.z);
            v = (punto_encuentro.y - min.y) / (max.y - min.y);
        } else if (punto_encuentro.y - min.y).abs() < 0.001 { 
            u = (punto_encuentro.x - min.x) / (max.x - min.x);
            v = (punto_encuentro.z - min.z) / (max.z - min.z);
        } else if (punto_encuentro.y - max.y).abs() < 0.001 { 
            u = (punto_encuentro.x - min.x) / (max.x - min.x);
            v = (punto_encuentro.z - min.z) / (max.z - min.z);
        } else if (punto_encuentro.z - min.z).abs() < 0.001 { 
            u = (punto_encuentro.x - min.x) / (max.x - min.x);
            v = (punto_encuentro.y - min.y) / (max.y - min.y);
        } else if (punto_encuentro.z - max.z).abs() < 0.001 { 
            u = (punto_encuentro.x - min.x) / (max.x - min.x);
            v = (punto_encuentro.y - min.y) / (max.y - min.y);
        }

        (u, v)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_diffuse_color(&self, face_index: usize, u: f32, v: f32) -> Color {
        if let Some(texture) = &self.materials[face_index].texture {
            let tex_x = ((u * texture.width as f32) as usize) % texture.width;
            let tex_y = ((v * texture.height as f32) as usize) % texture.height;
            
            // En lugar de usar pixel.r(), pixel.g(), etc., accedemos a los campos directamente
            let pixel = texture.data[tex_y * texture.width + tex_x];
            Color::new(pixel.r, pixel.g, pixel.b)
        } else {
            self.materials[face_index].diffuse.clone()
        }
    }
        
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let mitad = self.size / 2.0;
        let min = self.center - Vec3::new(mitad, mitad, mitad);
        let max = self.center + Vec3::new(mitad, mitad, mitad);

        let inv_dir = Vec3::new(1.0 / ray_direction.x, 1.0 / ray_direction.y, 1.0 / ray_direction.z);
        let t_min = (min - ray_origin).component_mul(&inv_dir);
        let t_max = (max - ray_origin).component_mul(&inv_dir);

        let t1 = t_min.x.min(t_max.x).max(t_min.y.min(t_max.y)).max(t_min.z.min(t_max.z));
        let t2 = t_min.x.max(t_max.x).min(t_min.y.max(t_max.y)).min(t_min.z.max(t_max.z));

        if t1 > t2 || t2 < 0.0 {
            return Intersect::empty();
        }

        let t_hit = if t1 < 0.0 { t2 } else { t1 };
        let punto_encuentro = ray_origin + ray_direction * t_hit;

        let mut normal = Vec3::new(0.0, 0.0, 0.0);
        let mut face_index = 0;

        for i in 0..3 {
            if (punto_encuentro[i] - min[i]).abs() < 0.001 {
                normal[i] = -1.0;
                face_index = match i {
                    0 => 0, 
                    1 => 2, 
                    2 => 4, 
                    _ => 0,
                };
            } else if (punto_encuentro[i] - max[i]).abs() < 0.001 {
                normal[i] = 1.0;
                face_index = match i {
                     0 => 1, 
                     1 => 3, 
                     2 => 5, 
                     _ => 1,
                };
            }
        }

        let (u, v) = self.get_uv(&punto_encuentro);
        let u = u.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        let textura_color = self.get_diffuse_color(face_index, u, v);

        Intersect::new(
            punto_encuentro,
            normal,
            t_hit,
            self.materials[face_index].clone(),
            u,
            v
        )
    }
}
