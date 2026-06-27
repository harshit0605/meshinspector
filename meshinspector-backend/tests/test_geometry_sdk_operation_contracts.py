from __future__ import annotations

import asyncio
import ast
import base64
from datetime import datetime, timezone
import io
import json
from pathlib import Path
import re
from types import SimpleNamespace
import zipfile
import zlib

from fastapi import HTTPException
import numpy as np
from PIL import Image
import pytest

from api.routers import jobs as jobs_router
from api.routers import versions as versions_router
from api.routers.versions import _selection_counts, _selection_has_content
from domain.models import AnalysisSnapshotRecord, JobRecord, ModelArtifactRecord, ModelVersionRecord, OperationRequestRecord
from domain.schemas import (
    BrushReplayRequest,
    BrushReplayStroke,
    DecimateRequest,
    HollowRequest,
    InteractiveSelectionPayload,
    MakeDeloneRequest,
    MakeManufacturableRequest,
    MeasureInspectFeaturePair,
    MeasureInspectFeaturePrimitive,
    MeasureInspectFeatureRefineRequest,
    MeasureInspectPair,
    MeasureInspectRequest,
    MeasureInspectSurfaceDistanceRequest,
    MeshCutMeasureTopologyRequest,
    SelectionCommitRequest,
    ScoopRequest,
    SmoothRequest,
    SubdivideRequest,
    ThickenRequest,
)
from geometry_sdk import DecimateMeshResult, MeshDocument, PointCloudDocument, RegionEntry, SubdivideMeshResult
from geometry_sdk.testing.fixtures import closed_cube_with_flipped_top_triangle, crossing_triangles, cube
from geometry_sdk.testing.openvdb import synthetic_openvdb_single_dense_leaf
from services import ingest as ingest_service
from services import operations as operations_service
from services.operations import (
    _bounded_seed_indices,
    _brush_replay_stroke_to_sdk,
    _preserved_resize_region_ids,
    _region_indices_union,
    _region_ids_allowed_for_operation,
    _selection_seed_indices,
)


def _minimal_snapshot_payload(version_id: str) -> dict:
    return {
        "version_id": version_id,
        "mesh_health": {
            "is_closed": True,
            "holes_count": 0,
            "self_intersections": 0,
            "disconnected_shells": 1,
            "health_score": 100,
        },
        "dimensions": {
            "unit_system": "mm",
            "ring_axis": None,
            "ring_axis_confidence": 0.0,
            "estimated_ring_size_us": None,
            "inner_diameter_mm": None,
            "band_width_min_mm": None,
            "band_width_max_mm": None,
            "head_height_mm": None,
            "bbox_mm": [1.0, 2.0, 3.0],
            "needs_axis_confirmation": False,
        },
        "material_weight": {
            "gold_18k": {
                "volume_mm3": 1.0,
                "weight_g": 0.015,
            }
        },
        "thickness": {
            "min_mm": None,
            "avg_mm": None,
            "max_mm": None,
            "violation_count": 0,
            "threshold_mm": 0.8,
            "scalar_field_artifact_id": None,
        },
        "regions": [],
        "recommendations": [],
        "export_ready": True,
    }


def test_region_operation_guard_filters_duplicates_and_rejects_disallowed_regions() -> None:
    payload = {
        "regions": [
            {"region_id": "inner_band", "allowed_operations": ["scoop", "thicken", "smooth"], "vertex_indices": [0, 1]},
            {"region_id": "outer_band", "allowed_operations": ["thicken", "smooth"], "vertex_indices": [2, 3]},
            {"region_id": "unknown", "allowed_operations": [], "vertex_indices": [4]},
        ]
    }

    assert _region_ids_allowed_for_operation(payload, ["inner_band", "outer_band", "inner_band"], "thicken") == [
        "inner_band",
        "outer_band",
    ]

    with pytest.raises(RuntimeError, match="Region unknown does not allow smooth"):
        _region_ids_allowed_for_operation(payload, ["unknown"], "smooth")

    with pytest.raises(RuntimeError, match="Region missing not found"):
        _region_ids_allowed_for_operation(payload, ["missing"], "thicken")


def test_resize_preservation_regions_match_ui_protected_detail_regions() -> None:
    payload = {
        "regions": [
            {"region_id": "inner_band", "vertex_indices": [0, 1]},
            {"region_id": "head", "vertex_indices": [2, 3]},
            {"region_id": "gem_seat", "vertex_indices": [4, 5]},
            {"region_id": "ornament_relief", "vertex_indices": [6, 7]},
            {"region_id": "unknown", "vertex_indices": [8]},
        ]
    }

    assert _preserved_resize_region_ids(payload) == ["head", "gem_seat", "ornament_relief"]


def test_resize_preservation_can_skip_empty_protected_detail_regions() -> None:
    payload = {
        "regions": [
            {"region_id": "head", "vertex_indices": []},
            {"region_id": "gem_seat", "vertex_indices": []},
            {"region_id": "ornament_relief", "vertex_indices": []},
        ]
    }

    assert _region_indices_union(
        payload,
        ["head", "gem_seat", "ornament_relief"],
        require_non_empty=False,
    ) is None

    with pytest.raises(RuntimeError, match="Region head has no vertices"):
        _region_indices_union(payload, ["head"], require_non_empty=True)


def test_viewer_section_presets_cover_ui_protected_detail_regions() -> None:
    viewer_page = Path(__file__).resolve().parents[2] / "meshinspector-frontend/src/app/viewer/page.tsx"
    source = viewer_page.read_text(encoding="utf-8")
    match = re.search(r"for \(const regionId of (\[[^\]]+\]) as const\)", source)
    assert match is not None
    preset_region_ids = ast.literal_eval(match.group(1))

    assert {"head", "gem_seat", "ornament_relief"}.issubset(preset_region_ids)


def test_viewer_region_overlay_colors_cover_ui_protected_detail_regions() -> None:
    viewer_engine = Path(__file__).resolve().parents[2] / "meshinspector-frontend/src/features/editor/viewer/ViewerEngine.tsx"
    source = viewer_engine.read_text(encoding="utf-8")

    for region_id in ("head", "gem_seat", "ornament_relief"):
        assert re.search(rf"\b{region_id}\s*:", source)


def test_viewer_engine_applies_meshlib_texture_artifact_to_mesh_materials() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    viewer_page = repo_root / "meshinspector-frontend/src/app/viewer/page.tsx"
    viewer_engine = repo_root / "meshinspector-frontend/src/features/editor/viewer/ViewerEngine.tsx"
    page_source = viewer_page.read_text(encoding="utf-8")
    engine_source = viewer_engine.read_text(encoding="utf-8")

    assert "const textureArtifactUrl = useMemo" in page_source
    assert "textureArtifactUrl={textureArtifactUrl}" in page_source
    assert "textureMetadata={viewerQuery.data?.texture_metadata ?? {}}" in page_source
    assert "textureArtifactUrl?: string | null;" in engine_source
    assert "textureMetadata?: Record<string, unknown>;" in engine_source
    assert "textureArtifactUrl" in engine_source
    assert "useLoader(THREE.TextureLoader, textureUrls)" in engine_source
    assert "texture.minFilter = THREE.LinearFilter" in engine_source
    assert "texture.magFilter = THREE.LinearFilter" in engine_source
    assert "texture.wrapS = THREE.ClampToEdgeWrapping" in engine_source
    assert "texture.wrapT = THREE.ClampToEdgeWrapping" in engine_source
    assert "material.map = texture" in engine_source
    assert "material.color.set(0xffffff)" in engine_source


def test_viewer_engine_applies_meshlib_texture_per_face_artifacts_to_material_groups() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    viewer_page = repo_root / "meshinspector-frontend/src/app/viewer/page.tsx"
    viewer_engine = repo_root / "meshinspector-frontend/src/features/editor/viewer/ViewerEngine.tsx"
    page_source = viewer_page.read_text(encoding="utf-8")
    engine_source = viewer_engine.read_text(encoding="utf-8")

    assert "const textureArtifacts = useMemo" in page_source
    assert "textureArtifacts={textureArtifacts}" in page_source
    assert "texturePerFace={viewerQuery.data?.texture_per_face ?? []}" in page_source
    assert "textureArtifacts?: TextureArtifactManifest[];" in engine_source
    assert "texturePerFace?: number[];" in engine_source
    assert "textureEntries.map((texture) => texture.artifact_url)" in engine_source
    assert "applyMeshLibTexturePerFaceGroups" in engine_source
    assert "geometry.clearGroups()" in engine_source
    assert "geometry.addGroup(faceIndex * 3, 3, materialIndex)" in engine_source
    assert "const textureId = texturePerFace[faceIndex] ?? 0" in engine_source


def test_viewer_engine_uses_meshlib_texture_array_shader_before_material_group_fallback() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    viewer_engine = repo_root / "meshinspector-frontend/src/features/editor/viewer/ViewerEngine.tsx"
    engine_source = viewer_engine.read_text(encoding="utf-8")

    assert "createMeshLibTextureArray" in engine_source
    assert "new THREE.DataArrayTexture" in engine_source
    assert "createMeshLibTexturePerFaceTexture" in engine_source
    assert "new THREE.DataTexture" in engine_source
    assert "THREE.RedIntegerFormat" in engine_source
    assert "createMeshLibTextureArrayMaterial" in engine_source
    assert "glslVersion: THREE.GLSL3" in engine_source
    assert "uniform highp sampler2DArray tex;" in engine_source
    assert "uniform highp usampler2D texturePerFace;" in engine_source
    assert "texelFetch(texturePerFace" in engine_source
    assert "texture(tex, vec3(vUv, float(textId)))" in engine_source
    assert "applyMeshLibTextureArrayShader" in engine_source
    assert "applyMeshLibTexturePerFaceGroups" in engine_source
    assert engine_source.index("applyMeshLibTextureArrayShader") < engine_source.index(
        "applyMeshLibTexturePerFaceGroups"
    )


def test_meshlib_object_mesh_scene_payload_matches_object_mesh_holder_fields() -> None:
    from geometry_sdk.accelerators import _rust_common
    from geometry_sdk.io.trimesh_adapter import meshlib_object_mesh_scene_payload

    assert _rust_common._rs is not None
    assert hasattr(_rust_common._rs, "meshlib_object_mesh_scene_payload")

    mesh = MeshDocument(
        vertices=np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
        metadata={
            "texture_images": [
                {
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[255, 0, 0, 255]],
                },
                {
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[0, 0, 255, 255]],
                },
            ],
            "texture_per_face": [0, 1],
            "tri_corner_uvs": [
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            ],
        },
    )

    payload = meshlib_object_mesh_scene_payload(
        mesh,
        object_name='Ring/Scene:Object*With?VeryLongName',
        child_index=2,
        model_extension=".ply",
    )

    assert payload["FormatVersion"] == 1.0
    assert payload["Key"] == "2_Ring_Scene_O"
    assert payload["ModelFile"] == "2_Ring_Scene_O.ply"
    assert payload["Name"] == 'Ring/Scene:Object*With?VeryLongName'
    assert payload["Type"] == ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
    assert payload["ShowTexture"] == 4294967295
    assert payload["ShowFaces"] == 4294967295
    assert payload["ShowLines"] == 0
    assert payload["ColoringType"] == "Solid"
    assert payload["TextureCount"] == 2
    assert payload["Textures"]["0"]["FilterType"] == "Linear"
    assert payload["Textures"]["0"]["WrapType"] == "Clamp"
    assert payload["Textures"]["0"]["Resolution"] == {"x": 1, "y": 1}
    assert base64.b64decode(payload["Textures"]["0"]["Data"]) == bytes([255, 0, 0, 255])

    texture_per_face = np.frombuffer(
        base64.b64decode(payload["TexturePerFace"]["Data"]),
        dtype="<i4",
    )
    assert payload["TexturePerFace"]["Size"] == 2
    assert texture_per_face.tolist() == [0, 1]

    uv_coordinates = np.frombuffer(
        base64.b64decode(payload["UVCoordinates"]["Data"]),
        dtype="<f4",
    )
    assert payload["UVCoordinates"]["Size"] == 6
    np.testing.assert_allclose(
        uv_coordinates.reshape((-1, 2)),
        np.array(
            [
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
            ],
            dtype=np.float32,
        ),
    )
    assert payload["meshlib_reference"] == "MR::ObjectMeshHolder::serializeFields_"
    assert payload["meshlib_source_language"] == "rust"


def test_ingest_registers_meshlib_object_mesh_scene_json_artifact(monkeypatch, tmp_path) -> None:
    registered: list[dict[str, object]] = []

    def fake_register_file_artifact(
        db,
        version_id,
        file_path,
        artifact_type,
        mime_type=None,
        metadata_json=None,
    ):
        registered.append(
            {
                "db": db,
                "version_id": version_id,
                "file_path": Path(file_path),
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json or {},
            }
        )
        return SimpleNamespace(
            id="art_scene",
            artifact_type=artifact_type,
            mime_type=mime_type,
            storage_key=f"{version_id}/{artifact_type}.json",
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(ingest_service, "register_file_artifact", fake_register_file_artifact)
    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={"texture_per_face": [0]},
    )

    artifact = ingest_service._register_meshlib_object_mesh_scene_artifact(
        None,
        "ver_scene",
        mesh,
        tmp_path,
        object_name="Ring/Scene",
        model_extension=".ply",
    )

    assert artifact.id == "art_scene"
    assert len(registered) == 1
    record = registered[0]
    assert record["artifact_type"] == "meshlib_object_mesh_scene_json"
    assert record["mime_type"] == "application/json"
    assert record["metadata_json"] == {
        "source": "rust_meshlib_object_mesh_scene_json",
        "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
        "object_type": "ObjectMesh",
        "model_file": "0_Ring_Scene.ply",
    }
    payload = json.loads(Path(record["file_path"]).read_text(encoding="utf-8"))
    assert payload["Key"] == "0_Ring_Scene"
    assert payload["ModelFile"] == "0_Ring_Scene.ply"
    assert payload["Type"] == ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
    assert payload["meshlib_source_language"] == "rust"
    assert payload["TexturePerFace"]["Size"] == 1


def test_meshlib_object_mesh_mru_scene_matches_serialize_object_tree_layout(tmp_path) -> None:
    from geometry_sdk.accelerators import _rust_common
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    assert _rust_common._rs is not None
    assert hasattr(_rust_common._rs, "meshlib_object_mesh_mru_scene")

    mesh = MeshDocument(
        vertices=np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={
            "texture_per_face": [0],
            "tri_corner_uvs": [[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]],
        },
    )
    model_path = save_mesh(mesh, tmp_path / "normalized.ply", file_type="ply")
    scene_path = save_meshlib_object_mesh_mru_scene(
        mesh,
        tmp_path / "scene.mru",
        object_name='Ring/Scene:Object*With?VeryLongName',
        model_path=model_path,
        model_extension=".ply",
    )

    with zipfile.ZipFile(scene_path) as archive:
        assert sorted(archive.namelist()) == ["0_Root/0_Ring_Scene_O.ply", "Root.json"]
        assert archive.read("0_Root/0_Ring_Scene_O.ply") == Path(model_path).read_bytes()
        root = json.loads(archive.read("Root.json").decode("utf-8"))

    assert root["FormatVersion"] == 1.0
    assert root["Name"] == "Root"
    assert root["Key"] == "0_Root"
    assert root["Type"] == ["Object", "RootObject"]
    child = root["Children"]["0"]
    assert child["Name"] == 'Ring/Scene:Object*With?VeryLongName'
    assert child["Key"] == "0_Ring_Scene_O"
    assert child["Type"] == ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
    assert child["TexturePerFace"]["Size"] == 1
    assert child["UVCoordinates"]["Size"] == 3
    assert child["meshlib_source_language"] == "rust"
    assert "ModelFile" not in child


def test_ingest_registers_meshlib_mru_scene_artifact(monkeypatch, tmp_path) -> None:
    registered: list[dict[str, object]] = []

    def fake_register_file_artifact(
        db,
        version_id,
        file_path,
        artifact_type,
        mime_type=None,
        metadata_json=None,
    ):
        registered.append(
            {
                "db": db,
                "version_id": version_id,
                "file_path": Path(file_path),
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json or {},
            }
        )
        return SimpleNamespace(
            id="art_mru_scene",
            artifact_type=artifact_type,
            mime_type=mime_type,
            storage_key=f"{version_id}/{artifact_type}.mru",
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(ingest_service, "register_file_artifact", fake_register_file_artifact)
    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={},
    )
    model_path = tmp_path / "ver_scene.ply"
    model_path.write_text("ply\nformat ascii 1.0\nend_header\n", encoding="utf-8")

    artifact = ingest_service._register_meshlib_mru_scene_artifact(
        None,
        "ver_scene",
        mesh,
        tmp_path,
        object_name="Ring/Scene",
        model_path=model_path,
        model_extension=".ply",
    )

    assert artifact.id == "art_mru_scene"
    assert len(registered) == 1
    record = registered[0]
    assert record["artifact_type"] == "meshlib_scene_mru"
    assert record["mime_type"] == "application/zip"
    assert record["metadata_json"] == {
        "source": "rust_meshlib_scene_mru",
        "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeModel_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObjectSave.cpp;MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
        "object_type": "ObjectMesh",
        "root_file": "Root.json",
        "root_key": "0_Root",
        "object_key": "0_Ring_Scene",
        "model_file": "0_Root/0_Ring_Scene.ply",
    }
    with zipfile.ZipFile(record["file_path"]) as archive:
        assert sorted(archive.namelist()) == ["0_Root/0_Ring_Scene.ply", "Root.json"]


def test_load_mesh_routes_mru_scene_through_rust_deserialize_object_tree(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.accelerators import _rust_common
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    assert _rust_common._rs is not None
    assert hasattr(_rust_common._rs, "mesh_from_mru_scene")

    scene_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={
            "texture_per_face": [0],
            "tri_corner_uvs": [[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]],
            "texture_images": [
                {
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[10, 20, 30, 255]],
                }
            ],
        },
    )
    plain_model = MeshDocument(
        vertices=scene_mesh.vertices,
        faces=scene_mesh.faces,
        metadata={},
    )
    model_path = save_mesh(plain_model, tmp_path / "plain.ply", file_type="ply")
    scene_path = save_meshlib_object_mesh_mru_scene(
        scene_mesh,
        tmp_path / "scene.mru",
        object_name="Ring/Scene",
        model_path=model_path,
        model_extension=".ply",
    )

    loaded = default_sdk.load_mesh(scene_path)

    np.testing.assert_allclose(loaded.vertices, scene_mesh.vertices)
    np.testing.assert_array_equal(loaded.faces, scene_mesh.faces)
    assert loaded.metadata["source"] == "rust_mesh_from_mru_scene"
    assert loaded.metadata["meshlib_reference"] == "MR::deserializeObjectTree"
    assert loaded.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRObjectLoad.cpp;MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp"
    assert loaded.metadata["source_path"] == str(scene_path)
    assert loaded.metadata["root_file"] == "Root.json"
    assert loaded.metadata["root_key"] == "0_Root"
    assert loaded.metadata["object_key"] == "0_Ring_Scene"
    assert loaded.metadata["object_name"] == "Ring/Scene"
    assert loaded.metadata["model_file"] == "0_Root/0_Ring_Scene.ply"
    assert loaded.metadata["texture_per_face"] == [0]
    assert loaded.metadata["texture_images"][0]["width"] == 1
    assert loaded.metadata["texture_images"][0]["height"] == 1
    assert loaded.metadata["texture_images"][0]["pixels_rgba"] == [[10, 20, 30, 255]]
    np.testing.assert_allclose(
        np.asarray(loaded.metadata["tri_corner_uvs"], dtype=np.float64),
        np.asarray(scene_mesh.metadata["tri_corner_uvs"], dtype=np.float64),
    )
    rust_mesh_bridge = (
        Path(__file__).resolve().parents[1] / "geometry_sdk/accelerators/_rust_mesh_scene.py"
    ).read_text(encoding="utf-8")
    mru_wrapper = re.search(
        r"def mesh_from_mru_scene\(.*\Z",
        rust_mesh_bridge,
        flags=re.S,
    )
    assert mru_wrapper is not None
    assert "mesh_from_ply(" not in mru_wrapper.group(0)
    assert "mesh_from_obj(" not in mru_wrapper.group(0)


def test_load_mesh_routes_multi_object_mru_scene_hierarchy_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh

    mesh_a = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    mesh_b = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    model_a = save_mesh(mesh_a, tmp_path / "a.ply", file_type="ply")
    model_b = save_mesh(mesh_b, tmp_path / "b.ply", file_type="ply")
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "10": {
                "Name": "Translated B",
                "Key": "1_Translated",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 10.0, "y": 0.0, "z": 0.0},
                },
            },
            "2": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
        },
    }
    scene_path = tmp_path / "multi.mru"
    with zipfile.ZipFile(scene_path, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(model_a, "0_Root/0_Base_A.ply")
        archive.write(model_b, "0_Root/1_Translated.ply")

    loaded = default_sdk.load_mesh(scene_path)

    assert loaded.metadata["source"] == "rust_mesh_from_mru_scene"
    assert loaded.metadata["scene_object_count"] == 2
    np.testing.assert_allclose(
        loaded.vertices,
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.5, 0.0, 0.0],
                [10.0, 0.5, 0.0],
            ],
            dtype=np.float64,
        ),
    )
    np.testing.assert_array_equal(loaded.faces, np.array([[0, 1, 2], [3, 4, 5]], dtype=np.int64))
    scene_objects = loaded.metadata["scene_objects"]
    assert [scene_object["object_name"] for scene_object in scene_objects] == ["Base A", "Translated B"]
    assert [scene_object["object_key"] for scene_object in scene_objects] == ["0_Base_A", "1_Translated"]
    assert [scene_object["model_file"] for scene_object in scene_objects] == [
        "0_Root/0_Base_A.ply",
        "0_Root/1_Translated.ply",
    ]
    assert [scene_object["hierarchy_path"] for scene_object in scene_objects] == [
        ["0_Root", "0_Base_A"],
        ["0_Root", "1_Translated"],
    ]
    assert [scene_object["vertex_range"] for scene_object in scene_objects] == [[0, 3], [3, 6]]
    assert [scene_object["face_range"] for scene_object in scene_objects] == [[0, 1], [1, 2]]
    assert scene_objects[1]["xf"]["b"] == [10.0, 0.0, 0.0]


def test_load_mesh_preserves_mru_shared_model_links_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh

    shared_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    shared_model = save_mesh(shared_mesh, tmp_path / "shared.ply", file_type="ply")
    linked_child = {
        "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
        "Link": "SharedModels/0_Shared",
    }
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                **linked_child,
                "Name": "Shared A",
                "Key": "0_Shared_A",
            },
            "1": {
                **linked_child,
                "Name": "Shared B",
                "Key": "1_Shared_B",
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 0.0, "y": 5.0, "z": 0.0},
                },
            },
        },
    }
    scene_path = tmp_path / "shared.mru"
    with zipfile.ZipFile(scene_path, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(shared_model, "SharedModels/0_Shared.ply")

    loaded = default_sdk.load_mesh(scene_path)

    np.testing.assert_allclose(
        loaded.vertices,
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 5.0, 0.0],
                [1.0, 5.0, 0.0],
                [0.0, 6.0, 0.0],
            ],
            dtype=np.float64,
        ),
    )
    scene_objects = loaded.metadata["scene_objects"]
    assert [scene_object["link"] for scene_object in scene_objects] == [
        "SharedModels/0_Shared",
        "SharedModels/0_Shared",
    ]
    assert scene_objects[0]["shared_model_source_index"] is None
    assert scene_objects[1]["shared_model_source_index"] == 0
    assert [scene_object["model_file"] for scene_object in scene_objects] == [
        "SharedModels/0_Shared.ply",
        "SharedModels/0_Shared.ply",
    ]


def test_save_meshlib_mru_scene_round_trips_multi_object_hierarchy_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    translated_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    translated_model = save_mesh(translated_mesh, tmp_path / "translated.ply", file_type="ply")
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Translated B",
                "Key": "1_Translated",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 4.0, "y": 0.0, "z": 0.0},
                },
            },
        },
    }
    source_scene = tmp_path / "source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.write(translated_model, "0_Root/1_Translated.ply")

    loaded = default_sdk.load_mesh(source_scene)
    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            "0_Root/1_Translated.ply",
            "Root.json",
        ]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert [root_payload["Children"][str(index)]["Name"] for index in (0, 1)] == [
        "Base A",
        "Translated B",
    ]
    assert root_payload["Children"]["1"]["XF"]["b"] == {"x": 4.0, "y": 0.0, "z": 0.0}

    reloaded = default_sdk.load_mesh(round_trip_scene)
    np.testing.assert_allclose(reloaded.vertices, loaded.vertices)
    np.testing.assert_array_equal(reloaded.faces, loaded.faces)
    assert [scene_object["object_name"] for scene_object in reloaded.metadata["scene_objects"]] == [
        "Base A",
        "Translated B",
    ]
    assert reloaded.metadata["scene_objects"][1]["xf"]["b"] == [4.0, 0.0, 0.0]


def test_save_meshlib_mru_scene_preserves_object_lines_type_management_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Profile Lines",
                "Key": "1_Profile_Lines",
                "Visibility": 0,
                "Selected": True,
                "Locked": True,
                "ParentLocked": False,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0},
                },
                "Type": ["Object", "VisualObject", "LinesHolder", "ObjectLines"],
                "ShowPoints": 0xFFFF_FFFF,
                "SmoothConnections": 0,
                "ColoringType": "PerLine",
                "LineColors": [],
                "VertColors": [],
                "LineWidth": 2.5,
                "Polyline": {
                    "Points": [
                        {"x": 0.0, "y": 0.0, "z": 0.0},
                        {"x": 1.0, "y": 0.0, "z": 0.0},
                        {"x": 1.0, "y": 1.0, "z": 0.0},
                    ],
                    "Lines": [0, 1, 1, 2],
                },
            },
        },
    }
    source_scene = tmp_path / "lines_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_line_object_count"] == 1
    line_object = loaded.metadata["scene_line_objects"][0]
    assert line_object["object_name"] == "Profile Lines"
    assert line_object["object_key"] == "1_Profile_Lines"
    assert line_object["parent_key"] == "0_Root"
    assert line_object["hierarchy_path"] == ["0_Root", "1_Profile_Lines"]
    assert line_object["points"] == [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    assert line_object["lines"] == [[0, 1], [1, 2]]
    assert line_object["show_points"] == 0xFFFF_FFFF
    assert line_object["smooth_connections"] == 0
    assert line_object["coloring_type"] == "PerLine"
    assert line_object["line_width"] == 2.5
    assert line_object["visibility_mask"] == 0
    assert line_object["selected"] is True
    assert line_object["locked"] is True
    assert line_object["parent_locked"] is False

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "lines_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == ["0_Root/0_Base_A.ply", "Root.json"]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    exported_lines = root_payload["Children"]["1"]
    assert exported_lines["Name"] == "Profile Lines"
    assert exported_lines["Type"] == ["Object", "VisualObject", "LinesHolder", "ObjectLines"]
    assert exported_lines["Polyline"]["Lines"] == [0, 1, 1, 2]
    assert exported_lines["Polyline"]["Points"][2]["y"] == 1.0
    assert exported_lines["ShowPoints"] == 0xFFFF_FFFF
    assert exported_lines["LineWidth"] == 2.5
    assert exported_lines["ColoringType"] == "PerLine"

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_line_object_count"] == 1
    assert reloaded.metadata["scene_line_objects"][0]["lines"] == [[0, 1], [1, 2]]


def test_save_meshlib_mru_scene_preserves_object_points_type_management_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    point_cloud_ply = (
        "ply\n"
        "format ascii 1.0\n"
        "comment MeshInspector.com\n"
        "element vertex 3\n"
        "property float x\n"
        "property float y\n"
        "property float z\n"
        "property float nx\n"
        "property float ny\n"
        "property float nz\n"
        "property uchar red\n"
        "property uchar green\n"
        "property uchar blue\n"
        "end_header\n"
        "0 0 0 0 0 1 255 0 0\n"
        "1 0 0 0 1 0 0 255 0\n"
        "1 1 0 1 0 0 0 0 255\n"
    )
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Probe Points",
                "Key": "1_Probe_Points",
                "Visibility": 0,
                "Selected": True,
                "Locked": True,
                "ParentLocked": False,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0},
                },
                "Type": ["Object", "VisualObject", "PointsHolder", "ObjectPoints"],
                "Colors": {"Selection": {"Points": {"x": 1.0, "y": 0.7, "z": 0.0, "w": 1.0}}},
                "SelectionVertBitSet": {},
                "ValidVertBitSet": {},
                "PointSize": 7.0,
                "MaxRenderingPoints": 123,
            },
        },
    }
    source_scene = tmp_path / "points_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr("0_Root/1_Probe_Points.ply", point_cloud_ply)

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_point_object_count"] == 1
    point_object = loaded.metadata["scene_point_objects"][0]
    assert point_object["object_name"] == "Probe Points"
    assert point_object["object_key"] == "1_Probe_Points"
    assert point_object["parent_key"] == "0_Root"
    assert point_object["hierarchy_path"] == ["0_Root", "1_Probe_Points"]
    assert point_object["model_file"] == "0_Root/1_Probe_Points.ply"
    assert point_object["model_extension"] == ".ply"
    assert point_object["points"] == [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    assert point_object["normals"] == [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]]
    assert point_object["vert_colors"] == [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
    assert point_object["point_size"] == 7.0
    assert point_object["max_rendering_points"] == 123
    assert point_object["visibility_mask"] == 0
    assert point_object["selected"] is True
    assert point_object["locked"] is True
    assert point_object["parent_locked"] is False

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "points_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            "0_Root/1_Probe_Points.ply",
            "Root.json",
        ]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    exported_points = root_payload["Children"]["1"]
    assert exported_points["Name"] == "Probe Points"
    assert exported_points["Type"] == ["Object", "VisualObject", "PointsHolder", "ObjectPoints"]
    assert exported_points["PointSize"] == 7.0
    assert exported_points["MaxRenderingPoints"] == 123

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_point_object_count"] == 1
    assert reloaded.metadata["scene_point_objects"][0]["points"][2] == [1.0, 1.0, 0.0]


def test_save_meshlib_mru_scene_preserves_object_distance_map_type_management_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    invalid_value = float(np.finfo(np.float32).min)
    raw_distance_map = np.array([2, 2], dtype="<u8").tobytes() + np.array(
        [0.0, 1.0, invalid_value, 2.5],
        dtype="<f4",
    ).tobytes()
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Depth Map",
                "Key": "1_Depth_Map",
                "Visibility": 0,
                "Selected": True,
                "Locked": True,
                "ParentLocked": False,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0},
                },
                "Type": ["Object", "VisualObject", "ObjectDistanceMap"],
                "PixelXVec": {"x": 0.5, "y": 0.0, "z": 0.0},
                "PixelYVec": {"x": 0.0, "y": 0.25, "z": 0.0},
                "DepthVec": {"x": 0.0, "y": 0.0, "z": 1.5},
                "OriginWorld": {"x": 1.0, "y": 2.0, "z": 3.0},
            },
        },
    }
    source_scene = tmp_path / "distance_map_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr("0_Root/1_Depth_Map.raw", raw_distance_map)

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_distance_map_object_count"] == 1
    distance_map_object = loaded.metadata["scene_distance_map_objects"][0]
    assert distance_map_object["object_name"] == "Depth Map"
    assert distance_map_object["object_key"] == "1_Depth_Map"
    assert distance_map_object["parent_key"] == "0_Root"
    assert distance_map_object["hierarchy_path"] == ["0_Root", "1_Depth_Map"]
    assert distance_map_object["model_file"] == "0_Root/1_Depth_Map.raw"
    assert distance_map_object["model_extension"] == ".raw"
    assert distance_map_object["width"] == 2
    assert distance_map_object["height"] == 2
    assert distance_map_object["values"] == [0.0, 1.0, invalid_value, 2.5]
    assert distance_map_object["valid_count"] == 3
    assert distance_map_object["min_value"] == 0.0
    assert distance_map_object["max_value"] == 2.5
    assert distance_map_object["pixel_x_vec"] == [0.5, 0.0, 0.0]
    assert distance_map_object["pixel_y_vec"] == [0.0, 0.25, 0.0]
    assert distance_map_object["depth_vec"] == [0.0, 0.0, 1.5]
    assert distance_map_object["origin_world"] == [1.0, 2.0, 3.0]
    assert distance_map_object["visibility_mask"] == 0
    assert distance_map_object["selected"] is True
    assert distance_map_object["locked"] is True
    assert distance_map_object["parent_locked"] is False

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "distance_map_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            "0_Root/1_Depth_Map.raw",
            "Root.json",
        ]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    exported_distance_map = root_payload["Children"]["1"]
    assert exported_distance_map["Name"] == "Depth Map"
    assert exported_distance_map["Type"] == ["Object", "VisualObject", "ObjectDistanceMap"]
    assert exported_distance_map["PixelXVec"]["x"] == 0.5
    assert exported_distance_map["PixelYVec"]["y"] == 0.25
    assert exported_distance_map["DepthVec"]["z"] == 1.5
    assert exported_distance_map["OriginWorld"]["z"] == 3.0

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_distance_map_object_count"] == 1
    assert reloaded.metadata["scene_distance_map_objects"][0]["values"][3] == 2.5


def test_save_meshlib_mru_scene_preserves_object_voxels_type_management_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    raw_voxels = np.array([0.0, 0.5, 1.0, 1.5], dtype="<f4").tobytes()
    voxel_file = "0_Root/W2_H2_S1_V500_250_1000_G1_F 1_Scan_Voxels.raw"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Visibility": 0,
                "Selected": True,
                "Locked": True,
                "ParentLocked": False,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0},
                },
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 0.5, "y": 0.25, "z": 1.0},
                "Dimensions": {"x": 2, "y": 2, "z": 1},
                "MinCorner": {"x": 0, "y": 0, "z": 0},
                "MaxCorner": {"x": 2, "y": 2, "z": 1},
                "SelectionVoxels": {"size": 4, "bits": "CgAAAAAAAAA="},
                "IsoValue": 0.75,
                "DualMarchingCubes": False,
            },
        },
    }
    source_scene = tmp_path / "voxels_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, raw_voxels)

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_voxel_object_count"] == 1
    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["object_name"] == "Scan Voxels"
    assert voxel_object["object_key"] == "1_Scan_Voxels"
    assert voxel_object["parent_key"] == "0_Root"
    assert voxel_object["hierarchy_path"] == ["0_Root", "1_Scan_Voxels"]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".raw"
    assert voxel_object["dimensions"] == [2, 2, 1]
    assert voxel_object["voxel_size"] == [0.5, 0.25, 1.0]
    assert voxel_object["grid_level_set"] is True
    assert voxel_object["values"] == [0.0, 0.5, 1.0, 1.5]
    assert voxel_object["min_value"] == 0.0
    assert voxel_object["max_value"] == 1.5
    assert voxel_object["min_corner"] == [0, 0, 0]
    assert voxel_object["max_corner"] == [2, 2, 1]
    assert voxel_object["iso_value"] == 0.75
    assert voxel_object["dual_marching_cubes"] is False
    assert voxel_object["selected_voxels"] == [1, 3]
    assert voxel_object["visibility_mask"] == 0
    assert voxel_object["selected"] is True
    assert voxel_object["locked"] is True
    assert voxel_object["parent_locked"] is False

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "voxels_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            voxel_file,
            "Root.json",
        ]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    exported_voxels = root_payload["Children"]["1"]
    assert exported_voxels["Name"] == "Scan Voxels"
    assert exported_voxels["Type"] == ["Object", "VisualObject", "ObjectVoxels"]
    assert exported_voxels["VoxelSize"]["x"] == 0.5
    assert exported_voxels["Dimensions"]["y"] == 2
    assert exported_voxels["IsoValue"] == 0.75
    assert exported_voxels["DualMarchingCubes"] is False
    assert exported_voxels["SelectionVoxels"] == {"size": 4, "bits": "CgAAAAAAAAA="}
    assert exported_voxels["Selected"] is True
    assert exported_voxels["Locked"] is True

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_voxel_object_count"] == 1
    assert reloaded.metadata["scene_voxel_objects"][0]["values"][3] == 1.5
    assert reloaded.metadata["scene_voxel_objects"][0]["selected_voxels"] == [1, 3]


def test_save_meshlib_mru_scene_preserves_object_voxels_gav_payloads_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    header = json.dumps(
        {
            "ValueType": "Float",
            "Dimensions": {"X": 2, "Y": 2, "Z": 1},
            "VoxelSize": {"X": 1.0, "Y": 1.5, "Z": 2.0},
            "Range": {"Min": -1.0, "Max": 2.0},
        }
    ).encode("utf-8")
    gav_voxels = (
        len(header).to_bytes(4, "little")
        + header
        + np.array([-1.0, 0.0, 1.0, 2.0], dtype="<f4").tobytes()
    )
    voxel_file = "0_Root/1_Scan_Voxels.gav"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 1.0, "y": 1.5, "z": 2.0},
                "Dimensions": {"x": 2, "y": 2, "z": 1},
                "MinCorner": {"x": 0, "y": 0, "z": 0},
                "MaxCorner": {"x": 2, "y": 2, "z": 1},
                "IsoValue": 0.5,
                "DualMarchingCubes": True,
            },
        },
    }
    source_scene = tmp_path / "voxels_gav_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, gav_voxels)

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_voxel_object_count"] == 1
    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".gav"
    assert voxel_object["dimensions"] == [2, 2, 1]
    assert voxel_object["voxel_size"] == [1.0, 1.5, 2.0]
    assert voxel_object["values"] == [-1.0, 0.0, 1.0, 2.0]
    assert voxel_object["min_value"] == -1.0
    assert voxel_object["max_value"] == 2.0
    assert voxel_object["dual_marching_cubes"] is True

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "voxels_gav_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            voxel_file,
            "Root.json",
        ]
        exported_gav = archive.read(voxel_file)
    header_len = int.from_bytes(exported_gav[:4], "little")
    exported_header = json.loads(exported_gav[4 : 4 + header_len].decode("utf-8"))
    assert exported_header["ValueType"] == "Float"
    assert exported_header["Dimensions"] == {"X": 2, "Y": 2, "Z": 1}
    assert exported_header["VoxelSize"]["Y"] == 1.5
    assert exported_header["Range"] == {"Min": -1.0, "Max": 2.0}

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_voxel_objects"][0]["model_extension"] == ".gav"
    assert reloaded.metadata["scene_voxel_objects"][0]["values"] == [-1.0, 0.0, 1.0, 2.0]


def test_load_meshlib_mru_scene_imports_uncompressed_vdb_dense_values_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh

    def push_u8(buffer: bytearray, value: int) -> None:
        buffer.append(value)

    def push_u32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=False))

    def push_u64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=False))

    def push_i32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=True))

    def push_i64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=True))

    def push_f32(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float32(value).tobytes())

    def push_f64(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float64(value).tobytes())

    def push_string(buffer: bytearray, value: str) -> None:
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_string(buffer: bytearray, name: str, value: str) -> None:
        push_string(buffer, name)
        push_string(buffer, "string")
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_i64(buffer: bytearray, name: str, value: int) -> None:
        push_string(buffer, name)
        push_string(buffer, "int64")
        push_u32(buffer, 8)
        push_i64(buffer, value)

    def push_metadata_vec3i(buffer: bytearray, name: str, values: tuple[int, int, int]) -> None:
        push_string(buffer, name)
        push_string(buffer, "vec3i")
        push_u32(buffer, 12)
        for value in values:
            push_i32(buffer, value)

    def push_dvec3(buffer: bytearray, values: tuple[float, float, float]) -> None:
        for value in values:
            push_f64(buffer, value)

    def push_node_mask(buffer: bytearray, log2_dim: int, enabled_offsets: list[int]) -> None:
        bit_count = 1 << (3 * log2_dim)
        mask = bytearray(bit_count // 8)
        for offset in enabled_offsets:
            mask[offset // 8] |= 1 << (offset % 8)
        buffer.extend(mask)

    def push_uncompressed_float_values(buffer: bytearray, count: int, value: float) -> None:
        push_u8(buffer, 6)
        for _ in range(count):
            push_f32(buffer, value)

    def synthetic_openvdb_single_dense_leaf(values: list[float]) -> bytes:
        assert len(values) == 512
        grid = bytearray()
        push_u32(grid, 0)
        push_u32(grid, 5)
        push_metadata_vec3i(grid, "file_bbox_min", (0, 0, 0))
        push_metadata_vec3i(grid, "file_bbox_max", (7, 7, 7))
        push_metadata_i64(grid, "file_voxel_count", 512)
        push_metadata_string(grid, "value_type", "float")
        push_metadata_string(grid, "class", "level set")
        push_string(grid, "UniformScaleMap")
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (2.0, 2.0, 2.0))
        push_dvec3(grid, (4.0, 4.0, 4.0))
        push_dvec3(grid, (1.0, 1.0, 1.0))

        push_f32(grid, 1000.0)
        push_u32(grid, 0)
        push_u32(grid, 1)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_node_mask(grid, 5, [0])
        push_node_mask(grid, 5, [])
        push_uncompressed_float_values(grid, 1 << 15, 1000.0)
        push_node_mask(grid, 4, [0])
        push_node_mask(grid, 4, [])
        push_uncompressed_float_values(grid, 1 << 12, 1000.0)
        push_node_mask(grid, 3, list(range(512)))

        push_node_mask(grid, 3, list(range(512)))
        push_u8(grid, 6)
        for x in range(8):
            for y in range(8):
                for z in range(8):
                    dense_index = x + y * 8 + z * 64
                    push_f32(grid, values[dense_index])

        payload = bytearray(b"\x20\x42\x44\x56\x00\x00\x00\x00")
        push_u32(payload, 223)
        push_u32(payload, 12)
        push_u32(payload, 0)
        push_u8(payload, 1)
        payload.extend(b"00000000-0000-0000-0000-000000000000")
        push_u32(payload, 0)
        push_u32(payload, 1)
        push_string(payload, "dense_leaf")
        push_string(payload, "Tree_float_5_4_3")
        push_string(payload, "")
        grid_pos_offset = len(payload)
        push_u64(payload, 0)
        block_pos_offset = len(payload)
        push_u64(payload, 0)
        end_pos_offset = len(payload)
        push_u64(payload, 0)
        grid_pos = len(payload)
        payload.extend(grid)
        end_pos = len(payload)
        payload[grid_pos_offset : grid_pos_offset + 8] = grid_pos.to_bytes(8, "little")
        payload[block_pos_offset : block_pos_offset + 8] = end_pos.to_bytes(8, "little")
        payload[end_pos_offset : end_pos_offset + 8] = end_pos.to_bytes(8, "little")
        return bytes(payload)

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    expected_values = [
        float(x + 10 * y + 100 * z)
        for z in range(8)
        for y in range(8)
        for x in range(8)
    ]
    voxel_file = "0_Root/1_Scan_Voxels.vdb"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1},
            },
        },
    }
    source_scene = tmp_path / "voxels_vdb_dense_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, synthetic_openvdb_single_dense_leaf(expected_values))

    loaded = default_sdk.load_mesh(source_scene)

    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".vdb"
    assert voxel_object["dimensions"] == [8, 8, 8]
    assert voxel_object["voxel_size"] == [0.5, 0.5, 0.5]
    assert voxel_object["grid_level_set"] is True
    assert voxel_object["values"] == expected_values
    assert voxel_object["min_value"] == 0.0
    assert voxel_object["max_value"] == 777.0


def test_load_meshlib_mru_scene_imports_half_float_active_mask_vdb_values_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh

    def push_u8(buffer: bytearray, value: int) -> None:
        buffer.append(value)

    def push_u16(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(2, "little", signed=False))

    def push_u32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=False))

    def push_u64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=False))

    def push_i32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=True))

    def push_i64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=True))

    def push_f32(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float32(value).tobytes())

    def push_f64(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float64(value).tobytes())

    def push_string(buffer: bytearray, value: str) -> None:
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_string(buffer: bytearray, name: str, value: str) -> None:
        push_string(buffer, name)
        push_string(buffer, "string")
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_i64(buffer: bytearray, name: str, value: int) -> None:
        push_string(buffer, name)
        push_string(buffer, "int64")
        push_u32(buffer, 8)
        push_i64(buffer, value)

    def push_metadata_vec3i(buffer: bytearray, name: str, values: tuple[int, int, int]) -> None:
        push_string(buffer, name)
        push_string(buffer, "vec3i")
        push_u32(buffer, 12)
        for value in values:
            push_i32(buffer, value)

    def push_dvec3(buffer: bytearray, values: tuple[float, float, float]) -> None:
        for value in values:
            push_f64(buffer, value)

    def push_node_mask(buffer: bytearray, log2_dim: int, enabled_offsets: list[int]) -> None:
        bit_count = 1 << (3 * log2_dim)
        mask = bytearray(bit_count // 8)
        for offset in enabled_offsets:
            mask[offset // 8] |= 1 << (offset % 8)
        buffer.extend(mask)

    def push_active_mask_header(buffer: bytearray) -> None:
        push_u8(buffer, 0)

    def synthetic_openvdb_single_half_active_leaf() -> bytes:
        active_offsets = [0, 83, 511]
        grid = bytearray()
        push_u32(grid, 2)
        push_u32(grid, 5)
        push_metadata_vec3i(grid, "file_bbox_min", (0, 0, 0))
        push_metadata_vec3i(grid, "file_bbox_max", (7, 7, 7))
        push_metadata_i64(grid, "file_voxel_count", 3)
        push_metadata_string(grid, "value_type", "float")
        push_metadata_string(grid, "class", "level set")
        push_string(grid, "UniformScaleMap")
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (2.0, 2.0, 2.0))
        push_dvec3(grid, (4.0, 4.0, 4.0))
        push_dvec3(grid, (1.0, 1.0, 1.0))

        push_f32(grid, 9.0)
        push_u32(grid, 0)
        push_u32(grid, 1)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_node_mask(grid, 5, [0])
        push_node_mask(grid, 5, [])
        push_active_mask_header(grid)
        push_node_mask(grid, 4, [0])
        push_node_mask(grid, 4, [])
        push_active_mask_header(grid)
        push_node_mask(grid, 3, active_offsets)

        push_node_mask(grid, 3, active_offsets)
        push_active_mask_header(grid)
        push_u16(grid, 0x3C00)
        push_u16(grid, 0x4100)
        push_u16(grid, 0xC200)

        payload = bytearray(b"\x20\x42\x44\x56\x00\x00\x00\x00")
        push_u32(payload, 223)
        push_u32(payload, 12)
        push_u32(payload, 0)
        push_u8(payload, 1)
        payload.extend(b"00000000-0000-0000-0000-000000000000")
        push_u32(payload, 0)
        push_u32(payload, 1)
        push_string(payload, "half_active_leaf")
        push_string(payload, "Tree_float_5_4_3_HalfFloat")
        push_string(payload, "")
        grid_pos_offset = len(payload)
        push_u64(payload, 0)
        block_pos_offset = len(payload)
        push_u64(payload, 0)
        end_pos_offset = len(payload)
        push_u64(payload, 0)
        grid_pos = len(payload)
        payload.extend(grid)
        end_pos = len(payload)
        payload[grid_pos_offset : grid_pos_offset + 8] = grid_pos.to_bytes(8, "little")
        payload[block_pos_offset : block_pos_offset + 8] = end_pos.to_bytes(8, "little")
        payload[end_pos_offset : end_pos_offset + 8] = end_pos.to_bytes(8, "little")
        return bytes(payload)

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    expected_values = [9.0] * 512
    expected_values[0] = 1.0
    expected_values[1 + 2 * 8 + 3 * 64] = 2.5
    expected_values[7 + 7 * 8 + 7 * 64] = -3.0
    voxel_file = "0_Root/1_Scan_Voxels.vdb"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1},
            },
        },
    }
    source_scene = tmp_path / "voxels_vdb_half_active_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, synthetic_openvdb_single_half_active_leaf())

    loaded = default_sdk.load_mesh(source_scene)

    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".vdb"
    assert voxel_object["dimensions"] == [8, 8, 8]
    assert voxel_object["voxel_size"] == [0.5, 0.5, 0.5]
    assert voxel_object["grid_level_set"] is True
    assert voxel_object["values"] == expected_values
    assert voxel_object["min_value"] == -3.0
    assert voxel_object["max_value"] == 9.0


def test_load_meshlib_mru_scene_imports_zip_compressed_vdb_values_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh

    def push_u8(buffer: bytearray, value: int) -> None:
        buffer.append(value)

    def push_u32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=False))

    def push_u64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=False))

    def push_i32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=True))

    def push_i64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=True))

    def push_f32(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float32(value).tobytes())

    def push_f64(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float64(value).tobytes())

    def push_string(buffer: bytearray, value: str) -> None:
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_string(buffer: bytearray, name: str, value: str) -> None:
        push_string(buffer, name)
        push_string(buffer, "string")
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_i64(buffer: bytearray, name: str, value: int) -> None:
        push_string(buffer, name)
        push_string(buffer, "int64")
        push_u32(buffer, 8)
        push_i64(buffer, value)

    def push_metadata_vec3i(buffer: bytearray, name: str, values: tuple[int, int, int]) -> None:
        push_string(buffer, name)
        push_string(buffer, "vec3i")
        push_u32(buffer, 12)
        for value in values:
            push_i32(buffer, value)

    def push_dvec3(buffer: bytearray, values: tuple[float, float, float]) -> None:
        for value in values:
            push_f64(buffer, value)

    def push_node_mask(buffer: bytearray, log2_dim: int, enabled_offsets: list[int]) -> None:
        bit_count = 1 << (3 * log2_dim)
        mask = bytearray(bit_count // 8)
        for offset in enabled_offsets:
            mask[offset // 8] |= 1 << (offset % 8)
        buffer.extend(mask)

    def push_zip_uncompressed_float_values(buffer: bytearray, count: int, value: float) -> None:
        push_u8(buffer, 6)
        push_i64(buffer, -(count * 4))
        for _ in range(count):
            push_f32(buffer, value)

    def push_zip_compressed_leaf_values(buffer: bytearray, values_by_openvdb_offset: list[float]) -> None:
        raw_values = b"".join(np.float32(value).tobytes() for value in values_by_openvdb_offset)
        compressed_values = zlib.compress(raw_values)
        push_u8(buffer, 6)
        push_i64(buffer, len(compressed_values))
        buffer.extend(compressed_values)

    def synthetic_openvdb_single_zip_dense_leaf(values_by_openvdb_offset: list[float]) -> bytes:
        assert len(values_by_openvdb_offset) == 512
        grid = bytearray()
        push_u32(grid, 1)
        push_u32(grid, 5)
        push_metadata_vec3i(grid, "file_bbox_min", (0, 0, 0))
        push_metadata_vec3i(grid, "file_bbox_max", (7, 7, 7))
        push_metadata_i64(grid, "file_voxel_count", 512)
        push_metadata_string(grid, "value_type", "float")
        push_metadata_string(grid, "class", "level set")
        push_string(grid, "UniformScaleMap")
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (2.0, 2.0, 2.0))
        push_dvec3(grid, (4.0, 4.0, 4.0))
        push_dvec3(grid, (1.0, 1.0, 1.0))

        push_f32(grid, 1000.0)
        push_u32(grid, 0)
        push_u32(grid, 1)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_node_mask(grid, 5, [0])
        push_node_mask(grid, 5, [])
        push_zip_uncompressed_float_values(grid, 1 << 15, 1000.0)
        push_node_mask(grid, 4, [0])
        push_node_mask(grid, 4, [])
        push_zip_uncompressed_float_values(grid, 1 << 12, 1000.0)
        push_node_mask(grid, 3, list(range(512)))

        push_node_mask(grid, 3, list(range(512)))
        push_zip_compressed_leaf_values(grid, values_by_openvdb_offset)

        payload = bytearray(b"\x20\x42\x44\x56\x00\x00\x00\x00")
        push_u32(payload, 223)
        push_u32(payload, 12)
        push_u32(payload, 0)
        push_u8(payload, 1)
        payload.extend(b"00000000-0000-0000-0000-000000000000")
        push_u32(payload, 0)
        push_u32(payload, 1)
        push_string(payload, "zip_dense_leaf")
        push_string(payload, "Tree_float_5_4_3")
        push_string(payload, "")
        grid_pos_offset = len(payload)
        push_u64(payload, 0)
        block_pos_offset = len(payload)
        push_u64(payload, 0)
        end_pos_offset = len(payload)
        push_u64(payload, 0)
        grid_pos = len(payload)
        payload.extend(grid)
        end_pos = len(payload)
        payload[grid_pos_offset : grid_pos_offset + 8] = grid_pos.to_bytes(8, "little")
        payload[block_pos_offset : block_pos_offset + 8] = end_pos.to_bytes(8, "little")
        payload[end_pos_offset : end_pos_offset + 8] = end_pos.to_bytes(8, "little")
        return bytes(payload)

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    values_by_offset = [0.0] * 512
    values_by_offset[0] = 1.0
    values_by_offset[83] = 2.5
    values_by_offset[511] = -3.0
    expected_values = [0.0] * 512
    expected_values[0] = 1.0
    expected_values[1 + 2 * 8 + 3 * 64] = 2.5
    expected_values[7 + 7 * 8 + 7 * 64] = -3.0
    voxel_file = "0_Root/1_Scan_Voxels.vdb"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1},
            },
        },
    }
    source_scene = tmp_path / "voxels_vdb_zip_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, synthetic_openvdb_single_zip_dense_leaf(values_by_offset))

    loaded = default_sdk.load_mesh(source_scene)

    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".vdb"
    assert voxel_object["dimensions"] == [8, 8, 8]
    assert voxel_object["voxel_size"] == [0.5, 0.5, 0.5]
    assert voxel_object["grid_level_set"] is True
    assert voxel_object["values"] == expected_values
    assert voxel_object["min_value"] == -3.0
    assert voxel_object["max_value"] == 2.5


def test_load_meshlib_mru_scene_imports_blosc_compressed_vdb_values_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh

    compressed_leaf_values = bytes(
        [
            2,
            1,
            33,
            4,
            0,
            8,
            0,
            0,
            0,
            8,
            0,
            0,
            104,
            0,
            0,
            0,
            20,
            0,
            0,
            0,
            12,
            0,
            0,
            0,
            31,
            0,
            1,
            0,
            255,
            232,
            80,
            0,
            0,
            0,
            0,
            0,
            12,
            0,
            0,
            0,
            31,
            0,
            1,
            0,
            255,
            232,
            80,
            0,
            0,
            0,
            0,
            0,
            22,
            0,
            0,
            0,
            47,
            128,
            0,
            1,
            0,
            62,
            31,
            32,
            82,
            0,
            62,
            15,
            2,
            0,
            255,
            68,
            80,
            0,
            0,
            0,
            0,
            64,
            22,
            0,
            0,
            0,
            47,
            63,
            0,
            1,
            0,
            62,
            31,
            64,
            82,
            0,
            62,
            15,
            2,
            0,
            255,
            68,
            80,
            0,
            0,
            0,
            0,
            192,
        ]
    )

    def push_u8(buffer: bytearray, value: int) -> None:
        buffer.append(value)

    def push_u32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=False))

    def push_u64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=False))

    def push_i32(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(4, "little", signed=True))

    def push_i64(buffer: bytearray, value: int) -> None:
        buffer.extend(int(value).to_bytes(8, "little", signed=True))

    def push_f32(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float32(value).tobytes())

    def push_f64(buffer: bytearray, value: float) -> None:
        buffer.extend(np.float64(value).tobytes())

    def push_string(buffer: bytearray, value: str) -> None:
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_string(buffer: bytearray, name: str, value: str) -> None:
        push_string(buffer, name)
        push_string(buffer, "string")
        payload = value.encode("utf-8")
        push_u32(buffer, len(payload))
        buffer.extend(payload)

    def push_metadata_i64(buffer: bytearray, name: str, value: int) -> None:
        push_string(buffer, name)
        push_string(buffer, "int64")
        push_u32(buffer, 8)
        push_i64(buffer, value)

    def push_metadata_vec3i(buffer: bytearray, name: str, values: tuple[int, int, int]) -> None:
        push_string(buffer, name)
        push_string(buffer, "vec3i")
        push_u32(buffer, 12)
        for value in values:
            push_i32(buffer, value)

    def push_dvec3(buffer: bytearray, values: tuple[float, float, float]) -> None:
        for value in values:
            push_f64(buffer, value)

    def push_node_mask(buffer: bytearray, log2_dim: int, enabled_offsets: list[int]) -> None:
        bit_count = 1 << (3 * log2_dim)
        mask = bytearray(bit_count // 8)
        for offset in enabled_offsets:
            mask[offset // 8] |= 1 << (offset % 8)
        buffer.extend(mask)

    def push_blosc_uncompressed_float_values(buffer: bytearray, count: int, value: float) -> None:
        push_u8(buffer, 6)
        push_i64(buffer, -(count * 4))
        for _ in range(count):
            push_f32(buffer, value)

    def push_blosc_compressed_leaf_values(buffer: bytearray) -> None:
        push_u8(buffer, 6)
        push_i64(buffer, len(compressed_leaf_values))
        buffer.extend(compressed_leaf_values)

    def synthetic_openvdb_single_blosc_dense_leaf() -> bytes:
        grid = bytearray()
        push_u32(grid, 4)
        push_u32(grid, 5)
        push_metadata_vec3i(grid, "file_bbox_min", (0, 0, 0))
        push_metadata_vec3i(grid, "file_bbox_max", (7, 7, 7))
        push_metadata_i64(grid, "file_voxel_count", 512)
        push_metadata_string(grid, "value_type", "float")
        push_metadata_string(grid, "class", "level set")
        push_string(grid, "UniformScaleMap")
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (0.5, 0.5, 0.5))
        push_dvec3(grid, (2.0, 2.0, 2.0))
        push_dvec3(grid, (4.0, 4.0, 4.0))
        push_dvec3(grid, (1.0, 1.0, 1.0))

        push_f32(grid, 1000.0)
        push_u32(grid, 0)
        push_u32(grid, 1)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_i32(grid, 0)
        push_node_mask(grid, 5, [0])
        push_node_mask(grid, 5, [])
        push_blosc_uncompressed_float_values(grid, 1 << 15, 1000.0)
        push_node_mask(grid, 4, [0])
        push_node_mask(grid, 4, [])
        push_blosc_uncompressed_float_values(grid, 1 << 12, 1000.0)
        push_node_mask(grid, 3, list(range(512)))

        push_node_mask(grid, 3, list(range(512)))
        push_blosc_compressed_leaf_values(grid)

        payload = bytearray(b"\x20\x42\x44\x56\x00\x00\x00\x00")
        push_u32(payload, 223)
        push_u32(payload, 12)
        push_u32(payload, 0)
        push_u8(payload, 1)
        payload.extend(b"00000000-0000-0000-0000-000000000000")
        push_u32(payload, 0)
        push_u32(payload, 1)
        push_string(payload, "blosc_dense_leaf")
        push_string(payload, "Tree_float_5_4_3")
        push_string(payload, "")
        grid_pos_offset = len(payload)
        push_u64(payload, 0)
        block_pos_offset = len(payload)
        push_u64(payload, 0)
        end_pos_offset = len(payload)
        push_u64(payload, 0)
        grid_pos = len(payload)
        payload.extend(grid)
        end_pos = len(payload)
        payload[grid_pos_offset : grid_pos_offset + 8] = grid_pos.to_bytes(8, "little")
        payload[block_pos_offset : block_pos_offset + 8] = end_pos.to_bytes(8, "little")
        payload[end_pos_offset : end_pos_offset + 8] = end_pos.to_bytes(8, "little")
        return bytes(payload)

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    expected_values = [0.0] * 512
    expected_values[0] = 1.0
    expected_values[1 + 2 * 8 + 3 * 64] = 2.5
    expected_values[7 + 7 * 8 + 7 * 64] = -3.0
    voxel_file = "0_Root/1_Scan_Voxels.vdb"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1},
            },
        },
    }
    source_scene = tmp_path / "voxels_vdb_blosc_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, synthetic_openvdb_single_blosc_dense_leaf())

    loaded = default_sdk.load_mesh(source_scene)

    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".vdb"
    assert voxel_object["dimensions"] == [8, 8, 8]
    assert voxel_object["voxel_size"] == [0.5, 0.5, 0.5]
    assert voxel_object["grid_level_set"] is True
    assert voxel_object["values"] == expected_values
    assert voxel_object["min_value"] == -3.0
    assert voxel_object["max_value"] == 2.5


def test_save_meshlib_mru_scene_preserves_object_voxels_vdb_payloads_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    vdb_voxels = b"OPENVDB_OPAQUE_PAYLOAD_FOR_MRU_SCENE_ROUNDTRIP"
    voxel_file = "0_Root/1_Scan_Voxels.vdb"
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Visibility": 4294967295,
                "Selected": True,
                "Locked": False,
                "ParentLocked": False,
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 0.25, "y": 0.5, "z": 1.0},
                "Dimensions": {"x": 2, "y": 2, "z": 1},
                "MinCorner": {"x": 0, "y": 0, "z": 0},
                "MaxCorner": {"x": 2, "y": 2, "z": 1},
                "SelectionVoxels": {"size": 4, "bits": "CgAAAAAAAAA="},
                "IsoValue": 0.125,
                "DualMarchingCubes": True,
            },
        },
    }
    source_scene = tmp_path / "voxels_vdb_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.writestr(voxel_file, vdb_voxels)

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_voxel_object_count"] == 1
    voxel_object = loaded.metadata["scene_voxel_objects"][0]
    assert voxel_object["model_file"] == voxel_file
    assert voxel_object["model_extension"] == ".vdb"
    assert voxel_object["dimensions"] == [2, 2, 1]
    assert voxel_object["voxel_size"] == [0.25, 0.5, 1.0]
    assert voxel_object["values"] == []
    assert voxel_object["model_bytes_base64"] == base64.b64encode(vdb_voxels).decode("ascii")
    assert voxel_object["selected_voxels"] == [1, 3]
    assert voxel_object["dual_marching_cubes"] is True

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "voxels_vdb_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            voxel_file,
            "Root.json",
        ]
        assert archive.read(voxel_file) == vdb_voxels
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    exported_voxels = root_payload["Children"]["1"]
    assert exported_voxels["VoxelSize"]["x"] == 0.25
    assert exported_voxels["Dimensions"]["z"] == 1
    assert exported_voxels["IsoValue"] == 0.125
    assert exported_voxels["DualMarchingCubes"] is True
    assert exported_voxels["SelectionVoxels"] == {"size": 4, "bits": "CgAAAAAAAAA="}

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_voxel_objects"][0]["model_extension"] == ".vdb"
    assert reloaded.metadata["scene_voxel_objects"][0]["model_bytes_base64"] == base64.b64encode(
        vdb_voxels
    ).decode("ascii")
    assert reloaded.metadata["scene_voxel_objects"][0]["selected_voxels"] == [1, 3]


def test_save_meshlib_mru_scene_preserves_feature_object_type_management_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
            },
            "1": {
                "Name": "Plane Feature",
                "Key": "1_Plane_Feature",
                "Visibility": 0,
                "Selected": True,
                "Locked": True,
                "ParentLocked": False,
                "XF": {
                    "A": {
                        "rowX": {"x": 2.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 3.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.5},
                    },
                    "b": {"x": 1.0, "y": 2.0, "z": 3.0},
                },
                "Type": ["Object", "VisualObject", "FeatureObject", "PlaneObject"],
                "SubfeatureVisibility": 15,
                "DetailsOnNameTag": 7,
                "DecorationsColorUnselected": {"x": 0.1, "y": 0.2, "z": 0.3, "w": 0.4},
                "DecorationsColorSelected": {"x": 0.5, "y": 0.6, "z": 0.7, "w": 0.8},
                "PointSize": 11.0,
                "LineWidth": 2.5,
                "SubPointSize": 6.5,
                "SubLineWidth": 1.5,
                "MainAlpha": 0.9,
                "SubAlphaPoints": 0.8,
                "SubAlphaLines": 0.7,
                "SubAlphaMesh": 0.6,
                "DimensionVisibility": {"Length": 3, "Angle": 5},
            },
        },
    }
    source_scene = tmp_path / "feature_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(base_model, "0_Root/0_Base_A.ply")

    loaded = default_sdk.load_mesh(source_scene)

    assert loaded.metadata["scene_feature_object_count"] == 1
    feature_object = loaded.metadata["scene_feature_objects"][0]
    assert feature_object["object_name"] == "Plane Feature"
    assert feature_object["object_key"] == "1_Plane_Feature"
    assert feature_object["parent_key"] == "0_Root"
    assert feature_object["hierarchy_path"] == ["0_Root", "1_Plane_Feature"]
    assert feature_object["feature_type"] == "PlaneObject"
    assert feature_object["xf"]["row_x"] == [2.0, 0.0, 0.0]
    assert feature_object["xf"]["row_y"] == [0.0, 3.0, 0.0]
    assert feature_object["xf"]["row_z"] == [0.0, 0.0, 1.5]
    assert feature_object["xf"]["b"] == [1.0, 2.0, 3.0]
    assert feature_object["subfeature_visibility"] == 15
    assert feature_object["details_on_name_tag"] == 7
    assert feature_object["decorations_color_unselected"] == [0.1, 0.2, 0.3, 0.4]
    assert feature_object["decorations_color_selected"] == [0.5, 0.6, 0.7, 0.8]
    assert feature_object["point_size"] == 11.0
    assert feature_object["line_width"] == 2.5
    assert feature_object["sub_point_size"] == 6.5
    assert feature_object["sub_line_width"] == 1.5
    assert feature_object["main_alpha"] == pytest.approx(0.9)
    assert feature_object["sub_alpha_points"] == pytest.approx(0.8)
    assert feature_object["sub_alpha_lines"] == pytest.approx(0.7)
    assert feature_object["sub_alpha_mesh"] == pytest.approx(0.6)
    assert feature_object["dimension_visibility"] == {"Angle": 5, "Length": 3}
    assert feature_object["visibility_mask"] == 0
    assert feature_object["selected"] is True
    assert feature_object["locked"] is True
    assert feature_object["parent_locked"] is False

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "feature_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == ["0_Root/0_Base_A.ply", "Root.json"]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    exported_feature = root_payload["Children"]["1"]
    assert exported_feature["Name"] == "Plane Feature"
    assert exported_feature["Type"] == ["Object", "VisualObject", "FeatureObject", "PlaneObject"]
    assert exported_feature["SubfeatureVisibility"] == 15
    assert exported_feature["DetailsOnNameTag"] == 7
    assert exported_feature["DecorationsColorSelected"]["w"] == 0.8
    assert exported_feature["PointSize"] == 11.0
    assert exported_feature["SubAlphaMesh"] == pytest.approx(0.6)
    assert exported_feature["DimensionVisibility"]["Length"] == 3

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert reloaded.metadata["scene_feature_object_count"] == 1
    assert reloaded.metadata["scene_feature_objects"][0]["feature_type"] == "PlaneObject"
    assert reloaded.metadata["scene_feature_objects"][0]["xf"]["b"] == [1.0, 2.0, 3.0]


def test_default_sdk_feature_object_render_payload_routes_through_rust() -> None:
    from geometry_sdk import default_sdk

    mesh = MeshDocument(
        vertices=np.empty((0, 3), dtype=np.float64),
        faces=np.empty((0, 3), dtype=np.int64),
        metadata={
            "scene_feature_objects": [
                {
                    "object_name": "Sphere",
                    "object_key": "7_SphereFeature",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "7_SphereFeature"],
                    "feature_type": "SphereObject",
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                    "locked": False,
                    "parent_locked": False,
                    "subfeature_visibility": 0xFFFFFFFF,
                    "details_on_name_tag": 0,
                    "dimension_visibility": {"Diameter": 0xFFFFFFFF},
                    "decorations_color_unselected": [1.0, 1.0, 1.0, 1.0],
                    "decorations_color_selected": [1.0, 1.0, 1.0, 1.0],
                    "point_size": 1.0,
                    "line_width": 1.0,
                    "sub_point_size": 1.0,
                    "sub_line_width": 1.0,
                    "main_alpha": 1.0,
                    "sub_alpha_points": 1.0,
                    "sub_alpha_lines": 1.0,
                    "sub_alpha_mesh": 1.0,
                    "xf": {
                        "row_x": [1.0, 0.0, 0.0],
                        "row_y": [0.0, 1.0, 0.0],
                        "row_z": [0.0, 0.0, 1.0],
                        "b": [0.0, 0.0, 0.0],
                    },
                }
            ]
        },
    )

    payload = default_sdk.meshlib_scene_feature_object_render_payload(mesh)

    render = payload["objects"][0]
    assert render["feature_type"] == "SphereObject"
    assert len(render["primary_mesh_vertices"]) == 2048
    assert len(render["primary_mesh_faces"]) == 4092
    assert render["subfeature_points"] == [[0.0, 0.0, 0.0]]
    assert render["dimensions"] == [
        {"kind": "Diameter", "points": [[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]]}
    ]


def test_save_meshlib_mru_scene_preserves_nested_object_tree_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    child_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    child_model = save_mesh(child_mesh, tmp_path / "child.ply", file_type="ply")
    source_scene = tmp_path / "nested_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Base A",
                            "Key": "0_Base_A",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                            "Children": {
                                "0": {
                                    "Name": "Child B",
                                    "Key": "0_Child_B",
                                    "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                                    "XF": {
                                        "A": {
                                            "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                                            "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                                            "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                                        },
                                        "b": {"x": 3.0, "y": 0.0, "z": 0.0},
                                    },
                                }
                            },
                        }
                    },
                }
            ),
        )
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.write(child_model, "0_Root/0_Base_A/0_Child_B.ply")

    loaded = default_sdk.load_mesh(source_scene)
    assert [scene_object["hierarchy_path"] for scene_object in loaded.metadata["scene_objects"]] == [
        ["0_Root", "0_Base_A"],
        ["0_Root", "0_Base_A", "0_Child_B"],
    ]

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "nested_roundtrip.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            "0_Root/0_Base_A/0_Child_B.ply",
            "Root.json",
        ]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert root_payload["Children"]["0"]["Name"] == "Base A"
    assert root_payload["Children"]["0"]["Children"]["0"]["Name"] == "Child B"
    assert "1" not in root_payload["Children"]

    reloaded = default_sdk.load_mesh(round_trip_scene)
    np.testing.assert_allclose(reloaded.vertices, loaded.vertices)
    np.testing.assert_array_equal(reloaded.faces, loaded.faces)
    assert [scene_object["parent_key"] for scene_object in reloaded.metadata["scene_objects"]] == [
        "0_Root",
        "0_Base_A",
    ]


def test_reparent_mru_scene_object_updates_tree_metadata_and_round_trips_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.core.mesh import meshlib_reparent_scene_object
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    child_mesh = MeshDocument(
        vertices=np.array(
            [[2.0, 0.0, 0.0], [2.5, 0.0, 0.0], [2.0, 0.5, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    child_model = save_mesh(child_mesh, tmp_path / "child.ply", file_type="ply")
    source_scene = tmp_path / "reparent_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Base A",
                            "Key": "0_Base_A",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                        "1": {
                            "Name": "Child B",
                            "Key": "1_Child_B",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                    },
                }
            ),
        )
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.write(child_model, "0_Root/1_Child_B.ply")

    loaded = default_sdk.load_mesh(source_scene)
    reparented = meshlib_reparent_scene_object(
        loaded,
        object_key="1_Child_B",
        new_parent_key="0_Base_A",
    )

    assert [scene_object["parent_key"] for scene_object in reparented.metadata["scene_objects"]] == [
        "0_Root",
        "0_Base_A",
    ]
    assert reparented.metadata["scene_objects"][1]["hierarchy_path"] == [
        "0_Root",
        "0_Base_A",
        "1_Child_B",
    ]
    assert reparented.metadata["scene_objects"][1]["model_file"] == "0_Root/0_Base_A/1_Child_B.ply"
    assert reparented.metadata["meshlib_operation"] == "MR::Object::addChild"

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        reparented,
        tmp_path / "reparent_roundtrip.mru",
        object_name="Root",
    )
    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == [
            "0_Root/0_Base_A.ply",
            "0_Root/0_Base_A/1_Child_B.ply",
            "Root.json",
        ]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert root_payload["Children"]["0"]["Children"]["0"]["Name"] == "Child B"
    assert "1" not in root_payload["Children"]

    reloaded = default_sdk.load_mesh(round_trip_scene)
    np.testing.assert_allclose(reloaded.vertices, loaded.vertices)
    np.testing.assert_array_equal(reloaded.faces, loaded.faces)
    assert reloaded.metadata["scene_objects"][1]["parent_key"] == "0_Base_A"


def test_set_mru_scene_object_state_updates_visibility_and_lock_flags_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.core.mesh import meshlib_set_scene_object_state
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    model = save_mesh(mesh, tmp_path / "state.ply", file_type="ply")
    source_scene = tmp_path / "state_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Stateful A",
                            "Key": "0_Stateful_A",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                    },
                }
            ),
        )
        archive.write(model, "0_Root/0_Stateful_A.ply")

    loaded = default_sdk.load_mesh(source_scene)
    updated = meshlib_set_scene_object_state(
        loaded,
        object_key="0_Stateful_A",
        visibility_mask=0,
        selected=True,
        locked=True,
        parent_locked=True,
    )

    scene_object = updated.metadata["scene_objects"][0]
    assert scene_object["visibility_mask"] == 0
    assert scene_object["selected"] is True
    assert scene_object["locked"] is True
    assert scene_object["parent_locked"] is True
    assert updated.metadata["meshlib_operation"] == "MR::Object::setVisible"

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        updated,
        tmp_path / "state_roundtrip.mru",
        object_name="Root",
    )
    with zipfile.ZipFile(round_trip_scene) as archive:
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    object_payload = root_payload["Children"]["0"]
    assert object_payload["Visibility"] == 0
    assert object_payload["Selected"] is True
    assert object_payload["Locked"] is True
    assert object_payload["ParentLocked"] is True

    reloaded = default_sdk.load_mesh(round_trip_scene)
    reloaded_object = reloaded.metadata["scene_objects"][0]
    assert reloaded_object["visibility_mask"] == 0
    assert reloaded_object["selected"] is True
    assert reloaded_object["locked"] is True
    assert reloaded_object["parent_locked"] is True


def test_select_mru_scene_objects_applies_meshinspector_name_tag_modifier_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.core.mesh import meshlib_select_scene_objects
    from geometry_sdk.io.trimesh_adapter import save_mesh

    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    model = save_mesh(mesh, tmp_path / "scene_select.ply", file_type="ply")
    source_scene = tmp_path / "scene_select_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Base A",
                            "Key": "0_Base_A",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                            "Selected": True,
                        },
                        "1": {
                            "Name": "Child B",
                            "Key": "1_Child_B",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                        "2": {
                            "Name": "Cover C",
                            "Key": "2_Cover_C",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                            "Selected": True,
                        },
                    },
                }
            ),
        )
        archive.write(model, "0_Root/0_Base_A.ply")
        archive.write(model, "0_Root/1_Child_B.ply")
        archive.write(model, "0_Root/2_Cover_C.ply")

    loaded = default_sdk.load_mesh(source_scene)
    select_one = meshlib_select_scene_objects(loaded, object_keys=["1_Child_B"], mode="select_one")

    assert [scene_object["selected"] for scene_object in select_one.metadata["scene_objects"]] == [
        False,
        True,
        False,
    ]
    assert select_one.metadata["selected_scene_object_keys"] == ["1_Child_B"]
    assert select_one.metadata["meshlib_operation"] == "MR::NameTagSelectionMode::selectOne"

    toggle = meshlib_select_scene_objects(
        loaded,
        object_keys=["1_Child_B", "2_Cover_C"],
        mode="toggle",
    )

    assert [scene_object["selected"] for scene_object in toggle.metadata["scene_objects"]] == [
        True,
        True,
        False,
    ]
    assert toggle.metadata["selected_scene_object_keys"] == ["0_Base_A", "1_Child_B"]
    assert toggle.metadata["meshlib_operation"] == "MR::NameTagSelectionMode::toggle"


def test_reorder_mru_scene_children_updates_export_order_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.core.mesh import meshlib_reorder_scene_children
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    meshes = [
        MeshDocument(
            vertices=np.array(
                [[offset, 0.0, 0.0], [offset + 1.0, 0.0, 0.0], [offset, 1.0, 0.0]],
                dtype=np.float64,
            ),
            faces=np.array([[0, 1, 2]], dtype=np.int64),
        )
        for offset in (0.0, 2.0, 4.0)
    ]
    model_paths = [
        save_mesh(mesh, tmp_path / f"child_{index}.ply", file_type="ply")
        for index, mesh in enumerate(meshes)
    ]
    source_scene = tmp_path / "order_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Base A",
                            "Key": "0_Base_A",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                        "1": {
                            "Name": "Child B",
                            "Key": "1_Child_B",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                        "2": {
                            "Name": "Child C",
                            "Key": "2_Child_C",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                    },
                }
            ),
        )
        for key, model_path in zip(("0_Base_A", "1_Child_B", "2_Child_C"), model_paths):
            archive.write(model_path, f"0_Root/{key}.ply")

    loaded = default_sdk.load_mesh(source_scene)
    reordered = meshlib_reorder_scene_children(
        loaded,
        parent_key="0_Root",
        ordered_child_keys=["2_Child_C", "0_Base_A", "1_Child_B"],
    )

    assert [
        scene_object["object_key"] for scene_object in reordered.metadata["scene_objects"]
    ] == ["2_Child_C", "0_Base_A", "1_Child_B"]
    assert reordered.metadata["meshlib_operation"] == "MR::ChangeSceneObjectsOrder"

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        reordered,
        tmp_path / "order_roundtrip.mru",
        object_name="Root",
    )
    with zipfile.ZipFile(round_trip_scene) as archive:
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert [root_payload["Children"][str(index)]["Name"] for index in range(3)] == [
        "Child C",
        "Base A",
        "Child B",
    ]

    reloaded = default_sdk.load_mesh(round_trip_scene)
    assert [scene_object["object_key"] for scene_object in reloaded.metadata["scene_objects"]] == [
        "2_Child_C",
        "0_Base_A",
        "1_Child_B",
    ]


def test_apply_mru_scene_ribbon_actions_and_rename_route_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.accelerators import _rust_common
    from geometry_sdk.core.mesh import meshlib_apply_scene_ribbon_action, meshlib_rename_scene_object
    from geometry_sdk.io.trimesh_adapter import save_mesh

    assert _rust_common._rs is not None
    assert hasattr(_rust_common._rs, "meshlib_apply_scene_ribbon_action")
    assert hasattr(_rust_common._rs, "meshlib_rename_scene_object")

    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    model = save_mesh(mesh, tmp_path / "scene_ribbon.ply", file_type="ply")
    source_scene = tmp_path / "scene_ribbon_source.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Zeta",
                            "Key": "0_Zeta",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                            "Visibility": 0,
                            "Children": {
                                "0": {
                                    "Name": "delta",
                                    "Key": "3_delta",
                                    "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                                    "Selected": True,
                                },
                                "1": {
                                    "Name": "Charlie",
                                    "Key": "4_Charlie",
                                    "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                                },
                            },
                        },
                        "1": {
                            "Name": "Alpha",
                            "Key": "1_Alpha",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                        "2": {
                            "Name": "beta",
                            "Key": "2_beta",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                    },
                }
            ),
        )
        for path in (
            "0_Root/0_Zeta.ply",
            "0_Root/0_Zeta/3_delta.ply",
            "0_Root/0_Zeta/4_Charlie.ply",
            "0_Root/1_Alpha.ply",
            "0_Root/2_beta.ply",
        ):
            archive.write(model, path)

    loaded = default_sdk.load_mesh(source_scene)
    select_all = meshlib_apply_scene_ribbon_action(loaded, action="select_all")
    assert all(scene_object["selected"] for scene_object in select_all.metadata["scene_objects"])
    assert all(
        scene_object["visibility_mask"] == 0xFFFFFFFF
        for scene_object in select_all.metadata["scene_objects"]
    )
    assert select_all.metadata["meshlib_operation"] == "Ribbon Scene Select all"

    show_next = meshlib_apply_scene_ribbon_action(loaded, action="show_only_next")
    assert show_next.metadata["selected_scene_object_keys"] == ["4_Charlie"]
    assert [
        (scene_object["object_key"], scene_object["visibility_mask"])
        for scene_object in show_next.metadata["scene_objects"]
        if scene_object["parent_key"] == "0_Zeta"
    ] == [("3_delta", 0), ("4_Charlie", 0xFFFFFFFF)]

    sorted_scene = meshlib_apply_scene_ribbon_action(loaded, action="sort_by_name")
    assert [scene_object["object_key"] for scene_object in sorted_scene.metadata["scene_objects"]] == [
        "1_Alpha",
        "2_beta",
        "0_Zeta",
        "4_Charlie",
        "3_delta",
    ]

    renamed = meshlib_rename_scene_object(sorted_scene, object_key="4_Charlie", object_name="Echo")
    assert next(
        scene_object
        for scene_object in renamed.metadata["scene_objects"]
        if scene_object["object_key"] == "4_Charlie"
    )["object_name"] == "Echo"
    assert renamed.metadata["meshlib_operation"] == "Ribbon Scene Rename"

    removed = meshlib_apply_scene_ribbon_action(loaded, action="remove_selected")
    assert removed.metadata["removed_scene_object_keys"] == ["3_delta"]
    assert [scene_object["object_key"] for scene_object in removed.metadata["scene_objects"]] == [
        "0_Zeta",
        "4_Charlie",
        "1_Alpha",
        "2_beta",
    ]


def test_mru_scene_tree_ribbon_actions_cover_imported_data_collections_through_rust(tmp_path) -> None:
    from geometry_sdk.accelerators import _rust_common
    from geometry_sdk.core.mesh import meshlib_apply_scene_ribbon_action, meshlib_rename_scene_object
    from geometry_sdk.io.trimesh_adapter import save_meshlib_object_mesh_mru_scene

    assert _rust_common._rs is not None
    assert hasattr(_rust_common._rs, "meshlib_apply_scene_ribbon_action")
    assert hasattr(_rust_common._rs, "meshlib_rename_scene_object")

    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={
            "root_key": "0_Root",
            "scene_objects": [
                {
                    "object_name": "Mesh",
                    "object_key": "0_Mesh",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "0_Mesh"],
                    "vertex_range": [0, 3],
                    "face_range": [0, 1],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                }
            ],
            "scene_line_objects": [
                {
                    "object_name": "Line",
                    "object_key": "1_Line",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "1_Line"],
                    "points": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                    "lines": [[0, 1]],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": True,
                }
            ],
            "scene_point_objects": [
                {
                    "object_name": "Point",
                    "object_key": "2_Point",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "2_Point"],
                    "points": [[0.0, 0.0, 0.0]],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                }
            ],
            "scene_distance_map_objects": [
                {
                    "object_name": "Distance",
                    "object_key": "3_Distance",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "3_Distance"],
                    "width": 1,
                    "height": 1,
                    "values": [0.0],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                }
            ],
            "scene_voxel_objects": [
                {
                    "object_name": "Voxels",
                    "object_key": "4_Voxels",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "4_Voxels"],
                    "dimensions": [1, 1, 1],
                    "voxel_size": [1.0, 1.0, 1.0],
                    "values": [1.0],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                }
            ],
            "scene_feature_objects": [
                {
                    "object_name": "Feature",
                    "object_key": "5_Feature",
                    "parent_key": "2_Point",
                    "hierarchy_path": ["0_Root", "2_Point", "5_Feature"],
                    "feature_type": "PlaneObject",
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                }
            ],
        },
    )

    selected = meshlib_apply_scene_ribbon_action(mesh, action="select_all")
    assert selected.metadata["selected_scene_object_keys"] == [
        "0_Mesh",
        "1_Line",
        "2_Point",
        "3_Distance",
        "4_Voxels",
        "5_Feature",
    ]
    assert selected.metadata["scene_line_objects"][0]["selected"] is True
    assert selected.metadata["scene_feature_objects"][0]["visibility_mask"] == 0xFFFFFFFF

    sorted_scene = meshlib_apply_scene_ribbon_action(mesh, action="sort_by_name")
    assert sorted_scene.metadata["scene_child_order"] == [
        {
            "parent_key": "2_Point",
            "child_keys": ["5_Feature"],
        },
        {
            "parent_key": "0_Root",
            "child_keys": ["3_Distance", "1_Line", "0_Mesh", "2_Point", "4_Voxels"],
        },
    ]
    sorted_scene_path = save_meshlib_object_mesh_mru_scene(
        sorted_scene,
        tmp_path / "sorted_mixed_scene.mru",
        object_name="Root",
    )
    with zipfile.ZipFile(sorted_scene_path) as archive:
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert [root_payload["Children"][str(index)]["Name"] for index in range(5)] == [
        "Distance",
        "Line",
        "Mesh",
        "Point",
        "Voxels",
    ]
    assert root_payload["Children"]["3"]["Children"]["0"]["Name"] == "Feature"

    shown_next = meshlib_apply_scene_ribbon_action(mesh, action="show_only_next")
    assert shown_next.metadata["selected_scene_object_keys"] == ["2_Point"]
    assert shown_next.metadata["scene_line_objects"][0]["visibility_mask"] == 0
    assert shown_next.metadata["scene_point_objects"][0]["visibility_mask"] == 0xFFFFFFFF
    assert shown_next.metadata["visible_scene_object_keys"] == ["2_Point", "5_Feature"]

    renamed = meshlib_rename_scene_object(
        shown_next,
        object_key="5_Feature",
        object_name="Renamed feature",
    )
    assert renamed.metadata["scene_feature_objects"][0]["object_name"] == "Renamed feature"

    removed = meshlib_apply_scene_ribbon_action(renamed, action="remove_selected")
    assert removed.metadata["removed_scene_object_keys"] == ["2_Point", "5_Feature"]
    assert removed.metadata["scene_point_objects"] == []
    assert removed.metadata["scene_feature_objects"] == []
    assert removed.metadata["scene_line_object_count"] == 1
    assert removed.metadata["scene_voxel_object_count"] == 1


def test_group_and_ungroup_mru_scene_objects_route_through_rust(tmp_path) -> None:
    from geometry_sdk.accelerators import _rust_common
    from geometry_sdk.core.mesh import meshlib_group_scene_objects, meshlib_ungroup_scene_objects
    from geometry_sdk.io.trimesh_adapter import save_meshlib_object_mesh_mru_scene

    assert _rust_common._rs is not None
    assert hasattr(_rust_common._rs, "meshlib_group_scene_objects")
    assert hasattr(_rust_common._rs, "meshlib_ungroup_scene_objects")

    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={
            "root_key": "0_Root",
            "scene_objects": [
                {
                    "object_name": "Mesh",
                    "object_key": "0_Mesh",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "0_Mesh"],
                    "vertex_range": [0, 3],
                    "face_range": [0, 1],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": True,
                }
            ],
            "scene_line_objects": [
                {
                    "object_name": "Line",
                    "object_key": "1_Line",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "1_Line"],
                    "points": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                    "lines": [[0, 1]],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": True,
                }
            ],
            "scene_point_objects": [
                {
                    "object_name": "Point",
                    "object_key": "2_Point",
                    "parent_key": "0_Root",
                    "hierarchy_path": ["0_Root", "2_Point"],
                    "points": [[0.0, 0.0, 0.0]],
                    "visibility_mask": 0xFFFFFFFF,
                    "selected": False,
                }
            ],
        },
    )

    grouped = meshlib_group_scene_objects(mesh, group_key="3_Group")
    assert grouped.metadata["meshlib_reference"] == "MR::RibbonMenu::drawGroupUngroupButton"
    assert grouped.metadata["scene_group_objects"] == [
        {
            "object_name": "Group",
            "object_key": "3_Group",
            "parent_key": "0_Root",
            "hierarchy_path": ["0_Root", "3_Group"],
            "xf": {
                "row_x": [1.0, 0.0, 0.0],
                "row_y": [0.0, 1.0, 0.0],
                "row_z": [0.0, 0.0, 1.0],
                "b": [0.0, 0.0, 0.0],
            },
            "visibility_mask": 0xFFFFFFFF,
            "selected": False,
            "locked": False,
            "parent_locked": False,
        }
    ]
    assert grouped.metadata["scene_objects"][0]["parent_key"] == "3_Group"
    assert grouped.metadata["scene_line_objects"][0]["parent_key"] == "3_Group"
    assert grouped.metadata["scene_child_order"] == [
        {"parent_key": "0_Root", "child_keys": ["2_Point", "3_Group"]},
        {"parent_key": "3_Group", "child_keys": ["0_Mesh", "1_Line"]},
    ]

    grouped_scene_path = save_meshlib_object_mesh_mru_scene(
        grouped,
        tmp_path / "grouped_scene.mru",
        object_name="Root",
    )
    with zipfile.ZipFile(grouped_scene_path) as archive:
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    group_payload = root_payload["Children"]["1"]
    assert group_payload["Type"] == ["Object"]
    assert group_payload["Name"] == "Group"
    assert [group_payload["Children"][str(index)]["Name"] for index in range(2)] == [
        "Mesh",
        "Line",
    ]

    grouped_metadata = dict(grouped.metadata)
    grouped_metadata["scene_group_objects"] = [
        {**grouped.metadata["scene_group_objects"][0], "selected": True}
    ]
    grouped_selected = MeshDocument(
        vertices=grouped.vertices,
        faces=grouped.faces,
        unit=grouped.unit,
        metadata=grouped_metadata,
    )
    ungrouped = meshlib_ungroup_scene_objects(grouped_selected)
    assert ungrouped.metadata["scene_group_objects"] == []
    assert ungrouped.metadata["removed_scene_object_keys"] == ["3_Group"]
    assert ungrouped.metadata["scene_objects"][0]["parent_key"] == "0_Root"
    assert ungrouped.metadata["scene_line_objects"][0]["parent_key"] == "0_Root"
    assert ungrouped.metadata["scene_child_order"] == [
        {"parent_key": "0_Root", "child_keys": ["2_Point", "0_Mesh", "1_Line"]}
    ]


def test_save_meshlib_mru_scene_round_trips_shared_model_links_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    shared_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    shared_model = save_mesh(shared_mesh, tmp_path / "shared.ply", file_type="ply")
    root = {
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Shared A",
                "Key": "0_Shared_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                "Link": "SharedModels/0_Shared",
            },
            "1": {
                "Name": "Shared B",
                "Key": "1_Shared_B",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                "Link": "SharedModels/0_Shared",
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                    },
                    "b": {"x": 0.0, "y": 3.0, "z": 0.0},
                },
            },
        },
    }
    source_scene = tmp_path / "source_shared.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr("Root.json", json.dumps(root))
        archive.write(shared_model, "SharedModels/0_Shared.ply")

    loaded = default_sdk.load_mesh(source_scene)
    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        loaded,
        tmp_path / "roundtrip_shared.mru",
        object_name="Root",
    )

    with zipfile.ZipFile(round_trip_scene) as archive:
        assert sorted(archive.namelist()) == ["Root.json", "SharedModels/0_Shared.ply"]
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert [root_payload["Children"][str(index)]["Link"] for index in (0, 1)] == [
        "SharedModels/0_Shared",
        "SharedModels/0_Shared",
    ]

    reloaded = default_sdk.load_mesh(round_trip_scene)
    np.testing.assert_allclose(reloaded.vertices, loaded.vertices)
    scene_objects = reloaded.metadata["scene_objects"]
    assert [scene_object["link"] for scene_object in scene_objects] == [
        "SharedModels/0_Shared",
        "SharedModels/0_Shared",
    ]
    assert scene_objects[0]["shared_model_source_index"] is None
    assert scene_objects[1]["shared_model_source_index"] == 0


def test_transform_mru_scene_object_updates_xf_and_round_trips_through_rust(tmp_path) -> None:
    from geometry_sdk import default_sdk
    from geometry_sdk.core.mesh import meshlib_transform_scene_object
    from geometry_sdk.io.trimesh_adapter import save_mesh, save_meshlib_object_mesh_mru_scene

    base_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    translated_mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 0.5, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    base_model = save_mesh(base_mesh, tmp_path / "base.ply", file_type="ply")
    translated_model = save_mesh(translated_mesh, tmp_path / "translated.ply", file_type="ply")
    source_scene = tmp_path / "source_transform.mru"
    with zipfile.ZipFile(source_scene, "w") as archive:
        archive.writestr(
            "Root.json",
            json.dumps(
                {
                    "FormatVersion": 1.0,
                    "Name": "Root",
                    "Key": "0_Root",
                    "Type": ["Object", "RootObject"],
                    "Children": {
                        "0": {
                            "Name": "Base A",
                            "Key": "0_Base_A",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                        },
                        "1": {
                            "Name": "Translated B",
                            "Key": "1_Translated",
                            "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
                            "XF": {
                                "A": {
                                    "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                                    "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                                    "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
                                },
                                "b": {"x": 4.0, "y": 0.0, "z": 0.0},
                            },
                        },
                    },
                }
            ),
        )
        archive.write(base_model, "0_Root/0_Base_A.ply")
        archive.write(translated_model, "0_Root/1_Translated.ply")

    loaded = default_sdk.load_mesh(source_scene)
    moved = meshlib_transform_scene_object(
        loaded,
        object_key="1_Translated",
        xf={
            "row_x": [1.0, 0.0, 0.0],
            "row_y": [0.0, 1.0, 0.0],
            "row_z": [0.0, 0.0, 1.0],
            "b": [8.0, 2.0, 0.0],
        },
    )

    np.testing.assert_allclose(
        moved.vertices,
        np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [8.0, 2.0, 0.0],
                [8.5, 2.0, 0.0],
                [8.0, 2.5, 0.0],
            ],
            dtype=np.float64,
        ),
    )
    assert moved.metadata["scene_objects"][1]["xf"]["b"] == [8.0, 2.0, 0.0]
    assert moved.metadata["meshlib_operation"] == "MR::Object::setXf/MR::FeatureObject::setXf"

    round_trip_scene = save_meshlib_object_mesh_mru_scene(
        moved,
        tmp_path / "transformed_roundtrip.mru",
        object_name="Root",
    )
    with zipfile.ZipFile(round_trip_scene) as archive:
        root_payload = json.loads(archive.read("Root.json").decode("utf-8"))
    assert root_payload["Children"]["1"]["XF"]["b"] == {"x": 8.0, "y": 2.0, "z": 0.0}

    reloaded = default_sdk.load_mesh(round_trip_scene)
    np.testing.assert_allclose(reloaded.vertices, moved.vertices)
    assert reloaded.metadata["scene_objects"][1]["xf"]["b"] == [8.0, 2.0, 0.0]


def test_codex_browser_stack_forces_backend_rust_sdk_attachment() -> None:
    run_script = Path(__file__).resolve().parents[2] / ".codex/run.sh"
    source = run_script.read_text(encoding="utf-8")

    assert 'GEOMETRY_SDK_ACCELERATOR="${GEOMETRY_SDK_ACCELERATOR:-rust}"' in source


def test_measure_inspect_endpoint_returns_rust_geodesic_path(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_measure",
        model_id="mdl_measure",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    artifact_path = tmp_path / "mesh.ply"
    artifact_path.write_text("ply\n", encoding="utf-8")
    artifact = ModelArtifactRecord(
        id="art_measure_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_measure/normalized_mesh_ply.ply",
        size_bytes=artifact_path.stat().st_size,
        metadata_json={},
    )

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return artifact
        return None

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: artifact_path)
    monkeypatch.setattr(versions_router.default_sdk, "load_mesh", lambda _path: mesh)

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        versions_router.measure_inspect(
            version.id,
            MeasureInspectRequest(
                points=[],
                point_pairs=[
                    MeasureInspectPair(
                        start=(0.0, 0.0, 0.0),
                        end=(1.0, 1.0, 0.0),
                        metric="geodesic",
                        start_vertex=0,
                        end_vertex=3,
                        control_vertices=[0, 1, 3],
                    ),
                    MeasureInspectPair(
                        start=(0.0, 0.0, 0.0),
                        end=(1.0, 1.0, 0.0),
                        metric="geodesic",
                        control_vertices=[0, 1, 3],
                        close_path=True,
                    ),
                    MeasureInspectPair(
                        start=(0.0, 0.0, 0.0),
                        end=(1.0, 1.0, 0.0),
                        metric="geodesic",
                        start_vertex=0,
                        end_vertex=3,
                        include_refined_surface_path=True,
                    ),
                ],
                features=[
                    MeasureInspectFeaturePrimitive(
                        feature_id="plane_xy",
                        kind="plane",
                        center=(0.0, 0.0, 0.0),
                        normal=(0.0, 0.0, 1.0),
                    ),
                    MeasureInspectFeaturePrimitive(
                        feature_id="axis_z",
                        kind="line",
                        center=(0.0, 0.0, 0.0),
                        direction=(0.0, 0.0, 1.0),
                        length_mm=4.0,
                    ),
                    MeasureInspectFeaturePrimitive(
                        feature_id="feature_sphere",
                        kind="sphere",
                        center=(3.0, 0.0, 0.0),
                        radius_mm=1.0,
                    ),
                ],
                feature_pairs=[
                    MeasureInspectFeaturePair(
                        first_feature_id="plane_xy",
                        second_feature_id="axis_z",
                        label="Plane to axis",
                    ),
                    MeasureInspectFeaturePair(
                        first_feature_id="axis_z",
                        second_feature_id="feature_sphere",
                        label="Axis to sphere center",
                    ),
                ],
                feature_refinements=[
                    MeasureInspectFeatureRefineRequest(
                        feature_id="plane_xy",
                        distance_limit_mm=0.1,
                        normal_tolerance_degrees=30.0,
                        max_iterations=4,
                        label="Refine plane",
                    )
                ],
                surface_distance=MeasureInspectSurfaceDistanceRequest(
                    seed=(0.0, 0.0, 0.0),
                    seed_vertex=0,
                    iso_value_mm=0.5,
                ),
                include_local_thickness=False,
            ),
            db=FakeDb(),
        )
    )

    pair = response.point_pairs[0]
    assert pair.metric == "geodesic"
    assert pair.distance_mm == pytest.approx(2.0)
    assert pair.line_segments == 2
    assert pair.control_vertex_indices == [0, 1, 3]
    assert pair.control_vertex_offsets == [0, 1, 2]
    assert pair.leg_lengths_mm == pytest.approx([1.0, 1.0])
    assert pair.leg_vertex_offsets == [0, 1]
    assert pair.edge_lengths_mm == pytest.approx([1.0, 1.0])
    assert pair.meshlib_reference == "MR::buildShortestPath control polyline"
    assert pair.path_vertex_indices[0] == 0
    assert pair.path_vertex_indices[-1] == 3
    assert len(pair.path_points) == 3
    assert pair.path_object_lines is not None
    assert pair.path_object_lines["Type"] == ["LinesHolder", "ObjectLines"]
    assert pair.path_object_lines["ShowPoints"] == 1
    assert pair.path_object_lines["MeshLibReference"] == "MR::ObjectLinesHolder / Polyline export"
    assert pair.path_object_lines["Polyline"]["Points"] == [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
    ]
    assert pair.path_object_lines["Polyline"]["Lines"] == [0, 1, 1, 2]
    assert pair.path_object_points is not None
    assert pair.path_object_points["Type"] == ["PointsHolder", "ObjectPoints"]
    assert pair.path_object_points["MeshLibReference"] == "MR::ObjectPointsHolder / PointCloud export"
    assert pair.path_object_points["PointCloud"]["Points"] == [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
    ]
    assert pair.path_object_points["PointCloud"]["Normals"] == [
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ]
    assert pair.cut_contours is not None
    assert pair.cut_contours["MeshLibReference"] == "MR::convertSurfacePathsToMeshContours / MR::cutMesh"
    assert pair.cut_contours["Contours"] == [
        {
            "Closed": False,
            "Intersections": [
                {"PrimitiveType": "VertId", "PrimitiveId": 0, "Coordinate": [0.0, 0.0, 0.0]},
                {"PrimitiveType": "VertId", "PrimitiveId": 1, "Coordinate": [1.0, 0.0, 0.0]},
                {"PrimitiveType": "VertId", "PrimitiveId": 3, "Coordinate": [1.0, 1.0, 0.0]},
            ],
        }
    ]
    assert pair.cut_contours["ResultCutVertexIndices"] == [[0, 1, 3]]
    assert pair.cut_contours["PivotIndices"] == [0, 1, 2]
    closed_pair = response.point_pairs[1]
    assert closed_pair.closed_path is True
    assert closed_pair.control_vertex_indices == [0, 1, 3, 0]
    assert closed_pair.path_vertex_indices[0] == 0
    assert closed_pair.path_vertex_indices[-1] == 0
    assert closed_pair.distance_mm == pytest.approx(4.0)
    assert closed_pair.line_segments == 4
    assert closed_pair.path_object_lines is not None
    assert closed_pair.path_object_lines["Polyline"]["Lines"] == [0, 1, 1, 2, 2, 3, 3, 0]
    assert closed_pair.cut_contours is not None
    assert closed_pair.cut_contours["ClosedPath"] is True
    assert closed_pair.cut_contours["ResultCutVertexIndices"] == [[0, 1, 3, 2, 0]]
    refined_pair = response.point_pairs[2]
    assert refined_pair.surface_path_refinement is not None
    assert refined_pair.surface_path_refinement["shared_edge"] == [1, 2]
    assert refined_pair.surface_path_refinement["crossing_t"] == pytest.approx(0.5)
    assert refined_pair.surface_path_refinement["crossing_point"] == pytest.approx((0.5, 0.5, 0.0))
    assert refined_pair.surface_path_refinement["length_mm"] == pytest.approx(np.sqrt(2.0))
    assert refined_pair.surface_path_refinement["graph_length_mm"] == pytest.approx(2.0)
    assert refined_pair.surface_path_refinement["meshlib_reference"] == "MR::shortestPathInQuadrangle / MR::reducePath"
    feature_pair = response.feature_pairs[0]
    assert feature_pair.first_feature_id == "plane_xy"
    assert feature_pair.second_feature_id == "axis_z"
    assert feature_pair.label == "Plane to axis"
    assert feature_pair.center_distance.status == "ok"
    assert feature_pair.center_distance.distance_mm == pytest.approx(0.0)
    assert feature_pair.distance.status == "ok"
    assert feature_pair.distance.distance_mm == pytest.approx(-2.0)
    assert feature_pair.distance.closest_point_a == pytest.approx((0.0, 0.0, 0.0))
    assert feature_pair.distance.closest_point_b == pytest.approx((0.0, 0.0, -2.0))
    assert len(feature_pair.intersections) == 1
    assert feature_pair.intersections[0].kind == "point"
    assert feature_pair.intersections[0].center == pytest.approx((0.0, 0.0, 0.0))
    assert feature_pair.intersections[0].meshlib_primitive == "MR::Features::Primitives::Sphere(point)"
    assert feature_pair.angle.status == "ok"
    assert feature_pair.angle.angle_degrees == pytest.approx(90.0)
    assert feature_pair.angle.is_surface_normal_a is True
    assert feature_pair.angle.is_surface_normal_b is False
    assert feature_pair.meshlib_reference == "MR::Features::MeasureResult"
    feature_pair = response.feature_pairs[1]
    assert feature_pair.distance.status == "ok"
    assert feature_pair.distance.distance_mm == pytest.approx(2.0)
    assert feature_pair.distance.closest_point_a == pytest.approx((0.0, 0.0, 0.0))
    assert feature_pair.distance.closest_point_b == pytest.approx((2.0, 0.0, 0.0))
    assert feature_pair.center_distance.distance_mm == pytest.approx(3.0)
    assert feature_pair.center_distance.closest_point_a == pytest.approx((0.0, 0.0, 0.0))
    assert feature_pair.center_distance.closest_point_b == pytest.approx((3.0, 0.0, 0.0))
    assert feature_pair.intersections == []
    assert feature_pair.angle.status == "bad_feature_pair"
    assert len(response.feature_objects) == 3
    assert response.feature_objects[0].feature_id == "plane_xy"
    assert response.feature_objects[0].object_type == "PlaneObject"
    assert [property.name for property in response.feature_objects[0].shared_properties] == [
        "Center",
        "Normal",
        "Size",
        "SizeX",
        "SizeY",
    ]
    assert response.feature_objects[0].shared_properties[2].scalar_value == pytest.approx(1000.0)
    assert response.feature_objects[1].object_type == "LineObject"
    assert response.feature_objects[2].object_type == "SphereObject"
    assert response.feature_objects[2].shared_properties[0].name == "Radius"
    assert response.feature_objects[2].meshlib_reference == "MR::Features::primitiveToObject"
    assert len(response.feature_refinements) == 1
    refinement = response.feature_refinements[0]
    assert refinement.feature_id == "plane_xy"
    assert refinement.kind == "plane"
    assert refinement.label == "Refine plane"
    assert refinement.center == pytest.approx((0.5, 0.5, 0.0))
    assert refinement.direction == pytest.approx((0.0, 0.0, 1.0))
    assert refinement.selected_vertex_indices == [0, 1, 2, 3]
    assert refinement.selected_count == 4
    assert refinement.converged is True
    assert refinement.feature_object is not None
    assert refinement.feature_object.object_type == "PlaneObject"
    assert refinement.feature_object.shared_properties[0].vector_value == pytest.approx((0.5, 0.5, 0.0))
    assert refinement.meshlib_reference == "MR::refineFeatureObject"
    assert response.surface_distance is not None
    assert response.surface_distance.seed_vertex == 0
    assert response.surface_distance.reachable_vertex_count == 4
    assert response.surface_distance.distances_mm == pytest.approx([0.0, 1.0, 1.0, 1.7071067811865475])
    assert response.surface_distance.max_distance_mm == pytest.approx(1.7071067811865475)
    assert response.surface_distance.iso_value_mm == pytest.approx(0.5)
    assert response.surface_distance.selected_vertex_indices == [0]
    assert response.surface_distance.selected_face_indices == []
    assert response.surface_distance.crossing_face_indices == [0]
    assert response.surface_distance.boundary_edges == [(0, 1), (0, 2)]
    assert response.surface_distance.iso_segments == pytest.approx([((0.5, 0.0, 0.0), (0.0, 0.5, 0.0))])
    assert response.surface_distance.clipped_vertices == pytest.approx(
        [(0.0, 0.0, 0.0), (0.5, 0.0, 0.0), (0.0, 0.5, 0.0)]
    )
    assert response.surface_distance.clipped_faces == [(0, 1, 2)]
    assert response.surface_distance.clipped_source_face_indices == [0]
    assert response.surface_distance.clipped_source_vertex_indices == [0, None, None]
    assert response.surface_distance.ridge_edges == []
    assert response.surface_distance.gorge_edges == []

    object_response = asyncio.run(
        versions_router.measure_inspect(
            version.id,
            MeasureInspectRequest(
                points=[],
                point_pairs=[],
                features=[
                    MeasureInspectFeaturePrimitive(
                        feature_id="feature_point",
                        kind="sphere",
                        center=(1.0, 2.0, 3.0),
                        radius_mm=0.0,
                    )
                ],
                include_local_thickness=False,
            ),
            db=FakeDb(),
        )
    )

    assert object_response.points == []
    assert object_response.point_pairs == []
    assert object_response.feature_pairs == []
    assert len(object_response.feature_objects) == 1
    assert object_response.feature_objects[0].object_type == "PointObject"
    assert object_response.feature_objects[0].shared_properties[0].name == "Point"
    assert object_response.feature_objects[0].shared_properties[0].vector_value == pytest.approx((1.0, 2.0, 3.0))

    cone_response = asyncio.run(
        versions_router.measure_inspect(
            version.id,
            MeasureInspectRequest(
                points=[],
                point_pairs=[],
                features=[
                    MeasureInspectFeaturePrimitive(
                        feature_id="feature_cone",
                        kind="cone",
                        center=(0.0, 0.0, 0.0),
                        direction=(0.0, 0.0, 1.0),
                        radius_mm=2.0,
                        length_mm=10.0,
                    )
                ],
                include_local_thickness=False,
            ),
            db=FakeDb(),
        )
    )

    assert len(cone_response.feature_objects) == 1
    assert cone_response.feature_objects[0].object_type == "ConeObject"
    assert [property.name for property in cone_response.feature_objects[0].shared_properties] == [
        "Angle",
        "Height",
        "Center",
        "Main axis",
    ]
    assert cone_response.feature_objects[0].shared_properties[0].kind == "angle"
    assert cone_response.feature_objects[0].shared_properties[0].scalar_value == pytest.approx(np.arctan(2.0 / 10.0))
    assert cone_response.feature_objects[0].shared_properties[2].vector_value == pytest.approx((0.0, 0.0, 0.0))

    source_response = asyncio.run(
        versions_router.measure_inspect(
            version.id,
            MeasureInspectRequest(
                points=[],
                point_pairs=[],
                surface_distance=MeasureInspectSurfaceDistanceRequest(
                    seed_edges=[(1, 3)],
                    seed_face_ids=[0],
                ),
                include_local_thickness=False,
            ),
            db=FakeDb(),
        )
    )

    assert source_response.surface_distance is not None
    assert source_response.surface_distance.seed_vertex == 0
    assert source_response.surface_distance.seed_vertices == [0, 1, 2, 3]
    assert source_response.surface_distance.seed_edges == [(1, 3)]
    assert source_response.surface_distance.seed_face_ids == [0]
    assert source_response.surface_distance.seed_face_boundary_edges == [(0, 1), (0, 2), (1, 2)]
    assert source_response.surface_distance.distances_mm == pytest.approx([0.0, 0.0, 0.0, 0.0])

    extreme_response = asyncio.run(
        versions_router.measure_inspect(
            version.id,
            MeasureInspectRequest(
                points=[],
                point_pairs=[],
                surface_distance=MeasureInspectSurfaceDistanceRequest(
                    seed_vertices=[0, 3],
                    include_extreme_edges=True,
                ),
                include_local_thickness=False,
            ),
            db=FakeDb(),
        )
    )

    assert extreme_response.surface_distance is not None
    assert extreme_response.surface_distance.distances_mm == pytest.approx([0.0, 1.0, 1.0, 0.0])
    assert extreme_response.surface_distance.ridge_edges == [(1, 2)]
    assert extreme_response.surface_distance.gorge_edges == []


def test_measure_inspect_endpoint_returns_rust_fast_marching_mesh_tri_point_path(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_measure_tri_point",
        model_id="mdl_measure_tri_point",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
        created_at=datetime(2026, 6, 15, tzinfo=timezone.utc),
    )
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    artifact_path = tmp_path / "tri_point_mesh.ply"
    artifact_path.write_text("ply\n", encoding="utf-8")
    artifact = ModelArtifactRecord(
        id="art_measure_tri_point_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_measure_tri_point/normalized_mesh_ply.ply",
        size_bytes=artifact_path.stat().st_size,
        metadata_json={},
    )

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return artifact
        return None

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: artifact_path)
    monkeypatch.setattr(versions_router.default_sdk, "load_mesh", lambda _path: mesh)

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        versions_router.measure_inspect(
            version.id,
            MeasureInspectRequest(
                point_pairs=[
                    MeasureInspectPair(
                        start=(0.25, 0.25, 0.0),
                        end=(0.75, 0.75, 0.0),
                        metric="geodesic",
                        start_face_index=0,
                        start_barycentric=(0.5, 0.25, 0.25),
                        end_face_index=1,
                        end_barycentric=(0.25, 0.25, 0.5),
                    )
                ],
                include_local_thickness=False,
            ),
            db=FakeDb(),
        )
    )

    pair = response.point_pairs[0]
    assert pair.metric == "geodesic"
    assert pair.meshlib_reference == "MR::computeFastMarchingPath"
    assert pair.distance_mm == pytest.approx(np.sqrt(0.5))
    assert pair.line_segments == 2
    assert pair.path_vertex_indices == []
    assert pair.edge_lengths_mm == pytest.approx([np.sqrt(0.125), np.sqrt(0.125)])
    assert pair.path_points == pytest.approx(
        [
            (0.25, 0.25, 0.0),
            (0.5, 0.5, 0.0),
            (0.75, 0.75, 0.0),
        ]
    )
    assert pair.surface_path_refinement is not None
    assert pair.surface_path_refinement["start_face_index"] == 0
    assert pair.surface_path_refinement["end_face_index"] == 1
    assert pair.surface_path_refinement["edges"] == [[1, 2]]
    assert pair.surface_path_refinement["positions"] == pytest.approx([0.5])
    assert pair.surface_path_refinement["stopped_reason"] == "end_triangle_reached"
    assert pair.surface_path_refinement["meshlib_reference"] == "MR::computeFastMarchingPath"
    assert pair.path_object_lines is not None
    assert pair.path_object_lines["Polyline"]["Lines"] == [0, 1, 1, 2]


def test_mesh_cut_measure_topology_endpoint_registers_rust_cut_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_cut_source",
        model_id="mdl_cut",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    cut_version = ModelVersionRecord(
        id="ver_cut_output",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="mesh_cut_measure",
        operation_label="Cut seam",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    output_mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [5, 4, 3]], dtype=np.int64),
        metadata={},
    )
    artifact_path = tmp_path / "mesh.ply"
    artifact_path.write_text("ply\n", encoding="utf-8")
    source_artifact = ModelArtifactRecord(
        id="art_cut_source",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="model/ply",
        storage_key="ver_cut_source/normalized_mesh.ply",
        size_bytes=artifact_path.stat().st_size,
        metadata_json={},
    )
    output_artifact = ModelArtifactRecord(
        id="art_cut_output",
        version_id=cut_version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="model/ply",
        storage_key="ver_cut_output/normalized_mesh.ply",
        size_bytes=123,
        metadata_json={},
    )
    calls: list[str] = []

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001, ANN201
            return version if model is ModelVersionRecord and key == version.id else None

        def commit(self) -> None:
            calls.append("commit")

        def refresh(self, record) -> None:  # noqa: ANN001
            calls.append(f"refresh:{record.id}")

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001, ANN202
        assert version_id == version.id
        assert artifact_type == "normalized_mesh_ply"
        return source_artifact

    def fake_create_version(db, *, model_id, parent_version_id, operation_type, operation_label, status):  # noqa: ANN001, ANN202
        assert model_id == version.model_id
        assert parent_version_id == version.id
        assert operation_type == "mesh_cut_measure"
        assert operation_label == "Cut seam"
        assert status == "ready"
        calls.append("create_version")
        return cut_version

    def fake_topology_cut(mesh_arg, *, control_vertices, close_path, max_path_len_mm):  # noqa: ANN001, ANN202
        assert mesh_arg is source_mesh
        assert control_vertices == [1, 2]
        assert close_path is False
        assert max_path_len_mm is None
        calls.append("mesh_cut_measure_edge_path_topology_cut")
        return {
            "mesh": output_mesh,
            "source_path_vertex_indices": [1, 2],
            "result_cut_vertex_indices": [[4, 5]],
            "duplicate_vertex_map": [[1, 4], [2, 5]],
            "cut_edge_pairs": [[1, 2]],
            "result_cut_edge_pairs": [[4, 5]],
            "bad_face_indices": [],
            "closed_path": False,
            "length_mm": 1.0,
            "meshlib_reference": "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset",
        }

    def fake_save_mesh(mesh_arg, path, *, file_type):  # noqa: ANN001, ANN202
        assert mesh_arg is output_mesh
        assert file_type == "ply"
        Path(path).write_text("ply", encoding="utf-8")
        calls.append(f"save_mesh:{Path(path).name}")
        return Path(path)

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type, metadata_json=None):  # noqa: ANN001, ANN202
        assert version_id == cut_version.id
        assert artifact_type == "normalized_mesh_ply"
        assert mime_type == "model/ply"
        assert metadata_json["source"] == "rust_mesh_cut_measure_edge_path_topology_cut"
        assert metadata_json["rust_backed"] is True
        assert metadata_json["meshlib_contract"] == "MR::convertSurfacePathsToMeshContours -> MR::cutMesh"
        assert metadata_json["duplicate_vertex_map"] == [[1, 4], [2, 5]]
        assert metadata_json["result_cut_edge_pairs"] == [[4, 5]]
        calls.append("register_cut_artifact")
        return output_artifact

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: artifact_path)
    monkeypatch.setattr(versions_router.default_sdk, "load_mesh", lambda _path: source_mesh)
    monkeypatch.setattr(
        versions_router.default_sdk,
        "mesh_cut_measure_edge_path_topology_cut",
        fake_topology_cut,
    )
    monkeypatch.setattr(versions_router.default_sdk, "save_mesh", fake_save_mesh)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.settings, "TEMP_DIR", tmp_path)

    response = asyncio.run(
        versions_router.run_mesh_cut_measure_topology_for_version(
            version.id,
            MeshCutMeasureTopologyRequest(control_vertices=[1, 2], operation_label="Cut seam"),
            db=FakeDb(),
        )
    )

    assert response.version.id == cut_version.id
    assert response.source_version_id == version.id
    assert response.artifact_id == output_artifact.id
    assert response.artifact_url == f"/api/artifacts/{output_artifact.id}"
    assert response.output_vertex_count == 6
    assert response.output_face_count == 2
    assert response.duplicate_vertex_map == [[1, 4], [2, 5]]
    assert response.result_cut_vertex_indices == [[4, 5]]
    assert response.metadata["rust_backed"] is True
    assert calls == [
        "mesh_cut_measure_edge_path_topology_cut",
        "create_version",
        "save_mesh:ver_cut_source_mesh_cut_measure_topology.ply",
        "register_cut_artifact",
        "commit",
        "refresh:ver_cut_output",
    ]


def test_viewer_optional_json_artifact_missing_file_degrades_to_absent_payload() -> None:
    artifact = ModelArtifactRecord(
        id="art_missing_regions",
        version_id="ver_missing_regions",
        artifact_type="analysis_regions_json",
        mime_type="application/json",
        storage_key="ver_missing_regions/analysis_regions_json.json",
        size_bytes=0,
        metadata_json={},
    )

    assert versions_router._load_json_artifact(artifact) is None


def test_artifact_download_missing_local_file_returns_404_not_500() -> None:
    artifact = ModelArtifactRecord(
        id="art_missing_mesh",
        version_id="ver_missing_mesh",
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_missing_mesh/normalized_mesh_ply.ply",
        size_bytes=0,
        metadata_json={},
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return artifact if model is ModelArtifactRecord and key == artifact.id else None

    with pytest.raises(HTTPException) as exc_info:
        asyncio.run(versions_router.download_artifact(artifact.id, db=FakeDb()))

    assert exc_info.value.status_code == 404
    assert exc_info.value.detail == "Artifact file not found"


def test_workbench_download_stl_capability_points_to_current_version_artifact(monkeypatch) -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    artifact = ModelArtifactRecord(
        id="art_ready_stl",
        version_id=version.id,
        artifact_type="manufacturing_stl",
        mime_type="model/stl",
        storage_key="ver_ready/manufacturing.stl",
        size_bytes=123,
        metadata_json={},
    )

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "manufacturing_stl":
            return artifact
        return None

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)

    urls = versions_router._workbench_endpoint_urls(version, db=object())
    capabilities = versions_router._workbench_command_capabilities(version, db=object())
    download_capability = next(capability for capability in capabilities if capability.command_id == "download-stl")

    assert urls["artifact_endpoint_url"] == "/api/artifacts/art_ready_stl"
    assert download_capability.endpoint_url == "/api/artifacts/art_ready_stl"


def test_workbench_job_activity_capability_points_to_version_jobs_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = versions_router._workbench_command_capabilities(version)
    job_capability = next(capability for capability in capabilities if capability.command_id == "job-activity")

    assert urls["jobs_endpoint_url"] == "/api/versions/ver_ready/jobs"
    assert job_capability.endpoint_url == "/api/versions/ver_ready/jobs"


def test_viewer_and_workbench_manifests_expose_meshlib_texture_artifact(monkeypatch) -> None:
    version = ModelVersionRecord(
        id="ver_textured",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    artifacts = {
        "normalized_mesh_ply": ModelArtifactRecord(
            id="art_ply",
            version_id=version.id,
            artifact_type="normalized_mesh_ply",
            mime_type="model/ply",
            storage_key="ver_textured/normalized_mesh_ply.ply",
            size_bytes=123,
            metadata_json={},
        ),
        "texture_image": ModelArtifactRecord(
            id="art_texture",
            version_id=version.id,
            artifact_type="texture_image",
            mime_type="image/png",
            storage_key="ver_textured/texture_image.png",
            size_bytes=68,
            metadata_json={
                "source": "rust_mesh_from_ply_texture",
                "meshlib_reference": "MR::loadPly TextureFile",
                "file": "jewel_surface.png",
                "width": 1,
                "height": 1,
                "filter": "Linear",
                "wrap": "Clamp",
            },
        ),
        "meshlib_object_mesh_scene_json": ModelArtifactRecord(
            id="art_scene",
            version_id=version.id,
            artifact_type="meshlib_object_mesh_scene_json",
            mime_type="application/json",
            storage_key="ver_textured/meshlib_object_mesh_scene_json.json",
            size_bytes=512,
            metadata_json={
                "source": "rust_meshlib_object_mesh_scene_json",
                "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeFields_",
                "object_type": "ObjectMesh",
                "model_file": "0_textured.ply",
            },
        ),
        "meshlib_scene_mru": ModelArtifactRecord(
            id="art_scene_mru",
            version_id=version.id,
            artifact_type="meshlib_scene_mru",
            mime_type="application/zip",
            storage_key="ver_textured/meshlib_scene_mru.mru",
            size_bytes=1024,
            metadata_json={
                "source": "rust_meshlib_scene_mru",
                "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeModel_",
                "object_type": "ObjectMesh",
                "root_file": "Root.json",
                "root_key": "0_Root",
                "object_key": "0_textured",
                "model_file": "0_Root/0_textured.ply",
            },
        ),
    }
    snapshot = AnalysisSnapshotRecord(
        id="snp_textured",
        version_id=version.id,
        snapshot_type="manufacturability",
        payload_json=_minimal_snapshot_payload(version.id),
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        return artifacts.get(artifact_type) if version_id == version.id else None

    def fake_get_version_artifacts(db, version_id):  # noqa: ANN001
        return list(artifacts.values()) if version_id == version.id else []

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "get_version_artifacts", fake_get_version_artifacts)
    monkeypatch.setattr(versions_router, "get_snapshot", lambda db, version_id: snapshot if version_id == version.id else None)

    viewer = asyncio.run(versions_router.get_viewer_manifest(version.id, db=FakeDb()))
    workbench = asyncio.run(versions_router.get_meshlib_workbench_manifest(version.id, db=FakeDb()))

    assert viewer.texture_artifact_url == "/api/artifacts/art_texture"
    assert viewer.texture_metadata == artifacts["texture_image"].metadata_json
    assert viewer.meshlib_scene_object_url == "/api/artifacts/art_scene"
    assert viewer.meshlib_scene_object_metadata == artifacts["meshlib_object_mesh_scene_json"].metadata_json
    assert viewer.meshlib_scene_mru_url == "/api/artifacts/art_scene_mru"
    assert viewer.meshlib_scene_mru_metadata == artifacts["meshlib_scene_mru"].metadata_json
    assert workbench.texture_artifact_url == "/api/artifacts/art_texture"
    assert workbench.texture_metadata == artifacts["texture_image"].metadata_json
    assert workbench.meshlib_scene_object_url == "/api/artifacts/art_scene"
    assert workbench.meshlib_scene_object_metadata == artifacts["meshlib_object_mesh_scene_json"].metadata_json
    assert workbench.meshlib_scene_mru_url == "/api/artifacts/art_scene_mru"
    assert workbench.meshlib_scene_mru_metadata == artifacts["meshlib_scene_mru"].metadata_json


def test_viewer_and_workbench_manifests_expose_ordered_meshlib_texture_artifacts(monkeypatch) -> None:
    version = ModelVersionRecord(
        id="ver_multi_texture",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    artifacts = {
        "normalized_mesh_ply": ModelArtifactRecord(
            id="art_ply",
            version_id=version.id,
            artifact_type="normalized_mesh_ply",
            mime_type="model/ply",
            storage_key="ver_multi_texture/normalized_mesh_ply.ply",
            size_bytes=123,
            metadata_json={},
        ),
        "texture_0": ModelArtifactRecord(
            id="art_texture_a",
            version_id=version.id,
            artifact_type="texture_image",
            mime_type="image/png",
            storage_key="ver_multi_texture/albedo_a.png",
            size_bytes=68,
            metadata_json={
                "source": "rust_mesh_from_obj_texture",
                "meshlib_reference": "MR::MeshLoad::fromSceneObjFile map_Kd",
                "texture_index": 0,
                "texture_count": 2,
                "texture_per_face": [0, 1],
                "file": "albedo_a.png",
                "width": 1,
                "height": 1,
                "filter": "Linear",
                "wrap": "Clamp",
            },
        ),
        "texture_1": ModelArtifactRecord(
            id="art_texture_b",
            version_id=version.id,
            artifact_type="texture_image",
            mime_type="image/png",
            storage_key="ver_multi_texture/albedo_b.png",
            size_bytes=68,
            metadata_json={
                "source": "rust_mesh_from_obj_texture",
                "meshlib_reference": "MR::MeshLoad::fromSceneObjFile map_Kd",
                "texture_index": 1,
                "texture_count": 2,
                "texture_per_face": [0, 1],
                "file": "albedo_b.png",
                "width": 1,
                "height": 1,
                "filter": "Linear",
                "wrap": "Clamp",
            },
        ),
    }
    snapshot = AnalysisSnapshotRecord(
        id="snp_multi_texture",
        version_id=version.id,
        snapshot_type="manufacturability",
        payload_json=_minimal_snapshot_payload(version.id),
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id != version.id:
            return None
        if artifact_type == "texture_image":
            return artifacts["texture_0"]
        return artifacts.get(artifact_type)

    def fake_get_version_artifacts(db, version_id):  # noqa: ANN001
        return list(artifacts.values()) if version_id == version.id else []

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "get_version_artifacts", fake_get_version_artifacts)
    monkeypatch.setattr(versions_router, "get_snapshot", lambda db, version_id: snapshot if version_id == version.id else None)

    viewer = asyncio.run(versions_router.get_viewer_manifest(version.id, db=FakeDb()))
    workbench = asyncio.run(versions_router.get_meshlib_workbench_manifest(version.id, db=FakeDb()))

    assert viewer.texture_artifact_url == "/api/artifacts/art_texture_a"
    assert viewer.texture_per_face == [0, 1]
    assert [item.texture_index for item in viewer.texture_artifacts] == [0, 1]
    assert [item.artifact_url for item in viewer.texture_artifacts] == [
        "/api/artifacts/art_texture_a",
        "/api/artifacts/art_texture_b",
    ]
    assert viewer.texture_artifacts[1].metadata["file"] == "albedo_b.png"
    assert workbench.texture_artifact_url == "/api/artifacts/art_texture_a"
    assert workbench.texture_per_face == [0, 1]
    assert [item.artifact_url for item in workbench.texture_artifacts] == [
        "/api/artifacts/art_texture_a",
        "/api/artifacts/art_texture_b",
    ]


def test_ingest_registers_first_rust_loaded_meshlib_texture_artifact(monkeypatch, tmp_path) -> None:
    texture_path = tmp_path / "jewel_surface.png"
    texture_path.write_bytes(b"texture")
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={
            "texture_images": [
                {
                    "file": "jewel_surface.png",
                    "resolved_path": str(texture_path),
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[255, 255, 255, 255]],
                }
            ]
        },
    )
    registered: list[dict[str, object]] = []

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.append(
            {
                "version_id": version_id,
                "file_path": Path(file_path),
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        return ModelArtifactRecord(
            id=f"art_{artifact_type}",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "application/octet-stream",
            storage_key=f"ver_ready/{artifact_type}",
            size_bytes=1,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(ingest_service, "register_file_artifact", fake_register_file_artifact)

    artifact = ingest_service._register_mesh_texture_artifact(object(), "ver_ready", mesh)

    assert artifact is not None
    assert registered == [
        {
            "version_id": "ver_ready",
            "file_path": texture_path,
            "artifact_type": "texture_image",
            "mime_type": "image/png",
            "metadata_json": {
                "source": "rust_mesh_from_ply_texture",
                "meshlib_reference": "MR::loadPly TextureFile",
                "meshlib_source": "MeshLib/source/MRMesh/MRPly.cpp",
                "texture_index": 0,
                "texture_count": 1,
                "texture_per_face": [],
                "file": "jewel_surface.png",
                "width": 1,
                "height": 1,
                "filter": "Linear",
                "wrap": "Clamp",
            },
        }
    ]


def test_ingest_registers_all_obj_map_kd_textures_with_meshlib_texture_per_face(monkeypatch, tmp_path) -> None:
    texture_a = tmp_path / "albedo_a.png"
    texture_b = tmp_path / "albedo_b.png"
    texture_a.write_bytes(b"texture-a")
    texture_b.write_bytes(b"texture-b")
    mesh = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        faces=np.array([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
        metadata={
            "source": "rust_mesh_from_obj",
            "meshlib_reference": "MR::MeshLoad::fromSceneObjFile",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshLoadObj.cpp",
            "texture_per_face": [0, 1],
            "texture_images": [
                {
                    "file": "albedo_a.png",
                    "resolved_path": str(texture_a),
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[255, 0, 0, 255]],
                },
                {
                    "file": "albedo_b.png",
                    "resolved_path": str(texture_b),
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[0, 0, 255, 255]],
                },
            ],
        },
    )
    registered: list[dict[str, object]] = []

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.append(
            {
                "version_id": version_id,
                "file_path": Path(file_path),
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        return ModelArtifactRecord(
            id=f"art_{metadata_json['texture_index']}",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "application/octet-stream",
            storage_key=f"ver_ready/{metadata_json['file']}",
            size_bytes=1,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(ingest_service, "register_file_artifact", fake_register_file_artifact)

    artifacts = ingest_service._register_mesh_texture_artifacts(object(), "ver_ready", mesh)

    assert [artifact.id for artifact in artifacts] == ["art_0", "art_1"]
    assert [item["file_path"] for item in registered] == [texture_a, texture_b]
    assert [item["metadata_json"]["texture_index"] for item in registered] == [0, 1]
    assert [item["metadata_json"]["texture_count"] for item in registered] == [2, 2]
    assert [item["metadata_json"]["texture_per_face"] for item in registered] == [[0, 1], [0, 1]]


def test_ingest_registers_obj_map_kd_texture_artifact_with_meshlib_obj_source(monkeypatch, tmp_path) -> None:
    texture_path = tmp_path / "albedo.png"
    texture_path.write_bytes(b"texture")
    mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
        metadata={
            "source": "rust_mesh_from_obj",
            "meshlib_reference": "MR::MeshLoad::fromSceneObjFile",
            "meshlib_source": "MeshLib/source/MRMesh/MRMeshLoadObj.cpp",
            "texture_images": [
                {
                    "file": "albedo.png",
                    "resolved_path": str(texture_path),
                    "width": 1,
                    "height": 1,
                    "filter": "Linear",
                    "wrap": "Clamp",
                    "pixels_rgba": [[255, 255, 255, 255]],
                }
            ],
        },
    )
    registered: list[dict[str, object]] = []

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.append(
            {
                "version_id": version_id,
                "file_path": Path(file_path),
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        return ModelArtifactRecord(
            id=f"art_{artifact_type}",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "application/octet-stream",
            storage_key=f"ver_ready/{artifact_type}",
            size_bytes=1,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(ingest_service, "register_file_artifact", fake_register_file_artifact)

    artifact = ingest_service._register_mesh_texture_artifact(object(), "ver_ready", mesh)

    assert artifact is not None
    assert registered == [
        {
            "version_id": "ver_ready",
            "file_path": texture_path,
            "artifact_type": "texture_image",
            "mime_type": "image/png",
            "metadata_json": {
                "source": "rust_mesh_from_obj_texture",
                "meshlib_reference": "MR::MeshLoad::fromSceneObjFile map_Kd",
                "meshlib_source": "MeshLib/source/MRMesh/MRMeshLoadObj.cpp",
                "texture_index": 0,
                "texture_count": 1,
                "texture_per_face": [],
                "file": "albedo.png",
                "width": 1,
                "height": 1,
                "filter": "Linear",
                "wrap": "Clamp",
            },
        }
    ]


def test_gcode_parse_paths_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["gcode-parse-paths"]

    assert urls["gcode_parse_paths_endpoint_url"] == "/api/versions/ver_ready/gcode/parse-paths"
    assert capability.endpoint_url_key == "gcode_parse_paths_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_ready/gcode/parse-paths"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["parse_gcode_paths"]


def test_mesh_to_voxels_sdf_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["mesh-to-voxels-sdf"]

    assert urls["voxelize_mesh_endpoint_url"] == "/api/versions/ver_ready/voxels/mesh-to-sdf"
    assert capability.endpoint_url_key == "voxelize_mesh_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_ready/voxels/mesh-to-sdf"
    assert capability.rust_backed is True
    assert capability.sdk_operations == [
        "sample_sdf_grid",
        "sdf_occupancy",
        "estimate_sdf_volume",
        "extract_sdf_isosurface",
    ]


def test_voxel_binary_operations_capability_exposes_meshlib_common_plugin_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-binary-operations"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == [
        "voxel_binary_values",
        "voxel_binary_iso_value",
    ]
    assert "MeshLib BinaryOperations" in capability.notes[0]


def test_voxel_binary_operations_endpoint_returns_rust_meshlib_scalar_grid_payload() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_binary",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-binary-operations"]

    assert urls["voxel_binary_operations_endpoint_url"] == "/api/versions/ver_voxel_binary/voxels/binary"
    assert capability.endpoint_url_key == "voxel_binary_operations_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_voxel_binary/voxels/binary"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_binary_operations_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelBinaryOperationsRequest(
        left_values=[1.0, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0],
        right_values=[0.5, -0.5, 1.5, -1.5, 2.0, -2.0, 4.0, -4.0],
        shape=(2, 2, 2),
        origin=(0.0, 0.0, 0.0),
        voxel_size_mm=1.0,
        operation="sum",
        left_iso_value=1.0,
        right_iso_value=2.0,
    )

    response = asyncio.run(
        versions_router.voxel_binary_operations_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.shape == (2, 2, 2)
    assert response.origin == (0.0, 0.0, 0.0)
    assert response.voxel_size_mm == 1.0
    assert response.operation == "sum"
    np.testing.assert_allclose(response.values, [1.5, 1.5, 4.5, 2.5, 1.0, -4.0, 1.0, -8.0])
    assert response.result_iso_value == pytest.approx(3.0)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_binary_operation"
    assert response.metadata["meshlib_reference"] == "MeshLib CommonPlugins BinaryOperations"
    assert response.metadata["meshlib_operations"] == ["max", "min", "sum", "multiply", "divide", "union", "intersection", "difference"]


def test_open_raw_voxels_capability_exposes_meshlib_common_plugin_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["open-raw-voxels"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == [
        "load_raw_voxels",
        "load_raw_voxels_auto",
        "voxel_default_iso_value",
    ]
    assert "VoxelsLoad::fromRaw" in capability.notes[0]


def test_open_raw_voxels_endpoint_returns_rust_meshlib_volume_payload() -> None:
    version = ModelVersionRecord(
        id="ver_open_raw_voxels",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["open-raw-voxels"]

    assert urls["open_raw_voxels_endpoint_url"] == "/api/versions/ver_open_raw_voxels/voxels/open-raw"
    assert capability.endpoint_url_key == "open_raw_voxels_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_open_raw_voxels/voxels/open-raw"
    assert capability.rust_backed is True
    assert getattr(versions_router, "VoxelRawLoadRequest", None) is not None
    assert getattr(versions_router, "VoxelVolumeLoadResponse", None) is not None
    assert hasattr(versions_router, "open_raw_voxels_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelRawLoadRequest(
        file_name="../unsafe/path/explicit.raw",
        contents_base64=base64.b64encode(np.array([0, 32768, 65535, 16384], dtype="<u2").tobytes()).decode("ascii"),
        dimensions=(2, 2, 1),
        voxel_size=(0.5, 1.0, 2.0),
        scalar_type="uint16",
    )

    response = asyncio.run(
        versions_router.open_raw_voxels_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.dimensions == (2, 2, 1)
    assert response.voxel_size == pytest.approx((0.5, 1.0, 2.0))
    assert response.scalar_type == "uint16"
    assert response.value_count == 4
    assert response.default_iso_value == pytest.approx(85.0 / 256.0)
    np.testing.assert_allclose(response.values, [0.0, 32768.0 / 65535.0, 1.0, 16384.0 / 65535.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "load_raw_voxels"
    assert response.metadata["meshlib_reference"] == "MR::VoxelsLoad::fromRaw"
    assert response.metadata["file_name"] == "explicit.raw"


def test_open_voxels_from_tiff_capability_exposes_meshlib_common_plugin_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["open-voxels-from-tiff"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["load_tiff_voxels_dir", "voxel_default_iso_value"]
    assert "VoxelsLoad::loadTiffDir" in capability.notes[0]


def _tiff_base64(values: np.ndarray) -> str:
    output = io.BytesIO()
    Image.fromarray(values).save(output, format="TIFF")
    return base64.b64encode(output.getvalue()).decode("ascii")


def test_open_voxels_from_tiff_endpoint_returns_rust_meshlib_volume_payload() -> None:
    version = ModelVersionRecord(
        id="ver_open_tiff_voxels",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["open-voxels-from-tiff"]

    assert urls["open_voxels_from_tiff_endpoint_url"] == (
        "/api/versions/ver_open_tiff_voxels/voxels/open-tiff-dir"
    )
    assert capability.endpoint_url_key == "open_voxels_from_tiff_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_open_tiff_voxels/voxels/open-tiff-dir"
    assert capability.rust_backed is True
    assert getattr(versions_router, "VoxelTiffLoadRequest", None) is not None
    assert getattr(versions_router, "VoxelVolumeLoadResponse", None) is not None
    assert hasattr(versions_router, "open_voxels_from_tiff_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelTiffLoadRequest(
        files={
            "../unsafe/path/slice_10.tiff": _tiff_base64(np.array([[10.0, 11.0]], dtype=np.float32)),
            "slice_02.tiff": _tiff_base64(np.array([[2.0, 3.0]], dtype=np.float32)),
        },
        voxel_size=(0.5, 0.25, 2.0),
    )

    response = asyncio.run(
        versions_router.open_voxels_from_tiff_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.dimensions == (2, 1, 2)
    assert response.voxel_size == pytest.approx((0.5, 0.25, 2.0))
    assert response.scalar_type == "tiff"
    assert response.default_iso_value == pytest.approx(2.0 + 85.0 * ((11.0 - 2.0) / 256.0))
    np.testing.assert_allclose(response.values, [2.0, 3.0, 10.0, 11.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "load_tiff_voxels_dir"
    assert response.metadata["meshlib_reference"] == "MR::VoxelsLoad::loadTiffDir"
    assert response.metadata["file_names"] == ["slice_10.tiff", "slice_02.tiff"]


def test_voxel_path_capability_exposes_meshlib_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-path"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_path"]
    assert "MRVoxelPath" in capability.notes[0]


def test_voxel_path_endpoint_returns_rust_meshlib_path_payload() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_path",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-path"]

    assert urls["voxel_path_endpoint_url"] == "/api/versions/ver_voxel_path/voxels/path"
    assert capability.endpoint_url_key == "voxel_path_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_voxel_path/voxels/path"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_path_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelPathRequest(
        values=[0.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0],
        shape=(3, 3, 1),
        start=(0, 1, 0),
        finish=(2, 1, 0),
        metric="difference",
    )

    response = asyncio.run(
        versions_router.voxel_path_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.coordinates[0] == (0, 1, 0)
    assert response.coordinates[-1] == (2, 1, 0)
    assert len(response.voxel_indices) == 5
    assert (1, 1, 0) not in response.coordinates
    assert response.total_metric == pytest.approx(0.0)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_path"
    assert response.metadata["meshlib_reference"] == "MRVoxelPath::buildSmallestMetricPath"
    assert response.metadata["meshlib_metrics"] == ["Difference", "Exponent"]


def test_voxel_path_build_four_capability_exposes_meshlib_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-path-build-four"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_path_build_four"]
    assert "QuarterBit" in capability.notes[0]


def test_voxel_path_build_four_endpoint_returns_rust_meshlib_quarter_paths() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_path_four",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-path-build-four"]

    assert urls["voxel_path_build_four_endpoint_url"] == "/api/versions/ver_voxel_path_four/voxels/path/build-four"
    assert capability.endpoint_url_key == "voxel_path_build_four_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_voxel_path_four/voxels/path/build-four"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_path_build_four_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelPathBuildFourRequest(
        values=[0.0] * 125,
        shape=(5, 5, 5),
        start=(0, 2, 2),
        finish=(4, 2, 2),
        metric="difference",
    )

    response = asyncio.run(
        versions_router.voxel_path_build_four_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert [entry.quarters_mask for entry in response.paths] == [1, 2, 4, 8]
    assert len(response.paths) == 4
    assert response.paths[0].path.coordinates[0] == (0, 2, 2)
    assert response.paths[0].path.coordinates[-1] == (4, 2, 2)
    assert response.paths[0].path.total_metric == pytest.approx(0.0)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_path_build_four"
    assert response.metadata["meshlib_reference"] == "MRVoxelPath::buildSmallestMetricPath"
    assert response.metadata["meshlib_quarter_masks"] == [1, 2, 4, 8]


def test_voxel_slice_capability_exposes_meshlib_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-slice"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_slice"]
    assert "MRVoxelsSave::saveSliceToImage" in capability.notes[0]


def test_voxel_slice_endpoint_returns_rust_meshlib_texture_payload() -> None:
    version = ModelVersionRecord(
        id="ver_slice",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-slice"]

    assert urls["voxel_slice_endpoint_url"] == "/api/versions/ver_slice/voxels/slice"
    assert capability.endpoint_url_key == "voxel_slice_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_slice/voxels/slice"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_slice_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = []
    for z in range(4):
        for y in range(3):
            for x in range(2):
                values.append(float(x + 10 * y + 100 * z))

    request = versions_router.VoxelSliceRequest(
        values=values,
        shape=(2, 3, 4),
        plane="xy",
        slice_index=2,
        min_value=200.0,
        max_value=221.0,
    )

    response = asyncio.run(
        versions_router.voxel_slice_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.width == 2
    assert response.height == 3
    assert response.coordinates[0] == (0, 0, 2)
    assert response.coordinates[-1] == (1, 2, 2)
    np.testing.assert_allclose(response.values, [200.0, 201.0, 210.0, 211.0, 220.0, 221.0])
    np.testing.assert_allclose(response.normalized_values, [0.0, 1.0 / 21.0, 10.0 / 21.0, 11.0 / 21.0, 20.0 / 21.0, 1.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_slice"
    assert response.metadata["meshlib_reference"] == "MRVoxelsSave::saveSliceToImage"
    assert response.metadata["meshlib_slice"] == "MRMarkedVoxelSlice"


def test_voxel_line_graph_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-line-graph"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_line_graph"]
    assert "Voxels Line Graph" in capability.notes[0]


def test_voxel_line_graph_endpoint_returns_rust_meshinspector_axis_probe_payload() -> None:
    version = ModelVersionRecord(
        id="ver_line_graph",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-line-graph"]

    assert urls["voxel_line_graph_endpoint_url"] == "/api/versions/ver_line_graph/voxels/line-graph"
    assert capability.endpoint_url_key == "voxel_line_graph_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_line_graph/voxels/line-graph"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_line_graph_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = []
    for z in range(2):
        for y in range(2):
            for x in range(3):
                values.append(float(x + 10 * y + 100 * z))

    request = versions_router.VoxelLineGraphRequest(
        values=values,
        shape=(3, 2, 2),
        axis="x",
        fixed_coordinate=(0, 1, 1),
    )

    response = asyncio.run(
        versions_router.voxel_line_graph_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.axis == 0
    assert response.positions == [0, 1, 2]
    assert response.voxel_indices == [9, 10, 11]
    assert response.coordinates == [(0, 1, 1), (1, 1, 1), (2, 1, 1)]
    np.testing.assert_allclose(response.values, [110.0, 111.0, 112.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_line_graph"
    assert response.metadata["meshlib_reference"] == "MeshInspector Voxels Line Graph"
    assert response.metadata["meshlib_indexing"] == "x-fastest dense voxel indexing"


def test_voxel_active_box_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-active-box"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_active_box"]
    assert "setActiveBounds" in capability.notes[0]


def test_voxel_active_box_endpoint_returns_rust_meshlib_crop_payload() -> None:
    version = ModelVersionRecord(
        id="ver_active_box",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-active-box"]

    assert urls["voxel_active_box_endpoint_url"] == "/api/versions/ver_active_box/voxels/active-box"
    assert capability.endpoint_url_key == "voxel_active_box_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_active_box/voxels/active-box"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_active_box_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = []
    for z in range(2):
        for y in range(3):
            for x in range(4):
                values.append(float(x + 10 * y + 100 * z))

    request = versions_router.VoxelActiveBoxRequest(
        values=values,
        shape=(4, 3, 2),
        min_corner=(1, 1, 0),
        dimensions=(2, 2, 2),
    )

    response = asyncio.run(
        versions_router.voxel_active_box_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.min_corner == (1, 1, 0)
    assert response.dimensions == (2, 2, 2)
    assert response.source_indices == [5, 6, 9, 10, 17, 18, 21, 22]
    assert response.coordinates == [
        (1, 1, 0),
        (2, 1, 0),
        (1, 2, 0),
        (2, 2, 0),
        (1, 1, 1),
        (2, 1, 1),
        (1, 2, 1),
        (2, 2, 1),
    ]
    np.testing.assert_allclose(response.values, [11.0, 12.0, 21.0, 22.0, 111.0, 112.0, 121.0, 122.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_active_box"
    assert response.metadata["meshlib_reference"] == "ObjectVoxels::setActiveBounds"
    assert response.metadata["meshlib_bounds"] == "max-excluded active voxel box"


def test_voxel_segmentation_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-segmentation"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_segmentation", "voxel_segmentation_mesh"]
    assert "MRVoxelGraphCut" in capability.notes[0]
    assert "MRVolumeSegment" in capability.notes[0]
    assert "createMeshFromSegmentation" in capability.notes[0]


def test_voxel_segmentation_endpoint_returns_rust_meshlib_mesh_payload() -> None:
    version = ModelVersionRecord(
        id="ver_segment_mesh",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-segmentation"]

    assert urls["voxel_segmentation_endpoint_url"] == "/api/versions/ver_segment_mesh/voxels/segmentation"
    assert capability.endpoint_url_key == "voxel_segmentation_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_segment_mesh/voxels/segmentation"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_segmentation_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelSegmentationRequest(
        values=[
            *([0.0] * 62),
            10.0,
            *([0.0] * 62),
        ],
        shape=(5, 5, 5),
        voxel_size=(0.5, 1.0, 2.0),
        inside_seeds=[(2, 2, 2)],
        outside_seeds=[],
        exponent_modifier=2.0,
        voxels_expansion=1,
        include_boundary_outside=True,
    )

    response = asyncio.run(
        versions_router.voxel_segmentation_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count > 0
    assert response.face_count > 0
    np.testing.assert_allclose(response.bounds_min, (0.75, 1.5, 3.0))
    np.testing.assert_allclose(response.bounds_max, (1.25, 2.5, 5.0))
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_segmentation_mesh"
    assert response.metadata["meshlib_reference"] == "MRVoxelGraphCut + MRVolumeSegment::createMeshFromSegmentation"
    assert response.metadata["segmentation"]["selected_coordinates"] == [(2, 2, 2)]


def test_voxel_mask_to_mesh_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-mask-to-mesh"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_mask_to_mesh"]
    assert "meshFromVoxelsMask" in capability.notes[0]
    assert "prepareVolumePart" in capability.notes[0]
    assert "VolumeMaskMeshingMode::Smooth" in capability.notes[0]


def test_voxel_mask_to_mesh_endpoint_returns_rust_meshlib_mesh_payload() -> None:
    version = ModelVersionRecord(
        id="ver_mask_mesh",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-mask-to-mesh"]

    assert urls["voxel_mask_to_mesh_endpoint_url"] == "/api/versions/ver_mask_mesh/voxels/mask-to-mesh"
    assert capability.endpoint_url_key == "voxel_mask_to_mesh_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_mask_mesh/voxels/mask-to-mesh"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_mask_to_mesh_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelMaskToMeshRequest(
        values=[
            *([0.0] * 62),
            10.0,
            *([0.0] * 62),
        ],
        shape=(5, 5, 5),
        voxel_size=(0.5, 1.0, 2.0),
        mask_coordinates=[(2, 2, 2)],
        mask_expansion=1,
        smooth_band_radius=3,
    )

    response = asyncio.run(
        versions_router.voxel_mask_to_mesh_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count > 0
    assert response.face_count > 0
    np.testing.assert_allclose(response.bounds_min, (0.75, 1.5, 3.0))
    np.testing.assert_allclose(response.bounds_max, (1.25, 2.5, 5.0))
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_mask_to_mesh"
    assert response.metadata["meshlib_reference"] == "MRVolumeSegment::meshFromVoxelsMask"
    assert response.metadata["mask"]["selected_coordinates"] == [(2, 2, 2)]


def test_voxel_to_mesh_simple_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-to-mesh-simple"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_to_mesh_simple"]
    assert "ObjectVoxels::recalculateIsoSurface" in capability.notes[0]
    assert "lessInside=false" in capability.notes[0]
    assert "lessInside=true" in capability.notes[0]
    assert "Dual Marching Cubes" in capability.notes[0]


def test_voxel_to_mesh_simple_endpoint_returns_rust_meshlib_mesh_payload() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_simple",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-to-mesh-simple"]

    assert urls["voxel_to_mesh_simple_endpoint_url"] == "/api/versions/ver_voxel_mesh_simple/voxels/to-mesh/simple"
    assert capability.endpoint_url_key == "voxel_to_mesh_simple_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_voxel_mesh_simple/voxels/to-mesh/simple"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_to_mesh_simple_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [0.0] * 125
    values[62] = 10.0
    request = versions_router.VoxelToMeshSimpleRequest(
        values=values,
        shape=(5, 5, 5),
        voxel_size=(0.5, 1.0, 2.0),
        iso_value=5.0,
    )

    response = asyncio.run(
        versions_router.voxel_to_mesh_simple_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count > 0
    assert response.face_count > 0
    np.testing.assert_allclose(response.bounds_min, [0.75, 1.5, 3.0])
    np.testing.assert_allclose(response.bounds_max, [1.25, 2.5, 5.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_to_mesh_simple"
    assert response.metadata["source"] == "voxel_to_mesh_simple"
    assert response.metadata["iso_value"] == 5.0
    assert response.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface"
    assert response.metadata["parity_status"] == "partial_dual_marching_cubes_pending"


def test_voxel_to_mesh_dual_endpoint_returns_rust_meshlib_mesh_payload() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_dual",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-to-mesh-dual"]

    assert urls["voxel_to_mesh_dual_endpoint_url"] == "/api/versions/ver_voxel_mesh_dual/voxels/to-mesh/dual"
    assert capability.endpoint_url_key == "voxel_to_mesh_dual_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_voxel_mesh_dual/voxels/to-mesh/dual"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_to_mesh_dual"]
    assert hasattr(versions_router, "voxel_to_mesh_dual_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [0.0] * 64
    for x in range(4):
        for y in range(4):
            for z in range(4):
                values[x + y * 4 + z * 16] = float(x)
    request = versions_router.VoxelToMeshDualRequest(
        values=values,
        shape=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        iso_value=1.5,
        grid_level_set=True,
        min_value=0.0,
        max_value=3.0,
    )

    response = asyncio.run(
        versions_router.voxel_to_mesh_dual_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count == 9
    assert response.face_count == 8
    np.testing.assert_allclose(response.bounds_min, [0.75, 0.5, 1.0])
    np.testing.assert_allclose(response.bounds_max, [0.75, 2.5, 5.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_to_mesh_dual"
    assert response.metadata["source"] == "voxel_to_mesh_dual"
    assert response.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface"
    assert (
        response.metadata["parity_status"]
        == "dense_dual_contouring_backed_sparse_openvdb_volume_to_mesh_pending"
    )


def test_voxel_to_mesh_dual_endpoint_enforces_meshlib_limits_through_rust() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_dual_limits",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [0.0] * 64
    for x in range(4):
        for y in range(4):
            for z in range(4):
                values[x + y * 4 + z * 16] = float(x)
    request = versions_router.VoxelToMeshDualRequest(
        values=values,
        shape=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        iso_value=1.5,
        grid_level_set=True,
        min_value=0.0,
        max_value=3.0,
        max_vertices=8,
    )

    with pytest.raises(HTTPException) as exc_info:
        asyncio.run(
            versions_router.voxel_to_mesh_dual_for_version(
                version.id,
                request,
                db=FakeDb(),
            )
        )

    assert exc_info.value.status_code == 400
    assert exc_info.value.detail == "Vertices number limit exceeded."


def test_voxel_to_mesh_dual_endpoint_exposes_meshlib_adaptivity_through_rust() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_dual_adaptivity",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [0.0] * 64
    for x in range(4):
        for y in range(4):
            for z in range(4):
                values[x + y * 4 + z * 16] = float(x)
    request = versions_router.VoxelToMeshDualRequest(
        values=values,
        shape=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        iso_value=1.5,
        grid_level_set=True,
        min_value=0.0,
        max_value=3.0,
        adaptivity=1.0,
    )

    response = asyncio.run(
        versions_router.voxel_to_mesh_dual_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count == 4
    assert response.face_count == 2
    np.testing.assert_allclose(response.bounds_min, [0.75, 0.5, 1.0])
    np.testing.assert_allclose(response.bounds_max, [0.75, 2.5, 5.0])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_to_mesh_dual"
    assert response.metadata["adaptivity"] == 1.0


def test_voxel_to_mesh_dual_endpoint_exposes_meshlib_relax_disoriented_triangles_through_rust() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_dual_relax_disoriented",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [0.0] * 64
    for x in range(4):
        for y in range(4):
            for z in range(4):
                values[x + y * 4 + z * 16] = float(x)
    request = versions_router.VoxelToMeshDualRequest(
        values=values,
        shape=(4, 4, 4),
        voxel_size=(0.5, 1.0, 2.0),
        iso_value=1.5,
        grid_level_set=True,
        min_value=0.0,
        max_value=3.0,
        relax_disoriented_triangles=False,
    )

    response = asyncio.run(
        versions_router.voxel_to_mesh_dual_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count == 9
    assert response.face_count == 8
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_to_mesh_dual"
    assert response.metadata["relax_disoriented_triangles"] is False


def test_voxel_to_mesh_dual_endpoint_accepts_openvdb_payload_through_rust() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_dual_vdb",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [float(x) for z in range(8) for y in range(8) for x in range(8)]
    request = versions_router.VoxelToMeshDualRequest(
        values=[],
        shape=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=3.5,
        grid_level_set=True,
        model_bytes_base64=base64.b64encode(synthetic_openvdb_single_dense_leaf(values)).decode("ascii"),
        model_extension=".vdb",
    )

    response = asyncio.run(
        versions_router.voxel_to_mesh_dual_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count == 49
    assert response.face_count == 72
    np.testing.assert_allclose(response.bounds_min, [1.75, 0.25, 0.25])
    np.testing.assert_allclose(response.bounds_max, [1.75, 3.25, 3.25])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_to_mesh_dual_vdb_payload"
    assert response.metadata["source"] == "voxel_to_mesh_dual_vdb_payload"
    assert response.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface"
    assert "direct .vdb" in response.metadata["meshlib_algorithm_reference"]
    assert (
        response.metadata["parity_status"]
        == "openvdb_dense_floatgrid_dual_meshing_backed_sparse_adaptivity_pending"
    )


def test_voxel_to_mesh_dual_endpoint_enforces_openvdb_payload_limits_through_rust() -> None:
    version = ModelVersionRecord(
        id="ver_voxel_mesh_dual_vdb_limits",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    values = [float(x) for z in range(8) for y in range(8) for x in range(8)]
    request = versions_router.VoxelToMeshDualRequest(
        values=[],
        shape=(1, 1, 1),
        voxel_size=(9.0, 9.0, 9.0),
        iso_value=3.5,
        grid_level_set=True,
        model_bytes_base64=base64.b64encode(synthetic_openvdb_single_dense_leaf(values)).decode("ascii"),
        model_extension=".vdb",
        max_faces=71,
    )

    with pytest.raises(HTTPException) as exc_info:
        asyncio.run(
            versions_router.voxel_to_mesh_dual_for_version(
                version.id,
                request,
                db=FakeDb(),
            )
        )

    assert exc_info.value.status_code == 400
    assert exc_info.value.detail == "Triangles number limit exceeded."


def test_voxel_volume_render_data_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-volume-render-data"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_volume_render_data"]
    assert "ObjectVoxels::prepareDataForVolumeRendering" in capability.notes[0]
    assert "vdbVolumeToSimpleVolumeNorm" in capability.notes[0]
    assert "GL shader compositing remains" in capability.notes[0]


def test_voxel_volume_render_lut_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-volume-render-lut"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_volume_render_lut"]
    assert "RenderVolumeObject::bindVolume_" in capability.notes[0]
    assert "VolumeRenderingParams::LutType" in capability.notes[0]
    assert "VolumeRenderingParams::AlphaType" in capability.notes[0]


def test_voxel_volume_render_data_endpoint_returns_rust_meshlib_payload() -> None:
    version = ModelVersionRecord(
        id="ver_volume_data",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-volume-render-data"]

    assert urls["voxel_volume_render_data_endpoint_url"] == "/api/versions/ver_volume_data/voxels/volume-render-data"
    assert capability.endpoint_url_key == "voxel_volume_render_data_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_volume_data/voxels/volume-render-data"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_volume_render_data_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelVolumeRenderDataRequest(
        values=list(range(24)),
        shape=(4, 3, 2),
        voxel_size=(0.5, 1.0, 2.0),
        active_min_corner=(1, 1, 0),
        active_dimensions=(2, 2, 2),
        source_min_value=0.0,
        source_max_value=23.0,
    )

    response = asyncio.run(
        versions_router.voxel_volume_render_data_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.dimensions == (2, 2, 2)
    assert response.voxel_size == (0.5, 1.0, 2.0)
    assert response.source_indices == [5, 6, 9, 10, 17, 18, 21, 22]
    assert response.coordinates == [
        (1, 1, 0),
        (2, 1, 0),
        (1, 2, 0),
        (2, 2, 0),
        (1, 1, 1),
        (2, 1, 1),
        (1, 2, 1),
        (2, 2, 1),
    ]
    np.testing.assert_allclose(
        response.values,
        np.array([5.0, 6.0, 9.0, 10.0, 17.0, 18.0, 21.0, 22.0], dtype=np.float32) / 23.0,
    )
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_volume_render_data"
    assert response.metadata["meshlib_reference"] == "ObjectVoxels::prepareDataForVolumeRendering"
    assert response.metadata["meshlib_conversion"] == "vdbVolumeToSimpleVolumeNorm"


def test_voxel_volume_render_lut_endpoint_returns_rust_meshlib_payload() -> None:
    version = ModelVersionRecord(
        id="ver_volume_lut",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-volume-render-lut"]

    assert urls["voxel_volume_render_lut_endpoint_url"] == "/api/versions/ver_volume_lut/voxels/volume-render-lut"
    assert capability.endpoint_url_key == "voxel_volume_render_lut_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_volume_lut/voxels/volume-render-lut"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_volume_render_lut_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelVolumeRenderLutRequest(
        lut_type="one_color",
        alpha_type="linear_decreasing",
        alpha_limit=10,
        one_color=(12, 34, 56, 200),
    )

    response = asyncio.run(
        versions_router.voxel_volume_render_lut_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.lut_type == "one_color"
    assert response.alpha_type == "linear_decreasing"
    assert response.alpha_limit == 10
    assert response.colors_rgba == [(12, 34, 56, 10), (12, 34, 56, 0)]
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_volume_render_lut"
    assert response.metadata["meshlib_reference"] == "RenderVolumeObject::bindVolume_ denseMap"
    assert response.metadata["meshlib_lut_type"] == "VolumeRenderingParams::LutType"
    assert response.metadata["meshlib_alpha_type"] == "VolumeRenderingParams::AlphaType"


def test_voxel_volume_render_ray_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-volume-render-ray"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_volume_render_ray"]
    assert "MRVolumeShader" in capability.notes[0]
    assert "samplingStep" in capability.notes[0]
    assert "front-to-back" in capability.notes[0]
    assert "rayVoxelIntersection" in capability.notes[0]
    assert "clipping-plane" in capability.notes[0]
    assert "shadingMode == 1" in capability.notes[0]
    assert "shadingMode == 2" in capability.notes[0]
    assert "shadeColor lighting" in capability.notes[0]
    assert "zero-normal" in capability.notes[0]
    assert "lighting color modulation" not in capability.notes[0]
    assert "alpha-gradient normals remain" not in capability.notes[0]


def test_voxel_volume_render_ray_endpoint_returns_rust_meshlib_shader_payload() -> None:
    version = ModelVersionRecord(
        id="ver_volume_render",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-volume-render-ray"]

    assert urls["voxel_volume_render_ray_endpoint_url"] == "/api/versions/ver_volume_render/voxels/volume-render-ray"
    assert capability.endpoint_url_key == "voxel_volume_render_ray_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_volume_render/voxels/volume-render-ray"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_volume_render_ray_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelVolumeRenderRayRequest(
        values=[0.0, 0.5, 1.0],
        shape=(3, 1, 1),
        voxel_size=(1.0, 1.0, 1.0),
        min_corner=(0, 0, 0),
        ray_start=(-0.5, 0.5, 0.5),
        ray_direction=(1.0, 0.0, 0.0),
        sampling_step=1.0,
        min_value=0.0,
        max_value=1.0,
        lut_type="one_color",
        alpha_type="constant",
        alpha_limit=128,
        one_color=(100, 50, 25, 255),
        shading_mode="value_gradient",
        light_pos_eye=(-10.0, 0.5, 0.5),
        ambient_strength=0.25,
        specular_strength=0.0,
        spec_exp=16.0,
        max_steps=16,
    )

    response = asyncio.run(
        versions_router.voxel_volume_render_ray_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.accepted_indices == [0, 1, 2]
    assert response.first_opaque_world == (0.5, 0.5, 0.5)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "MRVolumeShader"
    assert response.metadata["meshlib_shader"] == "shadeColor"
    assert response.metadata["shading_mode"] == "value_gradient"
    np.testing.assert_allclose(
        response.color_rgba,
        [1.25 * 100.0 / 255.0, 1.25 * 50.0 / 255.0, 1.25 * 25.0 / 255.0, 1.0 - (1.0 - 128.0 / 255.0) ** 3],
        atol=1e-6,
    )


def test_voxel_to_mesh_smart_capability_exposes_meshinspector_ct_tool_command() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-to-mesh-smart"]

    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_to_mesh_smart", "voxel_move_mesh_to_max_deriv"]
    assert "MR::moveMeshToVoxelMaxDeriv" in capability.notes[0]
    assert "samplePoints" in capability.notes[0]
    assert "degree=3" in capability.notes[0]
    assert "degree=3..6" in capability.notes[0]
    assert "Smart Conversion" in capability.notes[0]


def test_voxel_to_mesh_smart_endpoint_returns_rust_meshlib_mesh_payload() -> None:
    version = ModelVersionRecord(
        id="ver_smart_mesh",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-to-mesh-smart"]

    assert urls["voxel_to_mesh_smart_endpoint_url"] == "/api/versions/ver_smart_mesh/voxels/to-mesh/smart"
    assert capability.endpoint_url_key == "voxel_to_mesh_smart_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_smart_mesh/voxels/to-mesh/smart"
    assert capability.rust_backed is True
    assert hasattr(versions_router, "voxel_to_mesh_smart_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    request = versions_router.VoxelToMeshSmartRequest(
        values=[
            *([0.0] * 62),
            10.0,
            *([0.0] * 62),
        ],
        shape=(5, 5, 5),
        voxel_size=(0.5, 1.0, 2.0),
        iso_value=5.0,
        iters=1,
        sample_points=6,
        degree=3,
        outlier_threshold=1.0,
        intermediate_smooth_force=0.0,
        preparation_smooth_force=0.0,
        smooth_shift_iterations=0,
        final_relax_iterations=0,
        final_relax_force=0.0,
    )

    response = asyncio.run(
        versions_router.voxel_to_mesh_smart_for_version(
            version.id,
            request,
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.vertex_count > 0
    assert response.face_count > 0
    np.testing.assert_allclose(response.bounds_min, (0.75, 1.5, 3.0))
    np.testing.assert_allclose(response.bounds_max, (1.3, 2.55, 5.05))
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "voxel_to_mesh_smart"
    assert response.metadata["meshlib_reference"] == "ObjectVoxels::recalculateIsoSurface + MR::moveMeshToVoxelMaxDeriv"
    assert response.metadata["smart_conversion"]["settings"]["sample_points"] == 6
    assert response.metadata["smart_conversion"]["settings"]["degree"] == 3


def test_voxel_boolean_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["voxel-boolean"]

    assert urls["voxel_boolean_endpoint_url"] == "/api/versions/ver_source/boolean/voxel"
    assert capability.endpoint_url_key == "voxel_boolean_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_source/boolean/voxel"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["voxel_boolean_mesh"]


def test_offset_shell_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    offset_capability = capabilities["offset-mesh"]
    assert urls["offset_mesh_endpoint_url"] == "/api/versions/ver_source/offset/voxel"
    assert offset_capability.endpoint_url_key == "offset_mesh_endpoint_url"
    assert offset_capability.endpoint_url == "/api/versions/ver_source/offset/voxel"
    assert offset_capability.rust_backed is True
    assert offset_capability.sdk_operations == ["voxel_offset_mesh"]

    shell_capability = capabilities["shell-mesh"]
    assert urls["shell_mesh_endpoint_url"] == "/api/versions/ver_source/shell/voxel"
    assert shell_capability.endpoint_url_key == "shell_mesh_endpoint_url"
    assert shell_capability.endpoint_url == "/api/versions/ver_source/shell/voxel"
    assert shell_capability.rust_backed is True
    assert shell_capability.sdk_operations == ["voxel_shell_mesh"]

    thickening_capability = capabilities["thicken-mesh"]
    assert urls["thicken_mesh_endpoint_url"] == "/api/versions/ver_source/offset/thicken"
    assert thickening_capability.endpoint_url_key == "thicken_mesh_endpoint_url"
    assert thickening_capability.endpoint_url == "/api/versions/ver_source/offset/thicken"
    assert thickening_capability.rust_backed is True
    assert thickening_capability.sdk_operations == ["voxel_thicken_mesh"]

    weighted_capability = capabilities["weighted-shell"]
    assert urls["weighted_shell_endpoint_url"] == "/api/versions/ver_source/offset/weighted-shell"
    assert weighted_capability.endpoint_url_key == "weighted_shell_endpoint_url"
    assert weighted_capability.endpoint_url == "/api/versions/ver_source/offset/weighted-shell"
    assert weighted_capability.rust_backed is True
    assert weighted_capability.sdk_operations == ["voxel_weighted_shell_mesh"]

    partial_capability = capabilities["partial-offset"]
    assert urls["partial_offset_endpoint_url"] == "/api/versions/ver_source/offset/partial"
    assert partial_capability.endpoint_url_key == "partial_offset_endpoint_url"
    assert partial_capability.endpoint_url == "/api/versions/ver_source/offset/partial"
    assert partial_capability.rust_backed is True
    assert partial_capability.sdk_operations == ["voxel_partial_offset_mesh"]

    offset_verts_capability = capabilities["offset-verts"]
    assert urls["offset_verts_endpoint_url"] == "/api/versions/ver_source/offset/verts"
    assert offset_verts_capability.endpoint_url_key == "offset_verts_endpoint_url"
    assert offset_verts_capability.endpoint_url == "/api/versions/ver_source/offset/verts"
    assert offset_verts_capability.rust_backed is True
    assert offset_verts_capability.sdk_operations == ["offset_verts_mesh"]


def test_offset_smoothing_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    expand_capability = capabilities["expand-shrink"]
    assert urls["expand_shrink_endpoint_url"] == "/api/versions/ver_source/offset/expand-shrink"
    assert expand_capability.endpoint_url_key == "expand_shrink_endpoint_url"
    assert expand_capability.endpoint_url == "/api/versions/ver_source/offset/expand-shrink"
    assert expand_capability.rust_backed is True
    assert expand_capability.sdk_operations == ["voxel_offset_mesh"]

    shrink_capability = capabilities["shrink-expand"]
    assert urls["shrink_expand_endpoint_url"] == "/api/versions/ver_source/offset/shrink-expand"
    assert shrink_capability.endpoint_url_key == "shrink_expand_endpoint_url"
    assert shrink_capability.endpoint_url == "/api/versions/ver_source/offset/shrink-expand"
    assert shrink_capability.rust_backed is True
    assert shrink_capability.sdk_operations == ["voxel_offset_mesh"]


def test_collision_detect_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["collision-detect"]

    assert urls["collision_endpoint_url"] == "/api/versions/ver_source/collision/detect"
    assert capability.endpoint_url_key == "collision_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_source/collision/detect"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["exact_mesh_intersections"]


def test_exact_boolean_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["exact-boolean"]

    assert urls["exact_boolean_endpoint_url"] == "/api/versions/ver_source/boolean/exact"
    assert capability.endpoint_url_key == "exact_boolean_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_source/boolean/exact"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["exact_boolean_mesh"]


def test_gcode_parse_paths_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "GcodeParsePathsRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "parse_gcode_paths_for_version", None)
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(source="G90\nG0 X0 Y0 Z0 F3000\nG1 X1 Y2 Z3 F600\n"),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.frame_count == 3
    assert response.command_count == 11
    assert response.segment_count == 2
    assert response.source_frame_indices == [1, 2]
    assert response.idle == [True, False]
    assert response.feedrates == [10000.0, 600.0]
    assert response.max_feedrate == 600.0
    assert response.segments[0] == [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
    assert response.segments[1] == [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]
    assert response.tool_directions[0] == [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]
    assert response.warnings == []


def test_gcode_file_source_endpoints_return_meshlib_style_rust_payloads() -> None:
    version = ModelVersionRecord(
        id="ver_gcode_files",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    assert urls["gcode_load_source_endpoint_url"] == "/api/versions/ver_gcode_files/gcode/load-source"
    assert urls["gcode_write_source_endpoint_url"] == "/api/versions/ver_gcode_files/gcode/write-source"
    assert urls["gcode_parse_file_paths_endpoint_url"] == "/api/versions/ver_gcode_files/gcode/parse-file-paths"
    assert capabilities["gcode-load-source"].endpoint_url_key == "gcode_load_source_endpoint_url"
    assert capabilities["gcode-write-source"].endpoint_url_key == "gcode_write_source_endpoint_url"
    assert capabilities["gcode-parse-file-paths"].endpoint_url_key == "gcode_parse_file_paths_endpoint_url"
    assert hasattr(versions_router, "load_gcode_source_for_version")
    assert hasattr(versions_router, "write_gcode_source_for_version")
    assert hasattr(versions_router, "parse_gcode_file_paths_for_version")

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    source = "\n; retained comment frame\nG90\n\nG0 X0 Y0 Z0 F3000\nG1 X1 Y2 Z3 F600\n"
    load_response = asyncio.run(
        versions_router.load_gcode_source_for_version(
            version.id,
            versions_router.GcodeLoadSourceRequest(file_name="program.NC", source=source),
            db=FakeDb(),
        )
    )

    assert load_response.version_id == version.id
    assert load_response.frame_count == 4
    assert load_response.source_frames == [
        "; retained comment frame",
        "G90",
        "G0 X0 Y0 Z0 F3000",
        "G1 X1 Y2 Z3 F600",
    ]
    assert load_response.metadata["rust_backed"] is True
    assert load_response.metadata["sdk_operation"] == "load_gcode_source"
    assert load_response.metadata["meshlib_reference"] == "GcodeLoad::fromAnySupportedFormat"

    write_response = asyncio.run(
        versions_router.write_gcode_source_for_version(
            version.id,
            versions_router.GcodeWriteSourceRequest(
                file_name="roundtrip.gcode",
                source_frames=["G90", "G0 X0 Y0 Z0", "G1 X1 Y0 F500"],
            ),
            db=FakeDb(),
        )
    )

    assert write_response.version_id == version.id
    assert write_response.frame_count == 3
    assert write_response.source_frames == ["G90", "G0 X0 Y0 Z0", "G1 X1 Y0 F500"]
    assert write_response.metadata["rust_backed"] is True
    assert write_response.metadata["sdk_operation"] == "write_gcode_source"
    assert write_response.metadata["meshlib_reference"] == "ObjectGcode source frames"

    parse_response = asyncio.run(
        versions_router.parse_gcode_file_paths_for_version(
            version.id,
            versions_router.GcodeParseFilePathsRequest(file_name="program.nc", source=source),
            db=FakeDb(),
        )
    )

    assert parse_response.version_id == version.id
    assert parse_response.frame_count == 4
    assert parse_response.command_count == 11
    assert parse_response.segment_count == 2
    assert parse_response.source_frame_indices == [2, 3]
    assert parse_response.idle == [True, False]
    assert parse_response.feedrates == [10000.0, 600.0]
    assert parse_response.metadata["rust_backed"] is True
    assert parse_response.metadata["sdk_operation"] == "parse_gcode_file_paths"
    assert parse_response.metadata["meshlib_reference"] == "GcodeLoad::fromAnySupportedFormat + GcodeProcessor"


def test_point_cloud_icp_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_point_cloud_icp",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["point-cloud-icp"]

    assert urls["point_cloud_icp_endpoint_url"] == "/api/versions/ver_point_cloud_icp/point-cloud/icp"
    assert capability.endpoint_url_key == "point_cloud_icp_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_point_cloud_icp/point-cloud/icp"
    assert capability.rust_backed is True
    assert "pairwise_point_to_point_icp" in capability.sdk_operations
    assert "pairwise_point_to_plane_icp" in capability.sdk_operations
    assert getattr(versions_router, "PointCloudIcpRequest", None) is not None
    assert getattr(versions_router, "PointCloudIcpResponse", None) is not None
    assert hasattr(versions_router, "run_point_cloud_icp_for_version")


def test_point_cloud_icp_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_point_cloud_icp_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "PointCloudIcpRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_point_cloud_icp_for_version", None)
    assert endpoint is not None

    reference_points = [
        (0.0, 0.0, 0.0),
        (10.0, 0.0, 0.0),
        (0.0, 10.0, 0.0),
        (0.0, 0.0, 10.0),
        (8.0, 8.0, 8.0),
    ]
    floating_points = [
        (x + 0.25, y - 0.1, z + 0.05)
        for x, y, z in reference_points
    ]

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                floating_points=floating_points,
                reference_points=reference_points,
                method="point_to_point",
                mode="translation",
                max_iterations=10,
                tolerance=1e-12,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.method == "point_to_point"
    assert response.mode == "translation"
    assert response.iterations >= 1
    assert response.active_pair_count == len(reference_points)
    np.testing.assert_allclose(response.rotation, np.eye(3), atol=1e-9)
    np.testing.assert_allclose(response.translation, [-0.25, 0.1, -0.05], atol=1e-9)
    np.testing.assert_allclose(
        response.transform,
        [
            [1.0, 0.0, 0.0, -0.25],
            [0.0, 1.0, 0.0, 0.1],
            [0.0, 0.0, 1.0, -0.05],
            [0.0, 0.0, 0.0, 1.0],
        ],
        atol=1e-9,
    )
    assert response.mean_square_distance <= 1e-18
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "pairwise_point_to_point_icp"


def test_point_cloud_triangulation_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_point_cloud_triangulate",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["point-cloud-triangulate"]

    assert (
        urls["point_cloud_triangulation_endpoint_url"]
        == "/api/versions/ver_point_cloud_triangulate/point-cloud/triangulate"
    )
    assert capability.endpoint_url_key == "point_cloud_triangulation_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_point_cloud_triangulate/point-cloud/triangulate"
    assert capability.rust_backed is True
    assert capability.sdk_operations == [
        "point_cloud_triangulate_candidate_mesh",
        "point_cloud_triangulate_cleaned_candidate_mesh",
        "point_cloud_triangulate_topology_candidate_mesh",
        "point_cloud_triangulate_filled_candidate_mesh",
    ]
    assert getattr(versions_router, "PointCloudTriangulationRequest", None) is not None
    assert getattr(versions_router, "PointCloudTriangulationResponse", None) is not None
    assert hasattr(versions_router, "run_point_cloud_triangulation_for_version")


def test_point_cloud_triangulation_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_point_cloud_triangulate_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "PointCloudTriangulationRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_point_cloud_triangulation_for_version", None)
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                points=[
                    (0.0, 0.0, 0.0),
                    (1.0, 0.0, 0.0),
                    (0.0, 1.0, 0.0),
                    (1.0, 1.0, 0.0),
                    (0.5, 0.5, 0.0),
                ],
                stage="filled",
                radius=2.0,
                num_neighbors=0,
                max_removes=0,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.stage == "filled"
    assert response.vertex_count == 5
    assert response.face_count == 3
    assert len(response.vertices) == response.vertex_count
    assert len(response.faces) == response.face_count
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "point_cloud_triangulate_filled_candidate_mesh"
    assert response.metadata["meshlib_reference"] == "MR::triangulatePointCloud"


def test_point_cloud_multiway_icp_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_point_cloud_multiway_icp",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["point-cloud-multiway-icp"]

    assert (
        urls["point_cloud_multiway_icp_endpoint_url"]
        == "/api/versions/ver_point_cloud_multiway_icp/point-cloud/icp/multiway"
    )
    assert capability.endpoint_url_key == "point_cloud_multiway_icp_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_point_cloud_multiway_icp/point-cloud/icp/multiway"
    assert capability.rust_backed is True
    assert "multiway_point_to_point_icp" in capability.sdk_operations
    assert "multiway_aabb_cascade_point_to_point_icp" in capability.sdk_operations
    assert "multiway_aabb_cascade_combined_icp" in capability.sdk_operations
    assert getattr(versions_router, "PointCloudMultiwayIcpRequest", None) is not None
    assert getattr(versions_router, "PointCloudMultiwayIcpResponse", None) is not None
    assert hasattr(versions_router, "run_point_cloud_multiway_icp_for_version")


def test_point_cloud_multiway_icp_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_point_cloud_multiway_icp_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "PointCloudMultiwayIcpRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_point_cloud_multiway_icp_for_version", None)
    assert endpoint is not None

    reference_points = [
        (0.0, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (1.0, 1.0, 0.0),
    ]
    shifted_points = [
        (x + 0.25, y - 0.1, z + 0.05)
        for x, y, z in reference_points
    ]

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                objects=[reference_points, shifted_points],
                method="point_to_point",
                grouping="aabb_cascade",
                mode="translation",
                fixed_object_index=0,
                max_group_size=64,
                max_iterations=10,
                tolerance=1e-12,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.method == "point_to_point"
    assert response.grouping == "aabb_cascade"
    assert response.mode == "translation"
    assert response.fixed_object_index == 0
    assert response.iterations >= 1
    assert response.active_pair_count == len(reference_points) * 2
    assert len(response.transforms) == 2
    np.testing.assert_allclose(response.transforms[0].translation, [0.0, 0.0, 0.0], atol=1e-12)
    np.testing.assert_allclose(response.transforms[1].translation, [-0.25, 0.1, -0.05], atol=1e-9)
    assert response.mean_square_distance <= 1e-18
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "multiway_aabb_cascade_point_to_point_icp"
    assert response.metadata["meshlib_reference"] == "MR::ICPGroupPair::calculateTransformation"


def test_offset_contours_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_offset_contours",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["offset-contours"]

    assert urls["offset_contours_endpoint_url"] == "/api/versions/ver_offset_contours/contours/offset"
    assert capability.endpoint_url_key == "offset_contours_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_offset_contours/contours/offset"
    assert capability.rust_backed is True
    assert "offset_contours" in capability.sdk_operations
    assert "offset_contours_with_origins" in capability.sdk_operations
    assert getattr(versions_router, "OffsetContoursRequest", None) is not None
    assert getattr(versions_router, "OffsetContoursResponse", None) is not None
    assert hasattr(versions_router, "run_offset_contours_for_version")


def test_offset_contours_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_offset_contours_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "OffsetContoursRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_offset_contours_for_version", None)
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                contours=[
                    [
                        (0.0, 0.0, 0.0),
                        (2.0, 0.0, 0.0),
                        (2.0, 2.0, 0.0),
                        (0.0, 2.0, 0.0),
                    ]
                ],
                offset=0.25,
                mode="offset",
                end_type="round",
                corner_type="round",
                include_origins=True,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.contour_count >= 1
    assert response.point_count >= 4
    assert len(response.contours) == response.contour_count
    assert len(response.contours[0]) >= 4
    assert response.origins
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "offset_contours_with_origins"
    assert response.metadata["meshlib_reference"] == "MR::offsetContours"


def test_distance_map_contours_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_contours",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["distance-map-contours"]

    assert urls["distance_map_contours_endpoint_url"] == (
        "/api/versions/ver_distance_map_contours/distance-map/contours"
    )
    assert capability.endpoint_url_key == "distance_map_contours_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_distance_map_contours/distance-map/contours"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["distance_map_from_contours"]
    assert getattr(versions_router, "DistanceMapContoursRequest", None) is not None
    assert getattr(versions_router, "DistanceMapResponse", None) is not None
    assert hasattr(versions_router, "run_distance_map_from_contours_for_version")


def test_distance_map_contours_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_contours_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "DistanceMapContoursRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_from_contours_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                contours=[
                    [
                        (0.0, 0.0),
                        (2.0, 0.0),
                        (2.0, 2.0),
                        (0.0, 2.0),
                        (0.0, 0.0),
                    ]
                ],
                width=3,
                height=3,
                origin=(0.0, 0.0),
                pixel_size=(1.0, 1.0),
                signed=True,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.width == 3
    assert response.height == 3
    assert response.origin == (0.0, 0.0)
    assert response.pixel_size == (1.0, 1.0)
    assert response.valid_count == 9
    assert response.values[0][0] == pytest.approx(-0.5)
    assert response.values[1][1] == pytest.approx(-0.5)
    assert response.values[0][2] == pytest.approx(0.5)
    assert response.min_value < 0.0
    assert response.max_value > 0.0
    assert response.model_transform is not None
    assert len(response.model_transform) == 16
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_from_contours"
    assert response.metadata["meshlib_reference"] == "MR::Cuda::distanceMapFromContours / MR::DistanceMap"


def test_distance_map_from_mesh_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_from_mesh",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["distance-map-from-mesh"]

    assert urls["distance_map_from_mesh_endpoint_url"] == (
        "/api/versions/ver_distance_map_from_mesh/distance-map/mesh"
    )
    assert capability.endpoint_url_key == "distance_map_from_mesh_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_distance_map_from_mesh/distance-map/mesh"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["distance_map_from_mesh"]
    assert getattr(versions_router, "DistanceMapFromMeshRequest", None) is not None
    assert getattr(versions_router, "DistanceMapResponse", None) is not None
    assert hasattr(versions_router, "run_distance_map_from_mesh_for_version")


def test_distance_map_from_mesh_endpoint_returns_current_meshlib_style_rust_payload(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_from_mesh_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 2.0],
                [2.0, 0.0, 2.0],
                [2.0, 2.0, 2.0],
                [0.0, 2.0, 2.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
        metadata={},
    )
    mesh_path = tmp_path / "distance-map-plane.ply"
    versions_router.default_sdk.save_mesh(mesh, mesh_path)
    artifact = ModelArtifactRecord(
        id="art_distance_map_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_distance_map_from_mesh/normalized_mesh_ply.ply",
        size_bytes=mesh_path.stat().st_size,
        metadata_json={},
    )
    request_cls = getattr(versions_router, "DistanceMapFromMeshRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_from_mesh_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return artifact
        return None

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: mesh_path)

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                width=2,
                height=2,
                origin=(0.0, 0.0, 0.0),
                x_range=(2.0, 0.0, 0.0),
                y_range=(0.0, 2.0, 0.0),
                direction=(0.0, 0.0, 1.0),
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.width == 2
    assert response.height == 2
    assert response.origin == (0.0, 0.0)
    assert response.pixel_size == (1.0, 1.0)
    assert response.valid_count == 4
    np.testing.assert_allclose(response.values, [[2.0, 2.0], [2.0, 2.0]])
    assert response.min_value == pytest.approx(2.0)
    assert response.max_value == pytest.approx(2.0)
    assert response.model_transform is not None
    assert len(response.model_transform) == 16
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_from_mesh"
    assert response.metadata["meshlib_reference"] == "MR::computeDistanceMap / MR::MeshToDistanceMapParams"


def test_distance_map_iso_lines_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_iso_lines",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["distance-map-iso-lines"]

    assert urls["distance_map_iso_lines_endpoint_url"] == (
        "/api/versions/ver_distance_map_iso_lines/distance-map/iso-lines"
    )
    assert capability.endpoint_url_key == "distance_map_iso_lines_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_distance_map_iso_lines/distance-map/iso-lines"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["distance_map_to_iso_segments"]
    assert getattr(versions_router, "DistanceMapIsoLinesRequest", None) is not None
    assert getattr(versions_router, "IsoLineSegmentsResponse", None) is not None
    assert hasattr(versions_router, "run_distance_map_iso_lines_for_version")


def test_distance_map_iso_lines_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_iso_lines_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "DistanceMapIsoLinesRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_iso_lines_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                width=2,
                height=2,
                origin=(10.0, 20.0),
                pixel_size=(2.0, 4.0),
                values=[[-1.0, 1.0], [-1.0, 1.0]],
                valid_count=4,
                min_value=-1.0,
                max_value=1.0,
                iso_value=0.0,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.iso_value == 0.0
    assert response.segment_count == 1
    assert response.segments[0][0] == pytest.approx((12.0, 26.0))
    assert response.segments[0][1] == pytest.approx((12.0, 22.0))
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_to_iso_segments"
    assert response.metadata["meshlib_reference"] == "MR::distanceMapTo2DIsoPolyline"


def test_distance_map_merge_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_merge",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["distance-map-merge"]

    assert urls["distance_map_merge_endpoint_url"] == (
        "/api/versions/ver_distance_map_merge/distance-map/merge"
    )
    assert capability.endpoint_url_key == "distance_map_merge_endpoint_url"
    assert capability.endpoint_url == "/api/versions/ver_distance_map_merge/distance-map/merge"
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["distance_map_merge"]
    assert getattr(versions_router, "DistanceMapMergeRequest", None) is not None
    assert getattr(versions_router, "DistanceMapResponse", None) is not None
    assert hasattr(versions_router, "run_distance_map_merge_for_version")


def test_distance_map_merge_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_merge_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "DistanceMapMergeRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_merge_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    invalid = float(np.finfo(np.float32).min)
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                left={
                    "width": 3,
                    "height": 2,
                    "origin": (10.0, 20.0),
                    "pixel_size": (2.0, 4.0),
                    "values": [[2.0, invalid, -1.0], [4.0, 8.0, 16.0]],
                    "valid_count": 5,
                    "min_value": -1.0,
                    "max_value": 16.0,
                },
                right={
                    "width": 2,
                    "height": 2,
                    "origin": (10.0, 20.0),
                    "pixel_size": (2.0, 4.0),
                    "values": [[3.0, 5.0], [invalid, 6.0]],
                    "valid_count": 3,
                    "min_value": 3.0,
                    "max_value": 6.0,
                },
                mode="max",
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.width == 3
    assert response.height == 2
    assert response.origin == (10.0, 20.0)
    assert response.pixel_size == (2.0, 4.0)
    assert response.valid_count == 6
    np.testing.assert_allclose(response.values, [[3.0, 5.0, -1.0], [4.0, 8.0, 16.0]])
    assert response.min_value == pytest.approx(-1.0)
    assert response.max_value == pytest.approx(16.0)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_merge"
    assert response.metadata["meshlib_reference"] == "MR::DistanceMap::max/min/operator-"


def test_distance_map_contour_boolean_capability_points_to_executable_rust_endpoint() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_contour_boolean",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    capability = capabilities["distance-map-contour-boolean"]

    assert urls["distance_map_contour_boolean_endpoint_url"] == (
        "/api/versions/ver_distance_map_contour_boolean/distance-map/contour-boolean"
    )
    assert capability.endpoint_url_key == "distance_map_contour_boolean_endpoint_url"
    assert capability.endpoint_url == (
        "/api/versions/ver_distance_map_contour_boolean/distance-map/contour-boolean"
    )
    assert capability.rust_backed is True
    assert capability.sdk_operations == ["distance_map_contour_boolean"]
    assert getattr(versions_router, "DistanceMapContourBooleanRequest", None) is not None
    assert getattr(versions_router, "IsoLineSegmentsResponse", None) is not None
    assert hasattr(versions_router, "run_distance_map_contour_boolean_for_version")


def test_distance_map_contour_boolean_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_contour_boolean_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "DistanceMapContourBooleanRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_contour_boolean_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                contours_a=[[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)]],
                contours_b=[[(1.0, 0.0), (3.0, 0.0), (3.0, 2.0), (1.0, 2.0), (1.0, 0.0)]],
                mode="intersection",
                width=6,
                height=5,
                origin=(-1.0, -1.0),
                pixel_size=(1.0, 1.0),
                iso_value=0.0,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.iso_value == 0.0
    assert response.segment_count == 6
    np.testing.assert_allclose(
        response.segments,
        [
            [[1.5, 0.0], [1.0, 0.5]],
            [[2.0, 0.5], [1.5, 0.0]],
            [[1.0, 0.5], [1.0, 1.5]],
            [[2.0, 1.5], [2.0, 0.5]],
            [[1.0, 1.5], [1.5, 2.0]],
            [[1.5, 2.0], [2.0, 1.5]],
        ],
    )
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_contour_boolean"
    assert response.metadata["mode"] == "intersection"
    assert response.metadata["meshlib_reference"] == (
        "MR::contourUnion / MR::contourIntersection / MR::contourSubtract"
    )


def test_distance_map_tiff_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_tiff",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }
    import_capability = capabilities["distance-map-from-tiff"]
    export_capability = capabilities["distance-map-to-tiff"]

    assert urls["distance_map_from_tiff_endpoint_url"] == (
        "/api/versions/ver_distance_map_tiff/distance-map/from-tiff"
    )
    assert urls["distance_map_to_tiff_endpoint_url"] == (
        "/api/versions/ver_distance_map_tiff/distance-map/to-tiff"
    )
    assert import_capability.endpoint_url_key == "distance_map_from_tiff_endpoint_url"
    assert import_capability.endpoint_url == "/api/versions/ver_distance_map_tiff/distance-map/from-tiff"
    assert import_capability.rust_backed is True
    assert import_capability.sdk_operations == ["distance_map_from_tiff"]
    assert export_capability.endpoint_url_key == "distance_map_to_tiff_endpoint_url"
    assert export_capability.endpoint_url == "/api/versions/ver_distance_map_tiff/distance-map/to-tiff"
    assert export_capability.rust_backed is True
    assert export_capability.sdk_operations == ["distance_map_to_tiff"]
    assert getattr(versions_router, "DistanceMapTiffImportRequest", None) is not None
    assert getattr(versions_router, "DistanceMapTiffExportRequest", None) is not None
    assert getattr(versions_router, "DistanceMapTiffExportResponse", None) is not None
    assert hasattr(versions_router, "run_distance_map_from_tiff_for_version")
    assert hasattr(versions_router, "run_distance_map_to_tiff_for_version")


def test_distance_map_from_tiff_endpoint_returns_meshlib_style_rust_payload(tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_from_tiff_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    source_path = tmp_path / "height-field.tiff"
    distance_map = versions_router.DistanceMapDocument(
        width=2,
        height=2,
        origin=(10.0, 20.0),
        pixel_size=(2.5, 4.0),
        values=np.asarray([[1.0, 2.0], [3.0, 4.0]], dtype=np.float32),
        valid_count=4,
        min_value=1.0,
        max_value=4.0,
    )
    versions_router.default_sdk.distance_map_to_tiff(distance_map, source_path)
    request_cls = getattr(versions_router, "DistanceMapTiffImportRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_from_tiff_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                file_name="height-field.tiff",
                contents_base64=base64.b64encode(source_path.read_bytes()).decode("ascii"),
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.width == 2
    assert response.height == 2
    assert response.origin == (10.0, 20.0)
    assert response.pixel_size == (2.5, 4.0)
    assert response.valid_count == 4
    np.testing.assert_allclose(response.values, [[1.0, 2.0], [3.0, 4.0]])
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_from_tiff"
    assert response.metadata["meshlib_reference"] == "MR::DistanceMapLoad::fromTiff"


def test_distance_map_to_tiff_endpoint_returns_meshlib_style_rust_payload(tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_distance_map_to_tiff_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "DistanceMapTiffExportRequest", None)
    endpoint = getattr(versions_router, "run_distance_map_to_tiff_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                file_name="exported-height-field.tiff",
                width=2,
                height=2,
                origin=(10.0, 20.0),
                pixel_size=(2.5, 4.0),
                values=[[1.0, 2.0], [3.0, 4.0]],
                valid_count=4,
                min_value=1.0,
                max_value=4.0,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.file_name == "exported-height-field.tiff"
    assert response.byte_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "distance_map_to_tiff"
    assert response.metadata["meshlib_reference"] == "MR::DistanceMapSave::toTiff"
    output_path = tmp_path / response.file_name
    output_path.write_bytes(base64.b64decode(response.contents_base64))
    reloaded = versions_router.default_sdk.distance_map_from_tiff(output_path)
    assert reloaded.origin == (10.0, 20.0)
    assert reloaded.pixel_size == (2.5, 4.0)
    np.testing.assert_allclose(reloaded.values, [[1.0, 2.0], [3.0, 4.0]])


def test_object_lines_contour_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    from_contours = capabilities["object-lines-from-contours"]
    to_contours = capabilities["object-lines-to-contours"]

    assert urls["object_lines_from_contours_endpoint_url"] == (
        "/api/versions/ver_object_lines/object-lines/from-contours"
    )
    assert urls["object_lines_to_contours_endpoint_url"] == (
        "/api/versions/ver_object_lines/object-lines/to-contours"
    )
    assert from_contours.endpoint_url_key == "object_lines_from_contours_endpoint_url"
    assert from_contours.endpoint_url == "/api/versions/ver_object_lines/object-lines/from-contours"
    assert from_contours.rust_backed is True
    assert "object_lines_from_contours" in from_contours.sdk_operations
    assert to_contours.endpoint_url_key == "object_lines_to_contours_endpoint_url"
    assert to_contours.endpoint_url == "/api/versions/ver_object_lines/object-lines/to-contours"
    assert to_contours.rust_backed is True
    assert "object_lines_to_contours" in to_contours.sdk_operations
    assert getattr(versions_router, "ObjectLinesFromContoursRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesResponse", None) is not None
    assert getattr(versions_router, "ObjectLinesToContoursRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesToContoursResponse", None) is not None
    assert hasattr(versions_router, "run_object_lines_from_contours_for_version")
    assert hasattr(versions_router, "run_object_lines_to_contours_for_version")


def test_object_lines_contour_endpoints_return_meshlib_style_rust_payloads() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_endpoint",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    from_request_cls = getattr(versions_router, "ObjectLinesFromContoursRequest", None)
    to_request_cls = getattr(versions_router, "ObjectLinesToContoursRequest", None)
    from_endpoint = getattr(versions_router, "run_object_lines_from_contours_for_version", None)
    to_endpoint = getattr(versions_router, "run_object_lines_to_contours_for_version", None)
    assert from_request_cls is not None
    assert to_request_cls is not None
    assert from_endpoint is not None
    assert to_endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    from_response = asyncio.run(
        from_endpoint(
            version.id,
            from_request_cls(
                contours=[
                    [
                        (0.0, 0.0, 0.0),
                        (1.0, 0.0, 0.0),
                        (1.0, 1.0, 0.0),
                    ]
                ],
                line_width=1.5,
                show_points=1,
                smooth_connections=0,
            ),
            db=FakeDb(),
        )
    )

    assert from_response.version_id == version.id
    assert from_response.point_count == 3
    assert from_response.line_count == 2
    assert from_response.line_width == 1.5
    assert from_response.object_lines["Type"] == ["LinesHolder", "ObjectLines"]
    assert from_response.object_lines["Polyline"]["Points"] == [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
    ]
    assert from_response.object_lines["Polyline"]["Lines"] == [0, 1, 1, 2]
    assert from_response.metadata["rust_backed"] is True
    assert from_response.metadata["sdk_operation"] == "object_lines_from_contours"
    assert from_response.metadata["meshlib_reference"] == "MR::ObjectLines / MR::PolylineTopology"

    to_response = asyncio.run(
        to_endpoint(
            version.id,
            to_request_cls(object_lines=from_response.object_lines),
            db=FakeDb(),
        )
    )

    assert to_response.version_id == version.id
    assert to_response.contour_count == 1
    assert to_response.point_count == 3
    assert to_response.contours == [[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0)]]
    assert to_response.metadata["rust_backed"] is True
    assert to_response.metadata["sdk_operation"] == "object_lines_to_contours"
    assert to_response.metadata["meshlib_reference"] == "MR::ObjectLines / MR::PolylineTopology"


def test_object_lines_pts_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_pts",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    load_pts = capabilities["object-lines-load-pts"]
    save_pts = capabilities["object-lines-save-pts"]

    assert urls["object_lines_load_pts_endpoint_url"] == (
        "/api/versions/ver_object_lines_pts/object-lines/load-pts"
    )
    assert urls["object_lines_save_pts_endpoint_url"] == (
        "/api/versions/ver_object_lines_pts/object-lines/save-pts"
    )
    assert load_pts.endpoint_url_key == "object_lines_load_pts_endpoint_url"
    assert load_pts.endpoint_url == "/api/versions/ver_object_lines_pts/object-lines/load-pts"
    assert load_pts.rust_backed is True
    assert load_pts.sdk_operations == ["object_lines_from_pts"]
    assert save_pts.endpoint_url_key == "object_lines_save_pts_endpoint_url"
    assert save_pts.endpoint_url == "/api/versions/ver_object_lines_pts/object-lines/save-pts"
    assert save_pts.rust_backed is True
    assert save_pts.sdk_operations == ["object_lines_to_pts"]
    assert getattr(versions_router, "ObjectLinesPtsLoadRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesTextExportRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesTextExportResponse", None) is not None
    assert hasattr(versions_router, "run_object_lines_load_pts_for_version")
    assert hasattr(versions_router, "run_object_lines_save_pts_for_version")


def test_object_lines_load_pts_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_load_pts",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesPtsLoadRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_load_pts_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    source = (
        "BEGIN_Polyline\n"
        "0 0 0\n"
        "1.25 0 0\n"
        "1.25 1.5 0\n"
        "END_Polyline\n"
        "BEGIN_Polyline\n"
        "2 -1 0.5\n"
        "3 -1 0.5\n"
        "END_Polyline\n"
    )
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(file_name="../unsafe/path/sample.pts", source=source),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.point_count == 5
    assert response.line_count == 3
    assert response.object_lines["Type"] == ["LinesHolder", "ObjectLines"]
    assert response.object_lines["Polyline"]["Points"][3] == [2.0, -1.0, 0.5]
    assert response.object_lines["Polyline"]["Lines"] == [0, 1, 1, 2, 3, 4]
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_from_pts"
    assert response.metadata["meshlib_reference"] == "MR::LinesLoad::fromPts"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesLoad.*"
    assert response.metadata["file_name"] == "sample.pts"


def test_object_lines_save_pts_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_save_pts",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesTextExportRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_save_pts_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    lines = versions_router.default_sdk.object_lines_from_contours(
        [[(0.0, 0.0, 0.0), (1.25, 0.0, 0.0), (1.25, 1.5, 0.0)]]
    )
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(file_name="../unsafe/path/exported.pts", object_lines=lines.to_meshlib_json()),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.file_name == "exported.pts"
    assert response.source == "BEGIN_Polyline\n0 0 0\n1.25 0 0\n1.25 1.5 0\nEND_Polyline\n"
    assert response.byte_count == len(response.source.encode("utf-8"))
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_to_pts"
    assert response.metadata["meshlib_reference"] == "MR::LinesSave::toPts"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesSave.*"


def test_object_lines_svg_dxf_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_svg_dxf",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    load_svg = capabilities["object-lines-load-svg"]
    save_dxf = capabilities["object-lines-save-dxf"]

    assert urls["object_lines_load_svg_endpoint_url"] == (
        "/api/versions/ver_object_lines_svg_dxf/object-lines/load-svg"
    )
    assert urls["object_lines_save_dxf_endpoint_url"] == (
        "/api/versions/ver_object_lines_svg_dxf/object-lines/save-dxf"
    )
    assert load_svg.endpoint_url_key == "object_lines_load_svg_endpoint_url"
    assert load_svg.endpoint_url == "/api/versions/ver_object_lines_svg_dxf/object-lines/load-svg"
    assert load_svg.rust_backed is True
    assert load_svg.sdk_operations == ["object_lines_from_svg"]
    assert save_dxf.endpoint_url_key == "object_lines_save_dxf_endpoint_url"
    assert save_dxf.endpoint_url == "/api/versions/ver_object_lines_svg_dxf/object-lines/save-dxf"
    assert save_dxf.rust_backed is True
    assert save_dxf.sdk_operations == ["object_lines_to_dxf"]
    assert getattr(versions_router, "ObjectLinesSvgLoadRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesTextExportRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesTextExportResponse", None) is not None
    assert hasattr(versions_router, "run_object_lines_load_svg_for_version")
    assert hasattr(versions_router, "run_object_lines_save_dxf_for_version")


def test_object_lines_load_svg_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_load_svg",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesSvgLoadRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_load_svg_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    source = (
        '<svg xmlns="http://www.w3.org/2000/svg">'
        '<line x1="1" y1="2" x2="4" y2="6" />'
        '<polyline points="0,0 2,0 2,2" />'
        "</svg>"
    )
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(file_name="../unsafe/path/sample.svg", source=source),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.point_count == 5
    assert response.line_count == 3
    assert response.object_lines["Type"] == ["LinesHolder", "ObjectLines"]
    assert response.object_lines["Polyline"]["Points"] == [
        [1.0, -2.0, 0.0],
        [4.0, -6.0, 0.0],
        [0.0, -0.0, 0.0],
        [2.0, -0.0, 0.0],
        [2.0, -2.0, 0.0],
    ]
    assert response.object_lines["Polyline"]["Lines"] == [0, 1, 2, 3, 3, 4]
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_from_svg"
    assert response.metadata["meshlib_reference"] == "MR::LinesLoad::fromSvg"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRIOExtras/MRSvg.*"
    assert response.metadata["file_name"] == "sample.svg"


def test_object_lines_save_dxf_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_save_dxf",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesTextExportRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_save_dxf_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    lines = versions_router.default_sdk.object_lines_from_contours(
        [[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 0.0)]]
    )
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(file_name="../unsafe/path/exported.dxf", object_lines=lines.to_meshlib_json()),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.file_name == "exported.dxf"
    assert response.source.startswith("0\nSECTION\n2\nENTITIES\n")
    assert "0\nPOLYLINE\n8\n0\n66\n1\n70\n9\n" in response.source
    assert "0\nVERTEX\n8\n0\n70\n32\n10\n1\n20\n0\n30\n0\n" in response.source
    assert response.source.endswith("0\nENDSEC\n0\nEOF\n")
    assert response.byte_count == len(response.source.encode("utf-8"))
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_to_dxf"
    assert response.metadata["meshlib_reference"] == "MR::LinesSave::toDxf"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesSave.*"


def _sample_object_lines_mrlines_payload() -> bytes:
    payload = bytearray()
    payload.extend((2).to_bytes(4, "little"))
    for value in (0, 0, 1, 1):
        payload.extend(int(value).to_bytes(4, "little", signed=True))
    payload.extend((2).to_bytes(4, "little"))
    for value in (0, 1):
        payload.extend(int(value).to_bytes(4, "little", signed=True))
    payload.extend((3).to_bytes(4, "little"))
    payload.extend((2).to_bytes(4, "little"))
    payload.extend(np.array([0.0, 0.0, 0.0, 1.0, 2.0, 3.0], dtype="<f4").tobytes())
    return bytes(payload)


def _sample_object_lines_ply_payload() -> bytes:
    payload = bytearray(
        b"ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n"
        b"element vertex 2\nproperty float x\nproperty float y\nproperty float z\n"
        b"element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
    )
    payload.extend(np.array([0.0, 0.0, 0.0, 1.0, 2.0, 3.0], dtype="<f4").tobytes())
    payload.extend(int(0).to_bytes(4, "little", signed=True))
    payload.extend(int(1).to_bytes(4, "little", signed=True))
    return bytes(payload)


def test_object_lines_mrlines_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_mrlines",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    load_mrlines = capabilities["object-lines-load-mrlines"]
    save_mrlines = capabilities["object-lines-save-mrlines"]

    assert urls["object_lines_load_mrlines_endpoint_url"] == (
        "/api/versions/ver_object_lines_mrlines/object-lines/load-mrlines"
    )
    assert urls["object_lines_save_mrlines_endpoint_url"] == (
        "/api/versions/ver_object_lines_mrlines/object-lines/save-mrlines"
    )
    assert load_mrlines.endpoint_url_key == "object_lines_load_mrlines_endpoint_url"
    assert load_mrlines.endpoint_url == "/api/versions/ver_object_lines_mrlines/object-lines/load-mrlines"
    assert load_mrlines.rust_backed is True
    assert load_mrlines.sdk_operations == ["object_lines_from_mrlines"]
    assert save_mrlines.endpoint_url_key == "object_lines_save_mrlines_endpoint_url"
    assert save_mrlines.endpoint_url == "/api/versions/ver_object_lines_mrlines/object-lines/save-mrlines"
    assert save_mrlines.rust_backed is True
    assert save_mrlines.sdk_operations == ["object_lines_to_mrlines"]
    assert getattr(versions_router, "ObjectLinesBinaryLoadRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesBinaryExportRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesBinaryExportResponse", None) is not None
    assert hasattr(versions_router, "run_object_lines_load_mrlines_for_version")
    assert hasattr(versions_router, "run_object_lines_save_mrlines_for_version")


def test_object_lines_load_mrlines_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_load_mrlines",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesBinaryLoadRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_load_mrlines_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                file_name="../unsafe/path/sample.mrlines",
                contents_base64=base64.b64encode(_sample_object_lines_mrlines_payload()).decode("ascii"),
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.point_count == 2
    assert response.line_count == 1
    assert response.object_lines["Type"] == ["LinesHolder", "ObjectLines"]
    assert response.object_lines["Polyline"]["Points"] == [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]
    assert response.object_lines["Polyline"]["Lines"] == [0, 1]
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_from_mrlines"
    assert response.metadata["meshlib_reference"] == "MR::LinesLoad::fromMrLines"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesLoad.*"
    assert response.metadata["file_name"] == "sample.mrlines"


def test_object_lines_save_mrlines_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_save_mrlines",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesBinaryExportRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_save_mrlines_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    lines = versions_router.default_sdk.object_lines_from_contours(
        [[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]]
    )
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(file_name="../unsafe/path/exported.mrlines", object_lines=lines.to_meshlib_json()),
            db=FakeDb(),
        )
    )

    decoded = base64.b64decode(response.contents_base64)
    assert response.version_id == version.id
    assert response.file_name == "exported.mrlines"
    assert decoded == _sample_object_lines_mrlines_payload()
    assert response.byte_count == len(decoded)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_to_mrlines"
    assert response.metadata["meshlib_reference"] == "MR::LinesSave::toMrLines"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesSave.*"


def test_object_lines_ply_capabilities_point_to_executable_rust_endpoints() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_ply",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    urls = versions_router._workbench_endpoint_urls(version)
    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    load_ply = capabilities["object-lines-load-ply"]
    save_ply = capabilities["object-lines-save-ply"]

    assert urls["object_lines_load_ply_endpoint_url"] == (
        "/api/versions/ver_object_lines_ply/object-lines/load-ply"
    )
    assert urls["object_lines_save_ply_endpoint_url"] == (
        "/api/versions/ver_object_lines_ply/object-lines/save-ply"
    )
    assert load_ply.endpoint_url_key == "object_lines_load_ply_endpoint_url"
    assert load_ply.endpoint_url == "/api/versions/ver_object_lines_ply/object-lines/load-ply"
    assert load_ply.rust_backed is True
    assert load_ply.sdk_operations == ["object_lines_from_ply"]
    assert save_ply.endpoint_url_key == "object_lines_save_ply_endpoint_url"
    assert save_ply.endpoint_url == "/api/versions/ver_object_lines_ply/object-lines/save-ply"
    assert save_ply.rust_backed is True
    assert save_ply.sdk_operations == ["object_lines_to_ply"]
    assert getattr(versions_router, "ObjectLinesBinaryLoadRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesBinaryExportRequest", None) is not None
    assert getattr(versions_router, "ObjectLinesBinaryExportResponse", None) is not None
    assert hasattr(versions_router, "run_object_lines_load_ply_for_version")
    assert hasattr(versions_router, "run_object_lines_save_ply_for_version")


def test_object_lines_load_ply_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_load_ply",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesBinaryLoadRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_load_ply_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                file_name="../unsafe/path/sample.ply",
                contents_base64=base64.b64encode(_sample_object_lines_ply_payload()).decode("ascii"),
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.point_count == 2
    assert response.line_count == 1
    assert response.object_lines["Type"] == ["LinesHolder", "ObjectLines"]
    assert response.object_lines["Polyline"]["Points"] == [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]
    assert response.object_lines["Polyline"]["Lines"] == [0, 1]
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_from_ply"
    assert response.metadata["meshlib_reference"] == "MR::LinesLoad::fromPly"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesLoad.*"
    assert response.metadata["file_name"] == "sample.ply"


def test_object_lines_save_ply_endpoint_returns_meshlib_style_rust_payload() -> None:
    version = ModelVersionRecord(
        id="ver_object_lines_save_ply",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    request_cls = getattr(versions_router, "ObjectLinesBinaryExportRequest", None)
    endpoint = getattr(versions_router, "run_object_lines_save_ply_for_version", None)
    assert request_cls is not None
    assert endpoint is not None

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    lines = versions_router.default_sdk.object_lines_from_contours(
        [[(0.0, 0.0, 0.0), (1.0, 2.0, 3.0)]]
    )
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(file_name="../unsafe/path/exported.ply", object_lines=lines.to_meshlib_json()),
            db=FakeDb(),
        )
    )

    decoded = base64.b64decode(response.contents_base64)
    assert response.version_id == version.id
    assert response.file_name == "exported.ply"
    assert decoded == _sample_object_lines_ply_payload()
    assert response.byte_count == len(decoded)
    assert response.metadata["rust_backed"] is True
    assert response.metadata["sdk_operation"] == "object_lines_to_ply"
    assert response.metadata["meshlib_reference"] == "MR::LinesSave::toPly"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MRLinesSave.*"


def test_mesh_to_voxels_sdf_endpoint_returns_meshlib_style_rust_payload(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    mesh_path = tmp_path / "cube.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), mesh_path)
    artifact = ModelArtifactRecord(
        id="art_ready_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_ready/normalized_mesh_ply.ply",
        size_bytes=mesh_path.stat().st_size,
        metadata_json={},
    )

    request_cls = getattr(versions_router, "MeshToVoxelsSdfRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "mesh_to_voxels_sdf_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return artifact
        return None

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: mesh_path)

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                voxel_size_mm=1.0,
                surface_offset_voxels=1.0,
                mode="signed",
                extract_surface=True,
            ),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.mode == "signed"
    assert response.voxel_size_mm == 1.0
    assert response.surface_offset_voxels == 1.0
    assert response.padding_mm == 1.0
    assert response.shape == (5, 5, 5)
    assert response.value_count == 125
    assert response.active_voxel_count > 0
    assert response.min_value < 0.0
    assert response.max_value > 0.0
    assert response.estimated_volume_mm3 > 0.0
    assert response.surface_vertex_count > 0
    assert response.surface_face_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "meshToLevelSet"


def test_collision_detect_endpoint_returns_meshlib_style_rust_payload(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    other_version = ModelVersionRecord(
        id="ver_target",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Target",
        status="ready",
    )
    first_mesh = MeshDocument(
        vertices=np.array([[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    second_mesh = MeshDocument(
        vertices=np.array([[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]], dtype=np.float64),
        faces=np.array([[0, 1, 2]], dtype=np.int64),
    )
    first_path = tmp_path / "source.ply"
    second_path = tmp_path / "target.ply"
    versions_router.default_sdk.save_mesh(first_mesh, first_path)
    versions_router.default_sdk.save_mesh(second_mesh, second_path)
    first_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=first_path.stat().st_size,
        metadata_json={},
    )
    second_artifact = ModelArtifactRecord(
        id="art_target_mesh",
        version_id=other_version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_target/normalized_mesh_ply.ply",
        size_bytes=second_path.stat().st_size,
        metadata_json={},
    )

    request_cls = getattr(versions_router, "CollisionDetectRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "detect_collision_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if artifact_type != "normalized_mesh_ply":
            return None
        return {version.id: first_artifact, other_version.id: second_artifact}.get(version_id)

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(
        versions_router,
        "_materialize_artifact_to_path",
        lambda artifact: first_path if artifact.id == first_artifact.id else second_path,
    )

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, other_version.id: other_version}.get(key)
            return None

    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(other_version_id=other_version.id, first_intersection_only=False),
            db=FakeDb(),
        )
    )

    assert response.version_id == version.id
    assert response.other_version_id == other_version.id
    assert response.colliding is True
    assert response.pair_count == 1
    assert response.first_face_indices == [0]
    assert response.second_face_indices == [0]
    assert response.pairs[0].first_face == 0
    assert response.pairs[0].second_face == 0
    assert response.pairs[0].intersection_count > 0
    assert response.truncated is False
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "findCollidingTriangles"


def test_exact_boolean_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    other_version = ModelVersionRecord(
        id="ver_tool",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Tool",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_boolean",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="boolean",
        operation_label="Boolean Difference",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    tool_path = tmp_path / "tool.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    versions_router.default_sdk.save_mesh(cube(size=1.0), tool_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    tool_artifact = ModelArtifactRecord(
        id="art_tool_mesh",
        version_id=other_version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_tool/normalized_mesh_ply.ply",
        size_bytes=tool_path.stat().st_size,
        metadata_json={},
    )
    registered: dict[str, object] = {}

    request_cls = getattr(versions_router, "ExactBooleanRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_exact_boolean_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if artifact_type != "normalized_mesh_ply":
            return None
        return {version.id: source_artifact, other_version.id: tool_artifact}.get(version_id)

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "boolean",
            "operation_label": "Boolean difference with ver_tool",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "file_path": file_path,
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_boolean_mesh",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(
        versions_router,
        "_materialize_artifact_to_path",
        lambda artifact: source_path if artifact.id == source_artifact.id else tool_path,
    )
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, other_version.id: other_version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(other_version_id=other_version.id, operation="difference"),
            db=db,
        )
    )

    assert response.version.id == output_version.id
    assert response.source_version_id == version.id
    assert response.other_version_id == other_version.id
    assert response.operation == "difference"
    assert response.artifact_id == "art_boolean_mesh"
    assert response.output_vertex_count > 0
    assert response.output_face_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "MR::boolean"
    assert response.diagnostics["mesh_stats"]["face_count"] == response.output_face_count
    assert registered["version_id"] == output_version.id
    assert registered["artifact_type"] == "normalized_mesh_ply"
    assert registered["metadata_json"]["source"] == "rust_exact_boolean"
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_exact_boolean_endpoint_rejects_dense_interactive_job_before_rust_kernel(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_dense_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    other_version = ModelVersionRecord(
        id="ver_dense_tool",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Tool",
        status="ready",
    )
    source_artifact = ModelArtifactRecord(
        id="art_dense_source",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_dense_source/normalized_mesh_ply.ply",
        size_bytes=1,
        metadata_json={},
    )
    tool_artifact = ModelArtifactRecord(
        id="art_dense_tool",
        version_id=other_version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_dense_tool/normalized_mesh_ply.ply",
        size_bytes=1,
        metadata_json={},
    )

    request_cls = getattr(versions_router, "ExactBooleanRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_exact_boolean_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if artifact_type != "normalized_mesh_ply":
            return None
        return {version.id: source_artifact, other_version.id: tool_artifact}.get(version_id)

    def fail_exact_boolean(*args, **kwargs):  # noqa: ANN002, ANN003, ANN202
        pytest.fail("dense interactive exact boolean should be rejected before the Rust kernel runs")

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda artifact: tmp_path / artifact.storage_key)
    monkeypatch.setattr(
        versions_router.default_sdk,
        "load_mesh",
        lambda path: SimpleNamespace(face_count=150_000, vertex_count=75_000),
    )
    monkeypatch.setattr(versions_router.default_sdk, "exact_boolean_mesh", fail_exact_boolean)
    monkeypatch.setattr(versions_router, "create_version", lambda *args, **kwargs: pytest.fail("version was created"))

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, other_version.id: other_version}.get(key)
            return None

    with pytest.raises(HTTPException) as exc_info:
        asyncio.run(
            endpoint(
                version.id,
                request_cls(other_version_id=other_version.id, operation="difference"),
                db=FakeDb(),
            )
        )

    assert exc_info.value.status_code == 400
    assert "Exact Boolean is limited to 100000 combined faces for interactive jobs" in exc_info.value.detail
    assert "source has 150000 faces" in exc_info.value.detail
    assert "tool has 150000 faces" in exc_info.value.detail


def test_voxel_boolean_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    other_version = ModelVersionRecord(
        id="ver_tool",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Tool",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_voxel_boolean",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="boolean",
        operation_label="Voxel Boolean Union",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    tool_path = tmp_path / "tool.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    versions_router.default_sdk.save_mesh(cube(size=1.5), tool_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    tool_artifact = ModelArtifactRecord(
        id="art_tool_mesh",
        version_id=other_version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_tool/normalized_mesh_ply.ply",
        size_bytes=tool_path.stat().st_size,
        metadata_json={},
    )
    registered: dict[str, object] = {}

    request_cls = getattr(versions_router, "VoxelBooleanRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_voxel_boolean_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if artifact_type != "normalized_mesh_ply":
            return None
        return {version.id: source_artifact, other_version.id: tool_artifact}.get(version_id)

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "boolean",
            "operation_label": "Voxel Boolean union with ver_tool",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "file_path": file_path,
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_voxel_boolean_mesh",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(
        versions_router,
        "_materialize_artifact_to_path",
        lambda artifact: source_path if artifact.id == source_artifact.id else tool_path,
    )
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, other_version.id: other_version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                other_version_id=other_version.id,
                operation="union",
                voxel_size_mm=0.5,
                padding_mm=1.0,
                refine=True,
            ),
            db=db,
        )
    )

    assert response.version.id == output_version.id
    assert response.source_version_id == version.id
    assert response.other_version_id == other_version.id
    assert response.operation == "union"
    assert response.voxel_size_mm == 0.5
    assert response.padding_mm == 1.0
    assert response.refine is True
    assert response.artifact_id == "art_voxel_boolean_mesh"
    assert response.output_vertex_count > 0
    assert response.output_face_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "MRVoxels::voxelBoolean"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MRBoolean.*"
    assert registered["version_id"] == output_version.id
    assert registered["artifact_type"] == "normalized_mesh_ply"
    assert registered["metadata_json"]["source"] == "rust_voxel_boolean"
    assert registered["metadata_json"]["voxel_size_mm"] == 0.5
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_offset_mesh_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_offset",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="offset",
        operation_label="Offset Mesh",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    registered: dict[str, object] = {}

    request_cls = getattr(versions_router, "OffsetMeshRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_offset_mesh_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "offset",
            "operation_label": "Offset Mesh 0.25 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "file_path": file_path,
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_offset_mesh",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                offset_mm=0.25,
                voxel_size_mm=0.5,
                padding_mm=1.0,
                refine=True,
            ),
            db=db,
        )
    )

    assert response.version.id == output_version.id
    assert response.source_version_id == version.id
    assert response.mode == "offset"
    assert response.offset_mm == 0.25
    assert response.voxel_size_mm == 0.5
    assert response.padding_mm == 1.0
    assert response.refine is True
    assert response.artifact_id == "art_offset_mesh"
    assert response.output_vertex_count > 0
    assert response.output_face_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "MR::generalOffsetMesh"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MROffset.*"
    assert registered["version_id"] == output_version.id
    assert registered["artifact_type"] == "normalized_mesh_ply"
    assert registered["metadata_json"]["source"] == "rust_voxel_offset"
    assert registered["metadata_json"]["offset_mm"] == 0.25
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_offset_mesh_endpoint_rejects_empty_rust_output(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )

    request_cls = getattr(versions_router, "OffsetMeshRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_offset_mesh_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fail_create_version(*_args, **_kwargs):  # noqa: ANN202
        pytest.fail("empty offset output must be rejected before creating a child version")

    def fail_register_file_artifact(*_args, **_kwargs):  # noqa: ANN202
        pytest.fail("empty offset output must be rejected before registering an artifact")

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "create_version", fail_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fail_register_file_artifact)
    monkeypatch.setattr(
        versions_router.default_sdk,
        "voxel_offset_mesh",
        lambda *_args, **_kwargs: SimpleNamespace(vertex_count=0, face_count=0),
    )

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

    db = FakeDb()
    with pytest.raises(HTTPException) as exc_info:
        asyncio.run(
            endpoint(
                version.id,
                request_cls(offset_mm=0.1, voxel_size_mm=8.0, padding_mm=8.0, refine=False),
                db=db,
            )
        )

    assert exc_info.value.status_code == 400
    assert "produced an empty mesh" in str(exc_info.value.detail)
    assert "Reduce voxel_size_mm below 8" in str(exc_info.value.detail)
    assert db.commits == 0


def test_offset_mesh_endpoint_rejects_low_resolution_voxel_output(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=1,
        metadata_json={},
    )

    request_cls = getattr(versions_router, "OffsetMeshRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_offset_mesh_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fail_create_version(*_args, **_kwargs):  # noqa: ANN202
        pytest.fail("low-resolution voxel output must be rejected before creating a child version")

    def fail_register_file_artifact(*_args, **_kwargs):  # noqa: ANN202
        pytest.fail("low-resolution voxel output must be rejected before registering an artifact")

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: tmp_path / "source.ply")
    monkeypatch.setattr(versions_router, "create_version", fail_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fail_register_file_artifact)
    monkeypatch.setattr(
        versions_router.default_sdk,
        "load_mesh",
        lambda *_args, **_kwargs: SimpleNamespace(vertex_count=50_000, face_count=100_000),
    )
    monkeypatch.setattr(
        versions_router.default_sdk,
        "voxel_offset_mesh",
        lambda *_args, **_kwargs: SimpleNamespace(vertex_count=1_200, face_count=5_000),
    )

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

    db = FakeDb()
    with pytest.raises(HTTPException) as exc_info:
        asyncio.run(
            endpoint(
                version.id,
                request_cls(offset_mm=0.1, voxel_size_mm=0.5, padding_mm=1.0, refine=True),
                db=db,
            )
        )

    assert exc_info.value.status_code == 400
    assert "low-resolution voxel remesh" in str(exc_info.value.detail)
    assert "5000 faces from 100000 source faces" in str(exc_info.value.detail)
    assert "Offset Verts" in str(exc_info.value.detail)
    assert db.commits == 0


def test_offset_mesh_endpoint_accepts_official_negative_inward_offset(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_offset_inward",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="offset",
        operation_label="Offset Mesh",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(cube(size=4.0), source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    registered: dict[str, object] = {}

    request_cls = getattr(versions_router, "OffsetMeshRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_offset_mesh_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "offset",
            "operation_label": "Offset Mesh -0.25 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update({"metadata_json": metadata_json})
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_offset_inward_mesh",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                offset_mm=-0.25,
                voxel_size_mm=0.5,
                padding_mm=1.0,
                refine=True,
            ),
            db=db,
        )
    )

    assert response.version.id == output_version.id
    assert response.mode == "offset"
    assert response.offset_mm == -0.25
    assert response.metadata["meshlib_reference"] == "MR::generalOffsetMesh"
    assert registered["metadata_json"]["source"] == "rust_voxel_offset"
    assert registered["metadata_json"]["offset_mm"] == -0.25
    assert db.commits == 1
    assert output_version.id in db.refreshed


@pytest.mark.parametrize(
    ("endpoint_name", "request_name", "distance_mm", "expected_mode", "operation_type", "operation_label", "offset_sequence", "artifact_id", "source"),
    [
        (
            "run_expand_shrink_for_version",
            "OffsetSmoothingRequest",
            0.3,
            "expand_shrink",
            "expand_shrink",
            "Expand/Shrink 0.3 mm",
            [0.3, -0.3],
            "art_expand_shrink_mesh",
            "rust_voxel_expand_shrink",
        ),
        (
            "run_shrink_expand_for_version",
            "OffsetSmoothingRequest",
            0.4,
            "shrink_expand",
            "shrink_expand",
            "Shrink/Expand 0.4 mm",
            [-0.4, 0.4],
            "art_shrink_expand_mesh",
            "rust_voxel_shrink_expand",
        ),
    ],
)
def test_offset_smoothing_endpoint_sequences_official_signed_offsets(
    monkeypatch,
    tmp_path,
    endpoint_name,
    request_name,
    distance_mm,
    expected_mode,
    operation_type,
    operation_label,
    offset_sequence,
    artifact_id,
    source,
) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id=f"ver_{expected_mode}",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type=operation_type,
        operation_label=operation_label,
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    source_mesh = cube(size=3.0)
    versions_router.default_sdk.save_mesh(source_mesh, source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    offset_calls: list[float] = []
    registered: dict[str, object] = {}

    request_cls = getattr(versions_router, request_name, None)
    assert request_cls is not None
    endpoint = getattr(versions_router, endpoint_name, None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": operation_type,
            "operation_label": operation_label,
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "artifact_type": artifact_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id=artifact_id,
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    def fake_voxel_offset_mesh(mesh, *, offset_mm, voxel_size_mm, padding_mm=None, refine=False):  # noqa: ANN001, ANN202
        offset_calls.append(float(offset_mm))
        assert voxel_size_mm == 0.5
        assert padding_mm == 1.0
        assert refine is True
        return mesh

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.default_sdk, "voxel_offset_mesh", fake_voxel_offset_mesh)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                distance_mm=distance_mm,
                voxel_size_mm=0.5,
                padding_mm=1.0,
                refine=True,
            ),
            db=db,
        )
    )

    assert offset_calls == offset_sequence
    assert response.version.id == output_version.id
    assert response.source_version_id == version.id
    assert response.mode == expected_mode
    assert response.distance_mm == distance_mm
    assert response.voxel_size_mm == 0.5
    assert response.padding_mm == 1.0
    assert response.refine is True
    assert response.artifact_id == artifact_id
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == f"MR::generalOffsetMesh {operation_label.split()[0]} Mode"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MROffset.*"
    assert registered["version_id"] == output_version.id
    assert registered["artifact_type"] == "normalized_mesh_ply"
    assert registered["metadata_json"]["source"] == source
    assert registered["metadata_json"]["distance_mm"] == distance_mm
    assert registered["metadata_json"]["offset_sequence_mm"] == offset_sequence
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_shell_mesh_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_shell",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="shell",
        operation_label="Shell Mesh",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    registered: dict[str, object] = {}

    request_cls = getattr(versions_router, "ShellMeshRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_shell_mesh_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "shell",
            "operation_label": "Shell Mesh 0.6 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "file_path": file_path,
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_shell_mesh",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                wall_thickness_mm=0.6,
                voxel_size_mm=0.4,
                padding_mm=1.2,
                refine=True,
            ),
            db=db,
        )
    )

    assert response.version.id == output_version.id
    assert response.source_version_id == version.id
    assert response.mode == "shell"
    assert response.wall_thickness_mm == 0.6
    assert response.voxel_size_mm == 0.4
    assert response.padding_mm == 1.2
    assert response.refine is True
    assert response.artifact_id == "art_shell_mesh"
    assert response.output_vertex_count > 0
    assert response.output_face_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "MR::generalOffsetMesh Shell Mode"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MROffset.*"
    assert registered["version_id"] == output_version.id
    assert registered["artifact_type"] == "normalized_mesh_ply"
    assert registered["metadata_json"]["source"] == "rust_voxel_shell"
    assert registered["metadata_json"]["wall_thickness_mm"] == 0.6
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_thicken_mesh_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_thicken",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="thicken_mesh",
        operation_label="Thickening -0.35 mm",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    # Sheet-thicken is for OPEN surfaces; feed an open quad. (A closed solid like
    # cube() is now correctly rejected by _reject_sheet_thicken_on_closed_solid,
    # since thickenMesh would emit two interpenetrating solids on a closed mesh.)
    open_sheet = MeshDocument(
        vertices=np.array(
            [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 0.0], [0.0, 2.0, 0.0]]
        ),
        faces=np.array([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )
    versions_router.default_sdk.save_mesh(open_sheet, source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    registered: dict[str, object] = {}
    calls: list[tuple[float, float, float | None, bool]] = []

    request_cls = getattr(versions_router, "ThickenMeshRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_thicken_mesh_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "thicken_mesh",
            "operation_label": "Thickening -0.35 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "file_path": file_path,
                "artifact_type": artifact_type,
                "mime_type": mime_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_thicken_mesh",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    def fake_voxel_thicken_mesh(mesh, *, thickness_mm, voxel_size_mm, padding_mm=None, refine=False):  # noqa: ANN001, ANN202
        calls.append((float(thickness_mm), float(voxel_size_mm), padding_mm, refine))
        return mesh

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.default_sdk, "voxel_thicken_mesh", fake_voxel_thicken_mesh)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                thickness_mm=-0.35,
                voxel_size_mm=0.4,
                padding_mm=1.1,
                refine=True,
            ),
            db=db,
        )
    )

    assert calls == [(-0.35, 0.4, 1.1, True)]
    assert response.version.id == output_version.id
    assert response.source_version_id == version.id
    assert response.mode == "thicken"
    assert response.thickness_mm == -0.35
    assert response.voxel_size_mm == 0.4
    assert response.padding_mm == 1.1
    assert response.refine is True
    assert response.artifact_id == "art_thicken_mesh"
    assert response.output_vertex_count > 0
    assert response.output_face_count > 0
    assert response.metadata["rust_backed"] is True
    assert response.metadata["meshlib_reference"] == "MR::thickenMesh"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MROffset.*"
    assert registered["version_id"] == output_version.id
    assert registered["artifact_type"] == "normalized_mesh_ply"
    assert registered["metadata_json"]["source"] == "rust_voxel_thicken"
    assert registered["metadata_json"]["thickness_mm"] == -0.35
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_weighted_shell_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_weighted_shell",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="weighted_shell",
        operation_label="Weighted Shell 0.2 mm",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    regions_artifact = ModelArtifactRecord(
        id="art_regions",
        version_id=version.id,
        artifact_type="analysis_regions_json",
        mime_type="application/json",
        storage_key="ver_source/analysis_regions_json.json",
        size_bytes=100,
        metadata_json={},
    )
    region_payload = {
        "regions": [
            {
                "region_id": "gem_seat",
                "label": "Gem Seat",
                "vertex_indices": [0, 1, 2],
                "coverage_pct": 10.0,
                "protected_by_default": True,
                "allowed_operations": ["weighted_shell"],
            }
        ]
    }
    registered: dict[str, object] = {}
    calls: list[dict[str, object]] = []

    request_cls = getattr(versions_router, "WeightedShellRequest", None)
    assert request_cls is not None
    region_cls = getattr(versions_router, "WeightedShellRegionWeight", None)
    assert region_cls is not None
    endpoint = getattr(versions_router, "run_weighted_shell_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        if version_id == version.id and artifact_type == "analysis_regions_json":
            return regions_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "weighted_shell",
            "operation_label": "Weighted Shell 0.2 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "artifact_type": artifact_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_weighted_shell",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    def fake_voxel_weighted_shell_mesh(mesh, *, regions, region_weights, offset_mm, voxel_size_mm, padding_mm=None, interpolation_distance_mm=0.0, refine=False):  # noqa: ANN001, ANN202
        calls.append(
            {
                "region_ids": [region.region_id for region in regions],
                "region_weights": dict(region_weights),
                "offset_mm": offset_mm,
                "voxel_size_mm": voxel_size_mm,
                "padding_mm": padding_mm,
                "interpolation_distance_mm": interpolation_distance_mm,
                "refine": refine,
            }
        )
        return mesh

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "_load_json_artifact", lambda _artifact: region_payload)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.default_sdk, "voxel_weighted_shell_mesh", fake_voxel_weighted_shell_mesh)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                offset_mm=0.2,
                region_weights=[region_cls(region_id="gem_seat", weight_mm=0.45)],
                voxel_size_mm=0.4,
                padding_mm=1.2,
                interpolation_distance_mm=1.5,
                refine=True,
            ),
            db=db,
        )
    )

    assert calls == [
        {
            "region_ids": ["gem_seat"],
            "region_weights": {"gem_seat": 0.45},
            "offset_mm": 0.2,
            "voxel_size_mm": 0.4,
            "padding_mm": 1.2,
            "interpolation_distance_mm": 1.5,
            "refine": True,
        }
    ]
    assert response.version.id == output_version.id
    assert response.mode == "weighted_shell"
    assert response.offset_mm == 0.2
    assert response.region_weights == {"gem_seat": 0.45}
    assert response.metadata["meshlib_reference"] == "MR::WeightedShell::meshShell"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MRWeightedPointsShell.*"
    assert response.metadata["rust_backed"] is True
    assert registered["metadata_json"]["source"] == "rust_voxel_weighted_shell"
    assert registered["metadata_json"]["region_weights"] == {"gem_seat": 0.45}
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_partial_offset_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_partial_offset",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="partial_offset",
        operation_label="Partial Offset 0.2 mm",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(cube(size=2.0), source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    regions_artifact = ModelArtifactRecord(
        id="art_regions",
        version_id=version.id,
        artifact_type="analysis_regions_json",
        mime_type="application/json",
        storage_key="ver_source/analysis_regions_json.json",
        size_bytes=100,
        metadata_json={},
    )
    region_payload = {
        "regions": [
            {
                "region_id": "gem_seat",
                "label": "Gem Seat",
                "vertex_indices": [0, 1, 2],
                "coverage_pct": 10.0,
                "protected_by_default": True,
                "allowed_operations": ["partial_offset"],
            }
        ]
    }
    registered: dict[str, object] = {}
    calls: list[dict[str, object]] = []

    request_cls = getattr(versions_router, "PartialOffsetRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_partial_offset_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        if version_id == version.id and artifact_type == "analysis_regions_json":
            return regions_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "partial_offset",
            "operation_label": "Partial Offset 0.2 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "artifact_type": artifact_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_partial_offset",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    def fake_voxel_partial_offset_mesh(mesh, *, regions, selected_region_ids, offset_mm, voxel_size_mm, padding_mm=None, refine=False):  # noqa: ANN001, ANN202
        calls.append(
            {
                "region_ids": [region.region_id for region in regions],
                "selected_region_ids": list(selected_region_ids),
                "offset_mm": offset_mm,
                "voxel_size_mm": voxel_size_mm,
                "padding_mm": padding_mm,
                "refine": refine,
            }
        )
        return mesh

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "_load_json_artifact", lambda _artifact: region_payload)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.default_sdk, "voxel_partial_offset_mesh", fake_voxel_partial_offset_mesh)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                offset_mm=0.2,
                region_ids=["gem_seat"],
                voxel_size_mm=0.4,
                padding_mm=1.2,
                refine=True,
            ),
            db=db,
        )
    )

    assert calls == [
        {
            "region_ids": ["gem_seat"],
            "selected_region_ids": ["gem_seat"],
            "offset_mm": 0.2,
            "voxel_size_mm": 0.4,
            "padding_mm": 1.2,
            "refine": True,
        }
    ]
    assert response.version.id == output_version.id
    assert response.mode == "partial_offset"
    assert response.offset_mm == 0.2
    assert response.selected_region_ids == ["gem_seat"]
    assert response.metadata["meshlib_reference"] == "MR::partialOffsetMesh"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRVoxels/MRPartialOffset.*"
    assert response.metadata["rust_backed"] is True
    assert registered["metadata_json"]["source"] == "rust_voxel_partial_offset"
    assert registered["metadata_json"]["selected_region_ids"] == ["gem_seat"]
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_offset_verts_endpoint_creates_rust_backed_child_version(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    output_version = ModelVersionRecord(
        id="ver_offset_verts",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="offset_verts",
        operation_label="Offset Verts 0.15 mm",
        status="ready",
        created_at=datetime(2026, 6, 6, tzinfo=timezone.utc),
    )
    source_mesh = cube(size=2.0)
    source_path = tmp_path / "source.ply"
    versions_router.default_sdk.save_mesh(source_mesh, source_path)
    source_artifact = ModelArtifactRecord(
        id="art_source_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="application/octet-stream",
        storage_key="ver_source/normalized_mesh_ply.ply",
        size_bytes=source_path.stat().st_size,
        metadata_json={},
    )
    regions_artifact = ModelArtifactRecord(
        id="art_regions",
        version_id=version.id,
        artifact_type="analysis_regions_json",
        mime_type="application/json",
        storage_key="ver_source/analysis_regions_json.json",
        size_bytes=100,
        metadata_json={},
    )
    region_payload = {
        "regions": [
            {
                "region_id": "gem_seat",
                "label": "Gem Seat",
                "vertex_indices": [0, 1, 2],
                "coverage_pct": 10.0,
                "protected_by_default": True,
                "allowed_operations": ["offset_verts"],
            }
        ]
    }
    registered: dict[str, object] = {}
    calls: list[dict[str, object]] = []

    request_cls = getattr(versions_router, "OffsetVertsRequest", None)
    assert request_cls is not None
    endpoint = getattr(versions_router, "run_offset_verts_for_version", None)
    assert endpoint is not None

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return source_artifact
        if version_id == version.id and artifact_type == "analysis_regions_json":
            return regions_artifact
        return None

    def fake_create_version(db, **kwargs):  # noqa: ANN001, ANN202
        assert kwargs == {
            "model_id": version.model_id,
            "parent_version_id": version.id,
            "operation_type": "offset_verts",
            "operation_label": "Offset Verts 0.15 mm",
            "status": "ready",
        }
        return output_version

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type=None, metadata_json=None):  # noqa: ANN001, ANN202
        registered.update(
            {
                "version_id": version_id,
                "artifact_type": artifact_type,
                "metadata_json": metadata_json,
            }
        )
        assert file_path.exists()
        return ModelArtifactRecord(
            id="art_offset_verts",
            version_id=version_id,
            artifact_type=artifact_type,
            mime_type=mime_type or "model/ply",
            storage_key=f"{version_id}/{artifact_type}.ply",
            size_bytes=file_path.stat().st_size,
            metadata_json=metadata_json or {},
        )

    def fake_offset_verts_mesh(mesh, offsets_mm):  # noqa: ANN001, ANN202
        calls.append(
            {
                "offsets_mm": np.asarray(offsets_mm, dtype=np.float32).tolist(),
            }
        )
        return mesh

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda _artifact: source_path)
    monkeypatch.setattr(versions_router, "_load_json_artifact", lambda _artifact: region_payload)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.default_sdk, "offset_verts_mesh", fake_offset_verts_mesh)

    class FakeDb:
        def __init__(self) -> None:
            self.commits = 0
            self.refreshed = []

        def get(self, model, key):  # noqa: ANN001
            if model is ModelVersionRecord:
                return {version.id: version, output_version.id: output_version}.get(key)
            return None

        def commit(self) -> None:
            self.commits += 1

        def refresh(self, record):  # noqa: ANN001
            self.refreshed.append(record.id)

    db = FakeDb()
    response = asyncio.run(
        endpoint(
            version.id,
            request_cls(
                offset_mm=0.15,
                region_ids=["gem_seat"],
            ),
            db=db,
        )
    )

    assert len(calls) == 1
    np.testing.assert_allclose(
        calls[0]["offsets_mm"],
        [0.15, 0.15, 0.15, 0.0, 0.0, 0.0, 0.0, 0.0],
        rtol=0,
        atol=1e-7,
    )
    assert response.version.id == output_version.id
    assert response.mode == "offset_verts"
    assert response.offset_mm == 0.15
    assert response.selected_region_ids == ["gem_seat"]
    assert response.metadata["meshlib_reference"] == "MR::offsetVerts"
    assert response.metadata["meshlib_source"] == "MeshLib/source/MRMesh/MROffsetVerts.*"
    assert response.metadata["rust_backed"] is True
    assert registered["metadata_json"]["source"] == "rust_offset_verts"
    assert registered["metadata_json"]["selected_region_ids"] == ["gem_seat"]
    assert db.commits == 1
    assert output_version.id in db.refreshed


def test_runtime_brush_capabilities_advertise_brush_replay_sdk_operation() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )

    capabilities = {
        capability.command_id: capability
        for capability in versions_router._workbench_command_capabilities(version)
    }

    for command_id in ("runtime-thicken-brush", "runtime-scoop-brush", "runtime-smooth-brush"):
        capability = capabilities[command_id]
        assert capability.endpoint_url_key == "brush_endpoint_url"
        assert capability.sdk_operations == ["apply_brush_strokes"]


def test_make_delone_operation_passes_official_delone_settings_to_sdk(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_delone",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="make_delone",
        operation_label="Make Delone",
        status="ready",
    )
    job = JobRecord(
        id="job_make_delone",
        version_id=source_version.id,
        operation_type="make_delone",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [2.0, 2.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [0, 2, 3]], dtype=np.int64),
    )
    captured: dict[str, object] = {}

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_make_delone(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return mesh, 1

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "make_delone_edge_flips", fake_make_delone)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_make_delone_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        MakeDeloneRequest(
            num_iters=3,
            region_faces=[0, 1],
            max_deviation_after_flip=0.1,
            max_angle_change=0.5,
            critical_tri_aspect_ratio=2.0,
            not_flippable_edges=[[2, 0]],
            vert_region=[1, 3],
        ),
    )

    assert result is new_version
    assert captured == {
        "num_iters": 3,
        "region_faces": [0, 1],
        "max_deviation_after_flip": 0.1,
        "max_angle_change": 0.5,
        "critical_tri_aspect_ratio": 2.0,
        "not_flippable_edges": [[2, 0]],
        "vert_region": [1, 3],
    }


def test_subdivide_operation_passes_official_subdivide_settings_to_sdk(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_subdivided",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="subdivide",
        operation_label="Subdivide",
        status="ready",
    )
    job = JobRecord(
        id="job_subdivide",
        version_id=source_version.id,
        operation_type="subdivide",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )
    captured: dict[str, object] = {}

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_subdivide(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return SubdivideMeshResult(
            mesh=mesh,
            splits_done=1,
            region_faces=np.asarray([0], dtype=np.int64),
            region_face_count=1,
        )

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "get_artifact_by_type", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "subdivide_mesh", fake_subdivide)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_subdivide_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        SubdivideRequest(
            max_edge_len=0.25,
            max_edge_splits=12,
            subdivide_border=False,
            region_faces=[0],
            curvature_priority=5.0,
            project_on_original_mesh=True,
            smooth_mode=True,
            min_sharp_dihedral_angle=999.0,
            max_tri_aspect_ratio=1.3,
            max_splittable_tri_aspect_ratio=6.0,
            not_flippable_edges=[[0, 2]],
            max_deviation_after_flip=0.25,
            max_angle_change_after_flip=0.5,
            critical_tri_aspect_ratio_flip=2.0,
        ),
    )

    assert result is new_version
    assert captured == {
        "max_edge_len": 0.25,
        "max_edge_splits": 12,
        "subdivide_border": False,
        "region_faces": [0],
        "curvature_priority": 5.0,
        "project_on_original_mesh": True,
        "smooth_mode": True,
        "min_sharp_dihedral_angle": 999.0,
        "max_tri_aspect_ratio": 1.3,
        "max_splittable_tri_aspect_ratio": 6.0,
        "not_flippable_edges": [[0, 2]],
        "max_deviation_after_flip": 0.25,
        "max_angle_change_after_flip": 0.5,
        "critical_tri_aspect_ratio_flip": 2.0,
    }


def test_subdivide_operation_rejects_noop_result(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source_noop_subdivide",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_subdivide_noop",
        version_id=source_version.id,
        operation_type="subdivide",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], dtype=np.float64),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_subdivide(mesh_arg, **_kwargs):  # noqa: ANN001, ANN202
        assert mesh_arg is mesh
        return SubdivideMeshResult(
            mesh=mesh,
            splits_done=0,
            region_faces=np.asarray([0], dtype=np.int64),
            region_face_count=1,
        )

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "subdivide_mesh", fake_subdivide)
    monkeypatch.setattr(
        operations_service.default_sdk,
        "save_mesh",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            AssertionError("no-op subdivision must not save a mesh")
        ),
    )
    monkeypatch.setattr(
        operations_service,
        "_finalize_version",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            AssertionError("no-op subdivision must not finalize a version")
        ),
    )

    with pytest.raises(RuntimeError, match="Subdivision did not modify"):
        operations_service.run_subdivide_operation(
            FakeDb(),
            source_version,
            job,
            tmp_path,
            SubdivideRequest(max_edge_len=0.25, region_faces=[0]),
        )


def test_subdivide_operation_rejects_large_mesh_before_rust_kernel(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_subdivide_large",
        version_id=source_version.id,
        operation_type="subdivide",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
    )

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fail_subdivide(*_args, **_kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("large mesh should be rejected before Rust subdivision")

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_SUBDIVIDE_MAX_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "subdivide_mesh", fail_subdivide)

    with pytest.raises(RuntimeError, match="Subdivision is limited"):
        operations_service.run_subdivide_operation(
            FakeDb(),
            source_version,
            job,
            tmp_path,
            SubdivideRequest(max_edge_len=0.25),
        )


def test_subdivide_operation_allows_large_mesh_selected_region(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_subdivided",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="subdivide",
        operation_label="Subdivide",
        status="ready",
    )
    job = JobRecord(
        id="job_subdivide_region",
        version_id=source_version.id,
        operation_type="subdivide",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
    )
    captured: dict[str, object] = {}

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_subdivide(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return SubdivideMeshResult(
            mesh=mesh,
            splits_done=1,
            region_faces=np.asarray([0], dtype=np.int64),
            region_face_count=1,
        )

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_SUBDIVIDE_MAX_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "get_artifact_by_type", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "subdivide_mesh", fake_subdivide)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_subdivide_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        SubdivideRequest(max_edge_len=0.25, region_faces=[0]),
    )

    assert result is new_version
    assert captured["region_faces"] == [0]


def test_hollow_operation_uses_weighted_inner_offset_for_large_interactive_mesh(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_hollow_large",
        version_id=source_version.id,
        operation_type="hollow",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "label": "Inner Band",
                "vertex_indices": [0, 1, 2],
                "allowed_operations": ["hollow"],
            },
            {
                "region_id": "head",
                "label": "Head",
                "vertex_indices": [3],
                "allowed_operations": ["hollow"],
            },
            {
                "region_id": "gem_seat",
                "label": "Gem Seat",
                "vertex_indices": [2, 3],
                "allowed_operations": ["hollow"],
            },
            {
                "region_id": "ornament_relief",
                "label": "Ornament Relief",
                "vertex_indices": [1, 3],
                "allowed_operations": ["hollow"],
            },
        ]
    }

    new_version = ModelVersionRecord(
        id="ver_hollowed_large",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="hollow",
        operation_label="Hollow Mesh",
        status="processing",
    )
    hollowed = MeshDocument(mesh.vertices * 0.95, mesh.faces.copy())
    calls: list[str] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_weighted_preview(mesh_arg, region_entries, protect_region_ids, *, wall_thickness_mm):  # noqa: ANN001, ANN202
        calls.append("weighted_inner_offset_preview")
        assert mesh_arg is mesh
        assert [entry.region_id for entry in region_entries] == ["inner_band", "head", "gem_seat", "ornament_relief"]
        assert protect_region_ids == ["inner_band", "head", "gem_seat", "ornament_relief"]
        assert wall_thickness_mm == 0.8
        return hollowed

    def fail_full_voxel_hollow(*_args, **_kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("large interactive mesh should not call full voxel hollowing")

    def fake_save_mesh(mesh_arg, path, *, file_type):  # noqa: ANN001, ANN202
        calls.append(f"save:{path.name}:{file_type}")
        np.testing.assert_allclose(mesh_arg.vertices, hollowed.vertices)
        np.testing.assert_array_equal(mesh_arg.faces, hollowed.faces)

    def fake_finalize(db, version, job_arg, normalized_mesh_path, workdir, **kwargs):  # noqa: ANN001, ANN202
        calls.append(f"finalize:{version.id}:{normalized_mesh_path.name}")
        assert version is new_version
        assert job_arg is job
        assert kwargs["completion_message"] == "Interactive hollow preview completed"
        assert kwargs["normalized_mesh_metadata"]["full_voxel_hollow_deferred"] is True
        assert kwargs["normalized_mesh_metadata"]["source"] == "rust_weighted_inner_offset_preview"

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_HOLLOW_MAX_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "weighted_inner_offset_preview", fake_weighted_preview)
    monkeypatch.setattr(operations_service.default_sdk, "protected_hollow_mesh", fail_full_voxel_hollow)
    monkeypatch.setattr(operations_service.default_sdk, "service_hollow", fail_full_voxel_hollow)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", fake_save_mesh)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", fake_finalize)

    result = operations_service.run_hollow_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        HollowRequest(
            mode="fixed_thickness",
            wall_thickness_mm=0.8,
            protect_regions=["inner_band"],
            add_drain_holes=False,
        ),
    )

    assert result is new_version
    assert calls == [
        "weighted_inner_offset_preview",
        "save:job_hollow_large_hollow.ply:ply",
        "finalize:ver_hollowed_large:job_hollow_large_hollow.ply",
    ]
    assert hollowed.metadata["full_voxel_hollow_deferred"] is True
    assert hollowed.metadata["drain_holes_deferred"] is False
    assert hollowed.metadata["target_weight_deferred"] is False


@pytest.mark.parametrize(
    ("hollow_request", "expected"),
    [
        (
            HollowRequest(
                mode="target_weight",
                target_weight_g=3.0,
                protect_regions=["head", "gem_seat"],
                add_drain_holes=False,
            ),
            {"requested_mode": "target_weight", "target_weight_deferred": True, "drain_holes_deferred": False},
        ),
        (
            HollowRequest(
                mode="fixed_thickness",
                wall_thickness_mm=0.75,
                protect_regions=["head", "gem_seat"],
                add_drain_holes=True,
            ),
            {"requested_mode": "fixed_thickness", "target_weight_deferred": False, "drain_holes_deferred": True},
        ),
    ],
)
def test_hollow_operation_large_interactive_preview_accepts_weight_and_drain_requests(
    monkeypatch,
    tmp_path,
    hollow_request,
    expected,
) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_hollow_large",
        version_id=source_version.id,
        operation_type="hollow",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {"region_id": "inner_band", "label": "Inner Band", "vertex_indices": [0, 1, 2]},
            {"region_id": "head", "label": "Head", "vertex_indices": [3]},
            {"region_id": "gem_seat", "label": "Gem Seat", "vertex_indices": [2, 3]},
            {"region_id": "ornament_relief", "label": "Ornament Relief", "vertex_indices": [1, 3]},
        ]
    }
    new_version = ModelVersionRecord(
        id="ver_hollowed_large",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="hollow",
        operation_label="Hollow Mesh",
        status="processing",
    )
    hollowed = MeshDocument(mesh.vertices * 0.95, mesh.faces.copy())
    calls: list[str] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_weighted_preview(mesh_arg, region_entries, protect_region_ids, *, wall_thickness_mm):  # noqa: ANN001, ANN202
        calls.append("weighted_inner_offset_preview")
        assert mesh_arg is mesh
        assert [entry.region_id for entry in region_entries] == ["inner_band", "head", "gem_seat", "ornament_relief"]
        assert protect_region_ids == ["head", "gem_seat", "ornament_relief", "inner_band"]
        assert wall_thickness_mm == (hollow_request.wall_thickness_mm or 0.8)
        return hollowed

    def fail_full_voxel_hollow(*_args, **_kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("large interactive mesh should not call full voxel hollowing")

    def fake_save_mesh(mesh_arg, path, *, file_type):  # noqa: ANN001, ANN202
        calls.append(f"save:{path.name}:{file_type}")
        np.testing.assert_allclose(mesh_arg.vertices, hollowed.vertices)
        np.testing.assert_array_equal(mesh_arg.faces, hollowed.faces)

    def fake_finalize(db, version, job_arg, normalized_mesh_path, workdir, **kwargs):  # noqa: ANN001, ANN202
        calls.append(f"finalize:{version.id}:{normalized_mesh_path.name}")
        assert version is new_version
        assert job_arg is job
        assert kwargs["completion_message"] == "Interactive hollow preview completed"
        assert kwargs["normalized_mesh_metadata"]["full_voxel_hollow_deferred"] is True
        assert kwargs["normalized_mesh_metadata"]["requested_mode"] == expected["requested_mode"]
        assert kwargs["normalized_mesh_metadata"]["target_weight_deferred"] is expected["target_weight_deferred"]
        assert kwargs["normalized_mesh_metadata"]["drain_holes_deferred"] is expected["drain_holes_deferred"]

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_HOLLOW_MAX_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "weighted_inner_offset_preview", fake_weighted_preview)
    monkeypatch.setattr(operations_service.default_sdk, "protected_hollow_mesh", fail_full_voxel_hollow)
    monkeypatch.setattr(operations_service.default_sdk, "service_hollow", fail_full_voxel_hollow)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", fake_save_mesh)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", fake_finalize)

    result = operations_service.run_hollow_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        hollow_request,
    )

    assert result is new_version
    assert calls == [
        "weighted_inner_offset_preview",
        "save:job_hollow_large_hollow.ply:ply",
        "finalize:ver_hollowed_large:job_hollow_large_hollow.ply",
    ]
    assert hollowed.metadata["full_voxel_hollow_deferred"] is True
    assert hollowed.metadata["requested_mode"] == expected["requested_mode"]
    assert hollowed.metadata["target_weight_deferred"] is expected["target_weight_deferred"]
    assert hollowed.metadata["drain_holes_deferred"] is expected["drain_holes_deferred"]


def test_hollow_operation_large_full_resolution_rejects_unbounded_dense_mesh(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_hollow_full_large",
        version_id=source_version.id,
        operation_type="hollow",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
    )

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fail_geometry(*_args, **_kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("large full-resolution rejection should happen before hollow geometry")

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_HOLLOW_MAX_FACES", 1)
    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_HOLLOW_FULL_RESOLUTION_MAX_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_guard_protected_regions_for_hollow", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "weighted_inner_offset_preview", fail_geometry)
    monkeypatch.setattr(operations_service.default_sdk, "service_hollow", fail_geometry)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", fail_geometry)

    with pytest.raises(RuntimeError, match="Full-resolution voxel hollowing is limited to 1 faces"):
        operations_service.run_hollow_operation(
            FakeDb(),
            source_version,
            job,
            tmp_path,
            HollowRequest(
                mode="fixed_thickness",
                processing_mode="full_resolution",
                wall_thickness_mm=0.8,
                protect_regions=[],
                add_drain_holes=False,
            ),
        )


def test_hollow_operation_large_full_resolution_bypasses_interactive_preview(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_hollow_full",
        version_id=source_version.id,
        operation_type="hollow",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
    )
    hollowed = MeshDocument(mesh.vertices * 0.9, mesh.faces.copy())
    new_version = ModelVersionRecord(
        id="ver_hollowed_full",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="hollow",
        operation_label="Hollow Mesh",
        status="processing",
    )
    calls: list[str] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_service_hollow(mesh_arg, *, wall_thickness_mm):  # noqa: ANN001, ANN202
        calls.append("service_hollow")
        assert mesh_arg is mesh
        assert wall_thickness_mm == 0.8
        return hollowed

    def fail_weighted_preview(*_args, **_kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("full-resolution hollowing should not call the interactive preview")

    def fake_save_mesh(mesh_arg, path, *, file_type):  # noqa: ANN001, ANN202
        calls.append(f"save:{path.name}:{file_type}")
        np.testing.assert_allclose(mesh_arg.vertices, hollowed.vertices)
        np.testing.assert_array_equal(mesh_arg.faces, hollowed.faces)

    def fake_finalize(db, version, job_arg, normalized_mesh_path, workdir, **kwargs):  # noqa: ANN001, ANN202
        calls.append(f"finalize:{version.id}:{normalized_mesh_path.name}")
        assert version is new_version
        assert job_arg is job
        assert kwargs["completion_message"] == "Hollowing completed"
        assert kwargs["normalized_mesh_metadata"] is None

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_HOLLOW_MAX_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_guard_protected_regions_for_hollow", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "weighted_inner_offset_preview", fail_weighted_preview)
    monkeypatch.setattr(operations_service.default_sdk, "service_hollow_voxel_size", lambda *args, **kwargs: 0.2)
    monkeypatch.setattr(operations_service.default_sdk, "service_hollow", fake_service_hollow)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", fake_save_mesh)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", fake_finalize)

    result = operations_service.run_hollow_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        HollowRequest(
            mode="fixed_thickness",
            processing_mode="full_resolution",
            wall_thickness_mm=0.8,
            protect_regions=[],
            add_drain_holes=False,
        ),
    )

    assert result is new_version
    assert calls == [
        "service_hollow",
        "save:job_hollow_full_hollow.ply:ply",
        "finalize:ver_hollowed_full:job_hollow_full_hollow.ply",
    ]


def test_finalize_version_surfaces_deferred_hollow_metadata_in_snapshot_recommendations(monkeypatch, tmp_path) -> None:
    version = ModelVersionRecord(
        id="ver_deferred_hollow",
        model_id="mdl_ready",
        parent_version_id="ver_source",
        operation_type="hollow",
        operation_label="Interactive Hollow Preview",
        status="processing",
    )
    job = JobRecord(
        id="job_deferred_hollow",
        version_id="ver_source",
        operation_type="hollow",
        status="running",
        progress_pct=70,
    )
    normalized_mesh_path = tmp_path / "hollow_preview.ply"
    normalized_mesh_path.write_text("ply", encoding="utf-8")
    snapshot = SimpleNamespace(
        version_id="",
        thickness=SimpleNamespace(scalar_field_artifact_id=None),
        recommendations=["Thickness analysis deferred for this large mesh; use focused Measure/Inspect before casting."],
    )

    def snapshot_payload(*, mode):  # noqa: ANN202
        assert mode == "json"
        return {
            "version_id": snapshot.version_id,
            "thickness": {"scalar_field_artifact_id": snapshot.thickness.scalar_field_artifact_id},
            "recommendations": list(snapshot.recommendations),
        }

    snapshot.model_dump = snapshot_payload
    artifacts = SimpleNamespace(
        thickness_scalar_path=tmp_path / "thickness.npz",
        region_json_path=tmp_path / "regions.json",
    )
    artifacts.thickness_scalar_path.write_bytes(b"npz")
    artifacts.region_json_path.write_text("{}", encoding="utf-8")
    calls: list[tuple[str, object]] = []

    class FakeDb:
        def in_transaction(self) -> bool:
            return True

        def commit(self) -> None:
            calls.append(("commit", None))

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type, metadata_json=None):  # noqa: ANN001, ANN202
        if artifact_type == "normalized_mesh_ply":
            calls.append(("normalized_metadata", metadata_json))
        return SimpleNamespace(id=f"art_{artifact_type}")

    def fake_upsert_snapshot(db, version_id, snapshot_type, payload):  # noqa: ANN001, ANN202
        calls.append(("snapshot", payload))

    monkeypatch.setattr(operations_service, "to_glb", lambda source, target: Path(target).write_bytes(b"glb"))
    monkeypatch.setattr(operations_service, "to_stl", lambda source, target: Path(target).write_bytes(b"stl"))
    monkeypatch.setattr(operations_service, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(operations_service, "compute_manufacturability_snapshot", lambda *_args, **_kwargs: (snapshot, artifacts))
    monkeypatch.setattr(operations_service, "upsert_snapshot", fake_upsert_snapshot)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)

    operations_service._finalize_version(
        FakeDb(),
        version,
        job,
        normalized_mesh_path,
        tmp_path,
        normalized_mesh_metadata={
            "source": "rust_weighted_inner_offset_preview",
            "full_voxel_hollow_deferred": True,
            "drain_holes_requested": True,
            "drain_holes_deferred": True,
            "target_weight_deferred": False,
        },
    )

    normalized_metadata = next(value for name, value in calls if name == "normalized_metadata")
    assert normalized_metadata["full_voxel_hollow_deferred"] is True
    snapshot_payload_value = next(value for name, value in calls if name == "snapshot")
    assert any("Full voxel hollow/drain finalization deferred" in item for item in snapshot_payload_value["recommendations"])


def test_decimate_operation_passes_official_qem_settings_to_sdk(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_decimated",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="decimate",
        operation_label="Decimate",
        status="ready",
    )
    job = JobRecord(
        id="job_decimate",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )
    captured: dict[str, object] = {}

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_decimate(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return DecimateMeshResult(
            mesh=mesh,
            verts_deleted=1,
            faces_deleted=1,
            error_introduced=0.2,
            cancelled=False,
        )

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "decimate_mesh", fake_decimate)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_decimate_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        DecimateRequest(
            strategy="minimize_error",
            max_error=0.2,
            target_face_count=120,
            subdivide_parts=8,
            decimate_between_parts=False,
            max_edge_len=1.5,
            max_bd_shift=0.25,
            stabilizer=0.75,
            region_faces=[0, 2],
            not_flippable_edges=[[1, 3], [2, 4]],
            edges_to_collapse=[[0, 1], [3, 4]],
            collapse_near_not_flippable=True,
            angle_weighted_dist_to_plane=True,
            max_deleted_vertices=12,
            max_deleted_faces=24,
            max_triangle_aspect_ratio=7.5,
            critical_tri_aspect_ratio=3.5,
            tiny_edge_length=0.25,
            max_angle_change=0.5,
            touch_near_bd_edges=False,
            touch_bd_verts=False,
            optimize_vertex_pos=False,
            pack_mesh=True,
        ),
    )

    assert result is new_version
    assert captured == {
        "strategy": "minimize_error",
        "max_error": 0.2,
        "target_face_count": 120,
        "target_face_ratio": None,
        "subdivide_parts": 8,
        "decimate_between_parts": False,
        "max_edge_len": 1.5,
        "max_bd_shift": 0.25,
        "stabilizer": 0.75,
        "region_faces": [0, 2],
        "not_flippable_edges": [[1, 3], [2, 4]],
        "edges_to_collapse": [[0, 1], [3, 4]],
        "collapse_near_not_flippable": True,
        "angle_weighted_dist_to_plane": True,
        "max_deleted_vertices": 12,
        "max_deleted_faces": 24,
        "max_triangle_aspect_ratio": 7.5,
        "critical_tri_aspect_ratio": 3.5,
        "tiny_edge_length": 0.25,
        "max_angle_change": 0.5,
        "touch_near_bd_edges": False,
        "touch_bd_verts": False,
        "optimize_vertex_pos": False,
        "pack_mesh": True,
    }


def test_decimate_operation_treats_empty_region_faces_as_global_selection(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source_global_decimate",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_decimated_global",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="decimate",
        operation_label="Decimate",
        status="ready",
    )
    job = JobRecord(
        id="job_decimate_global",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )
    captured: dict[str, object] = {}

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_decimate(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return DecimateMeshResult(
            mesh=mesh,
            verts_deleted=1,
            faces_deleted=1,
            error_introduced=0.1,
            cancelled=False,
        )

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "decimate_mesh", fake_decimate)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_decimate_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        DecimateRequest(
            strategy="minimize_error",
            max_error=100.0,
            target_face_count=4,
            region_faces=[],
            pack_mesh=True,
        ),
    )

    assert result is new_version
    assert captured["region_faces"] is None


def test_decimate_operation_allows_tiny_capped_dense_workbench_canvas_without_selected_faces(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source_workbench_decimate",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_decimate_workbench",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    new_version = ModelVersionRecord(
        id="ver_decimated_workbench",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="decimate",
        operation_label="Decimate",
        status="ready",
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [1, 3, 2]], dtype=np.int64),
    )
    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    captured: dict[str, object] = {}

    def fake_decimate(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return DecimateMeshResult(
            mesh=mesh,
            verts_deleted=1,
            faces_deleted=1,
            error_introduced=0.1,
            cancelled=False,
        )

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "get_artifact_by_type", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "decimate_mesh", fake_decimate)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_decimate_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        DecimateRequest(
            strategy="minimize_error",
            max_error=0.2,
            max_deleted_vertices=1,
            max_deleted_faces=1,
            metadata={"source": "meshlib_canvas_plugin_overlay"},
        ),
    )

    assert result is new_version
    assert captured["region_faces"] is None
    assert captured["max_deleted_vertices"] == 1
    assert captured["max_deleted_faces"] == 1


def test_decimate_operation_uses_committed_workbench_selection_faces(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source_workbench_selected_decimate",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_decimated_selection",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="decimate",
        operation_label="Decimate",
        status="processing",
    )
    job = JobRecord(
        id="job_decimate_workbench_selection",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 2], [1, 3, 2]], dtype=np.int64),
    )
    selection_path = tmp_path / "meshlib_selection.json"
    selection_path.write_text(
        '{"resolved_face_ids": [1], "selection": {"face_ids": [0]}}',
        encoding="utf-8",
    )
    selection_artifact = ModelArtifactRecord(
        id="art_selection_workbench_decimate",
        version_id=source_version.id,
        artifact_type="meshlib_selection_json",
        mime_type="application/json",
        storage_key="ver_source_workbench_selected_decimate/meshlib_selection_json.json",
        size_bytes=selection_path.stat().st_size,
        metadata_json={},
    )
    captured: dict[str, object] = {}

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_get_artifact_by_type(_db, version_id, artifact_type):  # noqa: ANN001, ANN202
        if version_id == source_version.id and artifact_type == "meshlib_selection_json":
            return selection_artifact
        return None

    def fake_decimate(mesh_arg, **kwargs):  # noqa: ANN001, ANN202
        captured.update(kwargs)
        assert mesh_arg is mesh
        return DecimateMeshResult(
            mesh=mesh,
            verts_deleted=1,
            faces_deleted=1,
            error_introduced=0.1,
            cancelled=False,
        )

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES", 10)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(operations_service, "materialize_artifact", lambda artifact, workdir: selection_path)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "decimate_mesh", fake_decimate)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_decimate_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        DecimateRequest(
            strategy="minimize_error",
            target_face_ratio=0.8,
            max_error=5.0,
            metadata={"source": "meshlib_canvas_plugin_overlay"},
        ),
    )

    assert result is new_version
    assert captured["region_faces"] == [1]


def test_decimate_operation_rejects_large_global_mesh_before_destructive_preview(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_decimate_preview",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.4, 0.0, 0.0],
                [0.8, 0.0, 0.0],
                [0.0, 0.4, 0.0],
                [0.4, 0.4, 0.0],
                [0.8, 0.4, 0.0],
            ],
            dtype=np.float64,
        ),
        np.asarray([[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4]], dtype=np.int64),
    )
    calls: list[str] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_DECIMATE_MAX_INTERACTIVE_FACES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(
        operations_service,
        "add_job_event",
        lambda _db, _job_id, message, progress, *_args, **_kwargs: calls.append(f"event:{progress}:{message}"),
    )
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(
        operations_service.default_sdk,
        "decimate_mesh",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("large global decimate should fail fast")),
    )
    monkeypatch.setattr(
        operations_service.default_sdk,
        "save_mesh",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("failed decimate must not save a mesh")),
    )
    monkeypatch.setattr(
        operations_service,
        "_finalize_version",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("failed decimate must not finalize a version")),
    )

    with pytest.raises(RuntimeError, match="Interactive full-mesh decimation is limited"):
        operations_service.run_decimate_operation(
            FakeDb(),
            source_version,
            job,
            tmp_path,
            DecimateRequest(strategy="minimize_error", target_face_ratio=0.4, max_error=5.0),
        )

    assert calls == [
        "event:5:Decimation started",
    ]


def test_decimate_operation_rejects_noop_result(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_decimate_noop",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(
        operations_service.default_sdk,
        "decimate_mesh",
        lambda *_args, **_kwargs: DecimateMeshResult(
            mesh=mesh,
            verts_deleted=0,
            faces_deleted=0,
            error_introduced=0.0,
            cancelled=False,
        ),
    )
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)

    with pytest.raises(RuntimeError, match="Decimation did not modify"):
        operations_service.run_decimate_operation(
            FakeDb(),
            source_version,
            job,
            tmp_path,
            DecimateRequest(strategy="minimize_error", target_face_ratio=0.8, max_error=5.0),
        )


def test_decimate_operation_rejects_closed_mesh_boundary_regression(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source_closed",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_decimate_closed_boundary_regression",
        version_id=source_version.id,
        operation_type="decimate",
        status="queued",
        progress_pct=0,
    )
    closed_mesh = MeshDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            dtype=np.float64,
        ),
        np.asarray(
            [
                [0, 2, 1],
                [0, 1, 3],
                [1, 2, 3],
                [2, 0, 3],
            ],
            dtype=np.int64,
        ),
    )
    opened_mesh = MeshDocument(
        closed_mesh.vertices,
        np.asarray(
            [
                [0, 2, 1],
                [0, 1, 3],
                [1, 2, 3],
            ],
            dtype=np.int64,
        ),
    )

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: closed_mesh)
    monkeypatch.setattr(
        operations_service.default_sdk,
        "decimate_mesh",
        lambda *_args, **_kwargs: DecimateMeshResult(
            mesh=opened_mesh,
            verts_deleted=1,
            faces_deleted=1,
            error_introduced=0.2,
            cancelled=False,
        ),
    )
    monkeypatch.setattr(
        operations_service.default_sdk,
        "save_mesh",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            AssertionError("quality guard must reject before saving a damaged decimate mesh")
        ),
    )
    monkeypatch.setattr(
        operations_service,
        "_finalize_version",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            AssertionError("quality guard must reject before finalizing a damaged decimate mesh")
        ),
    )

    with pytest.raises(RuntimeError, match="Decimation output failed quality guard: introduced"):
        operations_service.run_decimate_operation(
            FakeDb(),
            source_version,
            job,
            tmp_path,
            DecimateRequest(strategy="minimize_error", target_face_ratio=0.8, max_error=5.0),
        )


def test_scoop_operation_accepts_large_mesh_deferred_preview_thickness(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_scoop",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="scoop",
        operation_label="Scoop",
        status="ready",
    )
    job = JobRecord(
        id="job_scoop",
        version_id=source_version.id,
        operation_type="scoop",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [1.0, 0.0, 0.0]], dtype=np.float64),
        np.asarray([[0, 1, 2]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "label": "Inner Band",
                "vertex_indices": [0, 1, 2],
                "allowed_operations": ["scoop"],
                "min_thickness_mm": None,
            }
        ]
    }
    preview_snapshot = SimpleNamespace(
        thickness=SimpleNamespace(min_mm=None),
        recommendations=["Thickness analysis deferred for this large mesh; use focused Measure/Inspect before casting."],
    )

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "local_scoop", lambda *args, **kwargs: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "compute_manufacturability_snapshot", lambda *args, **kwargs: (preview_snapshot, None))
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_scoop_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        ScoopRequest(region_id="inner_band", depth_mm=0.05, falloff_mm=1.5, keep_min_thickness_mm=0.6),
    )

    assert result is new_version


def test_scoop_operation_bounds_dense_region_seed_count(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_scoop",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="scoop",
        operation_label="Scoop",
        status="ready",
    )
    job = JobRecord(
        id="job_scoop",
        version_id=source_version.id,
        operation_type="scoop",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[float(index), float(index % 3), 0.0] for index in range(12)], dtype=np.float64),
        np.asarray([[0, 1, 2], [3, 4, 5], [6, 7, 8], [9, 10, 11]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "label": "Inner Band",
                "vertex_indices": list(range(12)),
                "allowed_operations": ["scoop"],
                "min_thickness_mm": None,
            }
        ]
    }
    preview_snapshot = SimpleNamespace(
        thickness=SimpleNamespace(min_mm=None),
        recommendations=["Thickness analysis deferred for this large mesh; use focused Measure/Inspect before casting."],
    )
    captured_seed_counts: list[int] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_local_scoop(_mesh, seed_indices, **_kwargs):  # noqa: ANN001, ANN202
        captured_seed_counts.append(int(np.asarray(seed_indices).size))
        return mesh

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES", 3)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "local_scoop", fake_local_scoop)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "compute_manufacturability_snapshot", lambda *args, **kwargs: (preview_snapshot, None))
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_scoop_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        ScoopRequest(region_id="inner_band", depth_mm=0.05, falloff_mm=1.5, keep_min_thickness_mm=0.6),
    )

    assert result is new_version
    assert captured_seed_counts == [3]


def test_scoop_operation_skips_duplicate_preview_for_deferred_dense_mesh(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_scoop",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="scoop",
        operation_label="Scoop",
        status="ready",
    )
    job = JobRecord(
        id="job_scoop",
        version_id=source_version.id,
        operation_type="scoop",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[float(index), 0.0, 0.0] for index in range(4)], dtype=np.float64),
        np.asarray([[0, 1, 2], [1, 2, 3]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "label": "Inner Band",
                "vertex_indices": [0, 1, 2, 3],
                "allowed_operations": ["scoop"],
                "min_thickness_mm": None,
            }
        ]
    }

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    monkeypatch.setattr(operations_service.settings, "MANUFACTURABILITY_THICKNESS_MAX_VERTICES", 1)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "local_scoop", lambda *args, **kwargs: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(
        operations_service,
        "compute_manufacturability_snapshot",
        lambda *args, **kwargs: (_ for _ in ()).throw(
            AssertionError("dense scoop should not run a duplicate preview manufacturability pass")
        ),
    )
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_scoop_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        ScoopRequest(region_id="inner_band", depth_mm=0.05, falloff_mm=1.5, keep_min_thickness_mm=0.6),
    )

    assert result is new_version


def test_smooth_operation_bounds_dense_region_seed_count(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_smooth",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="smooth",
        operation_label="Smooth",
        status="ready",
    )
    job = JobRecord(
        id="job_smooth",
        version_id=source_version.id,
        operation_type="smooth",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[float(index), float(index % 2), 0.0] for index in range(10)], dtype=np.float64),
        np.asarray([[0, 1, 2], [3, 4, 5], [6, 7, 8]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "label": "Inner Band",
                "vertex_indices": list(range(10)),
                "allowed_operations": ["smooth"],
                "min_thickness_mm": None,
            }
        ]
    }
    captured_seed_counts: list[int] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_smooth(_mesh, *args, **kwargs):  # noqa: ANN001, ANN202
        captured_seed_counts.append(int(np.asarray(kwargs["seed_indices"]).size))
        return mesh

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES", 4)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "smooth", fake_smooth)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_smooth_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        SmoothRequest(region_id="inner_band", iterations=1, strength=0.25, global_mode=False),
    )

    assert result is new_version
    assert captured_seed_counts == [4]


def test_thicken_violations_uses_selected_region_when_thickness_is_deferred(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_thicken",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="thicken",
        operation_label="Thicken Mesh",
        status="ready",
    )
    job = JobRecord(
        id="job_thicken",
        version_id=source_version.id,
        operation_type="thicken",
        status="queued",
        progress_pct=0,
    )
    mesh = MeshDocument(
        np.asarray([[float(index), 0.0, 0.0] for index in range(8)], dtype=np.float64),
        np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
    )
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "label": "Inner Band",
                "vertex_indices": list(range(8)),
                "allowed_operations": ["thicken"],
            }
        ]
    }
    captured: dict[str, object] = {}
    events: list[str] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_local_thicken(_mesh, seed_indices, thickness_values, **kwargs):  # noqa: ANN001, ANN202
        captured["seed_indices"] = np.asarray(seed_indices, dtype=np.int64)
        captured["thickness_values"] = np.asarray(thickness_values, dtype=np.float32)
        captured["kwargs"] = kwargs
        return mesh

    monkeypatch.setattr(operations_service.settings, "MESH_EDIT_LOCAL_DEFORM_MAX_SEED_VERTICES", 4)
    monkeypatch.setattr(operations_service, "set_job_status", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "add_job_event", lambda *args, **kwargs: None)
    monkeypatch.setattr(
        operations_service,
        "_record_job_event",
        lambda _db, _job_id, message, *args, **kwargs: events.append(message),
    )
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_region_payload", lambda *args, **kwargs: region_payload)
    monkeypatch.setattr(
        operations_service,
        "_load_thickness_values",
        lambda *args, **kwargs: np.full(mesh.vertex_count, np.nan, dtype=np.float32),
    )
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "local_thicken_to_minimum", fake_local_thicken)
    monkeypatch.setattr(operations_service.default_sdk, "smooth", lambda thickened, **kwargs: thickened)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_thicken_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        ThickenRequest(
            mode="violations_only",
            min_target_thickness_mm=0.8,
            region_id="inner_band",
            smoothing_pass=True,
        ),
    )

    assert result is new_version
    np.testing.assert_array_equal(captured["seed_indices"], np.asarray([0, 2, 4, 6], dtype=np.int64))
    assert np.isnan(captured["thickness_values"]).all()
    assert captured["kwargs"]["min_target_thickness_mm"] == 0.8
    assert any("Thickness analysis is deferred" in message for message in events)


def test_decimate_request_defaults_keep_meshlib_unbounded_half_face_guard() -> None:
    request = DecimateRequest()

    assert request.max_error > 1e30
    assert request.target_face_count is None
    assert request.target_face_ratio is None
    assert request.subdivide_parts == 1
    assert request.decimate_between_parts is True
    assert request.max_deleted_vertices == 2_147_483_647
    assert request.max_deleted_faces == 2_147_483_647


def test_decimate_request_rejects_ambiguous_target_count_and_ratio() -> None:
    with pytest.raises(ValueError, match="target_face_count and target_face_ratio are mutually exclusive"):
        DecimateRequest(target_face_count=120, target_face_ratio=0.8)


def test_version_jobs_endpoint_returns_jobs_for_current_version() -> None:
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    job = JobRecord(
        id="job_ready",
        version_id=version.id,
        operation_type="repair",
        status="succeeded",
        progress_pct=100,
        created_at=datetime(2026, 6, 4, tzinfo=timezone.utc),
    )

    class FakeScalars:
        def all(self):  # noqa: ANN201
            return [job]

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001
            return version if model is ModelVersionRecord and key == version.id else None

        def scalars(self, statement):  # noqa: ANN001
            return FakeScalars()

    response = asyncio.run(jobs_router.list_version_jobs(version.id, db=FakeDb()))

    assert [item.id for item in response] == ["job_ready"]
    assert response[0].version_id == version.id


def test_make_manufacturable_flow_runs_advertised_service_health_validation(monkeypatch, tmp_path) -> None:
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    repaired_version = ModelVersionRecord(
        id="ver_repaired",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="repair",
        operation_label="Repair",
        status="ready",
    )
    resized_version = ModelVersionRecord(
        id="ver_resized",
        model_id=source_version.model_id,
        parent_version_id=repaired_version.id,
        operation_type="resize",
        operation_label="Resize",
        status="ready",
    )
    hollowed_version = ModelVersionRecord(
        id="ver_hollowed",
        model_id=source_version.model_id,
        parent_version_id=resized_version.id,
        operation_type="hollow",
        operation_label="Hollow",
        status="processing",
    )
    job = JobRecord(
        id="job_make",
        version_id=source_version.id,
        operation_type="make_manufacturable",
        status="queued",
        progress_pct=0,
    )
    job.operation_request = OperationRequestRecord(id="req_make", job_id=job.id, payload_json={})

    class FakeDb:
        def __init__(self) -> None:
            self.versions = {
                source_version.id: source_version,
                repaired_version.id: repaired_version,
                resized_version.id: resized_version,
                hollowed_version.id: hollowed_version,
            }
            self.commits = 0

        def get(self, model, key):  # noqa: ANN001, ANN201
            return self.versions.get(key) if model is ModelVersionRecord else None

        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

        def commit(self) -> None:
            self.commits += 1

    operation_calls: list[str] = []

    def fake_repair(*args, **kwargs):  # noqa: ANN002, ANN003, ANN202
        operation_calls.append("repair")
        return repaired_version

    def fake_resize(*args, **kwargs):  # noqa: ANN002, ANN003, ANN202
        operation_calls.append("resize")
        return resized_version

    def fake_hollow(*args, **kwargs):  # noqa: ANN002, ANN003, ANN202
        operation_calls.append("hollow")
        return hollowed_version

    final_mesh = object()

    def fake_load_normalized_artifact(db, version_id, workdir):  # noqa: ANN001, ANN202
        operation_calls.append(f"load:{version_id}")
        return tmp_path / f"{version_id}.ply"

    def fake_load_mesh(path):  # noqa: ANN001, ANN202
        operation_calls.append(f"load_mesh:{path.name}")
        return final_mesh

    def fake_service_health(mesh):  # noqa: ANN001, ANN202
        operation_calls.append("service_health")
        assert mesh is final_mesh

    monkeypatch.setattr(operations_service, "run_repair_operation", fake_repair)
    monkeypatch.setattr(operations_service, "run_resize_operation", fake_resize)
    monkeypatch.setattr(operations_service, "run_hollow_operation", fake_hollow)
    monkeypatch.setattr(operations_service, "_load_normalized_artifact", fake_load_normalized_artifact)
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", fake_load_mesh)
    monkeypatch.setattr(operations_service.default_sdk, "service_health", fake_service_health)

    result = operations_service.run_make_manufacturable_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        MakeManufacturableRequest(target_ring_size_us=8.0, target_weight_g=4.5, min_allowed_thickness_mm=0.8),
    )

    assert result is hollowed_version
    assert result.status == "ready"
    assert operation_calls == [
        "repair",
        "resize",
        "hollow",
        "load:ver_hollowed",
        "load_mesh:ver_hollowed.ply",
        "service_health",
    ]


def test_selection_commit_content_guard_requires_actual_selected_geometry() -> None:
    metadata_only = SelectionCommitRequest(
        selection=InteractiveSelectionPayload(metadata={"note": "operator opened the selection tool"}),
        metadata={"source": "meshlib_workbench_selection"},
    )
    face_selection = SelectionCommitRequest(
        selection=InteractiveSelectionPayload(mode="faces", face_ids=[7, 11]),
        metadata={"source": "meshlib_workbench_selection"},
    )

    assert _selection_counts(face_selection) == {
        "vertex_ids": 0,
        "face_ids": 2,
        "region_ids": 0,
        "brush_points_world": 0,
    }
    assert not _selection_has_content(metadata_only)
    assert _selection_has_content(face_selection)


def test_selection_commit_resolves_workbench_selection_to_valid_mesh_vertices() -> None:
    mesh = cube(size=2.0)
    first_face_vertices = [int(index) for index in mesh.faces[0]]
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "vertex_indices": [0, 1],
            }
        ]
    }
    selection = InteractiveSelectionPayload(
        mode="brush",
        vertex_ids=[2],
        face_ids=[0],
        region_ids=["inner_band"],
        brush_points_world=[(2.0, 0.0, 0.0)],
    )

    resolved = versions_router._resolve_selection_vertex_ids(mesh, selection, region_payload)

    assert {0, 1, 2}.issubset(set(resolved))
    assert set(first_face_vertices).issubset(set(resolved))
    assert len(resolved) == len(set(resolved))
    assert min(resolved) >= 0
    assert max(resolved) < mesh.vertex_count


def test_selection_commit_expands_selected_faces_to_meshlib_components() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [4, 5, 6]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        face_ids=[0],
        metadata={"expand_to_components": True},
    )

    resolved = versions_router._resolve_selection_vertex_ids(mesh, selection, None)

    assert resolved == [0, 1, 2, 3]


def test_selection_commit_applies_meshinspector_primary_ctrl_toggle_modifier() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [1, 4, 3]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        face_ids=[1, 2],
        metadata={
            "previous_face_ids": [0, 1],
            "modifier_primary_ctrl": True,
        },
    )

    resolved = versions_router._resolve_selection_face_ids(mesh, selection, None)

    assert resolved == [0, 2]


def test_selection_commit_can_create_meshlib_selection_to_object_version(monkeypatch, tmp_path) -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [1, 4, 3]], dtype=np.int64),
        metadata={},
    )
    selected_mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 3, 4]], dtype=np.int64),
        metadata={
            "meshlib_operation": "Mesh::cloneRegion",
            "source_face_indices": [0, 2],
            "source_vertex_indices": [0, 1, 2, 4, 3],
        },
    )
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ring",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ingest",
        status="ready",
    )
    selected_version = ModelVersionRecord(
        id="ver_selection",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="selection_to_object",
        operation_label="Selection to Object",
        status="ready",
    )
    normalized_artifact = ModelArtifactRecord(
        id="art_norm",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="model/ply",
        storage_key="ver_ready/normalized_mesh.ply",
        size_bytes=123,
        metadata_json={},
    )
    selection_artifact = ModelArtifactRecord(
        id="art_selection",
        version_id=version.id,
        artifact_type="meshlib_selection_json",
        mime_type="application/json",
        storage_key="ver_ready/meshlib_selection.json",
        size_bytes=123,
        metadata_json={},
    )
    object_artifact = ModelArtifactRecord(
        id="art_object",
        version_id=selected_version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="model/ply",
        storage_key="ver_selection/normalized_mesh.ply",
        size_bytes=456,
        metadata_json={},
    )
    calls: list[str] = []

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001, ANN201
            return version if model is ModelVersionRecord and key == version.id else None

        def commit(self) -> None:
            calls.append("commit")

        def refresh(self, artifact) -> None:  # noqa: ANN001
            calls.append(f"refresh:{artifact.id}")

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001, ANN202
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return normalized_artifact
        if version_id == version.id and artifact_type == "analysis_regions_json":
            return None
        raise AssertionError(f"unexpected artifact lookup: {version_id} {artifact_type}")

    def fake_create_version(db, *, model_id, parent_version_id, operation_type, operation_label, status):  # noqa: ANN001, ANN202
        assert model_id == version.model_id
        assert parent_version_id == version.id
        assert operation_type == "selection_to_object"
        assert operation_label == "Selection to Object"
        assert status == "ready"
        calls.append("create_version")
        return selected_version

    def fake_extract_selected_faces_as_mesh(mesh_arg, face_ids):  # noqa: ANN001, ANN202
        assert mesh_arg is mesh
        assert face_ids == [0, 2]
        calls.append("extract_selected_faces_as_mesh")
        return selected_mesh

    def fake_save_mesh(mesh_arg, path, *, file_type=None):  # noqa: ANN001, ANN202
        assert mesh_arg is selected_mesh
        assert file_type == "ply"
        Path(path).write_text("ply", encoding="utf-8")
        calls.append(f"save_mesh:{Path(path).name}")
        return Path(path)

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type, metadata_json=None):  # noqa: ANN001, ANN202
        if artifact_type == "meshlib_selection_json":
            assert version_id == version.id
            return selection_artifact
        assert version_id == selected_version.id
        assert artifact_type == "normalized_mesh_ply"
        assert mime_type == "model/ply"
        assert metadata_json["source"] == "meshlib_selection_to_object"
        assert metadata_json["meshlib_operation"] == "Mesh::cloneRegion"
        assert metadata_json["source_face_indices"] == [0, 2]
        assert metadata_json["source_vertex_indices"] == [0, 1, 2, 4, 3]
        calls.append("register_selected_object")
        return object_artifact

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda artifact: tmp_path / "mesh.ply")
    monkeypatch.setattr(versions_router.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(versions_router.default_sdk, "extract_selected_faces_as_mesh", fake_extract_selected_faces_as_mesh)
    monkeypatch.setattr(versions_router.default_sdk, "save_mesh", fake_save_mesh)
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.settings, "TEMP_DIR", tmp_path)

    response = asyncio.run(
        versions_router.commit_selection(
            version.id,
            SelectionCommitRequest(
                operation_label="Selection to Object",
                create_object=True,
                selection=InteractiveSelectionPayload(mode="faces", face_ids=[2, 0, 2]),
            ),
            db=FakeDb(),
        )
    )

    assert response.artifact_id == selection_artifact.id
    assert response.resolved_counts == {"vertex_ids": 5, "face_ids": 2}
    assert response.selected_object_version_id == selected_version.id
    assert response.selected_object_artifact_id == object_artifact.id
    assert response.selected_object_artifact_url == f"/api/artifacts/{object_artifact.id}"
    assert response.selected_object_artifact_type == "normalized_mesh_ply"
    assert response.selected_object_counts == {"vertex_ids": 5, "face_ids": 2}
    assert calls == [
        "extract_selected_faces_as_mesh",
        "create_version",
        "save_mesh:selection_object.ply",
        "register_selected_object",
        "commit",
        "refresh:art_selection",
        "refresh:art_object",
    ]


def test_selection_commit_can_create_meshlib_point_cloud_selection_to_object_version(monkeypatch, tmp_path) -> None:
    cloud = PointCloudDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        ),
        metadata={"source": "test_point_cloud"},
    )
    selected_cloud = PointCloudDocument(
        np.asarray([[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]], dtype=np.float64),
        metadata={
            "meshlib_operation": "ObjectPoints::cloneRegion",
            "source_point_indices": [1, 3],
        },
    )
    version = ModelVersionRecord(
        id="ver_points",
        model_id="mdl_scan",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Point Cloud Ingest",
        status="ready",
    )
    selected_version = ModelVersionRecord(
        id="ver_points_selection",
        model_id=version.model_id,
        parent_version_id=version.id,
        operation_type="selection_to_object",
        operation_label="Point Selection to Object",
        status="ready",
    )
    point_cloud_artifact = ModelArtifactRecord(
        id="art_points",
        version_id=version.id,
        artifact_type="normalized_point_cloud_ply",
        mime_type="model/ply",
        storage_key="ver_points/point_cloud.ply",
        size_bytes=123,
        metadata_json={},
    )
    selection_artifact = ModelArtifactRecord(
        id="art_points_selection_json",
        version_id=version.id,
        artifact_type="meshlib_selection_json",
        mime_type="application/json",
        storage_key="ver_points/meshlib_selection.json",
        size_bytes=123,
        metadata_json={},
    )
    object_artifact = ModelArtifactRecord(
        id="art_points_object",
        version_id=selected_version.id,
        artifact_type="normalized_point_cloud_ply",
        mime_type="model/ply",
        storage_key="ver_points_selection/normalized_point_cloud_ply.ply",
        size_bytes=456,
        metadata_json={},
    )
    calls: list[str] = []

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001, ANN201
            return version if model is ModelVersionRecord and key == version.id else None

        def commit(self) -> None:
            calls.append("commit")

        def refresh(self, artifact) -> None:  # noqa: ANN001
            calls.append(f"refresh:{artifact.id}")

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001, ANN202
        if version_id == version.id and artifact_type == "normalized_point_cloud_ply":
            return point_cloud_artifact
        if version_id == version.id and artifact_type in {"normalized_mesh_ply", "analysis_regions_json"}:
            return None
        raise AssertionError(f"unexpected artifact lookup: {version_id} {artifact_type}")

    def fake_create_version(db, *, model_id, parent_version_id, operation_type, operation_label, status):  # noqa: ANN001, ANN202
        assert model_id == version.model_id
        assert parent_version_id == version.id
        assert operation_type == "selection_to_object"
        assert operation_label == "Point Selection to Object"
        assert status == "ready"
        calls.append("create_version")
        return selected_version

    def fake_extract_selected_points_as_object(cloud_arg, point_ids):  # noqa: ANN001, ANN202
        assert cloud_arg is cloud
        assert point_ids == [1, 3]
        calls.append("point_cloud_extract_selected_points_as_object")
        return selected_cloud

    def fake_load_point_cloud(path):  # noqa: ANN001, ANN202
        assert Path(path).name == "point_cloud.ply"
        calls.append("load_point_cloud:point_cloud.ply")
        return cloud

    def fake_save_point_cloud(cloud_arg, path):  # noqa: ANN001, ANN202
        assert cloud_arg is selected_cloud
        Path(path).write_bytes(b"ply\n")
        calls.append(f"save_point_cloud:{Path(path).name}")
        return Path(path)

    def fake_register_file_artifact(db, version_id, file_path, artifact_type, mime_type, metadata_json=None):  # noqa: ANN001, ANN202
        if artifact_type == "meshlib_selection_json":
            assert version_id == version.id
            return selection_artifact
        assert version_id == selected_version.id
        assert artifact_type == "normalized_point_cloud_ply"
        assert mime_type == "model/ply"
        assert metadata_json["source"] == "meshlib_point_cloud_selection_to_object"
        assert metadata_json["meshlib_operation"] == "ObjectPoints::cloneRegion"
        assert metadata_json["source_point_indices"] == [1, 3]
        assert metadata_json["point_count"] == 2
        calls.append("register_selected_point_object")
        return object_artifact

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda artifact: tmp_path / "point_cloud.ply")
    monkeypatch.setattr(versions_router.default_sdk, "load_point_cloud_ply", fake_load_point_cloud)
    monkeypatch.setattr(versions_router.default_sdk, "save_point_cloud_ply", fake_save_point_cloud)
    monkeypatch.setattr(
        versions_router.default_sdk,
        "point_cloud_extract_selected_points_as_object",
        fake_extract_selected_points_as_object,
    )
    monkeypatch.setattr(versions_router, "create_version", fake_create_version)
    monkeypatch.setattr(versions_router, "register_file_artifact", fake_register_file_artifact)
    monkeypatch.setattr(versions_router.settings, "TEMP_DIR", tmp_path)

    response = asyncio.run(
        versions_router.commit_selection(
            version.id,
            SelectionCommitRequest(
                operation_label="Point Selection to Object",
                create_object=True,
                selection=InteractiveSelectionPayload(
                    mode="vertices",
                    vertex_ids=[3, 1, 3],
                    metadata={"object_type": "point_cloud"},
                ),
            ),
            db=FakeDb(),
        )
    )

    assert response.artifact_id == selection_artifact.id
    assert response.resolved_counts == {"vertex_ids": 2, "point_ids": 2}
    assert response.selected_object_version_id == selected_version.id
    assert response.selected_object_artifact_id == object_artifact.id
    assert response.selected_object_artifact_url == f"/api/artifacts/{object_artifact.id}"
    assert response.selected_object_artifact_type == "normalized_point_cloud_ply"
    assert response.selected_object_counts == {"point_ids": 2}
    assert calls == [
        "load_point_cloud:point_cloud.ply",
        "point_cloud_extract_selected_points_as_object",
        "create_version",
        "save_point_cloud:selection_object_points.ply",
        "register_selected_point_object",
        "commit",
        "refresh:art_points_selection_json",
        "refresh:art_points_object",
    ]


def test_point_cloud_selection_applies_meshinspector_primary_ctrl_toggle_modifier() -> None:
    cloud = PointCloudDocument(
        np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
            ],
            dtype=np.float64,
        )
    )
    selection = InteractiveSelectionPayload(
        mode="vertices",
        vertex_ids=[1, 2],
        metadata={
            "object_type": "point_cloud",
            "previous_point_ids": [0, 1],
            "modifier_primary_ctrl": True,
        },
    )

    resolved = versions_router._resolve_point_cloud_selection_ids(cloud, selection)

    assert resolved == [0, 2]


def test_mesh_vertex_selection_applies_meshinspector_primary_ctrl_toggle_modifier() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="vertices",
        vertex_ids=[1, 2],
        metadata={
            "previous_vertex_ids": [0, 1],
            "modifier_primary_ctrl": True,
        },
    )

    resolved = versions_router._resolve_selection_vertex_ids(mesh, selection, None)

    assert resolved == [0, 2]


def test_selection_commit_accepts_meshlib_largest_component_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [2, 1, 3], [4, 5, 6]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(mode="faces", metadata={"selector": "largest_component"})

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3]

    too_small = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "largest_component", "min_area_mm2": 1.1},
    )
    assert versions_router._resolve_selection_vertex_ids(mesh, too_small, None) == []


def test_selection_commit_accepts_meshlib_boundary_selectors() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [4.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [4.5, 1.0, 0.0],
                [4.5, 0.5, 1.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray(
            [
                [0, 1, 2],
                [2, 1, 3],
                [4, 6, 5],
                [4, 5, 7],
                [5, 6, 7],
                [6, 4, 7],
            ],
            dtype=np.int64,
        ),
        metadata={},
    )
    boundary_faces = InteractiveSelectionPayload(mode="faces", metadata={"selector": "boundary_faces"})
    boundary_edges = InteractiveSelectionPayload(mode="vertices", metadata={"selector": "boundary_edges"})

    assert _selection_has_content(SelectionCommitRequest(selection=boundary_faces))
    assert versions_router._resolve_selection_vertex_ids(mesh, boundary_faces, None) == [0, 1, 2, 3]
    assert versions_router._resolve_selection_vertex_ids(mesh, boundary_edges, None) == [0, 1, 2, 3]


def test_selection_commit_replays_workbench_lasso_mask() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="lasso",
        metadata={
            "selector": "screen_lasso_faces",
            "view_projection_4x4": np.eye(4, dtype=np.float64).reshape(-1).tolist(),
            "polygon_xy": [[-1.0, -1.0], [-0.05, -1.0], [-0.05, 1.0], [-1.0, 1.0]],
            "include_backfaces": True,
            "visible_only": False,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_replays_workbench_rect_mask() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="rect",
        metadata={
            "selector": "screen_rect_faces",
            "view_projection_4x4": np.eye(4, dtype=np.float64).reshape(-1).tolist(),
            "rect_min_xy": [-1.0, -1.0],
            "rect_max_xy": [-0.05, 1.0],
            "include_backfaces": True,
            "visible_only": False,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_replays_workbench_brush_mask() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="brush",
        metadata={
            "selector": "screen_brush_faces",
            "view_projection_4x4": np.eye(4, dtype=np.float64).reshape(-1).tolist(),
            "brush_path_xy": [[-0.9, -0.7], [-0.9, 0.7]],
            "radius_px": 0.12,
            "include_backfaces": True,
            "visible_only": False,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_replays_workbench_pick_mask() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-0.8, -0.8, 0.0],
                [-0.2, -0.8, 0.0],
                [-0.8, 0.8, 0.0],
                [0.2, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="pick",
        metadata={
            "selector": "pick_face",
            "ray_origin": [-0.5, -0.5, 1.0],
            "ray_direction": [0.0, 0.0, -1.0],
            "epsilon": 1e-8,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_accepts_meshlib_self_intersection_selector() -> None:
    mesh = crossing_triangles()
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "self_intersections", "touch_is_intersection": True},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3, 4, 5]


def test_selection_commit_accepts_meshlib_self_intersection_inside_part_mode() -> None:
    outer = cube(size=4.0)
    inner = cube(size=1.0)
    mesh = MeshDocument(
        vertices=np.vstack([outer.vertices, inner.vertices]),
        faces=np.vstack([outer.faces, inner.faces + outer.vertex_count]),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "self_intersections", "mode": "inside_part"},
    )
    direct = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "inside_part_faces"},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert _selection_has_content(SelectionCommitRequest(selection=direct))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == list(range(8, 16))
    assert versions_router._resolve_selection_vertex_ids(mesh, direct, None) == list(range(8, 16))


def test_selection_commit_accepts_meshinspector_camera_facing_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 5, 4]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "camera_facing_faces", "camera_direction": [0.0, 0.0, -1.0]},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_accepts_meshinspector_not_smooth_faces_selector() -> None:
    mesh = closed_cube_with_flipped_top_triangle()
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "not_smooth_faces", "min_angle_radians": 0.3},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [4, 5, 6, 7]


def test_selection_commit_accepts_meshlib_self_intersection_overlaps_mode() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 5e-6],
                [1.0, 0.0, 5e-6],
                [0.0, 1.0, 5e-6],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 5, 4]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "self_intersections", "mode": "overlaps"},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3, 4, 5]


def test_selection_commit_accepts_meshlib_overlapping_faces_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 5e-6],
                [1.0, 0.0, 5e-6],
                [0.0, 1.0, 5e-6],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 5, 4]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "overlapping_faces"},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3, 4, 5]


def test_selection_commit_accepts_meshlib_degenerate_face_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.5, 0.001, 0.0],
                [0.5, 0.4, 1.0],
                [3.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [3.5, 0.001, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray(
            [
                [0, 1, 2],
                [0, 3, 1],
                [1, 3, 2],
                [2, 3, 0],
                [4, 5, 6],
            ],
            dtype=np.int64,
        ),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "degenerate_faces", "min_aspect_ratio": 100.0, "boundary_only": True},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [4, 5, 6]


def test_selection_commit_accepts_meshlib_short_edge_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 3], [1, 2, 3]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="vertices",
        metadata={"selector": "short_edges", "max_edge_length_mm": 0.05},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1]


def test_selection_commit_accepts_meshlib_area_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [3.0, 0.0, 0.0],
                [5.0, 0.0, 0.0],
                [3.0, 2.0, 0.0],
                [7.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [7.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5], [6, 7, 8]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "area_faces", "area": 1.0, "scalar_type": "absolute", "compare_type": "less"},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_accepts_meshlib_crease_edge_selector() -> None:
    mesh = cube(size=2.0)
    selection = InteractiveSelectionPayload(
        mode="vertices",
        metadata={"selector": "crease_edges", "angle_from_planar_radians": 0.3},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == list(range(8))


def test_selection_commit_accepts_meshlib_overhang_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 2.0],
                [1.0, 0.0, 2.0],
                [0.0, 1.0, 2.0],
                [3.0, 0.0, 2.0],
                [4.0, 0.0, 2.0],
                [3.0, 1.0, 2.0],
                [6.0, 0.0, 0.0],
                [7.0, 0.0, 0.0],
                [6.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 2, 1], [3, 4, 5], [6, 8, 7]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={
            "selector": "overhang_faces",
            "axis": [0.0, 0.0, 1.0],
            "layer_height_mm": 0.5,
            "max_overhang_distance_mm": 0.5,
            "hops": 0,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_accepts_meshlib_outer_layer_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={"selector": "outer_layer_faces", "epsilon": 1e-8},
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2]


def test_selection_commit_accepts_meshlib_graph_cut_region_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [5.0, 5.0, 0.0],
                [0.0, 1.0, 0.0],
                [10.0, 1.0, 0.0],
                [5.0, 5.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 0, 3], [0, 3, 4], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={
            "selector": "graph_cut_region",
            "source_face_ids": [0],
            "sink_face_ids": [3],
            "boundary_weight": 1.0,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3]


def test_selection_commit_accepts_meshinspector_graph_cut_auto_not_region_selector() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [5.0, 5.0, 0.0],
                [0.0, 1.0, 0.0],
                [10.0, 1.0, 0.0],
                [5.0, 5.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 0, 3], [0, 3, 4], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={
            "selector": "graph_cut_region",
            "source_face_ids": [0],
            "uncertainty_distance_mm": 12.0,
            "boundary_weight": 1.0,
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3]


def test_selection_commit_accepts_meshinspector_graph_cut_curvature_preference() -> None:
    mesh = MeshDocument(
        vertices=np.asarray(
            [
                [0.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, -1.0],
                [2.0, 1.0, 0.0],
                [2.0, 2.0, 0.0],
            ],
            dtype=np.float64,
        ),
        faces=np.asarray([[0, 1, 2], [1, 0, 3], [3, 0, 4], [3, 4, 5]], dtype=np.int64),
        metadata={},
    )
    selection = InteractiveSelectionPayload(
        mode="faces",
        metadata={
            "selector": "graph_cut_region",
            "source_face_ids": [0],
            "sink_face_ids": [3],
            "curvature_preference": "concave",
        },
    )

    assert _selection_has_content(SelectionCommitRequest(selection=selection))
    assert versions_router._resolve_selection_vertex_ids(mesh, selection, None) == [0, 1, 2, 3, 4]


def test_selection_commit_detects_regions_when_region_artifact_is_missing(monkeypatch, tmp_path) -> None:
    mesh = cube(size=2.0)
    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    normalized_artifact = ModelArtifactRecord(
        id="art_mesh",
        version_id=version.id,
        artifact_type="normalized_mesh_ply",
        mime_type="model/ply",
        storage_key="ver_ready/mesh.ply",
        size_bytes=123,
        metadata_json={},
    )
    selection_artifact = ModelArtifactRecord(
        id="art_selection",
        version_id=version.id,
        artifact_type="meshlib_selection_json",
        mime_type="application/json",
        storage_key="ver_ready/meshlib_selection.json",
        size_bytes=123,
        metadata_json={},
    )
    calls: list[str] = []

    class FakeDb:
        def get(self, model, key):  # noqa: ANN001, ANN201
            return version if model is ModelVersionRecord and key == version.id else None

        def commit(self) -> None:
            calls.append("commit")

        def refresh(self, artifact) -> None:  # noqa: ANN001
            calls.append(f"refresh:{artifact.id}")

    def fake_get_artifact_by_type(db, version_id, artifact_type):  # noqa: ANN001, ANN202
        if version_id == version.id and artifact_type == "normalized_mesh_ply":
            return normalized_artifact
        if version_id == version.id and artifact_type == "analysis_regions_json":
            return None
        raise AssertionError(f"unexpected artifact lookup: {artifact_type}")

    def fake_detect_ring_regions(mesh_arg, measurement, **kwargs):  # noqa: ANN001, ANN202
        calls.append("detect_ring_regions")
        assert mesh_arg is mesh
        return [
            RegionEntry(
                region_id="inner_band",
                label="Inner Band",
                vertex_indices=np.asarray([0, 1], dtype=np.int64),
                coverage_pct=25.0,
                protected_by_default=False,
                allowed_operations=["scoop", "thicken", "smooth"],
            )
        ]

    monkeypatch.setattr(versions_router, "get_artifact_by_type", fake_get_artifact_by_type)
    monkeypatch.setattr(versions_router, "_materialize_artifact_to_path", lambda artifact: tmp_path / "mesh.ply")
    monkeypatch.setattr(versions_router.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(versions_router.default_sdk, "measure_ring", lambda mesh_arg: object())
    monkeypatch.setattr(versions_router.default_sdk, "detect_ring_regions", fake_detect_ring_regions)
    monkeypatch.setattr(versions_router, "register_file_artifact", lambda *args, **kwargs: selection_artifact)
    monkeypatch.setattr(versions_router.settings, "TEMP_DIR", tmp_path)

    response = asyncio.run(
        versions_router.commit_selection(
            version.id,
            SelectionCommitRequest(selection=InteractiveSelectionPayload(mode="regions", region_ids=["inner_band"])),
            db=FakeDb(),
        )
    )

    assert response.artifact_id == selection_artifact.id
    assert response.selection_counts["region_ids"] == 1
    assert response.resolved_counts == {"vertex_ids": 2}
    assert "detect_ring_regions" in calls


def test_brush_replay_selection_builds_rust_brush_stroke_from_workbench_selection() -> None:
    mesh = cube(size=2.0)
    first_face_vertices = [int(index) for index in mesh.faces[0]]
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "allowed_operations": ["thicken", "smooth"],
                "vertex_indices": [0, 1],
            }
        ]
    }
    stroke = BrushReplayStroke(
        tool_id="thicken_brush",
        selection=InteractiveSelectionPayload(
            mode="brush",
            vertex_ids=[2],
            face_ids=[0],
            region_ids=["inner_band"],
            brush_points_world=[(2.0, 0.0, 0.0)],
        ),
        amount_mm=0.18,
        falloff_mm=1.7,
        iterations=3,
        strength=0.35,
    )

    seeds = _selection_seed_indices(mesh, stroke.selection, region_payload)
    rust_stroke = _brush_replay_stroke_to_sdk(mesh, stroke, region_payload)

    assert {0, 1, 2}.issubset(set(int(index) for index in seeds))
    assert set(first_face_vertices).issubset(set(int(index) for index in seeds))
    assert np.all(seeds >= 0)
    assert np.all(seeds < mesh.vertex_count)
    assert rust_stroke.operation == "thicken"
    assert rust_stroke.amount_mm == pytest.approx(0.18)
    assert rust_stroke.falloff_mm == pytest.approx(1.7)
    assert rust_stroke.iterations == 3
    assert rust_stroke.strength == pytest.approx(0.35)


def test_brush_seed_helpers_delegate_to_rust_sdk(monkeypatch) -> None:
    mesh = cube(size=2.0)
    region_payload = {
        "regions": [
            {
                "region_id": "inner_band",
                "allowed_operations": ["thicken", "smooth"],
                "vertex_indices": [0, 1],
            }
        ]
    }
    selection = InteractiveSelectionPayload(
        mode="brush",
        vertex_ids=[2],
        face_ids=[0],
        region_ids=["inner_band"],
        brush_points_world=[(2.0, 0.0, 0.0)],
    )
    calls: list[tuple[str, np.ndarray | list[tuple[float, float, float]] | int]] = []

    def fake_selection_seed_indices(
        mesh_arg,
        *,
        vertex_ids,
        face_ids,
        region_vertex_indices,
        brush_points_world,
    ):
        assert mesh_arg is mesh
        calls.append(("selection", np.asarray(vertex_ids, dtype=np.int64)))
        calls.append(("region", np.asarray(region_vertex_indices, dtype=np.int64)))
        calls.append(("brush", list(brush_points_world)))
        return np.asarray([1, 2, 5], dtype=np.int64)

    def fake_bounded_seed_indices(mesh_arg, indices, max_count):
        assert mesh_arg is mesh
        calls.append(("bounded", np.asarray(indices, dtype=np.int64)))
        calls.append(("max_count", int(max_count)))
        return np.asarray([2, 5], dtype=np.int64)

    monkeypatch.setattr(operations_service.default_sdk, "selection_seed_indices", fake_selection_seed_indices)
    monkeypatch.setattr(operations_service.default_sdk, "bounded_seed_indices", fake_bounded_seed_indices)

    assert _selection_seed_indices(mesh, selection, region_payload).tolist() == [1, 2, 5]
    assert _bounded_seed_indices(mesh, np.asarray([0, 1, 2, 5]), 2).tolist() == [2, 5]

    assert any(name == "selection" and np.array_equal(value, np.asarray([2], dtype=np.int64)) for name, value in calls)
    assert any(name == "region" and np.array_equal(value, np.asarray([0, 1], dtype=np.int64)) for name, value in calls)
    assert ("brush", [(2.0, 0.0, 0.0)]) in calls
    assert any(name == "bounded" and np.array_equal(value, np.asarray([0, 1, 2, 5], dtype=np.int64)) for name, value in calls)
    assert ("max_count", 2) in calls


def test_bounded_seed_indices_filters_invalid_indices_and_caps_selection() -> None:
    mesh = MeshDocument(
        vertices=np.asarray([[float(index), 0.0, 0.0] for index in range(10)], dtype=np.float64),
        faces=np.empty((0, 3), dtype=np.int64),
    )

    seeds = _bounded_seed_indices(mesh, np.asarray([-1, 0, 0, 1, 2, 3, 4, 5, 99, 6, 7, 8, 9]), 4)

    assert seeds.size == 4
    assert seeds.tolist() == sorted(seeds.tolist())
    assert set(int(index) for index in seeds).issubset(set(range(10)))


def test_brush_replay_detects_regions_when_region_artifact_is_missing(monkeypatch, tmp_path) -> None:
    mesh = cube(size=2.0)
    source_version = ModelVersionRecord(
        id="ver_source",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    new_version = ModelVersionRecord(
        id="ver_brushed",
        model_id=source_version.model_id,
        parent_version_id=source_version.id,
        operation_type="interactive_brush_replay",
        operation_label="Brush",
        status="ready",
    )
    job = JobRecord(
        id="job_brush",
        version_id=source_version.id,
        operation_type="interactive_brush_replay",
        status="queued",
        progress_pct=0,
    )
    calls: list[str] = []

    class FakeDb:
        def add(self, record):  # noqa: ANN001, ANN201
            return None

        def flush(self) -> None:
            return None

    def fake_detect_ring_regions(mesh_arg, measurement, **kwargs):  # noqa: ANN001, ANN202
        calls.append("detect_ring_regions")
        assert mesh_arg is mesh
        return [
            RegionEntry(
                region_id="inner_band",
                label="Inner Band",
                vertex_indices=np.asarray([0, 1], dtype=np.int64),
                coverage_pct=25.0,
                protected_by_default=False,
                allowed_operations=["thicken", "smooth"],
            )
        ]

    def fake_apply_brush_strokes(mesh_arg, strokes):  # noqa: ANN001, ANN202
        calls.append("apply_brush_strokes")
        assert mesh_arg is mesh
        assert len(strokes) == 1
        assert set(int(index) for index in strokes[0].seed_indices) == {0, 1}
        return mesh

    monkeypatch.setattr(operations_service, "_load_normalized_artifact", lambda *args, **kwargs: tmp_path / "mesh.ply")
    monkeypatch.setattr(operations_service, "_load_optional_region_payload", lambda *args, **kwargs: {"regions": []})
    monkeypatch.setattr(operations_service.default_sdk, "load_mesh", lambda path: mesh)
    monkeypatch.setattr(operations_service.default_sdk, "measure_ring", lambda mesh_arg: object())
    monkeypatch.setattr(operations_service.default_sdk, "detect_ring_regions", fake_detect_ring_regions)
    monkeypatch.setattr(operations_service.default_sdk, "apply_brush_strokes", fake_apply_brush_strokes)
    monkeypatch.setattr(operations_service.default_sdk, "save_mesh", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "create_version", lambda *args, **kwargs: new_version)
    monkeypatch.setattr(operations_service, "register_file_artifact", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "upsert_snapshot", lambda *args, **kwargs: None)
    monkeypatch.setattr(operations_service, "_finalize_version", lambda *args, **kwargs: None)

    result = operations_service.run_interactive_brush_replay_operation(
        FakeDb(),
        source_version,
        job,
        tmp_path,
        BrushReplayRequest(
            strokes=[
                BrushReplayStroke(
                    tool_id="thicken_brush",
                    selection=InteractiveSelectionPayload(mode="regions", region_ids=["inner_band"]),
                )
            ]
        ),
    )

    assert result is new_version
    assert calls == ["detect_ring_regions", "apply_brush_strokes"]
