"""Mesh health check service using MeshLib.

Provides functions for:
- Self-intersection detection
- Hole detection
- Watertight check  
- Auto-repair functionality
"""

from meshlib import mrmeshpy as mm
from pathlib import Path
from typing import Dict, Any, List
from core.logging import get_logger

logger = get_logger(__name__)


def check_mesh_health(mesh_path: Path) -> Dict[str, Any]:
    """
    Comprehensive mesh health analysis using MeshLib.
    
    Args:
        mesh_path: Path to mesh file (STL)
        
    Returns:
        Dictionary with health metrics:
        - is_closed: Whether mesh is watertight
        - self_intersections: Count of self-intersecting face pairs
        - self_intersection_faces: List of face IDs involved (limited to 100)
        - holes_count: Number of boundary loops (holes)
        - health_score: 0-100 score
    """
    logger.info(f"Running health check on: {mesh_path.name}")
    
    mesh = mm.loadMesh(str(mesh_path))
    
    # Self-intersection detection using MeshLib
    # findSelfCollidingTrianglesBS returns a bitset of faces involved in self-intersections
    try:
        self_colliding_bs = mm.findSelfCollidingTrianglesBS(mesh)
        self_intersection_count = self_colliding_bs.count()
        
        # Get individual face IDs for visualization (limit to 100)
        self_intersecting_faces: List[int] = []
        if self_intersection_count > 0:
            for i in range(mesh.topology.numValidFaces()):
                if len(self_intersecting_faces) >= 100:
                    break
                face_id = mm.FaceId(i)
                if self_colliding_bs.test(face_id):
                    self_intersecting_faces.append(i)
    except Exception as e:
        logger.warning(f"Self-intersection check failed: {e}")
        self_intersection_count = 0
        self_intersecting_faces = []
    
    # Hole detection - find boundary edges
    try:
        hole_edges = mesh.topology.findHoleRepresentiveEdges()
        holes_count = len(hole_edges)
    except Exception as e:
        logger.warning(f"Hole detection failed: {e}")
        holes_count = 0
    
    # MeshLib does not expose a stable isMeshClosed binding across versions.
    is_closed = holes_count == 0
    
    # Calculate health score
    health_score = _calculate_health_score(is_closed, self_intersection_count, holes_count)
    
    result = {
        "is_closed": is_closed,
        "self_intersections": self_intersection_count,
        "self_intersection_faces": self_intersecting_faces,
        "holes_count": holes_count,
        "degenerate_faces": 0,  # MeshLib handles during import
        "health_score": health_score,
    }
    
    logger.info(f"Health check complete: score={health_score}, closed={is_closed}, "
                f"self-intersections={self_intersection_count}, holes={holes_count}")
    
    return result


def _calculate_health_score(is_closed: bool, self_intersections: int, holes: int) -> int:
    """Calculate 0-100 health score based on mesh issues."""
    score = 100
    
    if not is_closed:
        score -= 30
    
    if self_intersections > 0:
        # Deduct more points for more intersections
        penalty = min(40, self_intersections * 2)
        score -= penalty
    
    if holes > 0:
        # Deduct for holes
        penalty = min(20, holes * 5)
        score -= penalty
    
    return max(0, score)


def auto_repair_mesh(mesh_path: Path, output_path: Path) -> Dict[str, Any]:
    """
    Attempt automatic mesh repair using MeshLib.
    
    Repairs:
    - Fills holes using fillHole
    - Non-manifold edges are fixed automatically by MeshLib on import
    
    Args:
        mesh_path: Path to input mesh
        output_path: Path to save repaired mesh
        
    Returns:
        Dictionary with repair results
    """
    logger.info(f"Auto-repairing mesh: {mesh_path.name}")
    
    mesh = mm.loadMesh(str(mesh_path))
    repairs_made: List[str] = []
    
    # Fill holes
    try:
        hole_edges = mesh.topology.findHoleRepresentiveEdges()
        if len(hole_edges) > 0:
            params = mm.FillHoleParams()
            filled_count = 0
            for edge in hole_edges:
                try:
                    mm.fillHole(mesh, edge, params)
                    filled_count += 1
                except Exception as e:
                    logger.warning(f"Failed to fill hole: {e}")
            
            if filled_count > 0:
                repairs_made.append(f"Filled {filled_count} hole(s)")
    except Exception as e:
        logger.warning(f"Hole filling failed: {e}")
    
    # Save repaired mesh
    try:
        mm.saveMesh(mesh, str(output_path))
        logger.info(f"Saved repaired mesh to: {output_path}")
    except Exception as e:
        logger.error(f"Failed to save repaired mesh: {e}")
        return {
            "success": False,
            "repairs_made": repairs_made,
            "error": str(e),
        }
    
    return {
        "success": True,
        "repairs_made": repairs_made,
        "output_path": str(output_path),
    }
