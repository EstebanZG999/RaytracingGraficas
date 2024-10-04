use nalgebra_glm::Vec3;

pub struct Camera {
    pub eye: Vec3,     // Posición de la cámara en el espacio
    pub center: Vec3,  // Punto en el espacio 3D que la cámara está observando
    pub up: Vec3,      // Vector "arriba"
}

impl Camera {

    // Método para mover la cámara en la dirección hacia adelante y lateralmente (WASD)
    pub fn move_camera(&mut self, forward: f32, rightward: f32) {
        // Dirección hacia adelante basada en la dirección actual de la cámara
        let forward_direction = (self.center - self.eye).normalize();
        // Vector "derecha" perpendicular a la dirección hacia adelante y "arriba"
        let right_direction = forward_direction.cross(&self.up).normalize();

        // Actualizamos la posición de la cámara en la dirección hacia adelante y hacia los lados
        self.eye += forward * forward_direction;
        self.center += forward * forward_direction;
        
        self.eye += rightward * right_direction;
        self.center += rightward * right_direction;
    }

    // Método para mover la cámara hacia arriba o abajo (movimiento vertical con W/S)
    pub fn move_vertical(&mut self, vertical: f32) {
        self.eye += vertical * self.up;
        self.center += vertical * self.up;
    }

    
    // Cambiar la base para transformar un vector usando los vectores right, up y forward
    pub fn basis_change(&self, vector: &Vec3) -> Vec3 {
        let forward = (self.center - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        let up = right.cross(&forward).normalize();

        let rotated = 
            vector.x * right +
            vector.y * up -
            vector.z * forward;

        rotated.normalize()
    }

    // Método para realizar la órbita de la cámara en base a los cambios en yaw y pitch
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        // Calcular el vector desde el center hacia el eye (vector del radio) y medir la distancia
        let radius_vector = self.eye - self.center;
        let radius = radius_vector.magnitude();

        // Calcular el yaw actual (rotación alrededor del eje Y)
        let current_yaw = radius_vector.z.atan2(radius_vector.x);

        // Calcular el pitch actual (rotación alrededor del eje X)
        let radius_xz = (radius_vector.x * radius_vector.x + radius_vector.z * radius_vector.z).sqrt();
        let current_pitch = (-radius_vector.y).atan2(radius_xz);

        // Aplicar las rotaciones delta
        let new_yaw = (current_yaw + delta_yaw) % (2.0 * std::f32::consts::PI);
        let new_pitch = (current_pitch + delta_pitch).clamp(-std::f32::consts::PI / 2.0 + 0.1, std::f32::consts::PI / 2.0 - 0.1);

        // Calcular la nueva posición de la cámara usando coordenadas esféricas
        let new_eye = self.center + Vec3::new(
            radius * new_yaw.cos() * new_pitch.cos(),
            -radius * new_pitch.sin(),
            radius * new_yaw.sin() * new_pitch.cos()
        );

        // Actualizar la posición de la cámara
        self.eye = new_eye;
    }
}
