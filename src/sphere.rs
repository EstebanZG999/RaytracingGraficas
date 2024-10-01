// sphere.rs
use nalgebra_glm::Vec3;
use crate::material::Material;
use crate::intersect::{Intersect, RayIntersect};

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl Sphere {
    pub fn get_uv(&self, point: &Vec3) -> (f32, f32) {
        // Normalizar el vector desde el centro de la esfera hasta el punto de intersección
        let relative_point = (point - self.center).normalize();

        // Cálculo de coordenadas esféricas para UV
        let theta = relative_point.z.atan2(relative_point.x);
        let phi = relative_point.y.asin();

        // Convertir de coordenadas esféricas a UV
        let u = 0.5 + theta / (2.0 * std::f32::consts::PI);
        let v = 0.5 - phi / std::f32::consts::PI;

        (u, v)
    }
}


impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        // Vector desde el origen del rayo al centro de la esfera
        let oc = ray_origin - self.center;
        let a = ray_direction.dot(ray_direction);
        let b = 2.0 * oc.dot(ray_direction);
        let c = oc.dot(&oc) - self.radius * self.radius;

        // Calcular el discriminante
        let discriminant = b * b - 4.0 * a * c;

        if discriminant > 0.0 {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t > 0.0 {
                let point = ray_origin + ray_direction * t;
                let normal = (point - self.center).normalize();
                let distance = t;

                // Calcular las coordenadas UV en el punto de intersección
                let (u, v) = self.get_uv(&point);

                return Intersect::new(point, normal, distance, self.material.clone(), u, v);
            }
        }

        // Si no hay intersección, devolver un objeto Intersect vacío
        Intersect::empty()
    }
}