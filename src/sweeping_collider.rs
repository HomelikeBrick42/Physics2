use cgmath::prelude::*;

use crate::Collider;

#[derive(Debug, Clone, Copy)]
pub struct SweepingCollider<'a, C: Collider + ?Sized> {
    pub collider: &'a C,
    pub position_a: cgmath::Vector2<f32>,
    pub position_b: cgmath::Vector2<f32>,
}

impl<'a, C: Collider + ?Sized> Collider for SweepingCollider<'a, C> {
    fn center(&self) -> cgmath::Vector2<f32> {
        self.position_a.lerp(self.position_b, 0.5)
    }

    fn furthest_point_in_direction(&self, direction: cgmath::Vector2<f32>) -> cgmath::Vector2<f32> {
        let point_a = (self.collider.furthest_point_in_direction(direction)
            - self.collider.center())
            + self.position_a;
        let point_b = (self.collider.furthest_point_in_direction(direction)
            - self.collider.center())
            + self.position_b;

        let distance_a = point_a.dot(direction);
        let distance_b = point_b.dot(direction);
        if distance_a > distance_b {
            point_a
        } else {
            point_b
        }
    }
}
