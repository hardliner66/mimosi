use macroquad::math::Vec2;

use crate::maze::Wall;

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec2,
    pub direction: Vec2,
}

impl Ray {
    fn intersect(&self, wall: &Wall) -> Option<Vec2> {
        let edges = [
            (wall.p1, wall.p2),
            (wall.p2, wall.p3),
            (wall.p3, wall.p4),
            (wall.p4, wall.p1),
        ];

        let mut found = None;

        for (p1, p2) in edges {
            let wall_dir = p2 - p1;
            let perp_wall_dir = wall_dir.perp();

            let ray_to_wall_start = p1 - self.origin;

            let denom = self.direction.dot(perp_wall_dir);

            if denom.abs() < f32::EPSILON {
                continue;
            }

            let t1 = ray_to_wall_start.dot(perp_wall_dir) / denom;
            let t2 = ray_to_wall_start.dot(self.direction.perp()) / denom;

            if t1 >= 0.0 && (0.0..=1.0).contains(&t2) {
                found = Some(Vec2 {
                    x: self.origin.x + t1 * self.direction.x,
                    y: self.origin.y + t1 * self.direction.y,
                });
            }
        }
        found
    }

    pub fn find_nearest_intersection(&self, walls: &[Wall]) -> Option<(Vec2, f32)> {
        let mut nearest_intersection: Option<Vec2> = None;
        let mut nearest_distance = f32::MAX;

        for wall in walls {
            if let Some(intersection) = self.intersect(wall) {
                let distance = (intersection.x - self.origin.x).powi(2)
                    + (intersection.y - self.origin.y).powi(2);

                if distance < nearest_distance {
                    nearest_distance = distance;
                    nearest_intersection = Some(intersection);
                }
            }
        }

        nearest_intersection.map(|i| (i, nearest_distance))
    }
}
