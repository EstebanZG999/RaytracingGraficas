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

    for pixel in img.pixels() {
        let rgba = pixel.2;
        data.push(Color::new(rgba[0], rgba[1], rgba[2]));
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
    light: &Light,
    depth: u32,
) -> color::Color {
    if depth > 3 {
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

    // Calcular la dirección de la luz
    let light_dir = (light.position - closest_intersection.point).normalize();
    let view_dir = (ray_origin - closest_intersection.point).normalize();
    let reflect_dir = reflect(&-ray_direction, &closest_intersection.normal).normalize();

    // Calcular la intensidad de la sombra
    let shadow_intensity = cast_shadow(&closest_intersection, light, objects);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    // Componente difuso usando la ley del coseno de Lambert
    let diffuse_intensity = closest_intersection
        .normal
        .dot(&light_dir)
        .max(0.0)
        .min(1.0);
    let diffuse = color::Color {
        r: (diffuse_color.r as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * light_intensity).min(255.0) as u8,
        g: (diffuse_color.g as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * light_intensity).min(255.0) as u8,
        b: (diffuse_color.b as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * light_intensity).min(255.0) as u8,
    };

    // Componente especular usando el modelo de Phong
    let specular_intensity = view_dir
        .dot(&reflect_dir)
        .max(0.0)
        .powf(closest_intersection.material.specular);
    let specular = color::Color {
        r: (light.color.r as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
        g: (light.color.g as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
        b: (light.color.b as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
    };

    // Componente de reflexión
    let mut reflect_color = color::Color::new(0, 0, 0);
    let reflectivity = closest_intersection.material.albedo[2];
    if reflectivity > 0.0 {
        let reflect_origin = closest_intersection.point + closest_intersection.normal * 1e-3;
        reflect_color = cast_ray(&reflect_origin, &reflect_dir, objects, light, depth + 1);

        reflect_color = color::Color {
            r: (reflect_color.r as f32 * reflectivity).min(255.0) as u8,
            g: (reflect_color.g as f32 * reflectivity).min(255.0) as u8,
            b: (reflect_color.b as f32 * reflectivity).min(255.0) as u8,
        };
    }

    // Componente de refracción
    let mut refract_color = color::Color::new(0, 0, 0);
    let transparency = closest_intersection.material.albedo[3];
    if transparency > 0.0 {
        let refract_dir = refract(&ray_direction, &closest_intersection.normal, closest_intersection.material.refractive_index).normalize();
        let refract_origin = closest_intersection.point + closest_intersection.normal * 1e-3;  // Evitar acné de sombras
        refract_color = cast_ray(&refract_origin, &refract_dir, objects, light, depth + 1);

        refract_color = color::Color {
            r: (refract_color.r as f32 * transparency).min(255.0) as u8,
            g: (refract_color.g as f32 * transparency).min(255.0) as u8,
            b: (refract_color.b as f32 * transparency).min(255.0) as u8,
        };
    }

    // Combinar los componentes difuso, especular, reflejado y refractado
    color::Color {
        r: ((diffuse.r as f32 * (1.0 - reflectivity - transparency)) + (reflect_color.r as f32 * reflectivity) + (refract_color.r as f32 * transparency)).min(255.0) as u8,
        g: ((diffuse.g as f32 * (1.0 - reflectivity - transparency)) + (reflect_color.g as f32 * reflectivity) + (refract_color.g as f32 * transparency)).min(255.0) as u8,
        b: ((diffuse.b as f32 * (1.0 - reflectivity - transparency)) + (reflect_color.b as f32 * reflectivity) + (refract_color.b as f32 * transparency)).min(255.0) as u8,
    }
}




pub fn render(
    framebuffer: &mut [u32], 
    width: usize, 
    height: usize, 
    objects: &[Box<dyn RayIntersect>], 
    camera: &Camera, 
    light: &Light
) {
    // Reemplazamos `par_chunks_mut` con un bucle tradicional sobre las filas
    for (y, row) in framebuffer.chunks_mut(width).enumerate() {
        let screen_y = -((2.0 * y as f32) / height as f32 - 1.0);

        // Iteramos secuencialmente sobre los píxeles de la fila
        for (x, pixel) in row.iter_mut().enumerate() {
            let screen_x = (2.0 * x as f32) / width as f32 - 1.0;
            let screen_x = screen_x * (width as f32 / height as f32);

            let ray_direction = nalgebra_glm::normalize(&Vec3::new(screen_x, screen_y, -1.0));
            let transformed_direction = camera.basis_change(&ray_direction);
            let pixel_color = cast_ray(&camera.eye, &transformed_direction, objects, light, 0);

            *pixel = ((pixel_color.r as u32) << 16)
                | ((pixel_color.g as u32) << 8)
                | (pixel_color.b as u32);
        }
    }
}






fn main() {
    let width = 600;
    let height = 600;


    // Cargar las texturas desde archivos PNG
    let grama_texture = load_texture("textures/grama.png");
    let tierra_texture = load_texture("textures/tierra2.png");

    // Inicializar la cámara
    let eye = Vec3::new(0.0, 0.0, 5.0);
    let center = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let mut camera = Camera { eye, center, up };

    // Inicializar la luz
    let light = Light::new(
        Vec3::new(5.0, 5.0, 5.0),
        color::Color::new(255, 255, 255),
        1.0,
    );

    // Definir los materiales 
    let tierra_material = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 50.0,
        albedo: [0.6, 0.3, 0.1, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(tierra_texture),
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
    let cube = Cube::new(
        Vec3::new(0.0, 0.0, -1.0), // Centro del cubo
        2.0, // Tamaño del cubo
        [   // Materiales para las 6 caras del cubo
            tierra_material.clone(), tierra_material.clone(), // Front, Back
            tierra_material.clone(), tierra_material.clone(), // Left, Right
            grama_material.clone(), empty_material.clone(),   // Top, Bottom
        ],
    );

    // Crear una lista de objetos con el cubo
    let objects: Vec<Box<dyn RayIntersect>> = vec![Box::new(cube)];

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

        if camera_moved {
            render(&mut framebuffer_low, width / 2, height / 2, &objects, &camera, &light);
            let scaled_framebuffer = upscale_framebuffer(
                &framebuffer_low,
                width / 2,
                height / 2,
                width,
                height,
            );
            window.update_with_buffer(&scaled_framebuffer, width, height).unwrap();
        } else if should_render {
            render(&mut framebuffer_high, width, height, &objects, &camera, &light);
            window.update_with_buffer(&framebuffer_high, width, height).unwrap();
            should_render = false;
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
