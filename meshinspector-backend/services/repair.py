"""Mesh repair service."""

import trimesh
from pathlib import Path
from typing import Union, Tuple, List

from core.logging import get_logger
from services.convert import load_mesh

logger = get_logger(__name__)


def repair_mesh(
    input_path: Union[str, Path],
    output_path: Union[str, Path]
) -> Tuple[Path, List[str]]:
    """
    Repair mesh for manufacturing readiness.
    
    Performs:
    - Fix face normals
    - Fill small holes
    - Remove degenerate faces
    - Merge close vertices
    
    Args:
        input_path: Path to input mesh file
        output_path: Path for repaired mesh
        
    Returns:
        Tuple of (output path, list of repair actions taken)
    """
    input_path = Path(input_path)
    output_path = Path(output_path)
    
    logger.info(f"Repairing mesh: {input_path.name}")
    
    mesh = load_mesh(input_path)
    repair_log = []
    
    initial_faces = len(mesh.faces)
    initial_vertices = len(mesh.vertices)
    
    # Fix normals to be consistent
    mesh.fix_normals()
    repair_log.append("Fixed face normals")
    
    # Fill holes (small ones only)
    if not mesh.is_watertight:
        holes_before = len(mesh.outline())
        mesh.fill_holes()
        holes_after = len(mesh.outline()) if hasattr(mesh, 'outline') else 0
        if holes_before != holes_after:
            repair_log.append(f"Filled {holes_before - holes_after} holes")
    
    # Remove degenerate faces
    mesh.remove_degenerate_faces()
    degenerate_removed = initial_faces - len(mesh.faces)
    if degenerate_removed > 0:
        repair_log.append(f"Removed {degenerate_removed} degenerate faces")
    
    # Merge duplicate/close vertices
    mesh.merge_vertices()
    vertices_merged = initial_vertices - len(mesh.vertices)
    if vertices_merged > 0:
        repair_log.append(f"Merged {vertices_merged} duplicate vertices")
    
    # Export repaired mesh
    mesh.export(str(output_path))
    
    logger.info(f"Repair complete: {len(repair_log)} actions")
    
    return output_path, repair_log


def validate_mesh(mesh: trimesh.Trimesh) -> Tuple[bool, List[str]]:
    """
    Validate mesh for manufacturing.
    
    Returns:
        Tuple of (is_valid, list of issues)
    """
    issues = []
    
    if not mesh.is_watertight:
        issues.append("Mesh is not watertight")
    
    if not mesh.is_winding_consistent:
        issues.append("Inconsistent face winding")
    
    # Check for degenerate geometry
    if len(mesh.faces) == 0:
        issues.append("No faces in mesh")
    
    if mesh.volume <= 0:
        issues.append("Non-positive volume")
    
    return len(issues) == 0, issues
