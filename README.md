# Escena Raytrazada al Estilo Minecraft

Un simple raytrazador escrito en Rust que renderiza una escena al estilo Minecraft, con cubos texturizados, iluminación, sombras y animaciones básicas.

## Video Demo

[Check out a demo of the render on YouTube:](https://youtu.be/HAaGBVeIfWE)

## Características

- **Renderizado Raytrazado**: Renderizado realista de una escena 3D utilizando técnicas de ray tracing.
- **Materiales Texturizados**: Varios materiales con texturas únicas, incluyendo grama, tierra, arena, agua, madera, hojas y cactus.
- **Controles de Cámara**: Muévete por la escena usando controles de teclado (WASD para movimiento, flechas para rotación, Q/E para movimiento vertical).
- **Múltiples Fuentes de Luz**: La escena está iluminada por múltiples luces con diferentes posiciones e intensidades.
- **Animación Básica del Agua**: Los cubos de agua se animan verticalmente para simular movimiento.
- **Sombras Dinámicas**: Los objetos proyectan sombras basadas en las fuentes de luz.

## Instalación

1. **Clona el repositorio**:

   ```bash
   git clone https://github.com/tuusuario/tuproyecto.git

2. **Navega al directorio del proyecto**:

cd tuproyecto

3. **Instala las dependencias**:

Asegúrate de tener Rust instalado. Si no, instálalo desde rust-lang.org.

Luego, construye el proyecto para instalar los crates requeridos:
cargo build

4. **Ejecuta el proyecto**:
cargo run

## Controles

- **Movimiento de Cámara**:
  - `W`: Mover hacia adelante
  - `S`: Mover hacia atrás
  - `A`: Mover a la izquierda
  - `D`: Mover a la derecha
  - `Q`: Mover hacia arriba
  - `E`: Mover hacia abajo
- **Rotación de Cámara**:
  - `Flecha Izquierda`: Rotar a la izquierda
  - `Flecha Derecha`: Rotar a la derecha
  - `Flecha Arriba`: Rotar hacia arriba
  - `Flecha Abajo`: Rotar hacia abajo
- **Salir**:
  - `Esc`: Salir de la aplicación
 
## Dependencias

- **Rust**: Lenguaje de programación utilizado para el desarrollo.
- **Crates de Rust**:
  - `nalgebra-glm`: Para matemáticas de vectores y matrices.
  - `image`: Para cargar texturas.
  - `minifb`: Para gestión de ventana y framebuffer.
  - `rayon`: Para procesamiento en paralelo.
 
## Estructura de Archivos

- `main.rs`: Punto de entrada principal de la aplicación.
- `mod color;`: Módulo que maneja representaciones de color.
- `mod material;`: Módulo para definiciones y propiedades de materiales.
- `mod intersect;`: Módulo para lógica de intersección de rayos.
- `mod camera;`: Módulo que maneja el movimiento y orientación de la cámara.
- `mod light;`: Módulo que define propiedades de la luz.
- `mod texture;`: Módulo para carga y mapeo de texturas.
- `mod cube;`: Módulo que define geometría de cubos e intersecciones.
