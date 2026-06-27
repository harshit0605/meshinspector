"""Versioned geometry operations."""

from __future__ import annotations

import json
from pathlib import Path
from collections.abc import Callable

import numpy as np
from sqlalchemy.orm import Session

from core.config import settings
from domain.models import JobRecord, ModelVersionRecord
from domain.schemas import (
    BrushReplayRequest,
    BrushReplayStroke,
    CompareRequest,
    CompareResponse,
    DecimateRequest,
    HollowRequest,
    InteractiveCommitRequest,
    MakeDeloneRequest,
    MakeManufacturableRequest,
    RepairRequest,
    ResizeRequest,
    ScoopRequest,
    SmoothRequest,
    SubdivideRequest,
    ThickenRequest,
)
from geometry_sdk import BrushStroke, MeshDocument, RegionEntry, default_sdk
from geometry_sdk.jewelry.ring_measurement import ring_diameter_for_size
from services.manufacturability import compute_manufacturability_snapshot
from services.sdk_conversion import to_glb, to_ply, to_stl
from services.versioning import materialize_artifact, register_file_artifact
from storage.repositories import (
    add_job_event,
    create_version,
    get_artifact_by_type,
    set_job_status,
    upsert_snapshot,
)

INTERACTIVE_TOOL_TO_OPERATION_TYPE = {
    "thicken_brush": "thicken",
    "scoop_brush": "scoop",
    "smooth_brush": "smooth",
    "meshlib_workbench_export": "interactive_commit",
}

BRUSH_TOOL_TO_OPERATION = {
    "thicken_brush": "thicken",
    "scoop_brush": "scoop",
    "smooth_brush": "smooth",
}

DEFAULT_HOLLOW_PROTECT_REGION_IDS = ("head", "gem_seat", "ornament_relief", "inner_band")
WORKBENCH_DECIMATE_SOURCES = {
    "meshlib_canvas_plugin_button",
    "meshlib_canvas_plugin_overlay",
    "meshlib_workbench",
    "meshlib_workbench_plugin",
}
DECIMATE_UNBOUNDED_DELETE_LIMIT = 2_147_483_647


def _bounded_seed_indices(mesh: MeshDocument, indices: np.ndarray, max_count: int) -> np.ndarray:
    return default_sdk.bounded_seed_indices(mesh, indices, max_count)


def _snapshot_thickness_was_deferred(snapshot) -> bool:
    return any(
        "Thickness analysis deferred" in str(recommendation)
        for recommendation in getattr(snapshot, "recommendations", []) or []
    )


def _load_normalized_artifact(db: Session, version_id: str, workdir: Path) -> Path:
    artifact = get_artifact_by_type(db, version_id, "normalized_mesh_ply")
    if artifact is None:
        raise FileNotFoundError(f"normalized_mesh_ply artifact missing for version {version_id}")
    return materialize_artifact(artifact, workdir)


def _has_interactive_decimate_delete_cap(request: DecimateRequest) -> bool:
    interactive_face_limit = settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES
    max_deleted_faces = request.max_deleted_faces
    max_deleted_vertices = request.max_deleted_vertices
    has_face_cap = max_deleted_faces < DECIMATE_UNBOUNDED_DELETE_LIMIT
    has_vertex_cap = max_deleted_vertices < DECIMATE_UNBOUNDED_DELETE_LIMIT
    if interactive_face_limit <= 0:
        return has_face_cap or has_vertex_cap
    return (has_face_cap and max_deleted_faces <= interactive_face_limit) or (
        has_vertex_cap and max_deleted_vertices * 2 <= interactive_face_limit
    )


def _mesh_surface_area_mm2(mesh: MeshDocument) -> float:
    vertices = np.asarray(mesh.vertices, dtype=float)
    faces = np.asarray(mesh.faces, dtype=np.int64)
    if faces.size == 0:
        return 0.0
    tris = vertices[faces]
    return float(0.5 * np.linalg.norm(np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0]), axis=1).sum())


def _validate_hollow_output_quality(source: MeshDocument, output: MeshDocument, wall_thickness_mm: float) -> None:
    # Acceptance policy (thresholds + geometry facts) is computed by the Rust
    # mesh_quality kernel; the service only joins the clauses and rejects.
    failures = default_sdk.hollow_output_failures(source, output, wall_thickness_mm=wall_thickness_mm)
    if failures:
        raise RuntimeError(
            "Hollow output failed quality guard: "
            + "; ".join(failures)
            + ". Reduce the wall thickness, lower the voxel size, or relax the protected regions."
        )


def _validate_decimate_output_quality(source: MeshDocument, output: MeshDocument) -> None:
    failures = default_sdk.decimate_output_failures(source, output)
    if failures:
        raise RuntimeError(f"Decimation output failed quality guard: {'; '.join(failures)}")


def _nonnegative_integer_list(value: object) -> list[int]:
    if not isinstance(value, list):
        return []
    seen: set[int] = set()
    indices: list[int] = []
    for item in value:
        if isinstance(item, bool):
            continue
        if isinstance(item, int):
            index = item
        elif isinstance(item, float) and item.is_integer():
            index = int(item)
        elif isinstance(item, str) and item.strip().isdigit():
            index = int(item.strip())
        else:
            continue
        if index < 0 or index in seen:
            continue
        seen.add(index)
        indices.append(index)
    return indices


def _selection_payload_face_ids(payload: object) -> list[int]:
    if not isinstance(payload, dict):
        return []
    resolved_face_ids = _nonnegative_integer_list(payload.get("resolved_face_ids"))
    if resolved_face_ids:
        return resolved_face_ids
    selection = payload.get("selection")
    if isinstance(selection, dict):
        return _nonnegative_integer_list(selection.get("face_ids"))
    return []


def _workbench_selection_region_faces(db: Session, version_id: str, workdir: Path) -> list[int] | None:
    artifact = get_artifact_by_type(db, version_id, "meshlib_selection_json")
    if artifact is None:
        return None
    selection_path = materialize_artifact(artifact, workdir / "meshlib_selection")
    payload = json.loads(selection_path.read_text(encoding="utf-8"))
    face_ids = _selection_payload_face_ids(payload)
    return face_ids or None


def _mark_job_running(db: Session, job: JobRecord, message: str, progress_pct: int = 5) -> None:
    set_job_status(db, job, "running", progress_pct=progress_pct)
    add_job_event(db, job.id, message, progress_pct)
    _commit_open_transaction(db)


def _commit_open_transaction(db: Session) -> None:
    commit = getattr(db, "commit", None)
    if not callable(commit):
        return
    in_transaction = getattr(db, "in_transaction", None)
    if callable(in_transaction):
        if in_transaction():
            commit()
        return
    commit()


def _record_job_event(
    db: Session,
    job_id: str,
    message: str,
    progress_pct: int | None = None,
    level: str = "info",
    *,
    commit: bool = True,
) -> None:
    add_job_event(db, job_id, message, progress_pct, level)
    if commit:
        _commit_open_transaction(db)


def _load_region_payload(db: Session, version_id: str, workdir: Path) -> dict:
    artifact = get_artifact_by_type(db, version_id, "analysis_regions_json")
    if artifact is None:
        raise FileNotFoundError(f"analysis_regions_json artifact missing for version {version_id}")
    path = materialize_artifact(artifact, workdir)
    return json.loads(path.read_text(encoding="utf-8"))


def _load_optional_region_payload(db: Session, version_id: str, workdir: Path) -> dict:
    artifact = get_artifact_by_type(db, version_id, "analysis_regions_json")
    if artifact is None:
        return {"regions": []}
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


def _region_indices_union(payload: dict, region_ids: list[str], *, require_non_empty: bool = True) -> np.ndarray | None:
    unique_ids = []
    seen: set[str] = set()
    for region_id in region_ids:
        if region_id and region_id not in seen:
            seen.add(region_id)
            unique_ids.append(region_id)
    if not unique_ids:
        if not require_non_empty:
            return None
        raise RuntimeError("No region ids were provided")
    chunks: list[np.ndarray] = []
    missing_or_empty: list[str] = []
    for region_id in unique_ids:
        try:
            chunks.append(_region_indices(payload, region_id))
        except RuntimeError as exc:
            if require_non_empty:
                raise
            missing_or_empty.append(f"{region_id}: {exc}")
    if not chunks:
        if not require_non_empty:
            return None
        raise RuntimeError("; ".join(missing_or_empty) or "Selected regions have no vertices")
    merged = np.unique(np.concatenate(chunks))
    if merged.size == 0:
        if not require_non_empty:
            return None
        raise RuntimeError("Selected regions have no vertices")
    return merged


def _selection_seed_indices(mesh: MeshDocument, selection, region_payload: dict) -> np.ndarray:
    region_indices = [
        _region_indices(region_payload, str(region_id))
        for region_id in selection.region_ids
    ]
    region_vertex_indices = (
        np.concatenate(region_indices)
        if region_indices
        else np.asarray([], dtype=np.int64)
    )
    return default_sdk.selection_seed_indices(
        mesh,
        vertex_ids=selection.vertex_ids,
        face_ids=selection.face_ids,
        region_vertex_indices=region_vertex_indices,
        brush_points_world=selection.brush_points_world,
    )


def _brush_replay_stroke_to_sdk(mesh: MeshDocument, stroke: BrushReplayStroke, region_payload: dict) -> BrushStroke:
    operation = BRUSH_TOOL_TO_OPERATION[stroke.tool_id]
    return BrushStroke(
        operation,
        _selection_seed_indices(mesh, stroke.selection, region_payload),
        amount_mm=stroke.amount_mm if operation != "smooth" else 0.0,
        falloff_mm=stroke.falloff_mm,
        iterations=stroke.iterations,
        strength=stroke.strength,
    )


def _validate_axis(axis: tuple[float, float, float]) -> tuple[float, float, float]:
    normalized = default_sdk.normalize_axis(axis)
    return (float(normalized[0]), float(normalized[1]), float(normalized[2]))


def _region_entries(payload: dict) -> list[RegionEntry]:
    entries: list[RegionEntry] = []
    for item in payload.get("regions", []):
        centroid = item.get("centroid_mm")
        entries.append(
            RegionEntry(
                region_id=str(item.get("region_id", "")),
                label=str(item.get("label") or item.get("region_id") or ""),
                vertex_indices=np.asarray(item.get("vertex_indices", []), dtype=np.int64),
                coverage_pct=float(item.get("coverage_pct", 0.0)),
                protected_by_default=bool(item.get("protected_by_default", False)),
                allowed_operations=[str(operation) for operation in item.get("allowed_operations", [])],
                min_thickness_mm=item.get("min_thickness_mm"),
                avg_thickness_mm=item.get("avg_thickness_mm"),
                violation_count=int(item.get("violation_count", 0)),
                centroid_mm=tuple(float(value) for value in centroid) if centroid is not None else None,
            )
        )
    return entries


def _region_payload_from_entries(entries: list[RegionEntry]) -> dict:
    return {
        "regions": [
            {
                "region_id": str(entry.region_id),
                "label": str(entry.label),
                "vertex_indices": [int(index) for index in entry.vertex_indices],
                "coverage_pct": float(entry.coverage_pct),
                "protected_by_default": bool(entry.protected_by_default),
                "allowed_operations": [str(operation) for operation in entry.allowed_operations],
                "min_thickness_mm": entry.min_thickness_mm,
                "avg_thickness_mm": entry.avg_thickness_mm,
                "violation_count": int(entry.violation_count),
                "centroid_mm": entry.centroid_mm,
            }
            for entry in entries
        ]
    }


def _region_payload_has_ids(payload: dict | None, region_ids: list[str]) -> bool:
    if not region_ids:
        return True
    if not payload:
        return False
    available_ids = {str(item.get("region_id")) for item in payload.get("regions", [])}
    return all(region_id in available_ids for region_id in region_ids)


def _selection_region_ids(selections) -> list[str]:
    seen: set[str] = set()
    region_ids: list[str] = []
    for selection in selections:
        for region_id in selection.region_ids:
            normalized = str(region_id)
            if normalized and normalized not in seen:
                seen.add(normalized)
                region_ids.append(normalized)
    return region_ids


def _detect_region_payload(mesh: MeshDocument) -> dict:
    measurement = default_sdk.measure_ring(mesh)
    return _region_payload_from_entries(default_sdk.detect_ring_regions(mesh, measurement))


def _available_region_ids(payload: dict, requested_ids: list[str]) -> list[str]:
    available_ids = {str(item.get("region_id")) for item in payload.get("regions", [])}
    filtered_ids: list[str] = []
    seen: set[str] = set()
    for region_id in requested_ids:
        if region_id in available_ids and region_id not in seen:
            seen.add(region_id)
            filtered_ids.append(region_id)
    return filtered_ids


def _large_hollow_preview_protect_region_ids(payload: dict, requested_ids: list[str]) -> list[str]:
    return _available_region_ids(payload, [*requested_ids, *DEFAULT_HOLLOW_PROTECT_REGION_IDS])


def _preserved_resize_region_ids(payload: dict) -> list[str]:
    return _available_region_ids(payload, ["head", "gem_seat", "ornament_relief"])


def _region_ids_allowed_for_operation(payload: dict, region_ids: list[str], operation: str) -> list[str]:
    region_map = {str(item.get("region_id")): item for item in payload.get("regions", [])}
    allowed_ids: list[str] = []
    seen: set[str] = set()
    for region_id in region_ids:
        if not region_id or region_id in seen:
            continue
        region = region_map.get(region_id)
        if region is None:
            raise RuntimeError(f"Region {region_id} not found")
        allowed_operations = {str(value) for value in region.get("allowed_operations", [])}
        if operation not in allowed_operations:
            raise RuntimeError(f"Region {region_id} does not allow {operation}")
        seen.add(region_id)
        allowed_ids.append(region_id)
    if not allowed_ids:
        raise RuntimeError(f"No region ids were provided for {operation}")
    return allowed_ids


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
    defer_analysis: bool = False,
    on_before_commit: Callable[[ModelVersionRecord, JobRecord], None] | None = None,
    normalized_mesh_metadata: dict[str, object] | None = None,
) -> None:
    _commit_open_transaction(db)

    preview_high = workdir / f"{version.id}_high.glb"
    preview_low = workdir / f"{version.id}_low.glb"
    manufacturing_stl = workdir / f"{version.id}.stl"

    to_glb(normalized_mesh_path, preview_high)
    # "low" = lightweight LOD for fast frontend loading — decimate it instead of
    # writing a full-resolution duplicate of "high".
    to_glb(normalized_mesh_path, preview_low, decimate_faces=20000)
    to_stl(normalized_mesh_path, manufacturing_stl)

    register_file_artifact(
        db,
        version.id,
        normalized_mesh_path,
        "normalized_mesh_ply",
        "model/ply",
        metadata_json=normalized_mesh_metadata,
    )
    register_file_artifact(db, version.id, preview_high, "preview_glb_high", "model/gltf-binary")
    register_file_artifact(db, version.id, preview_low, "preview_glb_low", "model/gltf-binary")
    register_file_artifact(db, version.id, manufacturing_stl, "manufacturing_stl", "application/sla")

    # Heaviest finalize step, and only the analysis panel needs it. When deferred,
    # the operation returns after the previews and the snapshot is computed lazily
    # on first request — off the operation's critical path. (Threads wouldn't help:
    # the kernels already saturate all cores via rayon, so concurrent finalize just
    # oversubscribes the CPU; deferring is the real lever.)
    if not defer_analysis:
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
        if normalized_mesh_metadata and normalized_mesh_metadata.get("full_voxel_hollow_deferred"):
            recommendation = (
                "Full voxel hollow/drain finalization deferred for this large mesh; "
                "use an offline full-resolution hollow pass before casting."
            )
            if recommendation not in snapshot.recommendations:
                snapshot.recommendations.append(recommendation)
        upsert_snapshot(db, version.id, "manufacturability", snapshot.model_dump(mode="json"))

    version.status = "ready" if complete_job else "processing"
    job.version_id = version.id
    if on_before_commit is not None:
        on_before_commit(version, job)
    if complete_job:
        set_job_status(db, job, "succeeded", progress_pct=progress_pct)
        add_job_event(db, job.id, completion_message, progress_pct)
    else:
        set_job_status(db, job, "running", progress_pct=progress_pct)
        add_job_event(db, job.id, completion_message, progress_pct)
    db.commit()


def _repair_regression_reason(source: MeshDocument, output: MeshDocument) -> str | None:
    """Do-no-harm guard: return a reason string if the repair made the mesh WORSE,
    else None. basic_repair's weld can fuse a self-intersection seam into
    non-manifold edges and open a watertight mesh — a net regression that the
    unguarded handler used to persist. We compare watertightness, manifoldness, and
    self-intersection count, and refuse to ship a result that lost ground.
    """
    try:
        hs = default_sdk.health(source)
        ho = default_sdk.health(output)
    except Exception:  # noqa: BLE001 - if health is unavailable, don't block
        return None
    if int(ho.nonmanifold_edge_count) > int(hs.nonmanifold_edge_count):
        return f"introduced {int(ho.nonmanifold_edge_count) - int(hs.nonmanifold_edge_count)} non-manifold edge(s)"
    if hs.is_closed and not ho.is_closed:
        return f"opened a watertight mesh ({int(ho.boundary_edge_count)} new boundary edges)"
    try:
        si_s = len(default_sdk.self_intersecting_faces(source, touch_is_intersection=False))
        si_o = len(default_sdk.self_intersecting_faces(output, touch_is_intersection=False))
        if si_o > si_s:
            return f"introduced {si_o - si_s} self-intersecting face(s)"
    except Exception:  # noqa: BLE001 - self-intersection check is best-effort
        pass
    return None


def _brush_fold_reason(source: MeshDocument, output: MeshDocument) -> str | None:
    """Return a reason if a brush stroke folded the surface onto itself, else None.
    thicken/scoop displace vertices along their normals with no fold check, so on a
    feature thinner than the brush amount adjacent triangles overlap (verified: a
    default 0.15mm thicken took the snake from 2 to 34 self-intersecting faces). We
    flag a meaningful INCREASE over the source (a tolerance absorbs pre-existing
    intersections and numerical noise; the smooth brush never folds).
    """
    try:
        si_in = len(default_sdk.self_intersecting_faces(source, touch_is_intersection=False))
        si_out = len(default_sdk.self_intersecting_faces(output, touch_is_intersection=False))
    except Exception:  # noqa: BLE001 - self-intersection check is best-effort
        return None
    new = si_out - si_in
    if new > 8:
        return f"folded the surface onto itself ({new} new self-intersecting faces)"
    return None


def _mesh_defect_score(mesh: MeshDocument) -> int:
    """Lower is healthier. Open surface dominates, then non-manifold edges, boundary
    edges, and self-intersections. Used to choose the cleanest voxel-remesh candidate.
    """
    try:
        health = default_sdk.health(mesh)
    except Exception:  # noqa: BLE001
        return 0
    score = int(health.nonmanifold_edge_count) * 1000 + int(health.boundary_edge_count)
    if not health.is_closed:
        score += 1_000_000
    try:
        score += len(default_sdk.self_intersecting_faces(mesh, touch_is_intersection=False)) * 10
    except Exception:  # noqa: BLE001
        pass
    return score


def _repair_remesh_candidate_sizes(mesh: MeshDocument, override_mm: float | None) -> list[float]:
    """Voxel sizes to try for the opt-in remesh repair, FINE -> COARSE. A finer voxel
    keeps more detail but may preserve the self-intersection; a coarser one smooths it
    away. We try finest first and stop at the first fully-clean result (best detail).
    An explicit override skips the search.
    """
    if override_mm and override_mm > 0:
        return [float(override_mm)]
    vertices = np.asarray(mesh.vertices, dtype=float)
    if vertices.size == 0:
        return [0.1]
    extent = float((vertices.max(axis=0) - vertices.min(axis=0)).max())
    return [extent / divisor for divisor in (96.0, 64.0, 48.0, 32.0)]


def run_repair_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: RepairRequest | None = None,
    *,
    complete_job: bool = True,
    on_before_commit: Callable[[ModelVersionRecord, JobRecord], None] | None = None,
) -> ModelVersionRecord:
    request = request or RepairRequest()
    _mark_job_running(db, job, "Repair started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_repair.ply"

    mesh = default_sdk.load_mesh(source_mesh)
    repaired, repair_report = default_sdk.basic_repair(mesh)
    if repair_report.removed_degenerate_faces:
        _record_job_event(db, job.id, f"Removed {repair_report.removed_degenerate_faces} degenerate faces", 30)
    if repair_report.merged_vertices:
        _record_job_event(db, job.id, f"Merged {repair_report.merged_vertices} duplicate vertices", 45)
    repaired, hole_report = default_sdk.service_fill_holes(repaired)
    if hole_report.filled_holes:
        _record_job_event(db, job.id, f"Filled {hole_report.filled_holes} hole(s)", 60)
    repaired = default_sdk.orient_faces_outward(repaired)
    if request.voxel_remesh:
        # User opted into aggressive healing: rebuild via a voxel grid, which closes
        # self-intersections the gentle path cannot, at the cost of resampling. Try
        # candidate voxel sizes and keep the cleanest (finest fully-clean) result that
        # beats the gentle result; the do-no-harm guard below is the final backstop.
        best, best_score = repaired, _mesh_defect_score(repaired)
        for voxel_size in _repair_remesh_candidate_sizes(mesh, request.voxel_size_mm):
            try:
                remeshed, _ = default_sdk.rebuild_via_sdf(mesh, voxel_size_mm=voxel_size)
            except (RuntimeError, ValueError):
                continue
            score = _mesh_defect_score(remeshed)
            if score < best_score:
                best, best_score = remeshed, score
            if best_score == 0:
                break
        if best is not repaired:
            repaired = best
            _record_job_event(
                db, job.id, f"Voxel-remesh repair healed residual defects ({repaired.face_count} faces).", 62
            )
        else:
            _record_job_event(db, job.id, "Voxel-remesh repair did not improve on gentle repair; kept it.", 62)
    regression = _repair_regression_reason(mesh, repaired)
    if regression is not None:
        _record_job_event(
            db,
            job.id,
            (
                f"Repair could not safely improve this mesh — it would have {regression}; "
                "keeping the input unchanged. For self-intersections, try a voxel remesh."
            ),
            65,
        )
        repaired = mesh
    default_sdk.save_mesh(repaired, output_mesh, file_type="ply")
    _record_job_event(db, job.id, "Repair geometry rebuilt", 70)

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
        on_before_commit=on_before_commit,
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
    on_before_commit: Callable[[ModelVersionRecord, JobRecord], None] | None = None,
) -> ModelVersionRecord:
    _mark_job_running(db, job, "Resize started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    axis_override = tuple(request.manual_axis) if request.axis_mode == "manual" and request.manual_axis else None
    if request.axis_mode == "manual" and axis_override is None:
        raise RuntimeError("manual axis mode requires manual_axis")
    if axis_override is not None:
        axis_override = _validate_axis(axis_override)
    mesh = default_sdk.load_mesh(source_mesh)
    measurement = default_sdk.measure_ring(mesh, axis_override=axis_override)
    current_size = measurement.estimated_ring_size_us or 7.0
    output_mesh = workdir / f"{job.id}_resize.ply"
    preserve_indices = None
    if request.preserve_head:
        region_payload = _load_region_payload(db, source_version.id, workdir)
        preserve_region_ids = _preserved_resize_region_ids(region_payload)
        if preserve_region_ids:
            preserve_indices = _region_indices_union(region_payload, preserve_region_ids, require_non_empty=False)
    # "Resize to US X" is an absolute fit: scale the measured bore to the target
    # diameter. The size-table delta (resize_ring) silently under-scales any model
    # whose estimated size is clamped to the chart bounds, e.g. unscaled
    # AI-generated miniatures.
    target_diameter_mm = ring_diameter_for_size(request.target_ring_size_us)
    measured_diameter_mm = measurement.inner_diameter_mm
    if measured_diameter_mm and measured_diameter_mm > 0 and target_diameter_mm:
        # Absolute fit to the target bore. The fold-avoidance decision (drop the
        # protected region and scale uniformly when the ratio is too extreme to
        # preserve it) lives in the Rust kernel; the service only surfaces the
        # outcome.
        fit = default_sdk.fit_ring_to_diameter(
            mesh,
            measured_diameter_mm=measured_diameter_mm,
            target_diameter_mm=target_diameter_mm,
            ring_axis=axis_override or measurement.ring_axis,
            preserve_indices=preserve_indices,
        )
        if fit.applied_uniform_fallback:
            _record_job_event(
                db,
                job.id,
                (
                    f"Resize ratio {fit.scale_factor:.2f}x exceeds the safe preserve-head range; "
                    "scaling the whole ring uniformly to avoid folding the protected region."
                ),
                40,
            )
        resized = fit.mesh
    else:
        resized = default_sdk.resize_ring(
            mesh,
            current_size=current_size,
            target_size=request.target_ring_size_us,
            ring_axis=axis_override or measurement.ring_axis,
            preserve_indices=preserve_indices,
        )
    default_sdk.save_mesh(resized, output_mesh, file_type="ply")
    _record_job_event(db, job.id, "Ring resized", 70)

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
        on_before_commit=on_before_commit,
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
    on_before_commit: Callable[[ModelVersionRecord, JobRecord], None] | None = None,
) -> ModelVersionRecord:
    _mark_job_running(db, job, "Hollowing started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    _guard_protected_regions_for_hollow(db, source_version, workdir, request)
    region_payload = _load_region_payload(db, source_version.id, workdir) if (request.protect_regions or request.add_drain_holes) else None
    output_mesh = workdir / f"{job.id}_hollow.ply"
    mesh = default_sdk.load_mesh(source_mesh)
    region_entries = _region_entries(region_payload or {})
    protect_region_ids = _available_region_ids(region_payload or {}, request.protect_regions)
    wall_thickness_mm = request.wall_thickness_mm or 0.8
    large_interactive_mesh = (
        request.processing_mode == "interactive"
        and settings.MESH_EDIT_HOLLOW_MAX_FACES > 0
        and mesh.face_count > settings.MESH_EDIT_HOLLOW_MAX_FACES
    )
    if (
        request.processing_mode == "full_resolution"
        and settings.MESH_EDIT_HOLLOW_FULL_RESOLUTION_MAX_FACES > 0
        and mesh.face_count > settings.MESH_EDIT_HOLLOW_FULL_RESOLUTION_MAX_FACES
    ):
        raise RuntimeError(
            "Full-resolution voxel hollowing is limited to "
            f"{settings.MESH_EDIT_HOLLOW_FULL_RESOLUTION_MAX_FACES} faces for the current worker; "
            f"this mesh has {mesh.face_count} faces. Use interactive hollow preview for dense meshes "
            "or run a dedicated offline MeshLib/Rust hollow worker with a higher face limit."
        )
    completion_message = "Hollowing completed" if complete_job else "Weight optimization step completed"
    operation_label = "Hollow Mesh"
    normalized_mesh_metadata: dict[str, object] | None = None
    if large_interactive_mesh:
        protect_region_ids = _large_hollow_preview_protect_region_ids(region_payload or {}, request.protect_regions)
        hollowed = default_sdk.weighted_inner_offset_preview(
            mesh,
            region_entries,
            protect_region_ids,
            wall_thickness_mm=wall_thickness_mm,
        )
        hollowed.metadata.update(
            {
                "operation": "interactive_hollow_preview",
                "source": "rust_weighted_inner_offset_preview",
                "requested_mode": request.mode,
                "wall_thickness_mm": wall_thickness_mm,
                "target_weight_g": request.target_weight_g,
                "protected_region_ids": protect_region_ids,
                "drain_holes_requested": bool(request.add_drain_holes),
                "drain_holes_deferred": bool(request.add_drain_holes),
                "target_weight_deferred": request.mode == "target_weight",
                "full_voxel_hollow_deferred": True,
                "source_face_count": mesh.face_count,
                "interactive_face_limit": settings.MESH_EDIT_HOLLOW_MAX_FACES,
            }
        )
        normalized_mesh_metadata = dict(hollowed.metadata)
        _record_job_event(
            db,
            job.id,
            "Generated Rust weighted inner-offset hollow preview; full voxel shell deferred for this large mesh.",
            70,
        )
        completion_message = "Interactive hollow preview completed"
        operation_label = "Interactive Hollow Preview"
    else:
        if request.processing_mode == "full_resolution" and settings.MESH_EDIT_HOLLOW_MAX_FACES > 0 and mesh.face_count > settings.MESH_EDIT_HOLLOW_MAX_FACES:
            _record_job_event(
                db,
                job.id,
                "Running full-resolution Rust voxel hollowing for a large mesh; this may take longer than interactive preview.",
                10,
            )
        voxel_size_mm = default_sdk.service_hollow_voxel_size(mesh, wall_thickness_mm=wall_thickness_mm)
        if request.mode == "target_weight":
            if protect_region_ids:
                hollowed, _ = default_sdk.adaptive_protected_hollow_to_weight(
                    mesh,
                    region_entries,
                    protect_region_ids,
                    target_weight_g=request.target_weight_g or 0.5,
                    material=request.material.value,
                    tolerance_g=0.1,
                    min_thickness_mm=request.min_allowed_thickness_mm,
                    max_thickness_mm=3.0,
                    voxel_size_mm=voxel_size_mm,
                )
            else:
                hollowed, _ = default_sdk.adaptive_hollow_to_weight(
                    mesh,
                    target_weight_g=request.target_weight_g or 0.5,
                    material=request.material.value,
                    tolerance_g=0.1,
                    min_thickness_mm=request.min_allowed_thickness_mm,
                    max_thickness_mm=3.0,
                    voxel_size_mm=voxel_size_mm,
                )
        else:
            if protect_region_ids:
                hollowed = default_sdk.protected_hollow_mesh(
                    mesh,
                    region_entries,
                    protect_region_ids,
                    wall_thickness_mm=wall_thickness_mm,
                    voxel_size_mm=voxel_size_mm,
                )
            else:
                hollowed = default_sdk.service_hollow(mesh, wall_thickness_mm=wall_thickness_mm)
        if request.add_drain_holes:
            measurement = default_sdk.measure_ring(mesh)
            plans = default_sdk.plan_drain_holes(
                mesh,
                region_entries,
                measurement.ring_axis,
                wall_thickness_mm=wall_thickness_mm or request.min_allowed_thickness_mm,
            )
            hollowed = default_sdk.apply_drain_holes_voxel(hollowed, plans, voxel_size_mm=voxel_size_mm)
        # The voxel/marching shell can leave coincident vertex records where the
        # cavity pinches to nothing, reading as zero-area boundary loops. Weld them
        # (eps far below voxel scale, so real drain holes are untouched) so the
        # shell is strictly watertight.
        hollowed, weld_report = default_sdk.weld_coincident_vertices(hollowed)
        if weld_report.get("merged_vertex_count") or weld_report.get("removed_face_count"):
            _record_job_event(
                db,
                job.id,
                f"Welded {weld_report['merged_vertex_count']} coincident vertices and dropped "
                f"{weld_report['removed_face_count']} degenerate faces for a watertight shell.",
                73,
            )
        # Remove tiny stray slivers an aggressive hollow can shed (e.g. a target-weight
        # hollow thinning a wall until a chip breaks off), then require a single solid:
        # a clean hollow is one connected piece, so >1 substantial component means the
        # cavity perforated the walls and fragmented the model — refuse rather than
        # ship an unprintable, multi-piece result. (The lenient Rust component guard
        # tolerates up to ~7 components for legitimate variation; this is the stricter
        # single-solid requirement for a finished hollow.)
        total_area_mm2 = _mesh_surface_area_mm2(hollowed)
        hollowed, prune_report = default_sdk.prune_small_components(
            hollowed, min_area_mm2=total_area_mm2 * 0.01
        )
        if prune_report.removed_component_count:
            _record_job_event(
                db,
                job.id,
                f"Pruned {prune_report.removed_component_count} stray sliver component(s) from the hollow shell.",
                74,
            )
        if prune_report.output_component_count > 1:
            raise RuntimeError(
                f"Hollowing fragmented the model into {prune_report.output_component_count} disconnected pieces "
                "— the cavity broke through thin walls. Increase the wall thickness or target weight, or "
                "thicken the model first."
            )
        _validate_hollow_output_quality(mesh, hollowed, wall_thickness_mm)
        _record_job_event(db, job.id, "Hollow mesh generated", 75)
    default_sdk.save_mesh(hollowed, output_mesh, file_type="ply")

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="hollow",
        operation_label=operation_label,
        status="processing",
    )
    _finalize_version(
        db,
        new_version,
        job,
        output_mesh,
        workdir,
        complete_job=complete_job,
        completion_message=completion_message,
        progress_pct=100 if complete_job else 85,
        on_before_commit=on_before_commit,
        normalized_mesh_metadata=normalized_mesh_metadata,
    )
    return new_version


def run_scoop_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: ScoopRequest,
) -> ModelVersionRecord:
    _mark_job_running(db, job, f"Scooping {request.region_id}")
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

    mesh = default_sdk.load_mesh(source_mesh)
    selected_indices = _region_indices(region_payload, request.region_id)
    bounded_indices = _bounded_seed_indices(mesh, selected_indices, settings.MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES)
    if bounded_indices.size < np.asarray(selected_indices).size:
        _record_job_event(
            db,
            job.id,
            (
                "Scoop seed set reduced from "
                f"{np.asarray(selected_indices).size} to {bounded_indices.size} representative vertices "
                "for interactive dense-mesh performance."
            ),
            20,
        )
    scooped = default_sdk.local_scoop(
        mesh,
        bounded_indices,
        depth_mm=request.depth_mm,
        falloff_mm=request.falloff_mm,
    )
    output_mesh = workdir / f"{job.id}_scoop.ply"
    default_sdk.save_mesh(scooped, output_mesh, file_type="ply")
    _record_job_event(db, job.id, "Scoop deformation applied", 65)

    preview_thickness_deferred = (
        settings.MANUFACTURABILITY_THICKNESS_MAX_VERTICES > 0
        and scooped.vertex_count > settings.MANUFACTURABILITY_THICKNESS_MAX_VERTICES
    )
    preview_snapshot = None
    if not preview_thickness_deferred:
        preview_snapshot, _ = compute_manufacturability_snapshot(
            output_mesh,
            workdir / f"{job.id}_preview_analysis",
            threshold_mm=request.keep_min_thickness_mm,
        )
        preview_thickness_deferred = _snapshot_thickness_was_deferred(preview_snapshot)
    if preview_thickness_deferred:
        _record_job_event(
            db,
            job.id,
            "Scoop preview accepted with deferred full thickness analysis; use focused Measure/Inspect for local wall checks.",
            70,
        )
    elif preview_snapshot is None or preview_snapshot.thickness.min_mm is None or preview_snapshot.thickness.min_mm < request.keep_min_thickness_mm:
        predicted = None if preview_snapshot is None else preview_snapshot.thickness.min_mm
        raise RuntimeError(
            f"Scoop would violate minimum thickness {request.keep_min_thickness_mm:.2f}mm "
            f"(predicted min {predicted}). Reduce scoop depth or thicken the target region first."
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
    _mark_job_running(db, job, "Thickening started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_thicken.ply"
    mesh = default_sdk.load_mesh(source_mesh)

    if request.mode == "global":
        thickened = default_sdk.global_thicken(mesh, min_target_thickness_mm=request.min_target_thickness_mm)
    else:
        region_payload = _load_region_payload(db, source_version.id, workdir)
        thickness = _load_thickness_values(db, source_version.id, workdir)

        if request.mode in {"selected_region", "selected_regions"}:
            target_region_ids = list(request.region_ids)
            if request.region_id and request.region_id not in target_region_ids:
                target_region_ids.append(request.region_id)
            if not target_region_ids:
                raise RuntimeError("selected region thickening requires region_id or region_ids")
            target_region_ids = _region_ids_allowed_for_operation(region_payload, target_region_ids, "thicken")
            seed_indices = _region_indices_union(region_payload, target_region_ids)
            bounded_seed_indices = _bounded_seed_indices(mesh, seed_indices, settings.MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES)
            if bounded_seed_indices.size < seed_indices.size:
                _record_job_event(
                    db,
                    job.id,
                    (
                        "Thicken seed set reduced from "
                        f"{seed_indices.size} to {bounded_seed_indices.size} representative vertices "
                        "for interactive dense-mesh performance."
                    ),
                    20,
                )
            seed_indices = bounded_seed_indices
            falloff_mm = 1.5
        else:
            finite_thickness = np.isfinite(thickness)
            seed_indices = np.flatnonzero(finite_thickness & (thickness < request.min_target_thickness_mm))
            if seed_indices.size == 0:
                if not finite_thickness.any():
                    target_region_ids = list(request.region_ids)
                    if request.region_id and request.region_id not in target_region_ids:
                        target_region_ids.append(request.region_id)
                    if not target_region_ids:
                        raise RuntimeError(
                            "Thickness analysis is deferred for this mesh; select a region to thicken locally."
                        )
                    target_region_ids = _region_ids_allowed_for_operation(region_payload, target_region_ids, "thicken")
                    seed_indices = _region_indices_union(region_payload, target_region_ids)
                    bounded_seed_indices = _bounded_seed_indices(
                        mesh,
                        seed_indices,
                        settings.MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES,
                    )
                    _record_job_event(
                        db,
                        job.id,
                        (
                            "Thickness analysis is deferred for this dense mesh; "
                            "using the selected region as the local thickening seed set."
                        ),
                        20,
                    )
                    if bounded_seed_indices.size < seed_indices.size:
                        _record_job_event(
                            db,
                            job.id,
                            (
                                "Thicken seed set reduced from "
                                f"{seed_indices.size} to {bounded_seed_indices.size} representative vertices "
                                "for interactive dense-mesh performance."
                            ),
                            20,
                        )
                    seed_indices = bounded_seed_indices
                    falloff_mm = 1.5
                else:
                    raise RuntimeError("No violating regions found for local thickening")
            else:
                falloff_mm = 1.2

        thickened = default_sdk.local_thicken_to_minimum(
            mesh,
            seed_indices,
            thickness,
            min_target_thickness_mm=request.min_target_thickness_mm,
            falloff_mm=falloff_mm,
            deficit_scale=0.75,
        )
        if request.smoothing_pass:
            thickened = default_sdk.smooth(thickened, iterations=4, strength=0.15, nu=-0.2)
    # Global (voxel) thickening can leave coincident vertex records at pinch
    # points, reading as zero-area boundary loops. Weld them so the result is
    # strictly watertight (no-op when the mesh is already clean).
    thickened, weld_report = default_sdk.weld_coincident_vertices(thickened)
    if weld_report.get("merged_vertex_count") or weld_report.get("removed_face_count"):
        _record_job_event(
            db,
            job.id,
            f"Welded {weld_report['merged_vertex_count']} coincident vertices and dropped "
            f"{weld_report['removed_face_count']} degenerate faces for a watertight result.",
            72,
        )
    default_sdk.save_mesh(thickened, output_mesh, file_type="ply")
    _record_job_event(db, job.id, "Mesh thickened", 75)

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
    _mark_job_running(db, job, "Compare started", 10)
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    other_mesh = _load_normalized_artifact(db, other_version.id, workdir)

    mesh_a = default_sdk.load_mesh(source_mesh)
    mesh_b = default_sdk.load_mesh(other_mesh)
    values = np.asarray(default_sdk.service_compare_field(mesh_a, mesh_b), dtype=np.float32)
    summary = default_sdk.service_compare(mesh_a, mesh_b)
    bbox_a = np.ptp(mesh_a.vertices, axis=0)
    bbox_b = np.ptp(mesh_b.vertices, axis=0)
    max_expected_distance = max(float(np.linalg.norm(bbox_a)), float(np.linalg.norm(bbox_b)), 1.0) * 4.0
    values = np.where(np.abs(values) <= max_expected_distance, values, np.nan).astype(np.float32)
    compare_npz = workdir / f"{job.id}_compare_{other_version.id}.npz"
    default_sdk.save_compare_npz(
        compare_npz,
        values,
        vertex_count=int(mesh_a.vertices.shape[0]),
        other_version_id=other_version.id,
    )
    register_file_artifact(
        db,
        source_version.id,
        compare_npz,
        f"analysis_compare_npz_{other_version.id}",
        "application/octet-stream",
        metadata_json={"other_version_id": other_version.id, "job_id": job.id, "generated_by": "compare_job"},
    )

    response = CompareResponse(
        version_id=source_version.id,
        other_version_id=other_version.id,
        volume_delta_mm3=round(summary.volume_delta_mm3, 4),
        weight_delta_g=round(summary.weight_delta_g, 4),
        bbox_delta_mm=(
            round(float(summary.bbox_delta_mm[0]), 4),
            round(float(summary.bbox_delta_mm[1]), 4),
            round(float(summary.bbox_delta_mm[2]), 4),
        ),
        min_signed_distance_mm=round(float(summary.min_signed_distance_mm), 4) if summary.min_signed_distance_mm is not None else None,
        max_signed_distance_mm=round(float(summary.max_signed_distance_mm), 4) if summary.max_signed_distance_mm is not None else None,
        mean_signed_distance_mm=round(float(summary.mean_signed_distance_mm), 4) if summary.mean_signed_distance_mm is not None else None,
    )
    compare_artifact = get_artifact_by_type(db, source_version.id, f"analysis_compare_npz_{other_version.id}")
    if compare_artifact is not None:
        compare_artifact.metadata_json = {
            **(compare_artifact.metadata_json or {}),
            "summary": response.model_dump(mode="json"),
        }
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
    _mark_job_running(db, job, "Smoothing started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_smooth.ply"

    mesh = default_sdk.load_mesh(source_mesh)
    iterations = max(1, request.iterations)
    lamb = min(max(request.strength, 0.01), 0.95)

    target_region_ids = list(request.region_ids)
    if request.region_id and request.region_id not in target_region_ids:
        target_region_ids.append(request.region_id)

    if target_region_ids:
        region_payload = _load_region_payload(db, source_version.id, workdir)
        target_region_ids = _region_ids_allowed_for_operation(region_payload, target_region_ids, "smooth")
        seed_indices = _region_indices_union(region_payload, target_region_ids)
        bounded_seed_indices = _bounded_seed_indices(mesh, seed_indices, settings.MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES)
        if bounded_seed_indices.size < seed_indices.size:
            _record_job_event(
                db,
                job.id,
                (
                    "Smooth seed set reduced from "
                    f"{seed_indices.size} to {bounded_seed_indices.size} representative vertices "
                    "for interactive dense-mesh performance."
                ),
                20,
            )
        seed_indices = bounded_seed_indices
        smoothed = default_sdk.smooth(mesh, iterations=iterations, strength=lamb, seed_indices=seed_indices, falloff_mm=1.8)
    elif request.global_mode:
        smoothed = default_sdk.smooth(mesh, iterations=iterations, strength=lamb)
    else:
        raise RuntimeError("Smooth requires global_mode=true or an explicit region_id")
    default_sdk.save_mesh(smoothed, output_mesh, file_type="ply")
    _record_job_event(db, job.id, "Mesh smoothing completed", 75)

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


def run_decimate_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: DecimateRequest,
) -> ModelVersionRecord:
    _mark_job_running(db, job, "Decimation started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_decimate.ply"

    mesh = default_sdk.load_mesh(source_mesh)
    requested_region_faces = request.region_faces or None
    metadata_source = str(request.metadata.get("source") or "")
    has_interactive_delete_cap = _has_interactive_decimate_delete_cap(request)
    if metadata_source in WORKBENCH_DECIMATE_SOURCES and requested_region_faces is None:
        requested_region_faces = _workbench_selection_region_faces(db, source_version.id, workdir)
        if requested_region_faces is None and not has_interactive_delete_cap:
            raise RuntimeError(
                "Workbench global decimation requires an explicit max_deleted_vertices or max_deleted_faces cap. "
                "Select a face region in the MeshLib workbench, or use a capped MeshLib decimation command."
            )
    selected_face_count = mesh.face_count if requested_region_faces is None else len(requested_region_faces)
    if (
        requested_region_faces is None
        and settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES > 0
        and mesh.face_count > settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES
        and not has_interactive_delete_cap
    ):
        raise RuntimeError(
            "Interactive full-mesh decimation is limited to "
            f"{settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES} faces for the current worker; "
            f"this mesh has {mesh.face_count} faces. Select a smaller face region or run a dedicated "
            "offline MeshLib/QEM decimation job."
        )
    if (
        settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES > 0
        and selected_face_count > settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES
        and (requested_region_faces is not None or not has_interactive_delete_cap)
    ):
        raise RuntimeError(
            "Interactive MeshLib-parity decimation is limited to "
            f"{settings.MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES} selected faces; "
            f"this request selected {selected_face_count}. The previous fast preview path was disabled "
            "because it can damage closed topology on dense meshes. Select a smaller region or run "
            "offline MeshLib/QEM decimation."
        )
    result = default_sdk.decimate_mesh(
        mesh,
        strategy=request.strategy,
        max_error=request.max_error,
        target_face_count=request.target_face_count,
        target_face_ratio=request.target_face_ratio,
        max_edge_len=request.max_edge_len,
        max_bd_shift=request.max_bd_shift,
        stabilizer=request.stabilizer,
        subdivide_parts=request.subdivide_parts,
        decimate_between_parts=request.decimate_between_parts,
        region_faces=requested_region_faces,
        not_flippable_edges=[list(edge) for edge in request.not_flippable_edges],
        edges_to_collapse=[list(edge) for edge in request.edges_to_collapse] or None,
        collapse_near_not_flippable=request.collapse_near_not_flippable,
        angle_weighted_dist_to_plane=request.angle_weighted_dist_to_plane,
        max_deleted_vertices=request.max_deleted_vertices,
        max_deleted_faces=request.max_deleted_faces,
        max_triangle_aspect_ratio=request.max_triangle_aspect_ratio,
        critical_tri_aspect_ratio=request.critical_tri_aspect_ratio,
        tiny_edge_length=request.tiny_edge_length,
        max_angle_change=request.max_angle_change,
        touch_near_bd_edges=request.touch_near_bd_edges,
        touch_bd_verts=request.touch_bd_verts,
        optimize_vertex_pos=request.optimize_vertex_pos,
        pack_mesh=request.pack_mesh,
    )
    if result.verts_deleted == 0 and result.faces_deleted == 0:
        raise RuntimeError(
            "Decimation did not modify this mesh with the selected constraints. "
            "Relax the decimation settings, choose a smaller region, or repair open boundaries first."
        )
    _validate_decimate_output_quality(mesh, result.mesh)
    default_sdk.save_mesh(result.mesh, output_mesh, file_type="ply")
    _record_job_event(
        db,
        job.id,
        f"Mesh decimation completed with {result.verts_deleted} vertex collapse(s) and {result.faces_deleted} deleted face(s)",
        75,
    )

    # Describe by the strategy actually requested. max_error defaults to ~DBL_MAX
    # (unbounded), so only show it when it is a real finite tolerance.
    if request.target_face_count:
        decimate_label = f"Decimate to {request.target_face_count} faces"
    elif request.target_face_ratio:
        decimate_label = f"Decimate to {request.target_face_ratio:.0%}"
    elif request.max_error is not None and request.max_error < 1e6:
        decimate_label = f"Decimate shortest edges <= {request.max_error:.3f}mm"
    else:
        decimate_label = "Decimate"
    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="decimate",
        operation_label=decimate_label,
        status="processing",
    )
    _finalize_version(db, new_version, job, output_mesh, workdir)
    return new_version


def run_subdivide_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: SubdivideRequest,
) -> ModelVersionRecord:
    _mark_job_running(db, job, "Subdivision started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_subdivide.ply"

    mesh = default_sdk.load_mesh(source_mesh)
    selected_face_count = len(request.region_faces) if request.region_faces else mesh.face_count
    if settings.MESH_EDIT_SUBDIVIDE_MAX_FACES > 0 and selected_face_count > settings.MESH_EDIT_SUBDIVIDE_MAX_FACES:
        raise RuntimeError(
            f"Subdivision is limited to {settings.MESH_EDIT_SUBDIVIDE_MAX_FACES} faces for interactive jobs; "
            f"this request selected {selected_face_count} faces. Use a smaller selected-region workflow "
            "or decimate/repair first."
        )
    result = default_sdk.subdivide_mesh(
        mesh,
        max_edge_len=request.max_edge_len,
        max_edge_splits=request.max_edge_splits,
        region_faces=request.region_faces or None,
        not_flippable_edges=[list(edge) for edge in request.not_flippable_edges],
        subdivide_border=request.subdivide_border,
        curvature_priority=request.curvature_priority,
        project_on_original_mesh=request.project_on_original_mesh,
        smooth_mode=request.smooth_mode,
        min_sharp_dihedral_angle=request.min_sharp_dihedral_angle,
        max_tri_aspect_ratio=request.max_tri_aspect_ratio,
        max_splittable_tri_aspect_ratio=request.max_splittable_tri_aspect_ratio,
        max_deviation_after_flip=request.max_deviation_after_flip,
        max_angle_change_after_flip=request.max_angle_change_after_flip,
        critical_tri_aspect_ratio_flip=request.critical_tri_aspect_ratio_flip,
    )
    if result.splits_done == 0:
        raise RuntimeError(
            "Subdivision did not modify this mesh with the selected constraints. "
            "Choose a region with edges longer than the target length or relax the subdivision settings."
        )
    default_sdk.save_mesh(result.mesh, output_mesh, file_type="ply")
    _record_job_event(
        db,
        job.id,
        f"Mesh subdivision completed with {result.splits_done} split(s)",
        75,
    )

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="subdivide",
        operation_label=f"Subdivide to {request.max_edge_len:.3f}mm edges",
        status="processing",
    )
    _finalize_version(db, new_version, job, output_mesh, workdir)
    return new_version


def run_make_delone_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: MakeDeloneRequest,
) -> ModelVersionRecord:
    _mark_job_running(db, job, "Make Delone started")
    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    output_mesh = workdir / f"{job.id}_make_delone.ply"

    mesh = default_sdk.load_mesh(source_mesh)
    output_document, flips_done = default_sdk.make_delone_edge_flips(
        mesh,
        num_iters=request.num_iters,
        region_faces=request.region_faces or None,
        max_deviation_after_flip=request.max_deviation_after_flip,
        max_angle_change=request.max_angle_change,
        critical_tri_aspect_ratio=request.critical_tri_aspect_ratio,
        not_flippable_edges=[list(edge) for edge in request.not_flippable_edges],
        vert_region=request.vert_region or None,
    )
    default_sdk.save_mesh(output_document, output_mesh, file_type="ply")
    _record_job_event(
        db,
        job.id,
        f"Make Delone completed with {flips_done} edge flip(s)",
        75,
    )

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="make_delone",
        operation_label=f"Make Delone x{request.num_iters}",
        status="processing",
    )
    _finalize_version(db, new_version, job, output_mesh, workdir)
    return new_version


def run_interactive_brush_replay_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: BrushReplayRequest,
) -> ModelVersionRecord:
    _mark_job_running(db, job, f"Replaying {len(request.strokes)} MeshLib brush stroke(s)")

    source_mesh = _load_normalized_artifact(db, source_version.id, workdir)
    mesh = default_sdk.load_mesh(source_mesh)
    region_payload = _load_optional_region_payload(db, source_version.id, workdir)
    requested_region_ids = _selection_region_ids([stroke.selection for stroke in request.strokes])
    if requested_region_ids and not _region_payload_has_ids(region_payload, requested_region_ids):
        region_payload = _detect_region_payload(mesh)
    strokes = [_brush_replay_stroke_to_sdk(mesh, stroke, region_payload) for stroke in request.strokes]
    brushed = default_sdk.apply_brush_strokes(mesh, strokes)
    fold = _brush_fold_reason(mesh, brushed)
    if fold is not None:
        raise RuntimeError(
            f"Brush stroke(s) {fold}. Reduce the brush amount or strength, or use smaller "
            "strokes; the smooth brush relaxes the surface without folding it."
        )
    output_mesh = workdir / f"{job.id}_brush_replay.ply"
    default_sdk.save_mesh(brushed, output_mesh, file_type="ply")
    _record_job_event(db, job.id, "Rust brush replay applied", 55)

    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="interactive_brush_replay",
        operation_label=request.operation_label,
        status="processing",
    )

    request_payload = {
        **request.model_dump(mode="json"),
        "source_version_id": source_version.id,
        "job_id": job.id,
        "stroke_count": len(strokes),
    }
    request_payload_path = workdir / f"{job.id}_interactive_brush_replay.json"
    request_payload_path.write_text(json.dumps(request_payload, indent=2), encoding="utf-8")
    register_file_artifact(
        db,
        new_version.id,
        request_payload_path,
        "interactive_brush_payload_json",
        "application/json",
        metadata_json={
            "source_version_id": source_version.id,
            "job_id": job.id,
            "stroke_count": len(strokes),
            **request.metadata,
        },
    )
    upsert_snapshot(db, new_version.id, "interactive_brush_replay", request_payload)
    _record_job_event(db, job.id, "Brush replay metadata registered", 70)

    _finalize_version(
        db,
        new_version,
        job,
        output_mesh,
        workdir,
        completion_message="Interactive brush replay committed",
    )
    return new_version


def run_make_manufacturable_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: MakeManufacturableRequest,
) -> ModelVersionRecord:
    raw_payload = dict(job.operation_request.payload_json or {}) if job.operation_request is not None else {}
    resume_current_version_id = raw_payload.get("_resume_current_version_id")
    completed_steps = list(raw_payload.get("_resume_completed_steps") or [])
    completed_step_set = set(str(step) for step in completed_steps)
    if resume_current_version_id:
        resumed_version = db.get(ModelVersionRecord, resume_current_version_id)
        current_version = resumed_version if resumed_version is not None else source_version
    else:
        current_version = source_version

    def make_resume_checkpoint(step_name: str) -> Callable[[ModelVersionRecord, JobRecord], None]:
        def _checkpoint(version: ModelVersionRecord, current_job: JobRecord) -> None:
            if current_job.operation_request is None:
                return
            payload_state = dict(current_job.operation_request.payload_json or {})
            prior_steps = [str(step) for step in payload_state.get("_resume_completed_steps") or []]
            if step_name not in prior_steps:
                prior_steps.append(step_name)
            payload_state["_resume_completed_steps"] = prior_steps
            payload_state["_resume_current_version_id"] = version.id
            current_job.operation_request.payload_json = payload_state

        return _checkpoint

    _mark_job_running(db, job, "Manufacturability flow started")

    if "repair" not in completed_step_set:
        current_version = run_repair_operation(
            db,
            current_version,
            job,
            workdir,
            complete_job=False,
            on_before_commit=make_resume_checkpoint("repair"),
        )
        completed_step_set.add("repair")

    if request.target_ring_size_us and "resize" not in completed_step_set:
        _record_job_event(db, job.id, "Sizing branch for manufacturability", 35)
        resize_req = ResizeRequest(target_ring_size_us=request.target_ring_size_us)
        current_version = run_resize_operation(
            db,
            current_version,
            job,
            workdir,
            resize_req,
            complete_job=False,
            on_before_commit=make_resume_checkpoint("resize"),
        )
        completed_step_set.add("resize")

    if request.target_weight_g and "hollow" not in completed_step_set:
        _record_job_event(db, job.id, "Weight optimization branch for manufacturability", 65)
        hollow_req = HollowRequest(
            mode="target_weight",
            material=request.material,
            target_weight_g=request.target_weight_g,
            min_allowed_thickness_mm=request.min_allowed_thickness_mm,
        )
        current_version = run_hollow_operation(
            db,
            current_version,
            job,
            workdir,
            hollow_req,
            complete_job=False,
            on_before_commit=make_resume_checkpoint("hollow"),
        )
        completed_step_set.add("hollow")

    final_mesh_path = _load_normalized_artifact(db, current_version.id, workdir)
    final_mesh = default_sdk.load_mesh(final_mesh_path)
    health = default_sdk.service_health(final_mesh)
    health_score = getattr(health, "health_score", None)
    validation_message = (
        f"Manufacturability validation completed with health score {health_score}"
        if health_score is not None
        else "Manufacturability validation completed"
    )
    _record_job_event(db, job.id, validation_message, 90)

    current_version.status = "ready"
    set_job_status(db, job, "succeeded", progress_pct=100)
    add_job_event(db, job.id, "Manufacturability flow completed", 100)
    if job.operation_request is not None:
        payload_state = dict(job.operation_request.payload_json or {})
        payload_state.pop("_resume_completed_steps", None)
        payload_state.pop("_resume_current_version_id", None)
        job.operation_request.payload_json = payload_state
    db.commit()
    return current_version


def run_interactive_commit_operation(
    db: Session,
    source_version: ModelVersionRecord,
    job: JobRecord,
    workdir: Path,
    request: InteractiveCommitRequest,
    edited_mesh_source_path: Path,
) -> ModelVersionRecord:
    _mark_job_running(db, job, f"Importing interactive edit from {request.tool_id}")

    if not edited_mesh_source_path.exists():
        raise RuntimeError(f"Edited mesh payload not found: {edited_mesh_source_path}")

    normalized_mesh = workdir / f"{job.id}_interactive_commit.ply"
    to_ply(edited_mesh_source_path, normalized_mesh)
    _record_job_event(db, job.id, "Interactive edit normalized to production mesh", 40)

    version_operation_type = INTERACTIVE_TOOL_TO_OPERATION_TYPE.get(request.tool_id, "interactive_commit")
    new_version = create_version(
        db,
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type=version_operation_type,
        operation_label=request.operation_label,
        status="processing",
    )

    request_payload = {
        **request.model_dump(mode="json"),
        "source_version_id": source_version.id,
        "job_id": job.id,
        "uploaded_mesh_filename": edited_mesh_source_path.name,
    }
    request_payload_path = workdir / f"{job.id}_interactive_edit.json"
    request_payload_path.write_text(json.dumps(request_payload, indent=2), encoding="utf-8")
    register_file_artifact(
        db,
        new_version.id,
        request_payload_path,
        "interactive_edit_payload_json",
        "application/json",
        metadata_json={
            "tool_id": request.tool_id,
            "source_version_id": source_version.id,
            "job_id": job.id,
        },
    )
    upsert_snapshot(db, new_version.id, "interactive_edit", request_payload)
    _record_job_event(db, job.id, "Interactive edit metadata registered", 65)

    _finalize_version(
        db,
        new_version,
        job,
        normalized_mesh,
        workdir,
        completion_message="Interactive edit committed",
    )
    return new_version
