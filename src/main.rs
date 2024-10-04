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
    let tierra_texture = load_texture("textures/tierra2.png");
    let tierra4_texture = load_texture("textures/tierra4.jpeg");

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

    let empty_material = material::Material {
        diffuse: color::Color::new(0, 0, 0),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: false,
        texture: None,
    };

    // Crear un cubo con materiales para cada cara
    let cube = Box::new(Cube {
        center: Vec3::new(0.0, -0.0, 0.0),  // Posición del cubo en el espacio
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
    let second_cube = Box::new(Cube {
        center: Vec3::new(2.0, 0.0, 0.0),  // Posicionamos el cubo 2 unidades a la derecha del primero
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
    // Crear una lista de objetos con el cubo
    let objects: Vec<Box<dyn RayIntersect>> = vec![cube, second_cube];

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
