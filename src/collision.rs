use arrayvec::ArrayVec;
use cgmath::prelude::*;

use crate::MAX_PHYSICS_ITERATIONS;

pub trait Collider {
    fn center(&self) -> cgmath::Vector2<f32>;
    fn furthest_point_in_direction(&self, direction: cgmath::Vector2<f32>) -> cgmath::Vector2<f32>;
}

pub struct Collision {
    pub normal: cgmath::Vector2<f32>,
    pub depth: f32,
}

pub fn get_collision<C: Collider + ?Sized>(s1: &C, s2: &C) -> Option<Collision> {
    gjk(s1, s2).and_then(|simplex| epa(simplex.into(), s1, s2))
}

fn support<C: Collider + ?Sized>(s1: &C, s2: &C, d: cgmath::Vector2<f32>) -> cgmath::Vector2<f32> {
    s1.furthest_point_in_direction(d) - s2.furthest_point_in_direction(-d)
}

fn gjk<C: Collider + ?Sized>(s1: &C, s2: &C) -> Option<[cgmath::Vector2<f32>; 3]> {
    fn handle_simplex(
        simplex: &mut ArrayVec<cgmath::Vector2<f32>, 3>,
        d: &mut cgmath::Vector2<f32>,
    ) -> bool {
        fn line_case(
            simplex: &mut ArrayVec<cgmath::Vector2<f32>, 3>,
            d: &mut cgmath::Vector2<f32>,
        ) -> bool {
            let &[b, a] = simplex.as_slice() else { unreachable!() };
            let ab = b - a;
            let ao = -a;
            let ab_perp = {
                let ab = cgmath::vec3(ab.x, ab.y, 0.0);
                let ao = cgmath::vec3(ao.x, ao.y, 0.0);
                ab.cross(ao).cross(ab).xy()
            };
            *d = ab_perp;
            false
        }

        fn triangle_case(
            simplex: &mut ArrayVec<cgmath::Vector2<f32>, 3>,
            d: &mut cgmath::Vector2<f32>,
        ) -> bool {
            let &[c, b, a] = simplex.as_slice() else { unreachable!() };
            let ab = b - a;
            let ac = c - a;
            let ao = -a;
            let ab_perp = {
                let ac = cgmath::vec3(ac.x, ac.y, 0.0);
                let ab = cgmath::vec3(ab.x, ab.y, 0.0);
                ac.cross(ab).cross(ab).xy()
            };
            let ac_perp = {
                let ab = cgmath::vec3(ab.x, ab.y, 0.0);
                let ac = cgmath::vec3(ac.x, ac.y, 0.0);
                ab.cross(ac).cross(ac).xy()
            };
            if cgmath::dot(ab_perp, ao) > 0.0 {
                simplex.remove(0);
                *d = ab_perp;
                false
            } else if cgmath::dot(ac_perp, ao) > 0.0 {
                simplex.remove(1);
                *d = ac_perp;
                false
            } else {
                true
            }
        }

        match simplex.len() {
            2 => line_case(simplex, d),
            3 => triangle_case(simplex, d),
            _ => unreachable!(),
        }
    }

    let mut d = (s2.center() - s1.center()).normalize();
    let mut simplex = ArrayVec::new();
    simplex.push(support(s1, s2, d));
    d = -simplex[0];
    loop {
        let a = support(s1, s2, d);
        if cgmath::dot(a, d) < 0.0 {
            return None;
        }
        simplex.push(a);
        if handle_simplex(&mut simplex, &mut d) {
            return Some(simplex.into_inner().unwrap());
        }
    }
}

fn epa<C: Collider + ?Sized>(
    mut polytype: Vec<cgmath::Vector2<f32>>,
    s1: &C,
    s2: &C,
) -> Option<Collision> {
    let mut min_index = 0;
    let mut min_distance = f32::INFINITY;
    let mut min_normal = cgmath::vec2(0.0, 0.0);

    let mut iterations = 0;
    while min_distance == f32::INFINITY {
        if iterations > MAX_PHYSICS_ITERATIONS {
            println!("Warning: reached maximum physics iterations when finding collision info, assuming there is no collision");
            return None;
        }

        for (i, &vertex_i) in polytype.iter().enumerate() {
            let j = (i + 1) % polytype.len();
            let vertex_j = polytype[j];

            let ij = vertex_j - vertex_i;

            let mut normal = cgmath::vec2(ij.y, -ij.x).normalize();
            let mut distance = normal.dot(vertex_i);

            if distance < 0.0 {
                distance *= -1.0;
                normal *= -1.0;
            }

            if distance < min_distance {
                min_distance = distance;
                min_normal = normal;
                min_index = j;
            }
        }

        let support = support(s1, s2, min_normal);
        let s_distance = min_normal.dot(support);

        if (s_distance - min_distance).abs() > 0.001 {
            min_distance = f32::INFINITY;
            polytype.insert(min_index, support);
        }

        iterations += 1;
    }

    Some(Collision {
        normal: min_normal,
        depth: min_distance + 0.001,
    })
}
