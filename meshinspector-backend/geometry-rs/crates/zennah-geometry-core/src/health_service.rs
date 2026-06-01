use crate::{mesh_health, self_intersecting_faces, GeometryError, ServiceMeshHealth};

pub fn service_mesh_health(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    max_listed_faces: usize,
    epsilon: f64,
) -> Result<ServiceMeshHealth, GeometryError> {
    let topology_health = mesh_health(vertices, faces, false, None, epsilon)?;
    let mut self_intersection_faces = self_intersecting_faces(vertices, faces, epsilon)?;
    self_intersection_faces.sort_unstable();
    let self_intersections = self_intersection_faces.len();
    self_intersection_faces.truncate(max_listed_faces);

    Ok(ServiceMeshHealth {
        is_closed: topology_health.holes_count == 0,
        self_intersections,
        self_intersection_faces,
        holes_count: topology_health.holes_count,
        degenerate_faces: 0,
        health_score: calculate_service_health_score(
            topology_health.holes_count == 0,
            self_intersections,
            topology_health.holes_count,
        ),
    })
}

fn calculate_service_health_score(
    is_closed: bool,
    self_intersections: usize,
    holes: usize,
) -> usize {
    let mut score = 100_usize;
    if !is_closed {
        score = score.saturating_sub(30);
    }
    score = score.saturating_sub((self_intersections * 2).min(40));
    score = score.saturating_sub((holes * 5).min(20));
    score
}
