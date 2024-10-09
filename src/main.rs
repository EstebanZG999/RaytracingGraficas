mod color;
mod material;
mod intersect;
mod sphere;
mod camera;
mod light;
mod texture;
mod cube;

use nalgebra_glm::Vec3;
use crate::intersect::{RayIntersect, Intersect};
use camera::Camera;
use rayon::prelude::*;
use crate::light::Light;
use crate::cube::Cube;
use image::GenericImageView;
use crate::texture::Texture;
use crate::color::Color;




fn load_texture(filename: &str) -> Texture {
    let img = image::open(filename).expect("Failed to load texture");
    let (width, height) = img.dimensions();
    let mut data = Vec::new();

    for y in (0..height).rev() {  // Flip vertically
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            data.push(Color::new(pixel[0], pixel[1], pixel[2]));
        }
    }

    Texture { width: width as usize, height: height as usize, data }
}


pub fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}

pub fn refract(incident: &Vec3, normal: &Vec3, eta_t: f32) -> Vec3 {
    let cosi = -incident.dot(normal).max(-1.0).min(1.0);

    let (n_cosi, eta, n_normal);

    if cosi < 0.0 {
        // Ray is entering the object
        n_cosi = -cosi;
        eta = 1.0 / eta_t;
        n_normal = -*normal;
    } else {
        // Ray is leaving the object
        n_cosi = cosi;
        eta = eta_t;
        n_normal = *normal;
    }

    let k = 1.0 - eta * eta * (1.0 - n_cosi * n_cosi);

    if k < 0.0 {
        // Total internal reflection, return reflected ray
        reflect(incident, &n_normal)
    } else {
        eta * incident + (eta * n_cosi - k.sqrt()) * n_normal
    }
}


pub fn cast_shadow(
    intersect: &Intersect,
    light: &Light,
    objects: &[Box<dyn RayIntersect>],
) -> f32 {
    // Dirección hacia la luz
    let light_dir = (light.position - intersect.point).normalize();
    // Desplazamos ligeramente el origen del rayo de sombra en la dirección de la normal para evitar el acné
    let shadow_ray_origin = intersect.point + intersect.normal * 1e-3;

    let mut shadow_intensity = 0.0;

    // Lanzamos un rayo de sombra para cada objeto
    for object in objects {
        let shadow_intersect = object.ray_intersect(&shadow_ray_origin, &light_dir);
        if shadow_intersect.is_intersecting {
            // Ajustamos la intensidad de la sombra en función de la distancia
            let distance_to_object = (shadow_intersect.point - intersect.point).magnitude();
            let distance_to_light = (light.position - intersect.point).magnitude();
            
            // Si el objeto está entre el punto de intersección y la luz, ajustamos la sombra
            if distance_to_object < distance_to_light {
                shadow_intensity = 1.0 - (distance_to_object / distance_to_light).min(1.0);
                break;
            }
        }
    }

    shadow_intensity
}


pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    lights: &Light,
    depth: u32,
) -> color::Color {
    if depth > 1 {
        return color::Color::new(4, 12, 36);  // Color de fondo
    }

    let mut closest_intersection = Intersect::empty();
    let mut closest_distance = f32::INFINITY;

    // Buscar la intersección más cercana con cualquier objeto
    for object in objects {
        let intersection = object.ray_intersect(ray_origin, ray_direction);
        if intersection.is_intersecting && intersection.distance < closest_distance {
            closest_distance = intersection.distance;
            closest_intersection = intersection;
        }
    }

    if !closest_intersection.is_intersecting {
        return color::Color::new(4, 12, 36);  // Color del cielo o fondo
    }

    // Obtener el color difuso del material
    let diffuse_color = closest_intersection
        .material
        .get_diffuse_color(closest_intersection.u, closest_intersection.v);

    // Luz ambiental global
    let ambient_light_intensity = 0.3;  // Ajusta la intensidad según sea necesario
    let ambient_light = color::Color {
        r: (diffuse_color.r as f32 * ambient_light_intensity).min(255.0) as u8,
        g: (diffuse_color.g as f32 * ambient_light_intensity).min(255.0) as u8,
        b: (diffuse_color.b as f32 * ambient_light_intensity).min(255.0) as u8,
    };

    // Calcular la dirección de la luz y la intensidad difusa usando la ley de Lambert
    let light_dir = (lights.position - closest_intersection.point).normalize();
    let diffuse_intensity = closest_intersection.normal.dot(&light_dir).max(0.0).min(1.0);

    // Componente difusa
    let diffuse = color::Color {
        r: (diffuse_color.r as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * lights.intensity).min(255.0) as u8,
        g: (diffuse_color.g as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * lights.intensity).min(255.0) as u8,
        b: (diffuse_color.b as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * lights.intensity).min(255.0) as u8,
    };

    // Calcular la intensidad de la sombra
    let shadow_intensity = cast_shadow(&closest_intersection, lights, objects);
    let light_intensity = lights.intensity * (1.0 - shadow_intensity);

    // Componente especular usando el modelo de Phong
    let view_dir = (ray_origin - closest_intersection.point).normalize();
    let reflect_dir = reflect(&-ray_direction, &closest_intersection.normal).normalize();
    let specular_intensity = view_dir
        .dot(&reflect_dir)
        .max(0.0)
        .powf(closest_intersection.material.specular);
    let specular = color::Color {
        r: (lights.color.r as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
        g: (lights.color.g as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
        b: (lights.color.b as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
    };

    // Suma de la luz ambiental, difusa y especular
    let final_color = color::Color {
        r: (ambient_light.r as u32 + diffuse.r as u32 + specular.r as u32).min(255) as u8,
        g: (ambient_light.g as u32 + diffuse.g as u32 + specular.g as u32).min(255) as u8,
        b: (ambient_light.b as u32 + diffuse.b as u32 + specular.b as u32).min(255) as u8,
    };

    // Componente de reflexión
    let reflectivity = closest_intersection.material.albedo[2];
    let mut reflect_color = color::Color::new(0, 0, 0);
    if reflectivity > 0.0 {
        let reflect_origin = closest_intersection.point + closest_intersection.normal * 1e-3;
        reflect_color = cast_ray(&reflect_origin, &reflect_dir, objects, lights, depth + 1);
        reflect_color = color::Color {
            r: (reflect_color.r as f32 * reflectivity).min(255.0) as u8,
            g: (reflect_color.g as f32 * reflectivity).min(255.0) as u8,
            b: (reflect_color.b as f32 * reflectivity).min(255.0) as u8,
        };
    }

    // Componente de refracción
    let transparency = closest_intersection.material.albedo[3];
    let mut refract_color = color::Color::new(0, 0, 0);
    if transparency > 0.0 {
        let refract_dir = refract(&ray_direction, &closest_intersection.normal, closest_intersection.material.refractive_index).normalize();
        let refract_origin = closest_intersection.point + closest_intersection.normal * 1e-3;  // Evitar acné de sombras
        refract_color = cast_ray(&refract_origin, &refract_dir, objects, lights, depth + 1);
        refract_color = color::Color {
            r: (refract_color.r as f32 * transparency).min(255.0) as u8,
            g: (refract_color.g as f32 * transparency).min(255.0) as u8,
            b: (refract_color.b as f32 * transparency).min(255.0) as u8,
        };
    }

    // Combinar difusa, especular, reflejada y refractada
    color::Color {
        r: ((final_color.r as f32 * (1.0 - reflectivity - transparency)) + (reflect_color.r as f32 * reflectivity) + (refract_color.r as f32 * transparency)).min(255.0) as u8,
        g: ((final_color.g as f32 * (1.0 - reflectivity - transparency)) + (reflect_color.g as f32 * reflectivity) + (refract_color.g as f32 * transparency)).min(255.0) as u8,
        b: ((final_color.b as f32 * (1.0 - reflectivity - transparency)) + (reflect_color.b as f32 * reflectivity) + (refract_color.b as f32 * transparency)).min(255.0) as u8,
    }
}





pub fn render(
    framebuffer: &mut [u32], 
    width: usize, 
    height: usize, 
    objects: &[Box<dyn RayIntersect>], 
    camera: &Camera, 
    lights: &[Light]
) {
    let chunk_size = 8;  // Tamaño de bloque para procesar en paralelo
    framebuffer.par_chunks_mut(width * chunk_size).enumerate().for_each(|(chunk_idx, chunk)| {
        let base_y = chunk_idx * chunk_size;
    
        for (y, row) in chunk.chunks_mut(width).enumerate() {
            let screen_y = -((2.0 * (base_y + y) as f32) / height as f32 - 1.0);
    
            row.iter_mut().enumerate().for_each(|(x, pixel)| {
                let screen_x = (2.0 * x as f32) / width as f32 - 1.0;
                let screen_x = screen_x * (width as f32 / height as f32);
    
                let ray_direction = nalgebra_glm::normalize(&Vec3::new(screen_x, screen_y, -1.0));
                let transformed_direction = camera.basis_change(&ray_direction);
    
                let pixel_color = lights.iter().fold(color::Color::new(0, 0, 0), |acc, light| {
                    let light_color = cast_ray(&camera.eye, &transformed_direction, objects, light, 0);
                    color::Color {
                        r: (acc.r as u32 + light_color.r as u32).min(255) as u8,
                        g: (acc.g as u32 + light_color.g as u32).min(255) as u8,
                        b: (acc.b as u32 + light_color.b as u32).min(255) as u8,
                    }
                });
    
                *pixel = ((pixel_color.r as u32) << 16)
                    | ((pixel_color.g as u32) << 8)
                    | (pixel_color.b as u32);
            });
        }
    });    
}







fn main() {
    let width = 600;
    let height = 600;


    // Cargar las texturas desde archivos PNG
    let grama_texture = load_texture("textures/grama.png");
    let tierra_texture = load_texture("textures/tierraG.jpeg");
    let tierra4_texture = load_texture("textures/tierra.jpeg");
    let arena_texture = load_texture("textures/arena.jpeg");
    let agua_texture = load_texture("textures/agua.jpeg");
    let madera_texture = load_texture("textures/madera.jpeg");

    // Inicializar la cámara
    let eye = Vec3::new(0.0, 0.0, 5.0);
    let center = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let mut camera = Camera { eye, center, up };

    // Inicializar la luz
    let lights = vec![
        Light::new(Vec3::new(5.0, 5.0, 5.0), color::Color::new(255, 255, 255), 0.8),  // Luz principal

    ];
    

    // Definir los materiales 
    let tierra_material = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(tierra_texture),
    };

    let tierra_material4 = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(tierra4_texture),
    };

    let grama_material = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(grama_texture),
    };

    let arena = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(arena_texture),
    };

    let agua = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(agua_texture),
    };

    // Crear un cubo con materiales para cada cara
    let floor = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor1 = Box::new(Cube {
        center: Vec3::new(0.0, 2.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor11 = Box::new(Cube {
        center: Vec3::new(0.0, 4.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    // Crear un cubo adicional al lado derecho del cubo existente
    let floor2 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, 0.0),  // Posicionamos el cubo 2 unidades a la derecha del primero
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor22 = Box::new(Cube {
        center: Vec3::new(2.0, 2.0, 0.0),  // Posicionamos el cubo 2 unidades a la derecha del primero
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor222 = Box::new(Cube {
        center: Vec3::new(2.0, 4.0, 0.0),  // Posicionamos el cubo 2 unidades a la derecha del primero
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });


    let floor3 = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor33 = Box::new(Cube {
        center: Vec3::new(0.0, 2.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor333 = Box::new(Cube {
        center: Vec3::new(0.0, 4.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });


    let floor4 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor44 = Box::new(Cube {
        center: Vec3::new(2.0, 2.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor444 = Box::new(Cube {
        center: Vec3::new(2.0, 4.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    let floor5 = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor55 = Box::new(Cube {
        center: Vec3::new(0.0, 2.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor555 = Box::new(Cube {
        center: Vec3::new(0.0, 4.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    let floor6 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor66 = Box::new(Cube {
        center: Vec3::new(4.0, 2.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor666 = Box::new(Cube {
        center: Vec3::new(4.0, 4.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });


    let floor7 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor77 = Box::new(Cube {
        center: Vec3::new(2.0, 2.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    let floor8 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor88 = Box::new(Cube {
        center: Vec3::new(4.0, 2.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    let floor9 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor99 = Box::new(Cube {
        center: Vec3::new(6.0, 2.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });


    let floor10 = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material4.clone(),  // Derecha (X+)
            tierra_material4.clone(),  // Izquierda (X-)
            tierra_material4.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material4.clone(),  // Frente (Z+)
            tierra_material4.clone()   // Atrás (Z-)
        ],
    });

    let floor1010 = Box::new(Cube {
        center: Vec3::new(0.0, 2.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    let floorii = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });

    let floor12 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),        // Arriba (Y+)
            tierra_material4.clone(),         // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone()   // Atrás (Z-)
        ],
    });


    let arena1 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena111 = Box::new(Cube {
        center: Vec3::new(6.0, 2.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena2 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena22 = Box::new(Cube {
        center: Vec3::new(2.0, 2.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena3 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena33 = Box::new(Cube {
        center: Vec3::new(4.0, 2.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena4 = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena5 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena6 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena7 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena8 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena9 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena10 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });


    let arena11 = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena12 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena13 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena14 = Box::new(Cube {
        center: Vec3::new(0.0, 0.0, -14.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena15 = Box::new(Cube {
        center: Vec3::new(14.0, 0.0, 0.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            arena.clone(),  // Derecha (X+)
            arena.clone(),  // Izquierda (X-)
            arena.clone(),        // Arriba (Y+)
            arena.clone(),         // Abajo (Y-)
            arena.clone(),  // Frente (Z+)
            arena.clone()   // Atrás (Z-)
        ],
    });

    let arena15 = Box::new(Cube { 
        center: Vec3::new(0.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena16 = Box::new(Cube { 
        center: Vec3::new(2.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena17 = Box::new(Cube { 
        center: Vec3::new(4.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena18 = Box::new(Cube { 
        center: Vec3::new(6.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena19 = Box::new(Cube { 
        center: Vec3::new(8.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena20 = Box::new(Cube { 
        center: Vec3::new(10.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena21 = Box::new(Cube { 
        center: Vec3::new(12.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena222 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });

    
    let arena23 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, 0.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena24 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -2.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena25 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -4.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena26 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -6.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena27 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -8.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena28 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -10.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena29 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -12.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena30 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let arena31 = Box::new(Cube { 
        center: Vec3::new(16.0, 0.0, -16.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });

    let agua1 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua2 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua3 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua4 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });


    let agua5 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua6 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua7 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua8 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua9 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });


    let agua10 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua11 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });


    let agua12 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua13 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua14 = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua15 = Box::new(Cube {
        center: Vec3::new(4.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua16 = Box::new(Cube {
        center: Vec3::new(6.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua17 = Box::new(Cube {
        center: Vec3::new(8.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua18 = Box::new(Cube {
        center: Vec3::new(10.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua19 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, -2.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua20 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, -4.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua21 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, -6.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua22 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, -8.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });


    let agua23 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, -10.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua24 = Box::new(Cube {
        center: Vec3::new(12.0, 0.0, -12.0),  // Posición del cubo en el espacio
        size: 2.0,                         // Tamaño del cubo
        materials: [
            agua.clone(),  // Derecha (X+)
            agua.clone(),  // Izquierda (X-)
            agua.clone(),        // Arriba (Y+)
            agua.clone(),         // Abajo (Y-)
            agua.clone(),  // Frente (Z+)
            agua.clone()   // Atrás (Z-)
        ],
    });

    let agua25 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, 0.0),  
        size: 2.0,                         
        materials: [
            arena.clone(),  
            arena.clone(),  
            arena.clone(),        
            arena.clone(),         
            arena.clone(),  
            arena.clone()   
        ],
    });
    
    let agua26 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -2.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua27 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -4.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua28 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -6.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua29 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -8.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua30 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -10.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua31 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -12.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua32 = Box::new(Cube { 
        center: Vec3::new(14.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });

    let agua33 = Box::new(Cube { 
        center: Vec3::new(2.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua34 = Box::new(Cube { 
        center: Vec3::new(4.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua35 = Box::new(Cube { 
        center: Vec3::new(6.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua36 = Box::new(Cube { 
        center: Vec3::new(8.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua37 = Box::new(Cube { 
        center: Vec3::new(10.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    
    let agua38 = Box::new(Cube { 
        center: Vec3::new(12.0, 0.0, -14.0),  
        size: 2.0,                         
        materials: [
            agua.clone(),  
            agua.clone(),  
            agua.clone(),        
            agua.clone(),         
            agua.clone(),  
            agua.clone()   
        ],
    });
    

    // Crear una lista de objetos con el cubo
    let objects: Vec<Box<dyn RayIntersect>> = vec![
        floor, floor1, floor11, 
        floor2, floor22, floor222,
        floor3, floor33, floor333,
        floor4, floor44, floor444,
        floor5, floor55, floor555,
        floor6, floor66, floor666,
        floor7, floor77,
        floor8, floor88,
        floor9, floor99,
        floor10, floor1010,
        floorii, 
        floor12,
        arena1, arena111, 
        arena2, arena22,
        arena3, arena33,
        arena4,
        arena5,
        arena6, 
        arena7, 
        arena8, 
        arena9,
        arena10, 
        arena11,
        arena12,
        arena13,
        arena14,
        arena15, 
        arena16,
        arena17,
        arena18,
        arena19, arena20, arena21, arena222,
        arena23, arena24, arena25, arena26, arena27, arena28, arena29, arena30, arena31,
        agua1, agua2, agua3, agua4, agua5, agua6, agua7, agua8, agua9, agua10, agua11, 
        agua12, agua13, agua14, agua15, agua16, agua17, agua18, agua19, agua20, agua21, agua22, agua23, agua24,
        agua25, agua26, agua27, agua28, agua29, agua30, agua31, agua32, agua33, agua34, agua35, agua36, agua37, agua38];


    // Ciclo principal del renderizado
    let mut framebuffer_high = vec![0; width * height];
    let mut framebuffer_low = vec![0; (width / 2) * (height / 2)];

    let mut window = minifb::Window::new(
        "Raytraced Cube",
        width,
        height,
        minifb::WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut should_render = true;
    let mut camera_moved = false;

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        camera_moved = false;
    
        // Manejo de teclas de flecha para la órbita
        if window.is_key_down(minifb::Key::Left) {
            camera.orbit(0.05, 0.0);
            camera_moved = true;
        }
        if window.is_key_down(minifb::Key::Right) {
            camera.orbit(-0.05, 0.0);
            camera_moved = true;
        }
        if window.is_key_down(minifb::Key::Up) {
            camera.orbit(0.0, 0.05);
            camera_moved = true;
        }
        if window.is_key_down(minifb::Key::Down) {
            camera.orbit(0.0, -0.05);
            camera_moved = true;
        }
    
        // Manejo de teclas WASD para movimiento
        let mut forward = 0.0;
        let mut rightward = 0.0;
        let mut vertical = 0.0;
    
        // Movimiento hacia adelante y hacia atrás (W/S)
        if window.is_key_down(minifb::Key::W) {
            forward += 0.1;
        }
        if window.is_key_down(minifb::Key::S) {
            forward -= 0.1;
        }
    
        // Movimiento lateral (A/D)
        if window.is_key_down(minifb::Key::A) {
            rightward -= 0.1;
        }
        if window.is_key_down(minifb::Key::D) {
            rightward += 0.1;
        }
    
        // Movimiento vertical (Q/E o puedes usar otras teclas)
        if window.is_key_down(minifb::Key::Q) {
            vertical += 0.1;
        }
        if window.is_key_down(minifb::Key::E) {
            vertical -= 0.1;
        }
    
        // Aplicar movimiento de la cámara
        if forward != 0.0 || rightward != 0.0 {
            camera.move_camera(forward, rightward);
            camera_moved = true;
        }
    
        if vertical != 0.0 {
            camera.move_vertical(vertical);
            camera_moved = true;
        }

        if camera_moved {
            render(&mut framebuffer_low, width / 4, height / 4, &objects, &camera, &lights[..]);
            let scaled_framebuffer = upscale_framebuffer(
                &framebuffer_low,
                width / 4,
                height / 4,
                width,
                height,
            );
            window.update_with_buffer(&scaled_framebuffer, width, height).unwrap();
        } else if should_render {
            render(&mut framebuffer_high, width, height, &objects, &camera, &lights);
            window.update_with_buffer(&framebuffer_high, width, height).unwrap();
            should_render = true;
        } else {
            window.update();
        }
    }
}

// Función para escalar el framebuffer de baja resolución al tamaño completo
fn upscale_framebuffer(
    low_res_buffer: &[u32],
    low_width: usize,
    low_height: usize,
    high_width: usize,
    high_height: usize,
) -> Vec<u32> {
    let mut high_res_buffer = vec![0; high_width * high_height];

    for y in 0..high_height {
        let src_y = y * low_height / high_height;
        for x in 0..high_width {
            let src_x = x * low_width / high_width;
            let src_index = src_y * low_width + src_x;
            let dst_index = y * high_width + x;
            high_res_buffer[dst_index] = low_res_buffer[src_index];
        }
    }

    high_res_buffer
}
