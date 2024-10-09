mod color;
mod material;
mod intersect;
mod camera;
mod light;
mod texture;
mod cube;

use std::time::Instant;

use material::Material;
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
    lights: &[Light],
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

    // Inicializar el color final
    let mut final_color = color::Color::new(0, 0, 0);

    // Iterar sobre cada luz
    for light in lights {
        // Si la luz es ambiental, sumamos su contribución y continuamos
        if light.intensity <= 0.3 {
            final_color = color::Color {
                r: (final_color.r as f32 + diffuse_color.r as f32 * light.intensity).min(255.0) as u8,
                g: (final_color.g as f32 + diffuse_color.g as f32 * light.intensity).min(255.0) as u8,
                b: (final_color.b as f32 + diffuse_color.b as f32 * light.intensity).min(255.0) as u8,
            };
            continue;
        }

        // Calcular la dirección de la luz y la intensidad difusa usando la ley de Lambert
        let light_dir = (light.position - closest_intersection.point).normalize();
        let diffuse_intensity = closest_intersection.normal.dot(&light_dir).max(0.0);

        // Calcular la intensidad de la sombra
        let shadow_intensity = cast_shadow(&closest_intersection, light, objects);
        let light_intensity = light.intensity * (1.0 - shadow_intensity);

        // Componente difusa
        let diffuse = color::Color {
            r: (diffuse_color.r as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * light_intensity).min(255.0) as u8,
            g: (diffuse_color.g as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * light_intensity).min(255.0) as u8,
            b: (diffuse_color.b as f32 * closest_intersection.material.albedo[0] * diffuse_intensity * light_intensity).min(255.0) as u8,
        };

        // Componente especular usando el modelo de Phong
        let view_dir = (ray_origin - closest_intersection.point).normalize();
        let reflect_dir = reflect(&-light_dir, &closest_intersection.normal).normalize();
        let specular_intensity = view_dir
            .dot(&reflect_dir)
            .max(0.0)
            .powf(closest_intersection.material.specular);
        let specular = color::Color {
            r: (light.color.r as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
            g: (light.color.g as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
            b: (light.color.b as f32 * closest_intersection.material.albedo[1] * specular_intensity * light_intensity).min(255.0) as u8,
        };

        // Sumar las contribuciones de esta luz al color final
        final_color = color::Color {
            r: (final_color.r as u32 + diffuse.r as u32 + specular.r as u32).min(255) as u8,
            g: (final_color.g as u32 + diffuse.g as u32 + specular.g as u32).min(255) as u8,
            b: (final_color.b as u32 + diffuse.b as u32 + specular.b as u32).min(255) as u8,
        };
    }

    // Componente de reflexión
    let reflectivity = closest_intersection.material.albedo[2];
    let mut reflect_color = color::Color::new(0, 0, 0);
    if reflectivity > 0.0 {
        let reflect_origin = closest_intersection.point + closest_intersection.normal * 1e-3;
        let reflect_dir = reflect(&-ray_direction, &closest_intersection.normal).normalize();
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
        let refract_origin = closest_intersection.point - closest_intersection.normal * 1e-3;  // Evitar acné de sombras
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
    
                // Llamar a cast_ray una sola vez por píxel
                let pixel_color = cast_ray(&camera.eye, &transformed_direction, objects, lights, 0);
    
                *pixel = ((pixel_color.r as u32) << 16)
                    | ((pixel_color.g as u32) << 8)
                    | (pixel_color.b as u32);
            });
        }
    });    
}



fn create_cube(
    center: Vec3,
    size: f32,
    materials: [Material; 6],
    is_water: bool,
) -> Box<Cube> {
    Box::new(Cube {
        center,
        original_center: center, // Inicializamos original_center con center
        size,
        materials,
        is_water,
    })
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
    let hoja_texture = load_texture("textures/hoja2.jpeg");
    let cactus_texture = load_texture("textures/cactus.jpeg");

    // Inicializar la cámara
    let eye = Vec3::new(8.0, 12.0, -25.0);
    let center = Vec3::new(0.0, 0.0, -1.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let mut camera = Camera { eye, center, up };

    // Inicializar las luces
    let lights = vec![
        // Luz ambiental tenue
        Light::new(
            Vec3::new(0.0, 0.0, 0.0),          // La posición es irrelevante para la luz ambiental
            color::Color::new(255, 255, 255),  // Color blanco
            0.2,                               // Intensidad baja
        ),
        // Luz fuerte detrás del árbol
        Light::new(
            Vec3::new(0.0, 8.0, 8.0),         // Posición detrás y arriba del árbol
            color::Color::new(255, 255, 255),  // Color blanco
            1.5,                               // Intensidad alta
        ),
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
        specular: 5.0,
        albedo: [0.9, 0.1, 0.0, 0.0],
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

    let madera = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 10.0,
        albedo: [0.6, 0.3, 0.0, 0.0],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(madera_texture),
    };

    let hoja = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 20.0,
        albedo: [0.7, 0.2, 0.0, 0.1],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(hoja_texture),
    };

    let cactus = material::Material {
        diffuse: color::Color::new(255, 255, 255),
        specular: 15.0,
        albedo: [0.6, 0.2, 0.0, 0.0],
        refractive_index: 1.5,
        has_texture: true,
        texture: Some(cactus_texture),
    };

    // Crear un cubo con materiales para cada cara
// Lista completa de posiciones para los cubos de tierra y grama
let floor_positions = vec![
    // Cubos en posición (x, y, z)
    // Bloque 1
    (0.0, 0.0, 0.0),
    (0.0, 2.0, 0.0),
    (0.0, 4.0, 0.0),

    // Bloque 2
    (2.0, 0.0, 0.0),
    (2.0, 2.0, 0.0),
    (2.0, 4.0, 0.0),

    // Bloque 3
    (0.0, 0.0, -2.0),
    (0.0, 2.0, -2.0),
    (0.0, 4.0, -2.0),

    // Bloque 4
    (2.0, 0.0, -2.0),
    (2.0, 2.0, -2.0),
    (2.0, 4.0, -2.0),

    // Bloque 5
    (0.0, 0.0, -4.0),
    (0.0, 2.0, -4.0),
    (0.0, 4.0, -4.0),

    // Bloque 6
    (2.0, 0.0, -4.0),
    (2.0, 2.0, -4.0),
    (2.0, 4.0, -4.0),

    // Bloque 7
    (4.0, 0.0, 0.0),
    (4.0, 2.0, 0.0),
    (4.0, 4.0, 0.0),

    // Bloque 8
    (4.0, 0.0, -2.0),
    (4.0, 2.0, -2.0),
    (4.0, 4.0, -2.0),

    // Bloque 9
    (2.0, 0.0, -4.0),
    (2.0, 2.0, -4.0),
    (2.0, 4.0, -4.0),

    // Bloque 10
    (0.0, 0.0, -6.0),
    (0.0, 2.0, -6.0),
    (0.0, 4.0, -6.0),

    // Bloque 11
    (2.0, 0.0, -6.0),
    (2.0, 2.0, -6.0),
    (2.0, 4.0, -6.0),

    // Bloque 12
    (6.0, 0.0, 0.0),
    (6.0, 2.0, 0.0),
    (6.0, 4.0, 0.0),

    // Bloque 13
    (4.0, 0.0, -4.0),
    (4.0, 2.0, -4.0),
    (4.0, 4.0, -4.0),

    // Bloque 14
    (2.0, 0.0, -4.0),
    (2.0, 2.0, -4.0),
    (2.0, 4.0, -4.0),

    // Bloque 15
    (0.0, 0.0, -8.0),
    (0.0, 2.0, -8.0),

    // Bloque 16
    (8.0, 0.0, 0.0),
];

// Crear los cubos de tierra y grama
let mut floor_cubes: Vec<Box<dyn RayIntersect>> = Vec::new();

for (x, y, z) in floor_positions {
    let materials = if y == 4.0 || y == 2.0 && z == -8.0 || (x == 8.0 && y == 0.0) {
        // Si está en la capa superior o en posiciones específicas, usa grama en la parte superior
        [
            tierra_material.clone(),  // Derecha (X+)
            tierra_material.clone(),  // Izquierda (X-)
            grama_material.clone(),   // Arriba (Y+)
            tierra_material4.clone(), // Abajo (Y-)
            tierra_material.clone(),  // Frente (Z+)
            tierra_material.clone(),  // Atrás (Z-)
        ]
    } else {
        // De lo contrario, usa tierra_material4 en todas las caras
        [
            tierra_material4.clone(),
            tierra_material4.clone(),
            tierra_material4.clone(),
            tierra_material4.clone(),
            tierra_material4.clone(),
            tierra_material4.clone(),
        ]
    };

    let cube = create_cube(
        Vec3::new(x, y, z),
        2.0,
        materials,
        false, // No es agua
    );
    floor_cubes.push(cube);
}


let arena_positions = vec![
    // arena1 y arena111
    (6.0, 0.0, -2.0),
    (6.0, 2.0, -2.0),
    
    // arena2 y arena22
    (2.0, 0.0, -6.0),
    (2.0, 2.0, -6.0),
    
    // arena3 y arena33
    (4.0, 0.0, -4.0),
    (4.0, 2.0, -4.0),
    
    // arena4
    (0.0, 0.0, -10.0),
    
    // arena5
    (10.0, 0.0, 0.0),
    
    // arena6
    (8.0, 0.0, -2.0),
    
    // arena7
    (2.0, 0.0, -8.0),
    
    // arena8
    (4.0, 0.0, -6.0),
    
    // arena9
    (6.0, 0.0, -4.0),
    
    // arena10
    (12.0, 0.0, 0.0),
    
    // arena11
    (0.0, 0.0, -12.0),
    
    // arena12
    (4.0, 0.0, -8.0),
    
    // arena13
    (8.0, 0.0, -4.0),
    
    // arena14
    (0.0, 0.0, -14.0),
    
    // arena15
    (14.0, 0.0, 0.0),
    
    // arena16 a arena22
    (0.0, 0.0, -16.0),
    (2.0, 0.0, -16.0),
    (4.0, 0.0, -16.0),
    (6.0, 0.0, -16.0),
    (8.0, 0.0, -16.0),
    (10.0, 0.0, -16.0),
    (12.0, 0.0, -16.0),
    (14.0, 0.0, -16.0),
    
    // arena23 a arena31
    (16.0, 0.0, 0.0),
    (16.0, 0.0, -2.0),
    (16.0, 0.0, -4.0),
    (16.0, 0.0, -6.0),
    (16.0, 0.0, -8.0),
    (16.0, 0.0, -10.0),
    (16.0, 0.0, -12.0),
    (16.0, 0.0, -14.0),
    (16.0, 0.0, -16.0),
];

let mut arena_cubes: Vec<Box<dyn RayIntersect>> = Vec::new();

for (x, y, z) in arena_positions {
    let materials = [
        arena.clone(), // Derecha (X+)
        arena.clone(), // Izquierda (X-)
        arena.clone(), // Arriba (Y+)
        arena.clone(), // Abajo (Y-)
        arena.clone(), // Frente (Z+)
        arena.clone(), // Atrás (Z-)
    ];

    let cube = create_cube(
        Vec3::new(x, y, z),
        2.0,
        materials,
        false, 
    );

    arena_cubes.push(cube);
}


let agua_positions = vec![
    (6.0, 0.0, -6.0),
    (8.0, 0.0, -6.0),
    (6.0, 0.0, -8.0),
    (8.0, 0.0, -8.0),
    (10.0, 0.0, -6.0),
    (8.0, 0.0, -10.0),
    (6.0, 0.0, -10.0),
    (10.0, 0.0, -8.0),
    (2.0, 0.0, -10.0),
    (4.0, 0.0, -10.0),
    (10.0, 0.0, -4.0),
    (10.0, 0.0, -2.0),
    (10.0, 0.0, -10.0),
    (2.0, 0.0, -12.0),
    (4.0, 0.0, -12.0),
    (6.0, 0.0, -12.0),
    (8.0, 0.0, -12.0),
    (10.0, 0.0, -12.0),
    (12.0, 0.0, -2.0),
    (12.0, 0.0, -4.0),
    (12.0, 0.0, -6.0),
    (12.0, 0.0, -8.0),
    (12.0, 0.0, -10.0),
    (12.0, 0.0, -12.0),
    (14.0, 0.0, -2.0),
    (14.0, 0.0, -4.0),
    (14.0, 0.0, -6.0),
    (14.0, 0.0, -8.0),
    (14.0, 0.0, -10.0),
    (14.0, 0.0, -12.0),
    (14.0, 0.0, -14.0),
    (2.0, 0.0, -14.0),
    (4.0, 0.0, -14.0),
    (6.0, 0.0, -14.0),
    (8.0, 0.0, -14.0),
    (10.0, 0.0, -14.0),
    (12.0, 0.0, -14.0),
];

// Crear los cubos de agua
let mut agua_cubes: Vec<Box<dyn RayIntersect>> = Vec::new();

for (x, y, z) in agua_positions {
    let materials = [
        agua.clone(), // Derecha (X+)
        agua.clone(), // Izquierda (X-)
        agua.clone(), // Arriba (Y+)
        agua.clone(), // Abajo (Y-)
        agua.clone(), // Frente (Z+)
        agua.clone(), // Atrás (Z-)
    ];

    let cube = create_cube(
        Vec3::new(x, y, z),
        2.0,
        materials,
        true, // Es agua
    );

    agua_cubes.push(cube);
}


let madera_positions = vec![
    (0.0, 6.0, 0.0),
    (0.0, 8.0, 0.0),
    (0.0, 10.0, 0.0),
    (0.0, 12.0, 0.0),
];

// Crear los cubos de madera
let mut madera_cubes: Vec<Box<dyn RayIntersect>> = Vec::new();

for (x, y, z) in madera_positions {
    let materials = [
        madera.clone(), // Derecha (X+)
        madera.clone(), // Izquierda (X-)
        madera.clone(), // Arriba (Y+)
        madera.clone(), // Abajo (Y-)
        madera.clone(), // Frente (Z+)
        madera.clone(), // Atrás (Z-)
    ];

    let cube = create_cube(
        Vec3::new(x, y, z),
        2.0,
        materials,
        false, // No es agua
    );

    madera_cubes.push(cube);
}


let hoja_positions = vec![
    // Nivel en y = 14.0
    (-4.0, 14.0, -4.0),
    (-2.0, 14.0, -4.0),
    (0.0, 14.0, -4.0),
    (2.0, 14.0, -4.0),
    (4.0, 14.0, -4.0),
    (-4.0, 14.0, -2.0),
    (-2.0, 14.0, -2.0),
    (0.0, 14.0, -2.0),
    (2.0, 14.0, -2.0),
    (4.0, 14.0, -2.0),
    (-4.0, 14.0, 0.0),
    (-2.0, 14.0, 0.0),
    (0.0, 14.0, 0.0), // Centro
    (2.0, 14.0, 0.0),
    (4.0, 14.0, 0.0),
    (-4.0, 14.0, 2.0),
    (-2.0, 14.0, 2.0),
    (0.0, 14.0, 2.0),
    (2.0, 14.0, 2.0),
    (4.0, 14.0, 2.0),
    (-4.0, 14.0, 4.0),
    (-2.0, 14.0, 4.0),
    (0.0, 14.0, 4.0),
    (2.0, 14.0, 4.0),
    (4.0, 14.0, 4.0),

    // Nivel en y = 16.0
    (-2.0, 16.0, -2.0),
    (0.0, 16.0, -2.0),
    (2.0, 16.0, -2.0),
    (-2.0, 16.0, 0.0),
    (0.0, 16.0, 0.0), // Centro en Y=16
    (2.0, 16.0, 0.0),
    (-2.0, 16.0, 2.0),
    (0.0, 16.0, 2.0),
    (2.0, 16.0, 2.0),

    // Nivel en y = 18.0
    (0.0, 18.0, 0.0),
];

// Crear los cubos de hojas
let mut hoja_cubes: Vec<Box<dyn RayIntersect>> = Vec::new();

for (x, y, z) in hoja_positions {
    let materials = [
        hoja.clone(), // Derecha (X+)
        hoja.clone(), // Izquierda (X-)
        hoja.clone(), // Arriba (Y+)
        hoja.clone(), // Abajo (Y-)
        hoja.clone(), // Frente (Z+)
        hoja.clone(), // Atrás (Z-)
    ];

    let cube = create_cube(
        Vec3::new(x, y, z),
        2.0,
        materials,
        false, // No es agua
    );

    hoja_cubes.push(cube);
}


let cactus_positions = vec![
    (16.0, 2.0, -16.0),
    (16.0, 4.0, -16.0),
    (16.0, 6.0, -16.0),
];

// Crear los cubos de cactus
let mut cactus_cubes: Vec<Box<dyn RayIntersect>> = Vec::new();

for (x, y, z) in cactus_positions {
    let materials = [
        cactus.clone(), // Derecha (X+)
        cactus.clone(), // Izquierda (X-)
        cactus.clone(), // Arriba (Y+)
        cactus.clone(), // Abajo (Y-)
        cactus.clone(), // Frente (Z+)
        cactus.clone(), // Atrás (Z-)
    ];

    let cube = create_cube(
        Vec3::new(x, y, z),
        2.0,
        materials,
        false, // No es agua
    );

    cactus_cubes.push(cube);
}
    

    let mut objects: Vec<Box<dyn RayIntersect>> = Vec::new();
    objects.extend(floor_cubes);
    objects.extend(arena_cubes);
    objects.extend(agua_cubes);
    objects.extend(madera_cubes);
    objects.extend(hoja_cubes);
    objects.extend(cactus_cubes);


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
let mut camera_moved;
let mut last_frame_time = Instant::now();
let mut scene_changed = false;
// Variables para animación (asegúrate de declararlas en un ámbito persistente)
let mut time = 0.0f32;
let amplitude = 0.5f32;
let frequency = 1.0f32;

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

    // Calcular delta_time
    let now = Instant::now();
    let delta_time = now.duration_since(last_frame_time).as_secs_f32();
    last_frame_time = now;

    // Incrementar el tiempo total de animación
    time += delta_time;

    // Animar los cubos de agua
    for object in objects.iter_mut() {
        if let Some(cube) = object.as_any_mut().downcast_mut::<Cube>() {
            // Verificar si el cubo es de agua
            if cube.is_water {
                // Animar el cubo de agua, por ejemplo, moverlo en el eje Y
                cube.center.y = cube.original_center.y + amplitude * (frequency * time).sin();
                scene_changed = true;  // La escena ha cambiado
            }
        }
    }

    if camera_moved || scene_changed {
        // Renderizar en baja resolución para una actualización rápida
        render(&mut framebuffer_low, width / 2, height / 2, &objects, &camera, &lights[..]);
        let scaled_framebuffer = upscale_framebuffer(
            &framebuffer_low,
            width / 2,
            height / 2,
            width,
            height,
        );
        window.update_with_buffer(&scaled_framebuffer, width, height).unwrap();
        should_render = true;  // Marcar para renderizar en alta resolución en el próximo ciclo
        scene_changed = false; // Restablecer la bandera
    } else if should_render {
        // Renderizar en alta resolución
        render(&mut framebuffer_high, width, height, &objects, &camera, &lights);
        window.update_with_buffer(&framebuffer_high, width, height).unwrap();
        should_render = false;  // Establecer a false después de renderizar
    } else {
        window.update();
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

}