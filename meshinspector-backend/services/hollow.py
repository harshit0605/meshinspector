"""Hollowing service with MeshLib for shell creation and adaptive weight targeting."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Any, Dict, Union

import numpy as np
from scipy.spatial import cKDTree
import trimesh

from core.logging import get_logger
from services.convert import normalize_mesh_to_mm
from utils.units import mm3_to_grams, MATERIAL_DENSITIES

logger = get_logger(__name__)

# Try to import MeshLib
try:
    import meshlib.mrmeshpy as mr
    MESHLIB_AVAILABLE = True
    logger.info("MeshLib available for hollowing operations")
except ImportError:
    logger.warning("MeshLib not available, using fallback hollowing")
    MESHLIB_AVAILABLE = False


def calculate_weight_from_file(
    file_path: Union[str, Path],
    material: str = "gold_18k"
) -> Dict[str, Any]:
    """
    Calculate weight from a mesh file.
    
    Args:
        file_path: Path to mesh file (STL, GLB, etc.)
        material: Material type for density lookup
        
    Returns:
        Dictionary with weight_g, volume_mm3, is_watertight
    """
    file_path = Path(file_path)
    mesh = trimesh.load(str(file_path), force='mesh')
    
    # Handle scene objects
    if isinstance(mesh, trimesh.Scene):
        meshes = [g for g in mesh.geometry.values() if isinstance(g, trimesh.Trimesh)]
        if meshes:
            mesh = trimesh.util.concatenate(meshes)
        else:
            raise ValueError("No valid mesh geometry found in file")
    
    # Get volume (absolute value, as sign depends on face winding)
    volume_mm3 = abs(float(mesh.volume))
    
    # Convert to weight
    weight_g = mm3_to_grams(volume_mm3, material)
    
    return {
        "weight_g": weight_g,
        "volume_mm3": volume_mm3,
        "is_watertight": bool(mesh.is_watertight)
    }


def hollow_mesh_meshlib(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    wall_thickness_mm: float
) -> Path:
    """
    Hollow a mesh using MeshLib's offset + boolean operations.
    
    Creates a shell by:
    1. Creating an inward offset surface
    2. Boolean difference between outer and inner
    
    Args:
        input_path: Path to input STL file
        output_path: Path for output hollowed STL
        wall_thickness_mm: Wall thickness in millimeters
        
    Returns:
        Path to the hollowed mesh
    """
    if not MESHLIB_AVAILABLE:
        raise RuntimeError("MeshLib is required for hollowing")
    
    input_path = Path(input_path)
    output_path = Path(output_path)
    
    logger.info(f"Hollowing mesh with {wall_thickness_mm}mm walls using MeshLib")
    
    # Load mesh with MeshLib
    mesh = mr.loadMesh(str(input_path))
    logger.debug(f"Loaded mesh from {input_path.name}")
    
    # Compute appropriate voxel size based on mesh size
    bbox = mesh.computeBoundingBox()
    diagonal = (bbox.max - bbox.min).length()
    
    # Voxel size: balance between accuracy and speed
    # Smaller = more accurate but slower
    voxel_size = max(diagonal * 5e-3, wall_thickness_mm / 4.0)
    logger.debug(f"Using voxel size: {voxel_size:.4f} (diagonal: {diagonal:.2f})")
    
    # Set up offset parameters for shell operation
    offset_params = mr.OffsetParameters()
    offset_params.voxelSize = voxel_size
    
    # Create inner surface (negative offset = inward)
    logger.info(f"Creating inner offset at -{wall_thickness_mm}mm...")
    try:
        inner_mesh = mr.offsetMesh(mesh, -wall_thickness_mm, offset_params)
    except Exception as e:
        logger.error(f"offsetMesh failed: {e}")
        raise RuntimeError(f"Failed to create inner surface: {e}")
    
    # Boolean difference: outer - inner = shell
    logger.info("Performing boolean difference (outer - inner)...")
    try:
        boolean_result = mr.boolean(mesh, inner_mesh, mr.BooleanOperation.DifferenceAB)
        
        # Extract mesh from BooleanResult
        # MeshLib's boolean returns a BooleanResult object with .mesh attribute
        if hasattr(boolean_result, 'mesh'):
            shell_mesh = boolean_result.mesh
        elif hasattr(boolean_result, 'resultMesh'):
            shell_mesh = boolean_result.resultMesh
        elif callable(getattr(boolean_result, 'mesh', None)):
            shell_mesh = boolean_result.mesh()
        else:
            # Try using the result directly if it's already a mesh
            shell_mesh = boolean_result
            
    except Exception as e:
        logger.error(f"Boolean operation failed: {e}")
        raise RuntimeError(f"Boolean difference failed: {e}")
    
    # Save result
    try:
        mr.saveMesh(shell_mesh, str(output_path))
        logger.info(f"Hollowing complete: saved to {output_path.name}")
    except Exception as e:
        logger.error(f"saveMesh failed: {e}")
        raise RuntimeError(f"Failed to save hollowed mesh: {e}")
    
    return output_path


def _safe_normalize(vectors: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(vectors, axis=1, keepdims=True)
    return vectors / np.clip(norms, 1e-8, None)


def _compute_inward_directions(mesh: trimesh.Trimesh) -> np.ndarray:
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    normals = _safe_normalize(np.asarray(mesh.vertex_normals, dtype=np.float64))
    center = vertices.mean(axis=0)
    toward_center = _safe_normalize(center - vertices)
    outward = np.where((np.einsum("ij,ij->i", normals, toward_center) >= 0.0)[:, None], -normals, normals)
    return -outward


def _build_hollow_scale_field(
    vertices: np.ndarray,
    region_payload: dict | None,
    protect_regions: list[str],
    base_thickness_mm: float,
) -> np.ndarray:
    scales = np.ones(len(vertices), dtype=np.float32)
    if not region_payload or not protect_regions:
        return scales

    min_hollow_mm = max(base_thickness_mm * 0.18, 0.08)
    min_scale = float(np.clip(min_hollow_mm / max(base_thickness_mm, 1e-6), 0.08, 0.45))
    protected_indices: list[np.ndarray] = []
    region_map = {region["region_id"]: region for region in region_payload.get("regions", [])}
    for region_id in protect_regions:
        region = region_map.get(region_id)
        if not region:
            continue
        indices = np.asarray(region.get("vertex_indices", []), dtype=np.int32)
        if indices.size:
            protected_indices.append(indices)

    if not protected_indices:
        return scales

    protected = np.unique(np.concatenate(protected_indices))
    protected_positions = vertices[protected]
    distances, _ = cKDTree(protected_positions).query(vertices, workers=-1)
    falloff_mm = max(base_thickness_mm * 3.5, 1.5)
    protection = np.exp(-0.5 * np.square(distances / falloff_mm))
    protection[distances > falloff_mm * 2.75] = 0.0
    scales = np.clip(1.0 - 0.92 * protection, min_scale, 1.0).astype(np.float32)
    scales[protected] = min_scale
    return scales


def _save_boolean_shell(outer_mesh_path: Path, inner_mesh_path: Path, output_path: Path) -> Path:
    if not MESHLIB_AVAILABLE:
        raise RuntimeError("MeshLib is required for protected hollowing")

    outer_mesh = mr.loadMesh(str(outer_mesh_path))
    inner_mesh = mr.loadMesh(str(inner_mesh_path))
    boolean_result = mr.boolean(outer_mesh, inner_mesh, mr.BooleanOperation.DifferenceAB)
    if hasattr(boolean_result, "mesh"):
        shell_mesh = boolean_result.mesh
    elif hasattr(boolean_result, "resultMesh"):
        shell_mesh = boolean_result.resultMesh
    elif callable(getattr(boolean_result, "mesh", None)):
        shell_mesh = boolean_result.mesh()
    else:
        shell_mesh = boolean_result
    mr.saveMesh(shell_mesh, str(output_path))
    return output_path


def _boolean_difference(target_mesh_path: Path, cutter_mesh_path: Path, output_path: Path) -> Path:
    if not MESHLIB_AVAILABLE:
        raise RuntimeError("MeshLib is required for boolean subtraction")
    target_mesh = mr.loadMesh(str(target_mesh_path))
    cutter_mesh = mr.loadMesh(str(cutter_mesh_path))
    boolean_result = mr.boolean(target_mesh, cutter_mesh, mr.BooleanOperation.DifferenceAB)
    if hasattr(boolean_result, "mesh"):
        result_mesh = boolean_result.mesh
    elif hasattr(boolean_result, "resultMesh"):
        result_mesh = boolean_result.resultMesh
    elif callable(getattr(boolean_result, "mesh", None)):
        result_mesh = boolean_result.mesh()
    else:
        result_mesh = boolean_result
    mr.saveMesh(result_mesh, str(output_path))
    return output_path


def _plan_drain_hole_cutters(
    mesh: trimesh.Trimesh,
    region_payload: dict | None,
    wall_thickness_mm: float,
    hole_diameter_mm: float = 0.8,
) -> trimesh.Trimesh:
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    center = vertices.mean(axis=0)
    axis = np.asarray((region_payload or {}).get("ring_axis", [0.0, 1.0, 0.0]), dtype=np.float64)
    axis = _safe_normalize(axis.reshape(1, 3))[0]

    region_map = {region["region_id"]: region for region in (region_payload or {}).get("regions", [])}
    inner_band = region_map.get("inner_band")
    if not inner_band or not inner_band.get("vertex_indices"):
        raise RuntimeError("Drain-hole planning requires inner_band region data")

    inner_indices = np.asarray(inner_band["vertex_indices"], dtype=np.int32)
    inner_vertices = vertices[inner_indices]
    centered = inner_vertices - center
    radial_vectors = centered - np.outer(centered @ axis, axis)
    radial_norms = np.linalg.norm(radial_vectors, axis=1)
    valid = radial_norms > 1e-6
    if not np.any(valid):
        raise RuntimeError("Unable to determine radial directions for drain holes")

    radial_dirs = radial_vectors[valid] / radial_norms[valid][:, None]
    radial_basis = radial_dirs.mean(axis=0)
    if np.linalg.norm(radial_basis) < 1e-6:
        radial_basis = radial_dirs[0]
    radial_basis = radial_basis / np.linalg.norm(radial_basis)
    opposite_basis = -radial_basis

    def pick_anchor(direction: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        scores = radial_dirs @ direction
        anchor_local = inner_vertices[valid][np.argmax(scores)]
        radial_direction = anchor_local - center
        radial_direction = radial_direction - axis * np.dot(radial_direction, axis)
        radial_direction = radial_direction / np.clip(np.linalg.norm(radial_direction), 1e-8, None)
        return anchor_local, radial_direction

    anchors = [pick_anchor(radial_basis), pick_anchor(opposite_basis)]
    bbox = mesh.bounds[1] - mesh.bounds[0]
    cutter_length = float(np.clip(np.max(bbox) * 0.18, max(wall_thickness_mm * 5.0, 3.0), 8.0))
    cutters: list[trimesh.Trimesh] = []
    for anchor, direction in anchors:
        center_point = anchor + direction * (wall_thickness_mm * 0.55)
        transform = trimesh.geometry.align_vectors([0.0, 0.0, 1.0], direction)
        if transform is None:
            continue
        transform = np.asarray(transform, dtype=np.float64)
        transform[:3, 3] = center_point
        cutter = trimesh.creation.cylinder(radius=hole_diameter_mm / 2.0, height=cutter_length, sections=32, transform=transform)
        cutters.append(cutter)

    if not cutters:
        raise RuntimeError("Drain-hole planning failed to create cutter geometry")
    return trimesh.util.concatenate(cutters)


def apply_drain_holes(
    shell_path: Union[str, Path],
    output_path: Union[str, Path],
    region_payload: dict | None,
    wall_thickness_mm: float,
    hole_diameter_mm: float = 0.8,
) -> Path:
    """Boolean subtract conservative drain holes from a hollow shell."""
    shell_path = Path(shell_path)
    output_path = Path(output_path)
    shell_mesh = normalize_mesh_to_mm(shell_path)
    cutters = _plan_drain_hole_cutters(shell_mesh, region_payload, wall_thickness_mm, hole_diameter_mm=hole_diameter_mm)

    scratch_dir = output_path.parent / f"{output_path.stem}_drains"
    scratch_dir.mkdir(parents=True, exist_ok=True)
    target_path = scratch_dir / "shell.ply"
    cutter_path = scratch_dir / "cutters.ply"
    shell_mesh.export(target_path, file_type="ply")
    cutters.export(cutter_path, file_type="ply")
    return _boolean_difference(target_path, cutter_path, output_path)


def protected_hollow_mesh(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    wall_thickness_mm: float,
    region_payload: dict | None,
    protect_regions: list[str],
) -> Path:
    """Create a conservative protected-detail shell by weighting the inner offset."""
    if not MESHLIB_AVAILABLE:
        raise RuntimeError("MeshLib is required for protected hollowing")

    input_path = Path(input_path)
    output_path = Path(output_path)
    mesh = normalize_mesh_to_mm(input_path)
    if not isinstance(mesh, trimesh.Trimesh):
        raise RuntimeError("Protected hollowing requires a mesh input")

    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    inward_dirs = _compute_inward_directions(mesh)
    hollow_scales = _build_hollow_scale_field(vertices, region_payload, protect_regions, wall_thickness_mm)
    displaced_vertices = vertices + inward_dirs * (wall_thickness_mm * hollow_scales)[:, None]

    inner_mesh = mesh.copy()
    inner_mesh.vertices = displaced_vertices
    trimesh.smoothing.filter_taubin(inner_mesh, lamb=0.1, nu=-0.11, iterations=4)

    scratch_dir = output_path.parent / f"{output_path.stem}_protected"
    scratch_dir.mkdir(parents=True, exist_ok=True)
    outer_mesh_path = scratch_dir / "outer.ply"
    inner_mesh_path = scratch_dir / "inner.ply"
    mesh.export(outer_mesh_path, file_type="ply")
    inner_mesh.export(inner_mesh_path, file_type="ply")
    return _save_boolean_shell(outer_mesh_path, inner_mesh_path, output_path)


def hollow_mesh_fallback(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    wall_thickness_mm: float
) -> Path:
    """
    Fallback hollowing using voxelization with scipy.
    
    Less accurate than MeshLib but works without dependencies.
    """
    from scipy import ndimage
    
    input_path = Path(input_path)
    output_path = Path(output_path)
    
    logger.info(f"Hollowing mesh with fallback method ({wall_thickness_mm}mm walls)")
    
    mesh = trimesh.load(str(input_path), force='mesh')
    
    # Handle scene objects
    if isinstance(mesh, trimesh.Scene):
        meshes = [g for g in mesh.geometry.values() if isinstance(g, trimesh.Trimesh)]
        if meshes:
            mesh = trimesh.util.concatenate(meshes)
        else:
            raise ValueError("No valid mesh geometry found")
    
    # Determine voxel size (smaller = more accurate but slower)
    voxel_pitch = max(wall_thickness_mm / 3.0, 0.2)
    
    # Voxelize the mesh
    voxels = mesh.voxelized(pitch=voxel_pitch)
    matrix = voxels.matrix.copy()
    
    # Erode to create inner surface
    erosion_iters = max(1, int(wall_thickness_mm / voxel_pitch))
    inner = ndimage.binary_erosion(matrix, iterations=erosion_iters)
    
    # Shell = outer - inner
    shell_matrix = matrix & ~inner
    
    # Convert back to mesh
    shell_voxels = trimesh.voxel.VoxelGrid(
        trimesh.voxel.encoding.DenseEncoding(shell_matrix),
        transform=voxels.transform
    )
    shell_mesh = shell_voxels.marching_cubes
    
    # Export
    shell_mesh.export(str(output_path))
    
    logger.info(f"Fallback hollowing complete: {output_path.name}")
    
    return output_path


def hollow_mesh(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    wall_thickness_mm: float
) -> Path:
    """
    Hollow a mesh using best available method.
    
    Prefers MeshLib if available, falls back to voxel method.
    """
    if MESHLIB_AVAILABLE:
        try:
            return hollow_mesh_meshlib(input_path, output_path, wall_thickness_mm)
        except Exception as e:
            logger.warning(f"MeshLib hollowing failed, trying fallback: {e}")
            return hollow_mesh_fallback(input_path, output_path, wall_thickness_mm)
    else:
        return hollow_mesh_fallback(input_path, output_path, wall_thickness_mm)


def adaptive_hollow_to_weight(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    target_weight_g: float,
    material: str = "gold_18k",
    tolerance_g: float = 0.1,
    min_thickness_mm: float = 0.5,
    max_thickness_mm: float = 3.0,
    max_iterations: int = 20
) -> Dict[str, Any]:
    """
    Automatically find wall thickness to achieve target weight.
    
    Uses binary search to efficiently converge on optimal wall thickness.
    
    Args:
        input_path: Path to input mesh file
        output_path: Path for output hollowed mesh
        target_weight_g: Target weight in grams
        material: Material for weight calculation
        tolerance_g: Acceptable weight difference (default 0.1g)
        min_thickness_mm: Minimum wall thickness (default 0.5mm)
        max_thickness_mm: Maximum wall thickness (default 3.0mm)
        max_iterations: Maximum binary search iterations (default 20)
        
    Returns:
        Dictionary with:
        - achieved_weight_g: Final weight achieved
        - wall_thickness_mm: Wall thickness used
        - iterations: Number of iterations taken
        - warning: Any warning message (e.g., target not achievable)
    """
    input_path = Path(input_path)
    output_path = Path(output_path)
    
    # Get original weight
    original = calculate_weight_from_file(input_path, material)
    original_weight = original["weight_g"]
    
    logger.info(f"Adaptive hollowing: original={original_weight:.2f}g, target={target_weight_g:.2f}g")
    
    # Validate target weight
    if target_weight_g >= original_weight:
        logger.warning(f"Target weight ({target_weight_g}g) >= original ({original_weight:.2f}g)")
        # Just copy the file as-is
        import shutil
        shutil.copy2(input_path, output_path)
        return {
            "achieved_weight_g": original_weight,
            "wall_thickness_mm": None,
            "iterations": 0,
            "warning": "Target weight is greater than or equal to original weight. No hollowing applied."
        }
    
    # Minimum achievable weight (with max hollowing)
    # We'll discover this during search
    
    min_t = min_thickness_mm
    max_t = max_thickness_mm
    best_result = None
    best_thickness = None
    best_weight = None
    
    temp_dir = Path(tempfile.gettempdir())
    
    for iteration in range(max_iterations):
        # Current guess: midpoint
        current_thickness = (min_t + max_t) / 2.0
        
        # Create temp output for this iteration
        temp_output = temp_dir / f"hollow_iter_{iteration}.stl"
        
        logger.info(f"Iteration {iteration + 1}: trying {current_thickness:.3f}mm")
        
        try:
            # Hollow with current thickness
            hollow_mesh(input_path, temp_output, current_thickness)
            
            # Calculate resulting weight
            result = calculate_weight_from_file(temp_output, material)
            current_weight = result["weight_g"]
            
            logger.info(f"  → Weight: {current_weight:.2f}g (target: {target_weight_g:.2f}g)")
            
            # Track best result
            if best_weight is None or abs(current_weight - target_weight_g) < abs(best_weight - target_weight_g):
                best_weight = current_weight
                best_thickness = current_thickness
                best_result = temp_output
            
            # Check if within tolerance
            if abs(current_weight - target_weight_g) < tolerance_g:
                logger.info(f"Target achieved! {current_weight:.2f}g with {current_thickness:.3f}mm walls")
                # Copy to final output
                import shutil
                shutil.copy2(temp_output, output_path)
                return {
                    "achieved_weight_g": round(current_weight, 3),
                    "wall_thickness_mm": round(current_thickness, 3),
                    "iterations": iteration + 1,
                    "warning": None
                }
            
            # Adjust search space
            if current_weight > target_weight_g:
                # Too heavy, need thinner walls (more hollow = less material)
                max_t = current_thickness
            else:
                # Too light, need thicker walls (less hollow = more material)
                min_t = current_thickness
            
            # Check if search space exhausted
            if max_t - min_t < 0.01:
                logger.warning(f"Search space exhausted, closest result: {current_weight:.2f}g")
                break
                
        except Exception as e:
            logger.error(f"Iteration {iteration + 1} failed: {e}")
            # Adjust search space to avoid failing thickness
            if current_thickness == min_t:
                min_t = current_thickness + 0.1
            else:
                max_t = current_thickness - 0.1
            continue
    
    # Max iterations or search exhausted - use best result
    if best_result and best_result.exists():
        import shutil
        shutil.copy2(best_result, output_path)
        
        warning = None
        if best_weight and abs(best_weight - target_weight_g) > tolerance_g:
            if best_weight > target_weight_g:
                warning = f"Target weight not achievable. Minimum achievable: {best_weight:.2f}g"
            else:
                warning = f"Close to target but outside tolerance. Achieved: {best_weight:.2f}g"
        
        return {
            "achieved_weight_g": round(best_weight, 3) if best_weight else None,
            "wall_thickness_mm": round(best_thickness, 3) if best_thickness else None,
            "iterations": max_iterations,
            "warning": warning or "Max iterations reached"
        }
    
    raise RuntimeError("Adaptive hollowing failed completely")


def adaptive_protected_hollow_to_weight(
    input_path: Union[str, Path],
    output_path: Union[str, Path],
    target_weight_g: float,
    region_payload: dict | None,
    protect_regions: list[str],
    material: str = "gold_18k",
    tolerance_g: float = 0.1,
    min_thickness_mm: float = 0.5,
    max_thickness_mm: float = 3.0,
    max_iterations: int = 20,
) -> Dict[str, Any]:
    """Binary-search a protected hollow thickness while keeping protected zones conservative."""
    input_path = Path(input_path)
    output_path = Path(output_path)

    original = calculate_weight_from_file(input_path, material)
    original_weight = original["weight_g"]
    if target_weight_g >= original_weight:
        import shutil

        shutil.copy2(input_path, output_path)
        return {
            "achieved_weight_g": original_weight,
            "wall_thickness_mm": None,
            "iterations": 0,
            "warning": "Target weight is greater than or equal to original weight. No hollowing applied.",
        }

    min_t = min_thickness_mm
    max_t = max_thickness_mm
    best_result: Path | None = None
    best_thickness: float | None = None
    best_weight: float | None = None
    temp_dir = Path(tempfile.gettempdir())

    for iteration in range(max_iterations):
        current_thickness = (min_t + max_t) / 2.0
        temp_output = temp_dir / f"protected_hollow_iter_{iteration}.stl"
        try:
            protected_hollow_mesh(
                input_path,
                temp_output,
                current_thickness,
                region_payload=region_payload,
                protect_regions=protect_regions,
            )
            result = calculate_weight_from_file(temp_output, material)
            current_weight = result["weight_g"]

            if best_weight is None or abs(current_weight - target_weight_g) < abs(best_weight - target_weight_g):
                best_weight = current_weight
                best_thickness = current_thickness
                best_result = temp_output

            if abs(current_weight - target_weight_g) < tolerance_g:
                import shutil

                shutil.copy2(temp_output, output_path)
                return {
                    "achieved_weight_g": round(current_weight, 3),
                    "wall_thickness_mm": round(current_thickness, 3),
                    "iterations": iteration + 1,
                    "warning": None,
                }

            if current_weight > target_weight_g:
                max_t = current_thickness
            else:
                min_t = current_thickness

            if max_t - min_t < 0.01:
                break
        except Exception as exc:
            logger.warning("Protected hollow iteration %s failed: %s", iteration + 1, exc)
            if current_thickness == min_t:
                min_t = current_thickness + 0.1
            else:
                max_t = current_thickness - 0.1

    if best_result and best_result.exists():
        import shutil

        shutil.copy2(best_result, output_path)
        warning = None
        if best_weight and abs(best_weight - target_weight_g) > tolerance_g:
            if best_weight > target_weight_g:
                warning = f"Target weight not achievable with protected hollowing. Minimum achieved: {best_weight:.2f}g"
            else:
                warning = f"Close to target but outside tolerance. Achieved: {best_weight:.2f}g"
        return {
            "achieved_weight_g": round(best_weight, 3) if best_weight else None,
            "wall_thickness_mm": round(best_thickness, 3) if best_thickness else None,
            "iterations": max_iterations,
            "warning": warning or "Max iterations reached",
        }

    raise RuntimeError("Adaptive protected hollowing failed completely")
