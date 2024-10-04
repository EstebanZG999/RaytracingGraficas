use nalgebra_glm::Vec3;
use crate::intersect::{Intersect, RayIntersect}; // Cambiado de ray_intersect a intersect
use crate::material::Material; // Cambiado de ray_intersect a material

pub struct Cube {
    pub center: Vec3,
    pub size: f32,
    pub materials: [Material; 6], 
}

impl Cube {

    pub fn get_uv_for_face(face_index: usize, local_pos: Vec3) -> (f32, f32) {
        match face_index {
            // Front Face (Z+)
            4 => Cube::map_uv(local_pos.x, -local_pos.y),
            // Back Face (Z-)
            5 => Cube::map_uv(-local_pos.x, -local_pos.y),
            // Left Face (X-)
            0 => Cube::map_uv(local_pos.z, -local_pos.y),
            // Right Face (X+)
            1 => Cube::map_uv(-local_pos.z, -local_pos.y),
            // Top Face (Y+)
            2 => Cube::map_uv(local_pos.x, local_pos.z),
            // Bottom Face (Y-)
            3 => Cube::map_uv(local_pos.x, -local_pos.z),
            _ => (0.0, 0.0),
        }
    }
    

    // Este método mapea las coordenadas UV en el rango [0, 1].
    fn map_uv(u: f32, v: f32) -> (f32, f32) {
        let u = (u + 1.0) * 0.5;
        let v = (v + 1.0) * 0.5;
        (u.fract(), v.fract())
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
            if (punto_encuentro[i] - min[i]).abs() < 1e-4 {
                normal[i] = -1.0;
                face_index = match i {
                    0 => 0, // Left Face (X-)
                    1 => 3, // Bottom Face (Y-)
                    2 => 5, // Back Face (Z-)
                    _ => 0,
                };
            } else if (punto_encuentro[i] - max[i]).abs() < 1e-4 {
                normal[i] = 1.0;
                face_index = match i {
                    0 => 1, // Right Face (X+)
                    1 => 2, // Top Face (Y+)
                    2 => 4, // Front Face (Z+)
                    _ => 1,
                };
            }
        }

        let local_pos = punto_encuentro - self.center;
        let (u, v) = Cube::get_uv_for_face(face_index, local_pos);


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