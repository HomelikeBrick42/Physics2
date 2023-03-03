use cgmath::prelude::*;
use serde::{Serialize, Deserialize};

use crate::Collider;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Quad {
    pub position: cgmath::Vector2<f32>,
    pub velocity: cgmath::Vector2<f32>,
    pub rotation: f32,
    pub angular_velocity: f32,
    pub scale: cgmath::Vector2<f32>,
    pub color: cgmath::Vector3<f32>,
    pub dynamic: bool,
}

impl Collider for Quad {
    fn center(&self) -> cgmath::Vector2<f32> {
        self.position
    }

    fn furthest_point_in_direction(&self, direction: cgmath::Vector2<f32>) -> cgmath::Vector2<f32> {
        let points = [
            cgmath::vec2(-self.scale.x * 0.5, -self.scale.y * 0.5),
            cgmath::vec2(-self.scale.x * 0.5, self.scale.y * 0.5),
            cgmath::vec2(self.scale.x * 0.5, -self.scale.y * 0.5),
            cgmath::vec2(self.scale.x * 0.5, self.scale.y * 0.5),
        ]
        .map(|point| {
            // Rotate the points
            cgmath::vec2(
                point.x * (-self.rotation).cos() - point.y * (-self.rotation).sin(),
                point.y * (-self.rotation).cos() + point.x * (-self.rotation).sin(),
            )
        })
        .map(|point| {
            // Translate the points
            point + self.position
        });

        let mut current_point = points[0];
        let mut max_dot = points[0].dot(direction);
        for &point in &points[1..] {
            let dot = point.dot(direction);
            if dot > max_dot {
                current_point = point;
                max_dot = dot;
            }
        }
        current_point
    }
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            position: cgmath::vec2(0.0, 0.0),
            velocity: cgmath::vec2(0.0, 0.0),
            rotation: 0.0,
            angular_velocity: 0.0,
            scale: cgmath::vec2(1.0, 1.0),
            color: cgmath::vec3(1.0, 1.0, 1.0),
            dynamic: true,
        }
    }
}
