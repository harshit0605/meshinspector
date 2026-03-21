"""Mesh analysis service: volume, weight, bounding box."""

import trimesh
import numpy as np
from pathlib import Path
from typing import Union, Dict, Any

from core.logging import get_logger
from utils.units import mm3_to_grams
from services.convert import load_mesh

logger = get_logger(__name__)


def analyze_mesh(
    file_path: Union[str, Path],
    material: str = "gold_18k"
) -> Dict[str, Any]:
    """
    Analyze mesh properties for manufacturability.
    
    Args:
        file_path: Path to mesh file (STL, GLB, etc.)
        material: Material type for weight calculation
        
    Returns:
        Dictionary with analysis results
    """
    file_path = Path(file_path)
    logger.info(f"Analyzing mesh: {file_path.name}")
    
    mesh = load_mesh(file_path)
    
    # Get bounding box to detect units
    bounds = mesh.bounds
    raw_bbox = bounds[1] - bounds[0]
    max_dim = float(max(raw_bbox))
    
    # Auto-detect and convert units to mm
    # glTF/GLB files are often in meters
    scale_factor = 1.0
    if max_dim < 0.1:
        # Model is likely in meters (a 50mm ring = 0.05m)
        scale_factor = 1000.0
        logger.info(f"Detected meters, scaling by {scale_factor}")
    elif max_dim < 10:
        # Model is likely in centimeters (a 50mm ring = 5cm)
        scale_factor = 10.0
        logger.info(f"Detected centimeters, scaling by {scale_factor}")
    elif max_dim > 10000:
        # Model might be in microns
        scale_factor = 0.001
        logger.info(f"Detected microns, scaling by {scale_factor}")
    
    # Scale mesh for accurate calculations
    if scale_factor != 1.0:
        mesh.apply_scale(scale_factor)
    
    # Volume in mm³ (now in proper units)
    # Use abs() as volume can be negative based on face winding
    volume_mm3 = abs(float(mesh.volume))
    
    # Weight in grams
    weight_g = mm3_to_grams(volume_mm3, material)
    
    # Bounding box dimensions in mm
    bounds = mesh.bounds  # Recalculate after scaling
    bbox_mm = tuple(float(x) for x in (bounds[1] - bounds[0]))
    
    # Watertight check (closed surface)
    is_watertight = bool(mesh.is_watertight)
    
    # Mesh statistics
    vertex_count = int(mesh.vertices.shape[0])
    face_count = int(mesh.faces.shape[0])
    
    result = {
        "volume_mm3": round(volume_mm3, 3),
        "weight_g": round(weight_g, 3),
        "bbox_mm": tuple(round(x, 2) for x in bbox_mm),
        "is_watertight": is_watertight,
        "vertex_count": vertex_count,
        "face_count": face_count,
    }
    
    logger.info(f"Analysis complete: {weight_g:.2f}g, bbox={bbox_mm}, watertight={is_watertight}")
    
    return result


def estimate_ring_diameter(mesh: trimesh.Trimesh) -> float:
    """
    Estimate the inner diameter of a ring mesh.
    
    Assumes ring is oriented with hole along Y axis.
    Uses bounding box as approximation.
    """
    bounds = mesh.bounds
    dims = bounds[1] - bounds[0]
    
    # Ring diameter is roughly the X or Z dimension (whichever is larger)
    # minus twice the band width
    diameter_estimate = max(dims[0], dims[2])
    
    return float(diameter_estimate)


def calculate_volume(mesh: trimesh.Trimesh) -> float:
    """Calculate mesh volume in mm³."""
    return abs(float(mesh.volume))
