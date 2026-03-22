"""Versioned geometry operations."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path

import meshlib.mrmeshpy as mm
import numpy as np
import trimesh
from scipy.spatial import cKDTree
from sqlalchemy.orm import Session

from core.logging import get_logger
from domain.models import JobRecord, ModelVersionRecord
from domain.schemas import CompareRequest, CompareResponse, HollowRequest, MakeManufacturableRequest, ResizeRequest, ScoopRequest, SmoothRequest, ThickenRequest
from services.health import auto_repair_mesh
from services.hollow import adaptive_hollow_to_weight, adaptive_protected_hollow_to_weight, apply_drain_holes, hollow_mesh, protected_hollow_mesh
from services.manufacturability import compute_manufacturability_snapshot
from services.measure_ring import measure_ring_from_path, normalize_axis
from services.resize import resize_ring
from services.versioning import materialize_artifact, register_file_artifact
from storage.repositories import (
    add_job_event,
    create_version,
    get_artifact_by_type,
    set_job_status,
    upsert_snapshot,
)

logger = get_logger(__name__)


def _load_normalized_artifact(db: Session, version_id: str, workdir: Path) -> Path:
    artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if artifact is None:
        raise FileNotFoundError(f"normalized_mesh_ply artifact missing for version {version_id}")
    return materialize_artifact(artifact, workdir)


def _load_region_payload(db: Session, version_id: str, workdir: Path) -> dict:
    artifact = get_artifact_by_type(db, version_id, "analysis_regions_json")
    if artifact is None:
        raise FileNotFoundError(f"analysis_regions_json artifact missing for version {version_id}")
    path = materialize_artifact(artifact, workdir)
    return json.loads(path.read_text(encoding="utf-8"))


def _load_thickness_values(db: Session, version_id: str, workdir: Path) -> np.ndarray:
    artifact = get_artifact_by_type(db, version_id, "analysis_thickness_npz")
    if artifact is None:
        raise FileNotFoundError(f"analysis_thickness_npz artifact missing for version {version_id}")
    path = materialize_artifact(artifact, workdir)
    payload = np.load(path)
    return payload["thickness"].astype(np.float32)


def _region_indices(payload: dict, region_id: str) -> np.ndarray:
    region = next((item for item in payload.get("regions", []) if item["region_id"] == region_id), None)
    if region is None:
        raise RuntimeError(f"Region {region_id} not found")
    indices = np.asarray(region.get("vertex_indices", []), dtype=np.int32)
    if indices.size == 0:
        raise RuntimeError(f"Region {region_id} has no vertices")
    return indices


def _region_indices_union(payload: dict, region_ids: list[str]) -> np.ndarray:
    unique_ids = []
    seen: set[str] = set()
    for region_id in region_ids:
        if region_id and region_id not in seen:
            seen.add(region_id)
            unique_ids.append(region_id)
    if not unique_ids:
        raise RuntimeError("No region ids were provided")
    chunks = [_region_indices(payload, region_id) for region_id in unique_ids]
    merged = np.unique(np.concatenate(chunks))
    if merged.size == 0:
        raise RuntimeError("Selected regions have no vertices")
    return merged


def _compute_outward_directions(vertices: np.ndarray, normals: np.ndarray) -> np.ndarray:
    center = vertices.mean(axis=0)
    toward_center = center - vertices
    toward_center = toward_center / np.clip(np.linalg.norm(toward_center, axis=1, keepdims=True), 1e-8, None)
    normalized_normals = normals / np.clip(np.linalg.norm(normals, axis=1, keepdims=True), 1e-8, None)
    return np.where((np.einsum("ij,ij->i", normalized_normals, toward_center) >= 0.0)[:, None], -normalized_normals, normalized_normals)


def _falloff_weights(vertices: np.ndarray, seed_indices: np.ndarray, falloff_mm: float) -> np.ndarray:
    seed_positions = vertices[seed_indices]
    distances, _ = cKDTree(seed_positions).query(vertices, workers=-1)
    weights = np.exp(-0.5 * np.square(distances / max(falloff_mm, 1e-3)))
    weights[distances > falloff_mm * 3.0] = 0.0
    return weights.astype(np.float32)


def _guard_protected_regions_for_hollow(db: Session, source_version: ModelVersionRecord, workdir: Path, request: HollowRequest) -> None:
    if request.add_drain_holes and "inner_band" in request.protect_regions:
        raise RuntimeError("Drain-hole planning requires inner_band to remain available for placement")


def _finalize_version(
    db: Session,
    version: ModelVersionRecord,
    job: JobRecord,
    normalized_mesh_path: Path,
    workdir: Path,
    *,
    complete_job: bool = True,
    completion_message: str = "Operation completed",
    progress_pct: int = 100,
) -> None:
    preview_high = workdir / f"{version.id}_high.glb"
    preview_low = workdir / f"{version.id}_low.glb"
    manufacturing_stl = workdir / f"{version.id}.stl"

    from services.convert import to_glb, to_stl

    to_glb(normalized_mesh_path, preview_high)
    to_glb(normalized_mesh_path, preview_low)
    to_stl(normalized_mesh_path, manufacturing_stl)

    register_file_artifact(db, version.id, normalized_mesh_path, "normalized_mesh_ply", "model/ply")
    register_file_artifact(db, version.id, preview_high, "preview_glb_high", "model/gltf-binary")
    register_file_artifact(db, version.id, preview_low, "preview_glb_low", "model/gltf-binary")
    register_file_artifact(db, version.id, manufacturing_stl, "manufacturing_stl", "application/sla")

    snapshot, artifacts = compute_manufacturability_snapshot(normalized_mesh_path, workdir)
    thickness_artifact = register_file_artifact(
        db,
        version.id,
        artifacts.thickness_scalar_path,
        "analysis_thickness_npz",
        "application/octet-stream",
    )
    register_file_artifact(
        db,
        version.id,
        artifacts.region_json_path,
        "analysis_regions_json",
        "application/json",
    )
    snapshot.version_id = version.id
    snapshot.thickness.scalar_field_artifact_id = thickness_artifact.id
    upsert_snapshot(db, version.id, "manufacturability", snapshot.model_dump(mode="json"))

    version.status = "ready"
    job.version_id = version.id
    if complete_job:
        set_job_status(db, job, "succeeded", progress_pct=progress_pct)
        add_job_event(db, job.id, completion_message, progress_pct)
    else:
        set_job_status(db, job, "running", progress_pct=progress_pct)
        add_job_event(db, job.id, completion_message, progress_pct)
    db.commit()


def run_repair_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    *,
    complete_job: bool = True,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, "Repair started", 5)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_repair.ply"
    temp_stl = workdir / f"{job.id}_repair.stl"

    from services.convert import to_stl, to_ply

    to_stl(source_mesh, temp_stl)
    result = auto_repair_mesh(temp_stl, temp_stl)
    if not result["success"]:
        raise RuntimeError(result.get("error", "repair failed"))
    to_ply(temp_stl, output_mesh)
    add_job_event(db, job.id, "Repair geometry rebuilt", 70)

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="repair",
        operation_label="Auto Repair",
        status="processing",
    )
    _finalize_version(
        db,
        new_version,
        job,
        output_mesh,
        workdir,
        complete_job=complete_job,
        completion_message="Repair completed" if complete_job else "Repair step completed",
        progress_pct=100 if complete_job else 25,
    )
    return new_version


def run_resize_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: ResizeRequest,
    *,
    complete_job: bool = True,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, "Resize started", 5)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    axis_override = tuple(request.manual_axis) if request.axis_mode == "manual" and request.manual_axis else None
    if request.axis_mode == "manual" and axis_override is None:
        raise RuntimeError("manual axis mode requires manual_axis")
    if axis_override is not None:
        normalize_axis(axis_override)
    measurement = measure_ring_from_path(source_mesh, axis_override=axis_override)
    current_size = measurement.estimated_ring_size_us or 7.0
    output_mesh = workdir / f"{job.id}_resize.ply"
    preserve_indices = None
    if request.preserve_head:
        region_payload = _load_region_payload(db, source_version.id, workdir)
        preserve_region_ids = [region_id for region_id in ("head", "ornament_relief") if any(region["region_id"] == region_id for region in region_payload.get("regions", []))]
        if preserve_region_ids:
            preserve_indices = _region_indices_union(region_payload, preserve_region_ids)
    resize_ring(
        source_mesh,
        output_mesh,
        current_size,
        request.target_ring_size_us,
        axis_override=axis_override or measurement.ring_axis,
        preserve_indices=preserve_indices,
    )
    add_job_event(db, job.id, "Ring resized", 70)

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="resize",
        operation_label=f"Resize to US {request.target_ring_size_us}",
        status="processing",
    )
    _finalize_version(
        db,
        new_version,
        job,
        output_mesh,
        workdir,
        complete_job=complete_job,
        completion_message="Resize completed" if complete_job else "Resize step completed",
        progress_pct=100 if complete_job else 55,
    )
    return new_version


def run_hollow_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: HollowRequest,
    *,
    complete_job: bool = True,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, "Hollowing started", 5)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    _guard_protected_regions_for_hollow(db, source_version, workdir, request)
    region_payload = _load_region_payload(db, source_version.id, workdir) if (request.protect_regions or request.add_drain_holes) else None
    temp_stl = workdir / f"{job.id}_input.stl"
    output_stl = workdir / f"{job.id}_hollow.stl"
    output_mesh = workdir / f"{job.id}_hollow.ply"

    from services.convert import to_stl, to_ply

    to_stl(source_mesh, temp_stl)
    if request.mode == "target_weight":
        if request.protect_regions:
            adaptive_protected_hollow_to_weight(
                temp_stl,
                output_stl,
                target_weight_g=request.target_weight_g or 0.5,
                region_payload=region_payload,
                protect_regions=request.protect_regions,
                material=request.material.value,
                tolerance_g=0.1,
                min_thickness_mm=request.min_allowed_thickness_mm,
                max_thickness_mm=3.0,
            )
        else:
            adaptive_hollow_to_weight(
                temp_stl,
                output_stl,
                target_weight_g=request.target_weight_g or 0.5,
                material=request.material.value,
                tolerance_g=0.1,
                min_thickness_mm=request.min_allowed_thickness_mm,
                max_thickness_mm=3.0,
            )
    else:
        if request.protect_regions:
            protected_hollow_mesh(
                temp_stl,
                output_stl,
                request.wall_thickness_mm or 0.8,
                region_payload=region_payload,
                protect_regions=request.protect_regions,
            )
        else:
            hollow_mesh(temp_stl, output_stl, request.wall_thickness_mm or 0.8)
    if request.add_drain_holes:
        drained_output = workdir / f"{job.id}_hollow_drained.stl"
        apply_drain_holes(
            output_stl,
            drained_output,
            region_payload=region_payload,
            wall_thickness_mm=request.wall_thickness_mm or request.min_allowed_thickness_mm,
        )
        output_stl = drained_output
    to_ply(output_stl, output_mesh)
    add_job_event(db, job.id, "Hollow mesh generated", 75)

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="hollow",
        operation_label="Hollow Mesh",
        status="processing",
    )
    _finalize_version(
        db,
        new_version,
        job,
        output_mesh,
        workdir,
        complete_job=complete_job,
        completion_message="Hollowing completed" if complete_job else "Weight optimization step completed",
        progress_pct=100 if complete_job else 85,
    )
    return new_version


def run_scoop_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: ScoopRequest,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, f"Scooping {request.region_id}", 5)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    region_payload = _load_region_payload(db, source_version.id, workdir)

    region_map = {region["region_id"]: region for region in region_payload.get("regions", [])}
    selected_region = region_map.get(request.region_id)
    if selected_region is None:
        raise RuntimeError(f"Region {request.region_id} not found")
    if "scoop" not in selected_region.get("allowed_operations", []):
        raise RuntimeError(f"Region {request.region_id} does not allow scooping")
    region_min_thickness = selected_region.get("min_thickness_mm")
    required_thickness = request.keep_min_thickness_mm + request.depth_mm
    if region_min_thickness is not None and region_min_thickness < required_thickness:
        raise RuntimeError(
            f"Region {selected_region.get('label', request.region_id)} is too thin for a "
            f"{request.depth_mm:.2f}mm scoop while keeping {request.keep_min_thickness_mm:.2f}mm minimum thickness "
            f"(region min {region_min_thickness:.2f}mm). Thicken the region first or reduce scoop depth."
        )

    selected_indices = _region_indices(region_payload, request.region_id)

    mesh = trimesh.load_mesh(source_mesh, process=False)
    if not isinstance(mesh, trimesh.Trimesh):
        raise RuntimeError("Unable to load mesh for scoop operation")

    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    normals = np.asarray(mesh.vertex_normals, dtype=np.float64)
    inward_dirs = -_compute_outward_directions(vertices, normals)
    weights = _falloff_weights(vertices, selected_indices, request.falloff_mm)

    displaced = vertices + inward_dirs * (request.depth_mm * weights[:, None])
    output_mesh = workdir / f"{job.id}_scoop.ply"
    mesh.vertices = displaced
    mesh.export(output_mesh, file_type="ply")
    add_job_event(db, job.id, "Scoop deformation applied", 65)

    preview_snapshot, _ = compute_manufacturability_snapshot(
        output_mesh,
        workdir / f"{job.id}_preview_analysis",
        threshold_mm=request.keep_min_thickness_mm,
    )
    if preview_snapshot.thickness.min_mm is None or preview_snapshot.thickness.min_mm < request.keep_min_thickness_mm:
        raise RuntimeError(
            f"Scoop would violate minimum thickness {request.keep_min_thickness_mm:.2f}mm "
            f"(predicted min {preview_snapshot.thickness.min_mm}). Reduce scoop depth or thicken the target region first."
        )

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="scoop",
        operation_label=f"Scoop {request.region_id}",
        status="processing",
    )
    _finalize_version(db, new_version, job, output_mesh, workdir)
    return new_version


def run_thicken_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: ThickenRequest,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, "Thickening started", 5)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_thicken.ply"

    if request.mode == "global":
        mesh = mm.loadMesh(str(source_mesh))
        params = mm.GeneralOffsetParameters()
        bbox = mesh.computeBoundingBox()
        diagonal = (bbox.max - bbox.min).length()
        params.voxelSize = max(diagonal * 0.0025, request.min_target_thickness_mm / 4.0)
        thickened = mm.generalOffsetMesh(mm.MeshPart(mesh), request.min_target_thickness_mm / 2.0, params)
        mm.saveMesh(thickened, str(output_mesh))
    else:
        region_payload = _load_region_payload(db, source_version.id, workdir)
        thickness = _load_thickness_values(db, source_version.id, workdir)
        mesh = trimesh.load_mesh(source_mesh, process=False)
        if not isinstance(mesh, trimesh.Trimesh):
            raise RuntimeError("Unable to load mesh for local thickening")
        vertices = np.asarray(mesh.vertices, dtype=np.float64)
        normals = np.asarray(mesh.vertex_normals, dtype=np.float64)
        outward_dirs = _compute_outward_directions(vertices, normals)

        if request.mode in {"selected_region", "selected_regions"}:
            target_region_ids = list(request.region_ids)
            if request.region_id and request.region_id not in target_region_ids:
                target_region_ids.append(request.region_id)
            if not target_region_ids:
                raise RuntimeError("selected region thickening requires region_id or region_ids")
            seed_indices = _region_indices_union(region_payload, target_region_ids)
            weights = _falloff_weights(vertices, seed_indices, 1.5)
        else:
            seed_indices = np.flatnonzero(np.isfinite(thickness) & (thickness < request.min_target_thickness_mm))
            if seed_indices.size == 0:
                raise RuntimeError("No violating regions found for local thickening")
            weights = _falloff_weights(vertices, seed_indices, 1.2)

        deficits = np.clip(request.min_target_thickness_mm - np.nan_to_num(thickness, nan=0.0), 0.0, request.min_target_thickness_mm)
        displaced = vertices + outward_dirs * ((deficits * weights)[:, None] * 0.75)
        mesh.vertices = displaced
        if request.smoothing_pass:
            trimesh.smoothing.filter_taubin(mesh, lamb=0.15, nu=-0.2, iterations=4)
        mesh.export(output_mesh, file_type="ply")
    add_job_event(db, job.id, "Mesh thickened", 75)

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="thicken",
        operation_label="Thicken Mesh",
        status="processing",
    )
    _finalize_version(db, new_version, job, output_mesh, workdir)
    return new_version


def run_compare_operation(
    db: Session,
    source_version: ModelVersionRecord,
    other_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
) -> CompareResponse:
    set_job_status(db, job, "running", progress_pct=10)
    add_job_event(db, job.id, "Compare started", 10)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    other_mesh = _load_normalized_artifact(db, other_version.id, workdir)

    a = mm.loadMesh(str(source_mesh))
    b = mm.loadMesh(str(other_mesh))
    scalars = mm.findSignedDistances(a, b)
    values = np.array([float(scalars.vec[i]) for i in range(scalars.size())], dtype=np.float32)
    finite = values[np.isfinite(values)]

    from services.convert import normalize_mesh_to_mm

    mesh_a = normalize_mesh_to_mm(source_mesh)
    mesh_b = normalize_mesh_to_mm(other_mesh)
    bbox_a = mesh_a.bounds[1] - mesh_a.bounds[0]
    bbox_b = mesh_b.bounds[1] - mesh_b.bounds[0]
    compare_npz = workdir / f"{job.id}_compare_{other_version.id}.npz"
    np.savez_compressed(compare_npz, values=np.nan_to_num(values, nan=0.0), other_version_id=np.array([other_version.id]))
    register_file_artifact(
        db,
        source_version.id,
        compare_npz,
        f"analysis_compare_npz_{other_version.id}",
        "application/octet-stream",
        metadata_json={"other_version_id": other_version.id, "job_id": job.id},
    )

    response = CompareResponse(
        version_id=source_version.id,
        other_version_id=other_version.id,
        volume_delta_mm3=round(abs(float(mesh_a.volume)) - abs(float(mesh_b.volume)), 4),
        weight_delta_g=0.0,
        bbox_delta_mm=(
            round(float(bbox_a[0] - bbox_b[0]), 4),
            round(float(bbox_a[1] - bbox_b[1]), 4),
            round(float(bbox_a[2] - bbox_b[2]), 4),
        ),
        min_signed_distance_mm=round(float(np.min(finite)), 4) if finite.size else None,
        max_signed_distance_mm=round(float(np.max(finite)), 4) if finite.size else None,
        mean_signed_distance_mm=round(float(np.mean(finite)), 4) if finite.size else None,
    )
    set_job_status(db, job, "succeeded", progress_pct=100)
    add_job_event(db, job.id, "Compare completed", 100)
    db.commit()
    return response


def run_smooth_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: SmoothRequest,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, "Smoothing started", 5)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_smooth.ply"

    smoothed = trimesh.load_mesh(source_mesh, process=False)
    if not isinstance(smoothed, trimesh.Trimesh):
        raise RuntimeError("Unable to load mesh for smoothing")
    iterations = max(1, request.iterations)
    lamb = min(max(request.strength, 0.01), 0.95)

    target_region_ids = list(request.region_ids)
    if request.region_id and request.region_id not in target_region_ids:
        target_region_ids.append(request.region_id)

    if target_region_ids:
        region_payload = _load_region_payload(db, source_version.id, workdir)
        seed_indices = _region_indices_union(region_payload, target_region_ids)
        vertices = np.asarray(smoothed.vertices, dtype=np.float64)
        weights = _falloff_weights(vertices, seed_indices, 1.8)
        neighbors = smoothed.vertex_neighbors
        for _ in range(iterations):
            updated = vertices.copy()
            active = np.flatnonzero(weights > 0.02)
            for index in active:
                neighbor_ids = neighbors[index]
                if not neighbor_ids:
                    continue
                neighbor_mean = vertices[np.asarray(neighbor_ids, dtype=np.int32)].mean(axis=0)
                updated[index] = vertices[index] + (neighbor_mean - vertices[index]) * lamb * float(weights[index])
            vertices = updated
        smoothed.vertices = vertices
    elif request.global_mode:
        trimesh.smoothing.filter_taubin(smoothed, lamb=lamb, nu=-0.53, iterations=iterations)
    else:
        raise RuntimeError("Smooth requires global_mode=true or an explicit region_id")
    smoothed.export(output_mesh, file_type="ply")
    add_job_event(db, job.id, "Mesh smoothing completed", 75)

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="smooth",
        operation_label=f"Smooth x{iterations}",
        status="processing",
    )
    _finalize_version(db, new_version, job, output_mesh, workdir)
    return new_version


def run_make_manufacturable_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: MakeManufacturableRequest,
) -> ModelVersionRecord:
    set_job_status(db, job, "running", progress_pct=5)
    add_job_event(db, job.id, "Manufacturability flow started", 5)
    current_version = run_repair_operation(db, source_version, job, workdir, complete_job=False)

    if request.target_ring_size_us:
        add_job_event(db, job.id, "Sizing branch for manufacturability", 35)
        resize_req = ResizeRequest(target_ring_size_us=request.target_ring_size_us)
        current_version = run_resize_operation(db, current_version, job, workdir, resize_req, complete_job=False)

    if request.target_weight_g:
        add_job_event(db, job.id, "Weight optimization branch for manufacturability", 65)
        hollow_req = HollowRequest(
            mode="target_weight",
            material=request.material,
            target_weight_g=request.target_weight_g,
            min_allowed_thickness_mm=request.min_allowed_thickness_mm,
        )
        current_version = run_hollow_operation(db, current_version, job, workdir, hollow_req, complete_job=False)

    set_job_status(db, job, "succeeded", progress_pct=100)
    add_job_event(db, job.id, "Manufacturability flow completed", 100)
    db.commit()
    return current_version
