"""Versioned ingest pipeline."""

from __future__ import annotations

from pathlib import Path

from sqlalchemy.orm import Session

from core.logging import get_logger
from domain.models import JobRecord, ModelRecord, ModelVersionRecord
from geometry_sdk import MeshDocument, default_sdk
from geometry_sdk.io.trimesh_adapter import (
    meshlib_object_mesh_scene_payload,
    save_meshlib_object_mesh_mru_scene,
    save_meshlib_object_mesh_scene_json,
)
from services.manufacturability import compute_manufacturability_snapshot
from services.sdk_conversion import to_glb, to_ply, to_stl
from services.versioning import register_file_artifact
from storage.repositories import add_job_event, set_job_status, upsert_snapshot

logger = get_logger(__name__)


def _texture_image_mime_type(texture_path: Path) -> str:
    suffix = texture_path.suffix.lower()
    if suffix in {".jpg", ".jpeg"}:
        return "image/jpeg"
    if suffix in {".tif", ".tiff"}:
        return "image/tiff"
    return "image/png"


def _texture_image_meshlib_metadata(mesh: MeshDocument) -> dict[str, str]:
    if mesh.metadata.get("source") == "rust_mesh_from_obj":
        return {
            "source": "rust_mesh_from_obj_texture",
            "meshlib_reference": "MR::MeshLoad::fromSceneObjFile map_Kd",
            "meshlib_source": str(
                mesh.metadata.get("meshlib_source") or "MeshLib/source/MRMesh/MRMeshLoadObj.cpp"
            ),
        }
    return {
        "source": "rust_mesh_from_ply_texture",
        "meshlib_reference": "MR::loadPly TextureFile",
        "meshlib_source": "MeshLib/source/MRMesh/MRPly.cpp",
    }


def _register_mesh_texture_artifact(db: Session, version_id: str, mesh: MeshDocument):
    artifacts = _register_mesh_texture_artifacts(db, version_id, mesh)
    return artifacts[0] if artifacts else None


def _register_mesh_texture_artifacts(db: Session, version_id: str, mesh: MeshDocument):
    texture_images = mesh.metadata.get("texture_images")
    if not isinstance(texture_images, list):
        return []

    texture_per_face = mesh.metadata.get("texture_per_face")
    if not isinstance(texture_per_face, list):
        texture_per_face = []

    artifacts = []
    texture_count = len([texture for texture in texture_images if isinstance(texture, dict)])
    for texture_index, texture in enumerate(texture_images):
        if not isinstance(texture, dict):
            continue
        resolved_path = texture.get("resolved_path")
        if not resolved_path:
            continue
        texture_path = Path(str(resolved_path))
        if not texture_path.is_file():
            continue
        meshlib_metadata = _texture_image_meshlib_metadata(mesh)
        metadata_json = {
            **meshlib_metadata,
            "texture_index": texture_index,
            "texture_count": texture_count,
            "texture_per_face": [int(texture_id) for texture_id in texture_per_face],
            "file": str(texture.get("file") or texture_path.name),
            "width": int(texture.get("width") or 0),
            "height": int(texture.get("height") or 0),
            "filter": str(texture.get("filter") or "Linear"),
            "wrap": str(texture.get("wrap") or "Clamp"),
        }
        artifacts.append(
            register_file_artifact(
                db,
                version_id,
                texture_path,
                "texture_image",
                _texture_image_mime_type(texture_path),
                metadata_json=metadata_json,
            )
        )
    return artifacts


def _register_meshlib_object_mesh_scene_artifact(
    db: Session,
    version_id: str,
    mesh: MeshDocument,
    workdir: Path,
    *,
    object_name: str,
    model_extension: str = ".ply",
):
    payload = meshlib_object_mesh_scene_payload(
        mesh,
        object_name=object_name,
        child_index=0,
        model_extension=model_extension,
    )
    scene_path = workdir / f"{version_id}_meshlib_object_mesh_scene.json"
    save_meshlib_object_mesh_scene_json(
        mesh,
        scene_path,
        object_name=object_name,
        child_index=0,
        model_extension=model_extension,
    )
    return register_file_artifact(
        db,
        version_id,
        scene_path,
        "meshlib_object_mesh_scene_json",
        "application/json",
        metadata_json={
            "source": "rust_meshlib_object_mesh_scene_json",
            "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeFields_",
            "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
            "object_type": "ObjectMesh",
            "model_file": str(payload["ModelFile"]),
        },
    )


def _register_meshlib_mru_scene_artifact(
    db: Session,
    version_id: str,
    mesh: MeshDocument,
    workdir: Path,
    *,
    object_name: str,
    model_path: Path,
    model_extension: str = ".ply",
):
    payload = meshlib_object_mesh_scene_payload(
        mesh,
        object_name=object_name,
        child_index=0,
        model_extension=model_extension,
    )
    object_key = str(payload["Key"])
    normalized_extension = model_extension if model_extension.startswith(".") else f".{model_extension}"
    mru_path = workdir / f"{version_id}_meshlib_scene.mru"
    save_meshlib_object_mesh_mru_scene(
        mesh,
        mru_path,
        object_name=object_name,
        model_path=model_path,
        child_index=0,
        model_extension=model_extension,
    )
    return register_file_artifact(
        db,
        version_id,
        mru_path,
        "meshlib_scene_mru",
        "application/zip",
        metadata_json={
            "source": "rust_meshlib_scene_mru",
            "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeModel_",
            "meshlib_source": "MeshLib/source/MRMesh/MRObjectSave.cpp;MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
            "object_type": "ObjectMesh",
            "root_file": "Root.json",
            "root_key": "0_Root",
            "object_key": object_key,
            "model_file": f"0_Root/{object_key}{normalized_extension}",
        },
    )


def run_ingest_pipeline(
    db: Session,
    model: ModelRecord,
    version: ModelVersionRecord,
    job: JobRecord,
    source_path: Path,
) -> None:
    """Materialize upload artifacts and compute the baseline manufacturability snapshot."""
    add_job_event(db, job.id, "Ingest started", 5)
    set_job_status(db, job, "running", progress_pct=5)

    register_file_artifact(
        db,
        version.id,
        source_path,
        artifact_type="original_upload",
        metadata_json={"source_filename": model.source_filename},
    )
    add_job_event(db, job.id, "Stored original upload", 15)
    source_mesh = default_sdk.load_mesh(source_path)

    workdir = source_path.parent
    normalized_ply = workdir / f"{version.id}.ply"
    preview_glb_high = workdir / f"{version.id}_high.glb"
    preview_glb_low = workdir / f"{version.id}_low.glb"
    manufacturing_stl = workdir / f"{version.id}.stl"

    to_ply(source_path, normalized_ply)
    to_glb(normalized_ply, preview_glb_high)
    to_glb(normalized_ply, preview_glb_low)
    to_stl(normalized_ply, manufacturing_stl)
    add_job_event(db, job.id, "Generated normalized and preview artifacts", 45)

    register_file_artifact(db, version.id, normalized_ply, "normalized_mesh_ply", "model/ply")
    register_file_artifact(db, version.id, preview_glb_high, "preview_glb_high", "model/gltf-binary")
    register_file_artifact(db, version.id, preview_glb_low, "preview_glb_low", "model/gltf-binary")
    register_file_artifact(db, version.id, manufacturing_stl, "manufacturing_stl", "application/sla")
    scene_artifact = _register_meshlib_object_mesh_scene_artifact(
        db,
        version.id,
        source_mesh,
        workdir,
        object_name=Path(model.source_filename).stem or version.id,
        model_extension=".ply",
    )
    mru_scene_artifact = _register_meshlib_mru_scene_artifact(
        db,
        version.id,
        source_mesh,
        workdir,
        object_name=Path(model.source_filename).stem or version.id,
        model_path=normalized_ply,
        model_extension=".ply",
    )
    texture_artifacts = _register_mesh_texture_artifacts(db, version.id, source_mesh)
    if scene_artifact:
        add_job_event(db, job.id, "Registered MeshLib object scene artifact", 56)
    if mru_scene_artifact:
        add_job_event(db, job.id, "Registered MeshLib MRU scene package", 57)
    if texture_artifacts:
        add_job_event(db, job.id, "Registered texture artifacts", 58)
    add_job_event(db, job.id, "Registered mesh artifacts", 60)

    snapshot, snapshot_artifacts = compute_manufacturability_snapshot(normalized_ply, workdir)
    thickness_artifact = register_file_artifact(
        db,
        version.id,
        snapshot_artifacts.thickness_scalar_path,
        "analysis_thickness_npz",
        "application/octet-stream",
    )
    register_file_artifact(
        db,
        version.id,
        snapshot_artifacts.region_json_path,
        "analysis_regions_json",
        "application/json",
    )
    snapshot.version_id = version.id
    snapshot.thickness.scalar_field_artifact_id = thickness_artifact.id
    upsert_snapshot(db, version.id, "manufacturability", snapshot.model_dump(mode="json"))
    add_job_event(db, job.id, "Manufacturability snapshot computed", 90)

    version.status = "ready"
    set_job_status(db, job, "succeeded", progress_pct=100)
    add_job_event(db, job.id, "Ingest completed", 100)
    db.commit()
    logger.info(f"Ingest pipeline completed for version {version.id}")
