mod color;
mod material;
mod intersect;
mod sphere;

use nalgebra_glm::Vec3;
use crate::sphere::Sphere;
use crate::intersect::{RayIntersect, Intersect};

pub fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3, objects: &[Sphere]) -> color::Color {
    let mut intersect = Intersect::empty();
    let mut zbuffer = f32::INFINITY;  

    for object in objects {
        let tmp = object.ray_intersect(ray_origin, ray_direction);
        if tmp.is_intersecting && tmp.distance < zbuffer {  
            zbuffer = tmp.distance;  
            intersect = tmp;
        }
    }

    if !intersect.is_intersecting {
        return color::Color::new(4, 12, 36);
    }

    intersect.material.diffuse
}

pub fn render(framebuffer: &mut Vec<u32>, width: usize, height: usize, objects: &[Sphere]) {
    let aspect_ratio = width as f32 / height as f32;

    for y in 0..height {
        for x in 0..width {
            let screen_x = (2.0 * x as f32) / width as f32 - 1.0;
            let screen_y = -((2.0 * y as f32) / height as f32 - 1.0);
            let screen_x = screen_x * aspect_ratio;

            let ray_direction = nalgebra_glm::normalize(&Vec3::new(screen_x, screen_y, -1.0));

            let pixel_color = cast_ray(&Vec3::new(0.0, 0.0, 0.0), &ray_direction, objects);
            framebuffer[y * width + x] = ((pixel_color.r as u32) << 16)
                                        | ((pixel_color.g as u32) << 8)
                                        | (pixel_color.b as u32);
        }
    }
}

fn main() {
    let width = 800;
    let height = 800;
    let mut framebuffer = vec![0; width * height];

    let white_material = material::Material {
        diffuse: color::Color::new(255, 255, 255),
    };

    let brown_material = material::Material {
        diffuse: color::Color::new(150, 75, 0),
    };

    let black_material = material::Material {
        diffuse: color::Color::new(0, 0, 0),
    };

    let blue_material = material::Material {
        diffuse: color::Color::new(0, 0, 139),
    };

    let pink_material = material::Material {
        diffuse: color::Color::new(255, 182, 193),
    };

    let objects = [
        // Cabeza 
        Sphere {
            center: Vec3::new(0.0, 0.0, -12.0),
            radius: 3.5,
            material: white_material,
        },
        // Nariz
        Sphere {
            center: Vec3::new(0.0, 0.2, -8.0),
            radius: 0.5,
            material: brown_material,
        },
        // Ojo izquierdo 
        Sphere {
            center: Vec3::new(-1.2, 0.7, -8.0),
            radius: 0.6,
            material: black_material,
        },
        // Ojo derecho 
        Sphere {
            center: Vec3::new(1.2, 0.7, -8.0),
            radius: 0.6,
            material: black_material,
        },
        // Punto blanco en el ojo izquierdo 
        Sphere {
            center: Vec3::new(-1.3, 0.9, -7.5),
            radius: 0.2,
            material: white_material,
        },
        // Punto blanco en el ojo derecho 
        Sphere {
            center: Vec3::new(1.1, 0.9, -7.5),
            radius: 0.2,
            material: white_material,
        },
        // Oreja izquierda 
        Sphere {
            center: Vec3::new(-2.5, 2.5, -10.5),
            radius: 0.8,
            material: blue_material,
        },
        // Oreja derecha 
        Sphere {
            center: Vec3::new(2.5, 2.5, -10.5),
            radius: 0.8,
            material: blue_material,
        },
        // Puntos en la mejilla izquierda 
        Sphere {
            center: Vec3::new(-1.5, -0.5, -7.9),
            radius: 0.1,
            material: black_material,
        },
        Sphere {
            center: Vec3::new(-1.0, -0.8, -7.9),
            radius: 0.1,
            material: black_material,
        },
        Sphere {
            center: Vec3::new(-1.5, -1.0, -7.9),
            radius: 0.1,
            material: black_material,
        },
        // Puntos en la mejilla derecha 
        Sphere {
            center: Vec3::new(1.5, -0.5, -7.9),
            radius: 0.1,
            material: black_material,
        },
        Sphere {
            center: Vec3::new(1.0, -0.8, -7.9),
            radius: 0.1,
            material: black_material,
        },
        Sphere {
            center: Vec3::new(1.5, -1.0, -7.9),
            radius: 0.1,
            material: black_material,
        },
        // Boca: círculo negro y círculo rosado 
        Sphere {
            center: Vec3::new(0.0, -1.3, -8.0),
            radius: 0.5,
            material: black_material,
        },
        Sphere {
            center: Vec3::new(0.0, -1.5, -7.6),  
            radius: 0.2,
            material: pink_material,
        },
    ];

    render(&mut framebuffer, width, height, &objects);

    let mut window = minifb::Window::new(
        "Raytraced Oshawott",
        width,
        height,
        minifb::WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        window
            .update_with_buffer(&framebuffer, width, height)
            .unwrap();
    }
}
