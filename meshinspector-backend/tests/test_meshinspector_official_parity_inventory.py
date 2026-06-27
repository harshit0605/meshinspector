from __future__ import annotations

from pathlib import Path

from api.routers import versions as versions_router
from domain.models import ModelVersionRecord
from domain.schemas import MeshLibOfficialParityFeature, MeshLibWorkbenchManifest


REPO_ROOT = Path(__file__).resolve().parents[2]


def test_official_parity_inventory_is_source_backed_and_exposes_current_status() -> None:
    inventory = versions_router._official_parity_inventory()

    assert inventory
    assert all(isinstance(feature, MeshLibOfficialParityFeature) for feature in inventory)

    feature_ids = {feature.official_feature_id for feature in inventory}
    assert {
        "file-scene-viewer",
        "selection-tools",
        "mesh-healer",
        "mesh-edit-simplify",
        "boolean-collision",
        "offset-shell",
        "features-measurement",
        "compare-report",
        "point-cloud-icp",
        "voxels-ct-sdf",
        "distance-maps-lines-gcode",
        "automation-plugin-api",
    }.issubset(feature_ids)

    statuses = {feature.status for feature in inventory}
    assert {"implemented", "partial"}.issubset(statuses)
    assert statuses <= {"implemented", "partial", "missing"}

    for feature in inventory:
        assert feature.official_sources, feature.official_feature_id
        assert feature.meshlib_source_paths, feature.official_feature_id
        assert feature.validation_gates, feature.official_feature_id


def test_current_workbench_commands_are_covered_by_official_parity_inventory() -> None:
    inventory = versions_router._official_parity_inventory()
    covered_commands = {
        command_id
        for feature in inventory
        for command_id in feature.backend_command_ids
    }
    current_commands = {
        capability["command_id"]
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }

    assert current_commands - covered_commands == set()


def test_automation_plugin_inventory_tracks_rust_backed_workbench_command_advertising() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    automation = inventory["automation-plugin-api"]

    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_architecture.py::test_enabled_rust_owned_workbench_commands_are_advertised_as_rust_backed -q"
        in automation.validation_gates
    )
    assert "Rust-backed command advertising" in automation.notes[0]


def test_implemented_or_partial_inventory_rows_explain_rust_ownership() -> None:
    capability_by_id = {
        capability["command_id"]: capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }

    for feature in versions_router._official_parity_inventory():
        if feature.status == "missing":
            continue

        if feature.non_geometry_reason:
            assert feature.bridge_modules or feature.hosted_tool_ids, feature.official_feature_id
        else:
            assert feature.rust_owner_modules, feature.official_feature_id
        assert feature.validation_gates, feature.official_feature_id
        assert all(
            module.startswith("geometry-rs/")
            for module in feature.rust_owner_modules
        ), feature.official_feature_id
        assert all(
            not module.startswith("geometry-rs/")
            for module in feature.bridge_modules
        ), feature.official_feature_id

        for command_id in feature.backend_command_ids:
            capability = capability_by_id[command_id]
            if capability.get("rust_backed") is True:
                assert capability.get("sdk_operations"), command_id
            else:
                assert feature.non_geometry_reason or feature.status == "partial", command_id


def test_file_scene_inventory_tracks_rust_mesh_ply_import_slice() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    file_scene = inventory["file-scene-viewer"]
    capabilities = {
        capability["command_id"]: capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }

    assert file_scene.status == "partial"
    assert "MeshLib/source/MRCommonPlugins/ViewerButtons/MRRibbonSceneButtons.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRSceneObjectsListDrawer.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRMeshLoadObj.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRPly.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRChangeSceneObjectsOrder.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectMeshHolder.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectLines.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectLinesHolder.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectDistanceMap.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRVoxels/MRObjectVoxels.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRVoxels/MRVoxelsLoad.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRVoxels/MRVoxelsSave.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRFeatureObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRPointObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRLineObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRPlaneObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRSphereObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRCircleObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRCylinderObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRConeObject.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRDistanceMapLoad.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRDistanceMapSave.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectLoad.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectSave.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRZip.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRMesh/miniply.*" in file_scene.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRRibbonMenu.cpp" in file_scene.meshlib_source_paths
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh_obj.rs" in file_scene.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh_ply.rs" in file_scene.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/meshlib_scene.rs" in file_scene.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/scene.rs" in file_scene.rust_owner_modules
    assert "geometry_sdk/core/mesh.py" in file_scene.bridge_modules
    assert "mesh_from_ply" in file_scene.notes[0]
    assert "default_sdk.load_mesh routes .ply uploads" in file_scene.notes[0]
    assert "ASCII and binary little-/big-endian mesh PLY" in file_scene.notes[0]
    assert "vertex normals" in file_scene.notes[0]
    assert "edge elements" in file_scene.notes[0]
    assert "polygon face colors per source face row" in file_scene.notes[0]
    assert "u/v-over-s/t-over-texture_u/texture_v-over-texture_s/texture_t" in file_scene.notes[0]
    assert "polygon texcoord list packing" in file_scene.notes[0]
    assert "TextureFile comments" in file_scene.notes[0]
    assert "miniply-style leading/trailing comment whitespace trimming" in file_scene.notes[0]
    assert "TextureFile image loading" in file_scene.notes[0]
    assert "Linear/Clamp texture settings" in file_scene.notes[0]
    assert "texture artifact URL/metadata handoff" in file_scene.notes[0]
    assert "viewer material texture application" in file_scene.notes[0]
    assert "normalized PLY UV/TextureFile export" in file_scene.notes[0]
    assert "preview GLB TEXCOORD_0 export" in file_scene.notes[0]
    assert "mesh_from_obj" in file_scene.notes[0]
    assert "default_sdk.load_mesh routes .obj uploads" in file_scene.notes[0]
    assert "negative index resolution" in file_scene.notes[0]
    assert "polygon fan triangulation" in file_scene.notes[0]
    assert "mtllib/usemtl material scopes" in file_scene.notes[0]
    assert "Kd diffuse color conversion" in file_scene.notes[0]
    assert "OBJ vt UV import" in file_scene.notes[0]
    assert "map_Kd texture-per-face metadata" in file_scene.notes[0]
    assert "map_Kd PNG/JPEG/TIFF texture image loading" in file_scene.notes[0]
    assert "OBJ texture artifact URL/metadata handoff" in file_scene.notes[0]
    assert "ordered multi-texture artifact manifests" in file_scene.notes[0]
    assert "MeshLib texturePerFace viewer material groups" in file_scene.notes[0]
    assert "native MeshLib texture-array shader sampling" in file_scene.notes[0]
    assert "Rust-backed MeshLib ObjectMeshHolder/ObjectLinesHolder/ObjectPointsHolder-style scene JSON serialization plus serializeObjectTree-style .mru package export/import, ObjectMesh multi-object hierarchy import/export round-trip with object XF transforms, nested object-tree export preservation, Link shared-model reuse, ObjectLines scene object import/export with Polyline.Points and flat Polyline.Lines preservation, ObjectPoints scene object import/export with MeshLib PointsSave/PointsLoad-style point PLY, normals, vertex colors, PointSize, MaxRenderingPoints, and state preservation, Rust-backed scene-object transform, reparent, state, reorder editing, RibbonMenu group/ungroup new-object workflows, and scene-tree ribbon Select all/Unselect all/Show all/Hide all/Show only previous/Show only next/Sort by name/Rename/Remove selected objects controls across ObjectMesh, ObjectLines, ObjectPoints, ObjectDistanceMap, ObjectVoxels, and FeatureObject collections, and artifact registration" in file_scene.notes[0]
    notes_text = "\n".join(file_scene.notes)
    assert "ObjectDistanceMap scene object import/export is Rust-backed" in notes_text
    assert "MeshLib .raw/.mrdistancemap parsing" in notes_text
    assert "ObjectVoxels scene object import/export is Rust-backed" in notes_text
    assert "raw .raw voxel payload import/export" in notes_text
    assert "MeshLib VoxelsLoad::fromGav/MRVoxelsSave::toGav-style Micro CT .gav payload import/export" in notes_text
    assert "OpenVDB .vdb FloatGrid metadata import" in notes_text
    assert "active bbox dimensions, transform voxel size, and level-set class" in notes_text
    assert "uncompressed Tree_float_5_4_3" in notes_text
    assert "ZIP-compressed Tree_float_5_4_3" in notes_text
    assert "Blosc-compressed Tree_float_5_4_3" in notes_text
    assert "zlib and Blosc/LZ4 chunk decompression" in notes_text
    assert "active-mask Tree_float_5_4_3_HalfFloat dense value import" in notes_text
    assert "half-float promotion" in notes_text
    assert "inactive background reconstruction" in notes_text
    assert "MeshLib x-fastest ordering" in notes_text
    assert "min/max stats" in notes_text
    assert "model payload preservation/import/export" in notes_text
    assert "filename-auto dimensions/voxelSize/gridLevelSet parsing" in notes_text
    assert "compact SelectionVoxels bitset import/export" in notes_text
    assert ".vdb scene payloads remain open" not in notes_text
    assert "FeatureObject scene object import/export is Rust-backed" in notes_text
    assert "MeshLib FeatureObject::serializeFields_ fields" in notes_text
    assert "PointObject/LineObject/PlaneObject/SphereObject/CircleObject/CylinderObject/ConeObject" in notes_text
    assert "FeatureObject render payload generation is Rust-backed" in notes_text
    assert "MRRenderFeatureObjects" in notes_text
    assert "MR::makeSphere/subdivideMesh edge-flip SphereObject topology" in notes_text
    assert any(
        "meshlib_scene_feature_object_render_payload" in gate
        for gate in file_scene.validation_gates
    )
    gap_note = next(
        note for note in file_scene.notes if note.startswith("Current runtime embeds")
    )
    assert "native MeshLib texture-array shader sampling" not in gap_note
    assert "exact cross-data-type Sort by name export ordering" not in gap_note
    assert "Blosc-compressed OpenVDB grid buffer decompression" not in gap_note
    assert "direct OpenVDB .vdb FloatGrid dense-payload dual meshing" in gap_note
    assert "MeshLib relaxDisorientedTriangles-style closed-surface ray-count face relaxation" in gap_note
    assert "disoriented-triangle relaxation remain" not in gap_note
    assert "exact sparse OpenVDB VolumeToMesh topology" in gap_note
    assert "Blosc/ZIP-compressed OpenVDB grid buffer decompression" not in gap_note
    assert "compressed/half-float OpenVDB grid buffer decoding" not in gap_note
    assert "dense OpenVDB grid value decoding" not in gap_note
    assert "scene-tree new-object workflows" not in gap_note
    assert "ObjectVoxels .vdb scene payloads" not in gap_note
    assert "ObjectVoxels .vdb/.gav scene payloads" not in gap_note
    assert "non-empty voxel-selection bitset persistence" not in gap_note
    assert "FeatureObject" not in gap_note
    assert "ObjectDistanceMap" not in gap_note
    assert "broader all-data-type scene-tree management beyond the current ObjectMesh scene ribbon controls" not in gap_note
    assert "ObjectLines scene object import/export" not in gap_note
    assert "ObjectPoints scene object import/export" not in gap_note
    assert "transform editing" not in gap_note
    assert "full native multi-object scene hierarchy import" not in gap_note
    assert "shared-model reuse" not in gap_note
    assert "multi-object .mru round-trip export" not in gap_note
    assert ".mru package import" not in gap_note
    assert "zipped .mru IO" not in gap_note
    assert "multi-texture per-face shader material sampling" not in gap_note
    assert "OBJ vt/UV import" not in gap_note
    assert (
        "cargo test -p zennah-geometry-core mesh_ply_import_prefers_meshlib_uv_short_names_over_texture_names"
        in file_scene.validation_gates
    )
    assert "cargo test -p zennah-geometry-core mesh_ply_import_reads_binary" in file_scene.validation_gates
    assert (
        "cargo test -p zennah-geometry-core mesh_ply_import_packs_polygon_texcoord_lists_like_meshlib"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_meshlib_object_mesh_scene_payload_matches_object_mesh_holder_fields tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_meshlib_object_mesh_scene_json_artifact tests/test_geometry_sdk_operation_contracts.py::test_meshlib_object_mesh_mru_scene_matches_serialize_object_tree_layout tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_meshlib_mru_scene_artifact tests/test_geometry_sdk_operation_contracts.py::test_load_mesh_routes_mru_scene_through_rust_deserialize_object_tree tests/test_geometry_sdk_operation_contracts.py::test_load_mesh_routes_multi_object_mru_scene_hierarchy_through_rust tests/test_geometry_sdk_operation_contracts.py::test_load_mesh_preserves_mru_shared_model_links_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_round_trips_multi_object_hierarchy_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_lines_type_management_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_round_trips_shared_model_links_through_rust tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_nested_object_tree_through_rust tests/test_geometry_sdk_operation_contracts.py::test_reparent_mru_scene_object_updates_tree_metadata_and_round_trips_through_rust tests/test_geometry_sdk_operation_contracts.py::test_set_mru_scene_object_state_updates_visibility_and_lock_flags_through_rust tests/test_geometry_sdk_operation_contracts.py::test_reorder_mru_scene_children_updates_export_order_through_rust tests/test_geometry_sdk_operation_contracts.py::test_transform_mru_scene_object_updates_xf_and_round_trips_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_transform_scene_object_updates_world_vertices_from_object_xf"
        in file_scene.validation_gates
    )
    assert "cargo test -p zennah-geometry-core meshlib_scene_ribbon" in file_scene.validation_gates
    assert (
        "cargo test -p zennah-geometry-core meshlib_scene_tree_ribbon_actions_cover_imported_data_object_types"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_scene_tree_sort_by_name_exports_mixed_data_children_in_meshlib_order"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_scene_tree_group_and_ungroup_match_official_new_object_workflow"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_apply_mru_scene_ribbon_actions_and_rename_route_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_mru_scene_tree_ribbon_actions_cover_imported_data_collections_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_group_and_ungroup_mru_scene_objects_route_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_multi_object_mru_scene_preserves_nested_object_children"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_lines_nodes"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_points_nodes"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_voxels_nodes"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_voxels_gav_nodes"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_object_voxels_vdb_payloads"
        in file_scene.validation_gates
    )
    assert "cargo test -p zennah-geometry-core object_voxels_vdb" in file_scene.validation_gates
    assert (
        "cargo test -p zennah-geometry-core meshlib_mru_scene_round_trips_feature_object_nodes"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_points_type_management_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_voxels_type_management_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_voxels_gav_payloads_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_object_voxels_vdb_payloads_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_load_meshlib_mru_scene_imports_half_float_active_mask_vdb_values_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_load_meshlib_mru_scene_imports_zip_compressed_vdb_values_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_load_meshlib_mru_scene_imports_blosc_compressed_vdb_values_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_save_meshlib_mru_scene_preserves_feature_object_type_management_through_rust -q"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_reparent_scene_object_updates_hierarchy_paths_like_add_child"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_set_scene_object_state_serializes_visibility_and_lock_flags"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core meshlib_reorder_scene_children_matches_change_scene_objects_order"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_ply_import_keeps_polygon_face_colors_per_meshlib_source_face_row"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_ply_import_exposes_meshlib_vertex_normals_and_edges"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_ply_import_loads_first_existing_texture_like_meshlib_texturefile"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_ply_import_trims_meshlib_texturefile_comment_trailing_spaces"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_obj_import_triangulates_meshlib_negative_index_quad"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_obj_import_loads_meshlib_mtl_diffuse_texture_metadata"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_exposes_meshlib_uv_and_color_metadata -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_exposes_binary_meshlib_metadata -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_packs_polygon_texcoord_lists_like_meshlib -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_ply_import_keeps_polygon_face_colors_per_meshlib_source_face_row -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_ply_uploads_through_rust_meshlib_parser -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_obj_import_triangulates_meshlib_negative_index_quad tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_obj_uploads_through_rust_meshlib_parser -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_obj_import_loads_meshlib_mtl_diffuse_texture_metadata tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_obj_mtl_metadata_through_rust_parser -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_exposes_meshlib_ply_normals_and_edges -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_loads_first_existing_texture_like_meshlib_texturefile -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_trims_meshlib_texturefile_comment_trailing_spaces -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_and_workbench_manifests_expose_meshlib_texture_artifact tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_first_rust_loaded_meshlib_texture_artifact -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_and_workbench_manifests_expose_ordered_meshlib_texture_artifacts tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_all_obj_map_kd_textures_with_meshlib_texture_per_face -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_ingest_registers_obj_map_kd_texture_artifact_with_meshlib_obj_source -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_engine_applies_meshlib_texture_artifact_to_mesh_materials -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_engine_applies_meshlib_texture_per_face_artifacts_to_material_groups -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_viewer_engine_uses_meshlib_texture_array_shader_before_material_group_fallback -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_save_mesh_preserves_meshlib_vertex_uvs_through_ply_and_glb_preview tests/test_geometry_sdk_core.py::test_save_mesh_preserves_meshlib_tri_corner_uvs_in_ply_and_flattens_preview_uvs -q"
        in file_scene.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core mesh_obj_import_loads_meshlib_map_kd_texture_image"
        in file_scene.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core mesh_obj_import_preserves_meshlib_vt_uvs_for_textured_faces"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_loads_meshlib_obj_map_kd_texture_image -q"
        in file_scene.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_default_sdk_load_mesh_routes_obj_vt_uvs_into_glb_preview -q"
        in file_scene.validation_gates
    )
    assert "mesh_from_ply" in capabilities["upload-new"]["notes"][1]
    assert "polygon face colors per source face row" in capabilities["upload-new"]["notes"][1]
    assert "tri-corner polygon texcoord list packing" in capabilities["upload-new"]["notes"][1]
    assert "miniply comment trimming" in capabilities["upload-new"]["notes"][1]
    assert "TextureFile image loading" in capabilities["upload-new"]["notes"][1]
    assert "texture artifact handoff" in capabilities["upload-new"]["notes"][1]
    assert "normalized PLY UV/TextureFile export" in capabilities["upload-new"]["notes"][1]
    assert "preview GLB TEXCOORD_0 export" in capabilities["upload-new"]["notes"][1]
    assert "PLY uploads now route through default_sdk.load_mesh" in capabilities["upload-new"]["notes"][1]
    assert "OBJ uploads now route through Rust mesh_from_obj" in capabilities["upload-new"]["notes"][1]
    assert "negative index resolution" in capabilities["upload-new"]["notes"][1]
    assert "OBJ vt UV import" in capabilities["upload-new"]["notes"][1]
    assert "map_Kd texture-per-face metadata" in capabilities["upload-new"]["notes"][1]
    assert "map_Kd texture image loading" in capabilities["upload-new"]["notes"][1]
    assert "OBJ texture artifact provenance handoff" in capabilities["upload-new"]["notes"][1]
    assert "ordered multi-texture artifact manifests" in capabilities["upload-new"]["notes"][1]
    assert "MeshLib texturePerFace viewer material groups" in capabilities["upload-new"]["notes"][1]
    assert "viewer material texture application" in capabilities["upload-new"]["notes"][1]


def test_voxels_inventory_tracks_dense_dual_contouring_slice() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    voxels = inventory["voxels-ct-sdf"]
    notes_text = "\n".join(voxels.notes)
    capability = next(
        capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability["command_id"] == "voxel-to-mesh-dual"
    )

    assert "voxel-to-mesh-dual" in voxels.backend_command_ids
    assert "voxel-to-mesh-dual exposes a Rust-backed dense dual-contouring slice" in notes_text
    assert "openvdb::tools::VolumeToMesh" in notes_text
    assert "maxVertices/maxFaces limit errors" in notes_text
    assert "dense planar adaptivity coalescing" in notes_text
    assert "OpenVDB active bbox origin preservation" in notes_text
    assert "distinct OpenVDB topology and value-buffer masks" in notes_text
    assert (
        "tight sparse active-bbox, active-window boundary, and full-leaf-span sparse active-mask background halo padding"
        in notes_text
    )
    assert "MeshLib relaxDisorientedTriangles-style closed-surface ray-count face relaxation" in notes_text
    assert "exact sparse Dual Marching Cubes/OpenVDB VolumeToMesh topology" in notes_text
    assert "cargo test -p zennah-geometry-core voxel_to_mesh_dual" in voxels.validation_gates
    assert (
        "cargo test -p zennah-geometry-core voxel_to_mesh_dual_values_with_settings_enforces_meshlib_limits"
        in voxels.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core voxel_to_mesh_dual_values_with_settings_applies_meshlib_planar_adaptivity"
        in voxels.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core relax_disoriented_mesh_triangles_flips_meshlib_ray_invalid_faces"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_extracts_meshlib_dense_dual_plane_slice -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_exposes_meshlib_face_and_vertex_limits -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_exposes_meshlib_adaptivity_setting -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_exposes_meshlib_relax_disoriented_triangles_setting -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_preserves_openvdb_active_bbox_origin_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_accepts_distinct_openvdb_topology_and_buffer_masks_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_pads_tight_openvdb_active_bbox_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_pads_sparse_openvdb_active_window_boundary_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_voxel.py::test_voxel_to_mesh_dual_vdb_payload_pads_full_leaf_span_sparse_openvdb_mask_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_returns_rust_meshlib_mesh_payload -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_enforces_meshlib_limits_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_exposes_meshlib_adaptivity_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_exposes_meshlib_relax_disoriented_triangles_through_rust -q"
        in voxels.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_voxel_to_mesh_dual_endpoint_enforces_openvdb_payload_limits_through_rust -q"
        in voxels.validation_gates
    )
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["voxel_to_mesh_dual"]
    assert "dense dual-contouring" in capability["notes"][0]
    assert "maxVertices/maxFaces limit errors" in capability["notes"][0]
    assert "dense planar adaptivity coalescing" in capability["notes"][0]
    assert "OpenVDB active bbox origin preservation" in capability["notes"][0]
    assert "distinct OpenVDB topology and value-buffer masks" in capability["notes"][0]
    assert (
        "tight sparse active-bbox, active-window boundary, and full-leaf-span sparse active-mask background halo padding"
        in capability["notes"][0]
    )
    assert "MeshLib relaxDisorientedTriangles-style closed-surface ray-count face relaxation" in capability["notes"][0]


def test_workbench_manifest_includes_official_parity_inventory() -> None:
    fields = MeshLibWorkbenchManifest.model_fields
    assert "official_parity_inventory" in fields

    version = ModelVersionRecord(
        id="ver_ready",
        model_id="mdl_ready",
        parent_version_id=None,
        operation_type="ingest",
        operation_label="Ready",
        status="ready",
    )
    inventory = versions_router._official_parity_inventory()
    manifest = MeshLibWorkbenchManifest(
        version_id=version.id,
        entry_html_url="/meshlib-workbench/index.html",
        runtime_asset_base_url="/meshlib-workbench/runtime",
        commit_endpoint_url=f"/api/versions/{version.id}/interactive-commit",
        selection_endpoint_url=f"/api/versions/{version.id}/selection-commit",
        brush_endpoint_url=f"/api/versions/{version.id}/brush-replay",
        measurement_endpoint_url=f"/api/versions/{version.id}/measure-inspect",
        mesh_cut_measure_topology_endpoint_url=f"/api/versions/{version.id}/mesh-cut-measure/topology",
        command_capabilities=versions_router._workbench_command_capabilities(version),
        official_parity_inventory=inventory,
    )

    assert manifest.official_parity_inventory == inventory


def test_official_parity_inventory_doc_matches_backend_inventory() -> None:
    doc_path = REPO_ROOT / "docs" / "MeshInspector Official Parity Inventory.md"
    assert doc_path.exists()
    doc = doc_path.read_text(encoding="utf-8")

    for feature in versions_router._official_parity_inventory():
        assert f"`{feature.official_feature_id}`" in doc


def test_mesh_edit_simplify_tracks_decimate_not_flippable_remap_slice() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    mesh_edit = inventory["mesh-edit-simplify"]
    capability = next(
        capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
        if capability["command_id"] == "decimate-mesh"
    )
    doc = (REPO_ROOT / "docs" / "MeshInspector Official Parity Inventory.md").read_text(
        encoding="utf-8"
    )

    assert "notFlippable dynamic remapping with remapped_not_flippable_edges metadata" in mesh_edit.notes[0]
    assert "edgesToCollapse collapse subset and remapping metadata" in mesh_edit.notes[0]
    assert "criticalTriAspectRatio aspect-relaxation guard" in mesh_edit.notes[0]
    assert "tinyEdgeLength endpoint aspect-bypass guard" in mesh_edit.notes[0]
    assert "maxAngleChange local Delone flip guard" in mesh_edit.notes[0]
    assert (
        "twinMap symmetric validation plus paired same-position collapse, paired maxAngleChange Delone flips, and collapse/flip/pack remapping metadata"
        in mesh_edit.notes[0]
    )
    assert "MeshLib preCollapseVertAttribute-style vertex_uvs and vertex_colors interpolation" in mesh_edit.notes[0]
    assert "preCollapse callbacks" in mesh_edit.notes[1]
    assert "true threaded execution" in mesh_edit.notes[1]
    assert "paired twin flip coupling" not in mesh_edit.notes[1]
    assert "MeshLib/source/MRMesh/MRMeshDecimateCallbacks.*" in mesh_edit.meshlib_source_paths
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh_edit/decimate/helpers.rs" in mesh_edit.rust_owner_modules
    assert "geometry_sdk/accelerators/_rust_mesh_edit.py" in mesh_edit.bridge_modules
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_interpolates_vertex_uvs_with_meshlib_pre_collapse_callback"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_interpolates_vertex_colors_with_meshlib_pre_collapse_truncation"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_edges_to_collapse_subset_and_remaps_it"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_empty_meshlib_edges_to_collapse_subset"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_critical_triangle_aspect_ratio_relaxation"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_tiny_edge_length_aspect_bypass"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_honors_meshlib_max_angle_change_delone_flip"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_flips_meshlib_twin_edge_with_max_angle_change"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_remaps_meshlib_twin_map_after_collapse"
        in mesh_edit.validation_gates
    )
    assert (
        "cargo test --manifest-path geometry-rs/Cargo.toml -p zennah-geometry-core decimate_mesh_collapses_meshlib_twin_edge_with_same_position"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_interpolates_vertex_uvs_with_meshlib_pre_collapse_callback -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_interpolates_vertex_colors_with_meshlib_pre_collapse_truncation -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_edges_to_collapse_subset_and_remaps_it -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_empty_meshlib_edges_to_collapse_subset -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_critical_triangle_aspect_ratio_relaxation -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_tiny_edge_length_aspect_bypass -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_honors_meshlib_max_angle_change_delone_flip -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_flips_meshlib_twin_edge_with_max_angle_change -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_remaps_meshlib_twin_map_after_collapse -q"
        in mesh_edit.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_mesh_edit.py::test_decimate_mesh_collapses_meshlib_twin_edge_with_same_position -q"
        in mesh_edit.validation_gates
    )
    assert "notFlippable dynamic remapping with remapped_not_flippable_edges metadata" in capability["notes"][0]
    assert "edgesToCollapse collapse subset and remapping metadata" in capability["notes"][0]
    assert "criticalTriAspectRatio aspect-relaxation guard" in capability["notes"][0]
    assert "tinyEdgeLength endpoint aspect-bypass guard" in capability["notes"][0]
    assert "maxAngleChange local Delone flip guard" in capability["notes"][0]
    assert (
        "twinMap symmetric validation plus paired same-position collapse, paired maxAngleChange Delone flips, and collapse/flip/pack remapping metadata"
        in capability["notes"][0]
    )
    assert "MeshLib preCollapseVertAttribute-style vertex_uvs and vertex_colors interpolation" in capability["notes"][0]
    assert "arbitrary preCollapse callbacks" in capability["notes"][0]
    assert "true threaded execution" in capability["notes"][0]
    assert "paired twin flip coupling" not in capability["notes"][0]
    assert "`notFlippable` dynamic remapping with `remapped_not_flippable_edges` metadata" in doc
    assert "`edgesToCollapse` collapse subset and remapping metadata" in doc
    assert "`criticalTriAspectRatio` aspect-relaxation guard" in doc
    assert "`tinyEdgeLength` endpoint aspect-bypass guard" in doc
    assert "`maxAngleChange` local Delone flip guard" in doc
    assert (
        "MeshLib `twinMap` symmetric validation plus paired same-position collapse, paired `maxAngleChange` Delone flips, and collapse/flip/pack remapping metadata"
        in doc
    )
    assert "MeshLib `preCollapseVertAttribute`-style `vertex_uvs` and `vertex_colors` interpolation" in doc
    assert "notFlippable dynamic remapping/twinMap" not in " ".join(mesh_edit.notes)
    assert "notFlippable dynamic remapping/twinMap" not in capability["notes"][0]
    assert "twinMap/preCollapse" not in capability["notes"][0]
    assert "MeshLib twinMap hooks" not in capability["notes"][0]
    assert "paired twin collapse/flip" not in capability["notes"][0]
    assert "callbacks/color attributes" not in " ".join(mesh_edit.notes)
    assert "callbacks/color attributes" not in capability["notes"][0]


def test_measurement_inventory_tracks_meshlib_cylinder_refinement_parity() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    measurement = inventory["features-measurement"]

    assert "MeshLib/source/MRMesh/MRConeApproximator.*" in measurement.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRConeObject.*" in measurement.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRCylinderApproximator.*" in measurement.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MROneMeshContours.*" in measurement.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRContoursCut.*" in measurement.meshlib_source_paths
    assert "geometry-rs/crates/zennah-geometry-core/src/features/cone_approx.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/features/cylinder_approx.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/fast_marching.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_extreme.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_quadrangle.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_descent.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/geodesic_strip.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/surface_path.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh/triangle_strip.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/fast_marching.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic_descent.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/geodesic_strip.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/surface_path.rs" in measurement.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh/triangle_strip.rs" in measurement.rust_owner_modules
    assert "geometry_sdk/accelerators/_rust_fast_marching.py" in measurement.bridge_modules
    assert "cargo test -p zennah-geometry-core cone_" in measurement.validation_gates
    assert (
        "cargo test -p zennah-geometry-core mesh_closest_surface_path_targets"
        in measurement.validation_gates
    )
    assert "cargo test -p zennah-geometry-core mesh_geodesic_extreme_edges" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_geodesic_quadrangle_path" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_cut_measure_contours" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_steepest_descent_triangle_step" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_steepest_descent_edge_step" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_steepest_descent_vertex_step" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_steepest_descent_path" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_fast_marching_surface_path" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_fast_marching_surface_path_tri_points" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_surface_path_tri_points" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_planar_triangle_strip_path" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_triangle_strip_unfolded_path" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_surface_edge_point_path" in measurement.validation_gates
    assert "cargo test -p zennah-geometry-core mesh_geodesic_edge_point_path" in measurement.validation_gates
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_quadrangle_path_matches_meshlib_reduce_path_crossing_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_cut_measure_contours_matches_meshlib_onemesh_contour_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_planar_triangle_strip_path_matches_meshlib_funnel_crossing_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_triangle_strip_unfolded_path_matches_meshlib_unfolder_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_edge_point_path_matches_meshlib_surface_path_length_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_geodesic_edge_point_path_matches_meshlib_geodesic_path_length_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_triangle_step_matches_meshlib_triangle_exit_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_edge_step_matches_meshlib_edgepoint_vertex_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_vertex_step_matches_meshlib_vertid_triangle_exit_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_steepest_descent_path_matches_meshlib_descent_path_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_fast_marching_surface_path_matches_meshlib_vertex_endpoint_contract -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_fast_marching_surface_path_tri_points_stops_in_end_triangle_like_meshlib -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_measure_inspect_endpoint_returns_rust_fast_marching_mesh_tri_point_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_reduces_single_crossing_like_meshlib_compute_surface_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_reduces_unfolded_triangle_strip_like_meshlib_compute_surface_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_avoids_adjacent_face_vertex_like_meshlib_reduce_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_avoids_non_adjacent_vertex_fan_like_meshlib_reduce_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_removes_repeated_edge_vertex_detour_like_meshlib_reduce_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_removes_duplicate_nonvertex_location_like_meshlib_reduce_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_removes_same_triangle_nonvertex_detour_like_meshlib_reduce_path -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_mesh_surface_path_tri_points_collapses_repeated_location_strip_vertex_run_like_meshlib_reduce_path -q"
        in measurement.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core cone_approximation_matches_meshlib_partial_arc_fixture"
        in measurement.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core cylinder_approximation_matches_meshlib_partial_arc_fixture"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_refine_feature_primitives_uses_meshlib_cylinder_approximation -q"
        in measurement.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_feature_pair_measurements_match_meshlib_parallel_cylinder_center_distance_fallback -q"
        in measurement.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core feature_center_distance_matches_meshlib_parallel_cylinder_fallback"
        in measurement.validation_gates
    )
    assert "ConeObject descriptors" in measurement.notes[0]
    assert "centerDistance including ConeSegment mostly-parallel cylinder fallback" in measurement.notes[0]
    assert "ConeObject projectPoint-style cone projection helpers" in measurement.notes[0]
    assert "Cylinder3Approximation-style cylinder refinement" in measurement.notes[0]
    assert "Cone3Approximation Levenberg-Marquardt-style cone refinement" in measurement.notes[0]
    assert "shortestPathInQuadrangle/reducePath-style two-triangle surface path refinement" in measurement.notes[0]
    assert "convertSurfacePathsToMeshContours / cutMesh-style OneMeshContour cut-input payloads" in measurement.notes[0]
    assert "edge-aligned MR::cutMesh seam topology mutation" in measurement.notes[0]
    assert "child-version normalized_mesh_ply export" in measurement.notes[0]
    assert (
        "computeSurfacePath/reducePath-style single-crossing, unfolded triangle-strip, adjacent-face plus non-adjacent vertex-fan avoidance, repeated-edge vertex-detour simplification, duplicate non-vertex location removal, same-triangle non-vertex detour pruning, repeated-location strip same-vertex run collapse, topology-changing return-count and max-iteration gating semantics, and unfolded-strip vertex-run collapse for MeshTriPoint surface paths"
        in measurement.notes[0]
    )
    assert "PathInPlanarTriangleStrip/reducePath-style unfolded strip funnel crossing" in measurement.notes[0]
    assert "TriangleStripUnfolder/reducePath-style mesh triangle-strip unfolding payloads" in measurement.notes[0]
    assert "surfacePathLength/surfacePathToContour3f edge-point contour payloads" in measurement.notes[0]
    assert "geodesicPathLength/geodesicPathToContour3f endpoint contour payloads" in measurement.notes[0]
    assert "closest-target mapping" in measurement.notes[0]
    assert "findExtremeEdges-style ridge/gorge" in measurement.notes[0]
    assert "broader cone and cylinder refinement oracle coverage" in measurement.notes[1]
    assert "remaining broad repeated-location topology simplification" in measurement.notes[1]
    assert "repeated-location strip vertex-run cases" in measurement.notes[1]
    assert "same-triangle non-vertex" in measurement.notes[1]
    assert "arbitrary-contour Mesh Cut & Measure topology mutation" in measurement.notes[1]
    assert "non-edge path child-version exports beyond the current edge-aligned Rust seam subset" in measurement.notes[1]


def test_selection_inventory_tracks_meshlib_component_expansion_parity() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    capabilities = {
        capability["command_id"]: capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }
    selection = inventory["selection-tools"]

    assert "MeshLib/source/MRMesh/MRMeshComponents.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRRibbonMenu.cpp" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRObjectMesh.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRMesh.cpp" in selection.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRSelectScreenLasso.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRMeshFixer.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRMeshMath.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRFilterCreaseEdges.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRMeshOverhangs.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRMeshDoubleLayer.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MRFillContourByGraphCut.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MREdgeMetric.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRSelectCurvaturePreference.*" in selection.meshlib_source_paths
    assert "MeshLib/source/MRMesh/MROverlappingTris.*" in selection.meshlib_source_paths
    assert "geometry-rs/crates/zennah-geometry-core/src/mesh.rs" in selection.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/repair_degeneracy.rs" in selection.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/repair_smoothness.rs" in selection.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/mesh.rs" in selection.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/repair_degeneracy.rs" in selection.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/repair_smoothness.rs" in selection.rust_owner_modules
    assert "geometry_sdk/accelerators/_rust_mesh.py" in selection.bridge_modules
    assert "geometry_sdk/accelerators/_rust_repair.py" in selection.bridge_modules
    assert "geometry_sdk/accelerators/_rust_smoothness.py" in selection.bridge_modules
    assert "geometry_sdk/core/mesh.py" in selection.bridge_modules
    assert "MeshComponents::getComponents-style shared-edge face-component expansion" in selection.notes[0]
    assert "selection.metadata.expand_to_components" in selection.notes[0]
    assert "MeshComponents::getLargestComponent-style largest-area component selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=largest_component" in selection.notes[0]
    assert "MeshTopology::findBdFaces" in selection.notes[0]
    assert "boundary face/edge selector metadata" in selection.notes[0]
    assert "MRSelectScreenLasso-style screen polygon selection" in selection.notes[0]
    assert "MRSelectScreenLasso-style screen rectangle selection" in selection.notes[0]
    assert "selection.metadata.selector=screen_rect_faces" in selection.notes[0]
    assert "MRSelectScreenLasso-style screen brush selection" in selection.notes[0]
    assert "selection.metadata.selector=screen_brush_faces" in selection.notes[0]
    assert "first_ray_hit-style primitive Pick selector" in selection.notes[0]
    assert "selection.metadata.selector=pick_face" in selection.notes[0]
    assert "Select Camera-Facing selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=camera_facing_faces" in selection.notes[0]
    assert "SelfIntersections::getFaces strict face selector metadata" in selection.notes[0]
    assert "FastWindingNumber::calcSelfIntersections-style Inside Part selector" in selection.notes[0]
    assert "selection.metadata.selector=inside_part_faces" in selection.notes[0]
    assert "findDegenerateFaces-style aspect-ratio selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=degenerate_faces" in selection.notes[0]
    assert "findShortEdges-style edge-length selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=short_edges" in selection.notes[0]
    assert "Mesh::area-style Select by Area selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=area_faces" in selection.notes[0]
    assert "findCreaseEdges-style Select Creases by Angle selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=crease_edges" in selection.notes[0]
    assert "findOverhangs-style Select Overhangs selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=overhang_faces" in selection.notes[0]
    assert "findOuterLayer-style Select Outer Layer selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=outer_layer_faces" in selection.notes[0]
    assert "findNotSmoothFaces-style Select Not Smooth Triangles selector metadata" in selection.notes[0]
    assert "selection.metadata.selector=not_smooth_faces" in selection.notes[0]
    assert "segmentByGraphCut-style seeded Select Region selector metadata" in selection.notes[0]
    assert "automatic not-region workflow via uncertainty-distance sink seeding" in selection.notes[0]
    assert "edgeCurvMetric-style Curvature Preference metadata" in selection.notes[0]
    assert "selection.metadata.curvature_preference" in selection.notes[0]
    assert "selection.metadata.selector=graph_cut_region" in selection.notes[0]
    assert "findOverlappingTris-style Select Self-Intersections Overlaps mode" in selection.notes[0]
    assert "selection.metadata.selector=overlapping_faces" in selection.notes[0]
    assert (
        "MeshInspector primary-control face-selection, mesh vertex-selection, point-cloud point-selection, and scene-tree object selection toggle semantics"
        in selection.notes[0]
    )
    assert "selection.metadata.modifier_primary_ctrl" in selection.notes[0]
    assert "meshlib_select_scene_objects" in selection.notes[0]
    assert "Mesh::cloneRegion-style mesh face Selection to Object" in selection.notes[0]
    assert "Point-cloud Selection to Object" not in selection.notes[1]
    assert "Scene-tree modifier-key selection semantics" not in selection.notes[1]
    assert "mesh vertex-only" not in selection.notes[1]
    assert "point-cloud" not in selection.notes[1]
    assert "selection-to-object commit workflows" not in selection.notes[1]
    assert "point-cloud primitive Pick selection" not in selection.notes[1]
    assert "primitive paint selectors" not in selection.notes[1]
    assert "primitive pick/paint selectors" not in selection.notes[1]
    assert "primitive pick/paint/rectangle selectors" not in selection.notes[1]
    assert "camera-facing selectors" not in selection.notes[1]
    assert "not-smooth triangle selection exposure" not in selection.notes[1]
    assert "outer-layer selectors" not in selection.notes[1]
    assert "Inside Part self-intersection selector mode" not in selection.notes[1]
    assert "Overlaps self-intersection selector modes" not in selection.notes[1]
    assert "automatic not-region graph-cut workflow" not in selection.notes[1]
    assert "broader curvature segmentation beyond Select Region Curvature Preference" in selection.notes[1]
    assert "cargo test -p zennah-geometry-core expand_face_selection_to_components" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_largest_component_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_boundary" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_faces_by_screen_polygon" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_faces_by_screen_rect" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_faces_by_screen_brush" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core ray_hits_cube_front_face" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_camera_facing_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_inside_part_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_degenerate_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_short_edges" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_faces_by_area" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_overhang_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_outer_layer_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core select_not_smooth_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core graph_cut_select_region" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core graph_cut_select_region_auto_not_region" in selection.validation_gates
    assert (
        "cargo test -p zennah-geometry-core graph_cut_select_region_uses_meshlib_curvature_preference_metric"
        in selection.validation_gates
    )
    assert "cargo test -p zennah-geometry-core select_overlapping_faces" in selection.validation_gates
    assert "cargo test -p zennah-geometry-core extract_selected_faces_as_mesh" in selection.validation_gates
    assert (
        "cargo test -p zennah-geometry-core apply_meshlib_selection_modifier_matches_primary_ctrl_toggle_contract"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_crease_edges_matches_meshlib_find_crease_edges_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_expand_face_selection_to_components_matches_meshlib_component_selection -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_largest_component_faces_matches_meshlib_surface_area_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_boundary_faces_and_edges_match_meshlib_boundary_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_screen_polygon_matches_meshlib_lasso_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_screen_rect_matches_meshlib_rect_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_screen_brush_matches_meshlib_near_polygon_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_face_by_ray_matches_meshlib_pick_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_camera_facing_faces_matches_meshinspector_view_direction_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_inside_part_faces_matches_meshlib_winding_self_intersection_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_expands_selected_faces_to_meshlib_components -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_extract_selected_faces_as_mesh_matches_meshlib_clone_region_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_extract_selected_faces_as_mesh_remaps_meshlib_clone_region_visual_attributes -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_apply_meshlib_selection_modifier_matches_primary_ctrl_toggle_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_applies_meshinspector_primary_ctrl_toggle_modifier -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_mesh_vertex_selection_applies_meshinspector_primary_ctrl_toggle_modifier -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_point_cloud_selection_applies_meshinspector_primary_ctrl_toggle_modifier -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_can_create_meshlib_selection_to_object_version -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_largest_component_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_boundary_selectors -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_lasso_mask -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_rect_mask -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_brush_mask -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_replays_workbench_pick_mask -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_camera_facing_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_spatial.py::test_triangle_intersection_detects_crossing_faces -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_inside_part_mode -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_degenerate_faces_matches_meshlib_aspect_ratio_and_boundary_filter -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_short_edges_matches_meshlib_critical_length_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_faces_by_area_matches_meshlib_area_threshold_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_degenerate_face_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_short_edge_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_area_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_crease_edge_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_overhang_faces_matches_meshlib_layer_basement_and_normal_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_overhang_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_outer_layer_faces_matches_meshlib_double_layer_seed_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_not_smooth_faces_matches_meshlib_neighbor_angle_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_outer_layer_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_not_smooth_faces_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_graph_cut_select_region_matches_meshlib_source_sink_edge_length_cut_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_graph_cut_region_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_graph_cut_select_region_auto_not_region_matches_meshinspector_uncertainty_workflow -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_graph_cut_auto_not_region_selector -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_graph_cut_select_region_matches_meshinspector_curvature_preference -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshinspector_graph_cut_curvature_preference -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_core.py::test_select_overlapping_faces_matches_meshlib_opposite_close_triangle_contract -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_self_intersection_overlaps_mode -q"
        in selection.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_accepts_meshlib_overlapping_faces_selector -q"
        in selection.validation_gates
    )
    assert "expand_face_selection_to_components" in capabilities["regions"]["sdk_operations"]
    assert "select_largest_component_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_camera_facing_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_face_by_ray" in capabilities["regions"]["sdk_operations"]
    assert "select_faces_by_screen_polygon" in capabilities["regions"]["sdk_operations"]
    assert "select_faces_by_screen_rect" in capabilities["regions"]["sdk_operations"]
    assert "select_faces_by_screen_brush" in capabilities["regions"]["sdk_operations"]
    assert "select_inside_part_faces" in capabilities["regions"]["sdk_operations"]
    assert "self_intersecting_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_degenerate_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_short_edges" in capabilities["regions"]["sdk_operations"]
    assert "select_faces_by_area" in capabilities["regions"]["sdk_operations"]
    assert "select_crease_edges" in capabilities["regions"]["sdk_operations"]
    assert "select_overhang_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_outer_layer_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_not_smooth_faces" in capabilities["regions"]["sdk_operations"]
    assert "graph_cut_select_region" in capabilities["regions"]["sdk_operations"]
    assert "graph_cut_select_region_auto_not_region" in capabilities["regions"]["sdk_operations"]
    assert "select_overlapping_faces" in capabilities["regions"]["sdk_operations"]
    assert "apply_meshlib_selection_modifier" in capabilities["regions"]["sdk_operations"]
    assert "meshlib_select_scene_objects" in capabilities["regions"]["sdk_operations"]
    assert "extract_selected_faces_as_mesh" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert capabilities["runtime-selection-to-object"]["create_object"] is True
    assert capabilities["runtime-selection-to-object"]["sdk_operations"] == [
        "apply_meshlib_selection_modifier",
        "extract_selected_faces_as_mesh",
    ]
    assert "expand_face_selection_to_components" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "apply_meshlib_selection_modifier" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "meshlib_select_scene_objects" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_largest_component_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_camera_facing_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_face_by_ray" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_faces_by_screen_polygon" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_faces_by_screen_rect" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_faces_by_screen_brush" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_inside_part_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "self_intersecting_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_degenerate_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_short_edges" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_faces_by_area" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_crease_edges" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_overhang_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_outer_layer_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_not_smooth_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "graph_cut_select_region" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "graph_cut_select_region_auto_not_region" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_overlapping_faces" in capabilities["runtime-select-mark-region"]["sdk_operations"]
    assert "select_boundary_faces" in capabilities["regions"]["sdk_operations"]
    assert "select_boundary_edges" in capabilities["runtime-select-mark-region"]["sdk_operations"]


def test_mesh_healer_inventory_tracks_meshlib_self_intersection_get_faces_parity() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    mesh_healer = inventory["mesh-healer"]
    capabilities = {
        capability["command_id"]: capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }

    assert "find_disoriented_faces" in capabilities["repair"]["sdk_operations"]
    assert "find_disoriented_faces" in capabilities["make-manufacturable"]["sdk_operations"]
    assert "flip_normals" in capabilities["repair"]["sdk_operations"]
    assert "flip_normals" in capabilities["make-manufacturable"]["sdk_operations"]
    assert "MeshLib/source/MRMesh/MRMeshTopology.*" in mesh_healer.meshlib_source_paths
    assert (
        "cargo test -p zennah-geometry-core find_disoriented_faces_matches_meshlib_ray_count_contract"
        in mesh_healer.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_find_disoriented_faces_matches_meshlib_ray_count_contract -q"
        in mesh_healer.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core flip_normals_matches_meshlib_full_orientation_flip_contract"
        in mesh_healer.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_flip_normals_matches_meshlib_orientation_flip_contract -q"
        in mesh_healer.validation_gates
    )
    assert "MeshLib findDisorientedFaces ray-count disorientation selection" in mesh_healer.notes[0]
    assert "MeshTopology::flipOrientation full-face normal flipping" in mesh_healer.notes[0]
    assert "SelfIntersections::getFaces strict non-touching face detection" in mesh_healer.notes[0]
    assert "Rust topological tunnel diagnostics" in mesh_healer.notes[0]
    assert "MeshLib-oracle 24x8/24x10/24x12 torus detectTunnelFaces face-band selection" in mesh_healer.notes[0]
    assert "MeshLib-oracle torus eliminateTunnels delete-and-fill repair" in mesh_healer.notes[0]
    assert "SelfIntersections::fix Relax topology-preserving repair with subdivision disabled" in mesh_healer.notes[0]
    assert "SDF rebuild self-intersection repair" in mesh_healer.notes[0]
    assert "MeshBuilder-style non-manifold edge face-pruning repair" in mesh_healer.notes[0]
    assert "duplicateNonManifoldVertices disconnected, repeated-neighbor, face-region scoped, partial-triangulation lastValidVert" in mesh_healer.notes[0]
    assert "single-pass path-orientation behavior" in mesh_healer.notes[0]
    assert "broader MRTunnelDetector arbitrary co-loop face-band selection and eliminateTunnels repair" in mesh_healer.notes[1]
    assert "SelfIntersections::fix CutAndFill, degeneracy preprocessing, and subdivision/remesh parity" in mesh_healer.notes[1]
    assert "duplicateNonManifoldVertices path-orientation edge cases" not in mesh_healer.notes[1]


def test_point_cloud_icp_inventory_exposes_rust_backed_pairwise_subset() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    point_cloud = inventory["point-cloud-icp"]
    capabilities = {
        capability["command_id"]: capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }

    assert point_cloud.status == "partial"
    assert "point-cloud-icp" in point_cloud.backend_command_ids
    assert "runtime-selection-to-object" in point_cloud.backend_command_ids
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/optimizer.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/repetitions.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/topology.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud/fan/fill.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/point_cloud/projection.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/point_cloud.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/point_cloud_topology.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/point_cloud_fill.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/point_cloud_projection.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/registration.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/registration/multiway.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/registration/multiway/all_object.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/registration/multiway/cascade.rs" in point_cloud.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-py/src/registration/cascade.rs" in point_cloud.rust_owner_modules
    assert "geometry_sdk/point_cloud/multiway.py" in point_cloud.bridge_modules
    assert "MeshLib/source/MRMesh/MRObjectPoints.*" in point_cloud.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRViewport.*" in point_cloud.meshlib_source_paths
    assert "MeshLib/source/MRViewer/MRSurfacePointPicker.*" in point_cloud.meshlib_source_paths
    assert "point_cloud_nearest_projections" in point_cloud.notes[0]
    assert "point_cloud_project_to_mesh" in point_cloud.notes[0]
    assert "point_cloud_n_closest_neighbors" in point_cloud.notes[0]
    assert "point_cloud_two_closest_points" in point_cloud.notes[0]
    assert "point_cloud_neighbors_in_radius" in point_cloud.notes[0]
    assert "point_cloud_select_by_screen_polygon" in point_cloud.notes[0]
    assert "point_cloud_select_by_screen_rect" in point_cloud.notes[0]
    assert "point_cloud_select_by_screen_brush" in point_cloud.notes[0]
    assert "point_cloud_pick_by_ray" in point_cloud.notes[0]
    assert "point_cloud_extract_selected_points_as_object" in point_cloud.notes[0]
    assert "MeshLib pickRenderObject/ObjectPointsHolder-style primitive Pick selection" in point_cloud.notes[0]
    assert "MeshLib ObjectPoints::cloneRegion/PointCloud::addPartByMask-style selected-point extraction" in point_cloud.notes[0]
    assert "point-cloud Selection to Object child-version creation with normalized_point_cloud_ply artifacts" in point_cloud.notes[0]
    assert "MRSelectScreenLasso::findVertsInViewportArea-style point primitive screen selection" in point_cloud.notes[0]
    assert "point_cloud_local_neighbor_fan" in point_cloud.notes[0]
    assert "point_cloud_local_fan_triangles" in point_cloud.notes[0]
    assert "point_cloud_local_triangulation_repetitions" in point_cloud.notes[0]
    assert "point_cloud_triangulate_candidate_mesh" in point_cloud.notes[0]
    assert "point_cloud_triangulate_cleaned_candidate_mesh" in point_cloud.notes[0]
    assert "point_cloud_triangulate_topology_candidate_mesh" in point_cloud.notes[0]
    assert "point_cloud_triangulate_filled_candidate_mesh" in point_cloud.notes[0]
    assert "point_cloud_uniform_sample" in point_cloud.notes[0]
    assert "point_cloud_grid_sample" in point_cloud.notes[0]
    assert "pairwise_point_to_point_icp" in point_cloud.notes[0]
    assert "pairwise_point_to_plane_icp" in point_cloud.notes[0]
    assert "multiway_point_to_point_icp" in point_cloud.notes[0]
    assert "multiway_point_to_plane_icp" in point_cloud.notes[0]
    assert "multiway_combined_icp" in point_cloud.notes[0]
    assert "multiway_all_object_point_to_point_icp" in point_cloud.notes[0]
    assert "multiway_all_object_point_to_plane_icp" in point_cloud.notes[0]
    assert "multiway_all_object_combined_icp" in point_cloud.notes[0]
    assert "MeshLib maxGroupSize=0-style all-object" in point_cloud.notes[0]
    assert "multiway_sequential_cascade_point_to_point_icp" in point_cloud.notes[0]
    assert "multiway_sequential_cascade_point_to_plane_icp" in point_cloud.notes[0]
    assert "multiway_sequential_cascade_combined_icp" in point_cloud.notes[0]
    assert "MeshLib maxGroupSize>1 sequential cascade" in point_cloud.notes[0]
    assert "multiway_aabb_cascade_point_to_point_icp" in point_cloud.notes[0]
    assert "multiway_aabb_cascade_point_to_plane_icp" in point_cloud.notes[0]
    assert "multiway_aabb_cascade_combined_icp" in point_cloud.notes[0]
    assert "MeshLib AABBTreeBased cascade" in point_cloud.notes[0]
    assert "half-edge origin-ring insertion guards" in point_cloud.notes[0]
    assert "rigid object/reference transforms" in point_cloud.notes[0]
    assert "face-region masks" in point_cloud.notes[0]
    assert "face/edge/vertex pseudonormal normals" in point_cloud.notes[0]
    assert "MultipleEdgesResolveMode None/Simple/Strong dispatch" in point_cloud.notes[0]
    assert "Simple-mode duplicate-edge avoidance" in point_cloud.notes[0]
    assert "Strong-mode reused generated chord repair" in point_cloud.notes[0]
    assert "outNewFaces new-face index reporting" in point_cloud.notes[0]
    assert "maxPolygonSubdivisions split sampling" in point_cloud.notes[0]
    assert "makeDegenerateBand duplicate-boundary band creation" in point_cloud.notes[0]
    assert "stopBeforeBadTriangulation bad-patch guarding" in point_cloud.notes[0]
    assert "smoothBd boundary-edge metric control" in point_cloud.notes[0]
    assert "getMinAreaMetric double-area triangulation" in point_cloud.notes[0]
    assert "getEdgeLengthFillMetric edge-length triangulation" in point_cloud.notes[0]
    assert "getUniversalMetric universal smooth triangulation" in point_cloud.notes[0]
    assert "getMaxDihedralAngleMetric max-dihedral-angle triangulation" in point_cloud.notes[0]
    assert "getParallelPlaneFillMetric parallel-plane projection triangulation" in point_cloud.notes[0]
    assert "getComplexFillMetric aspect-area edge-penalty triangulation" in point_cloud.notes[0]
    assert "getMinTriAngleMetric minimum-angle triangulation" in point_cloud.notes[0]
    assert "getPlaneFillMetric plane-normal triangulation" in point_cloud.notes[0]
    assert "getPlaneNormalizedFillMetric plane-normalized aspect triangulation" in point_cloud.notes[0]
    assert "getComplexStitchMetric aspect-ratio/dihedral stitch triangulation" in point_cloud.notes[0]
    assert "getEdgeLengthStitchMetric edge-length stitch triangulation" in point_cloud.notes[0]
    assert "getVerticalStitchMetric caller-supplied upDir vertical stitch triangulation" in point_cloud.notes[0]
    assert "getVerticalStitchMetricEdgeBased caller-supplied upDir vertical edge-projection stitch triangulation" in point_cloud.notes[0]
    assert "normal-cosine" in point_cloud.notes[0]
    assert "reciprocal closest" in point_cloud.notes[0]
    assert "reciprocal closest filtering" not in point_cloud.notes[1]
    assert "sampled mesh/point projections" not in point_cloud.notes[1]
    assert "non-rigid tree-accelerated/multi-object mesh projection workflows" in point_cloud.notes[1]
    assert "region-aware/non-rigid" not in point_cloud.notes[1]
    assert "full MeshLib pseudonormal projection semantics" not in point_cloud.notes[1]
    assert "full MeshLib mesh-topology materialization" in point_cloud.notes[1]
    assert "arbitrary callback FillHoleMetric parameterization" in point_cloud.notes[1]
    assert "remaining FillHoleMetric modes beyond" not in point_cloud.notes[1]
    assert "cargo test -p zennah-geometry-core point_cloud_pick_by_ray" in point_cloud.validation_gates
    assert "cargo test -p zennah-geometry-core point_cloud_extract_selected_points_as_object" in point_cloud.validation_gates
    assert "cargo test -p zennah-geometry-core meshlib_stitch_fill_metric_modes_are_selectable_rust_modes" in point_cloud.validation_gates
    assert "cargo test -p zennah-geometry-core vertical_stitch_metric_uses_meshlib_caller_supplied_up_dir" in point_cloud.validation_gates
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py::test_point_cloud_pick_by_ray_matches_meshlib_frontmost_point_pick_contract -q"
        in point_cloud.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py::test_point_cloud_extract_selected_points_as_object_matches_meshlib_clone_region_contract -q"
        in point_cloud.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_operation_contracts.py::test_selection_commit_can_create_meshlib_point_cloud_selection_to_object_version -q"
        in point_cloud.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_service_fill_holes_accepts_meshlib_stitch_metric_modes -q"
        in point_cloud.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_repair.py::test_service_fill_holes_exposes_meshlib_vertical_stitch_up_dir_param -q"
        in point_cloud.validation_gates
    )
    assert "MeshLib AABBTreeBased cascade grouping remain open" not in point_cloud.notes[1]
    assert "multiway cascade ICP remain open" not in point_cloud.notes[1]
    assert "multiway all-object/cascade ICP remain open" not in point_cloud.notes[1]
    assert "multiway all-object/cascade/combined ICP remain open" not in point_cloud.notes[1]
    assert "multiway ICP remain open" not in point_cloud.notes[1]
    assert "selection-to-object commit workflows" not in point_cloud.notes[1]
    assert "point-cloud Selection to Object host/version artifact pipeline" not in point_cloud.notes[1]
    assert "point-cloud primitive Pick selection" not in point_cloud.notes[1]
    assert "screen-area point primitive selection" not in point_cloud.notes[1]
    assert "cargo test -p zennah-geometry-core select_point_cloud_points_by_screen" in point_cloud.validation_gates
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_point_cloud.py::test_point_cloud_screen_selectors_match_meshlib_viewport_area_contract -q"
        in point_cloud.validation_gates
    )

    capability = capabilities["point-cloud-icp"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == [
        "point_cloud_nearest_projections",
        "point_cloud_project_to_mesh",
        "point_cloud_n_closest_neighbors",
        "point_cloud_two_closest_points",
        "point_cloud_neighbors_in_radius",
        "point_cloud_select_by_screen_polygon",
        "point_cloud_select_by_screen_rect",
        "point_cloud_select_by_screen_brush",
        "point_cloud_pick_by_ray",
        "point_cloud_extract_selected_points_as_object",
        "point_cloud_local_neighbor_fan",
        "point_cloud_local_fan_triangles",
        "point_cloud_local_triangulation_repetitions",
        "point_cloud_triangulate_candidate_mesh",
        "point_cloud_triangulate_cleaned_candidate_mesh",
        "point_cloud_triangulate_topology_candidate_mesh",
        "point_cloud_triangulate_filled_candidate_mesh",
        "point_cloud_uniform_sample",
        "point_cloud_grid_sample",
        "pairwise_point_to_point_icp",
        "pairwise_point_to_plane_icp",
        "multiway_point_to_point_icp",
        "multiway_point_to_plane_icp",
        "multiway_combined_icp",
        "multiway_all_object_point_to_point_icp",
        "multiway_all_object_point_to_plane_icp",
        "multiway_all_object_combined_icp",
        "multiway_sequential_cascade_point_to_point_icp",
        "multiway_sequential_cascade_point_to_plane_icp",
        "multiway_sequential_cascade_combined_icp",
        "multiway_aabb_cascade_point_to_point_icp",
        "multiway_aabb_cascade_point_to_plane_icp",
        "multiway_aabb_cascade_combined_icp",
    ]


def test_distance_maps_inventory_exposes_rust_backed_contour_subset() -> None:
    inventory = {
        feature.official_feature_id: feature
        for feature in versions_router._official_parity_inventory()
    }
    distance_maps = inventory["distance-maps-lines-gcode"]
    capabilities = {
        capability["command_id"]: capability
        for capability in versions_router.WORKBENCH_COMMAND_CAPABILITIES
    }

    assert distance_maps.status == "partial"
    assert "distance-map-from-mesh" in distance_maps.backend_command_ids
    assert "distance-map-contours" in distance_maps.backend_command_ids
    assert "object-lines-from-contours" in distance_maps.backend_command_ids
    assert "object-lines-to-contours" in distance_maps.backend_command_ids
    assert "offset-contours" in distance_maps.backend_command_ids
    assert "object-lines-load-mrlines" in distance_maps.backend_command_ids
    assert "object-lines-save-mrlines" in distance_maps.backend_command_ids
    assert "object-lines-load-ply" in distance_maps.backend_command_ids
    assert "object-lines-save-ply" in distance_maps.backend_command_ids
    assert "object-lines-load-pts" in distance_maps.backend_command_ids
    assert "object-lines-load-svg" in distance_maps.backend_command_ids
    assert "object-lines-save-pts" in distance_maps.backend_command_ids
    assert "object-lines-save-dxf" in distance_maps.backend_command_ids
    assert "distance-map-iso-lines" in distance_maps.backend_command_ids
    assert "distance-map-merge" in distance_maps.backend_command_ids
    assert "distance-map-contour-boolean" in distance_maps.backend_command_ids
    assert "distance-map-from-tiff" in distance_maps.backend_command_ids
    assert "distance-map-to-tiff" in distance_maps.backend_command_ids
    assert "gcode-parse-paths" in distance_maps.backend_command_ids
    assert "gcode-load-source" in distance_maps.backend_command_ids
    assert "gcode-write-source" in distance_maps.backend_command_ids
    assert "gcode-parse-file-paths" in distance_maps.backend_command_ids
    assert "geometry-rs/crates/zennah-geometry-core/src/distance.rs" in distance_maps.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/distance_tiff.rs" in distance_maps.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/lines.rs" in distance_maps.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/lines/offset_contours.rs" in distance_maps.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/lines/svg.rs" in distance_maps.rust_owner_modules
    assert "geometry-rs/crates/zennah-geometry-core/src/gcode.rs" in distance_maps.rust_owner_modules
    assert "distance_map_from_mesh" in distance_maps.notes[0]
    assert "distance_map_from_contours" in distance_maps.notes[1]
    assert "object_lines_from_contours" in distance_maps.notes[2]
    assert "offset_contours" in distance_maps.notes[3]
    assert "CornerType::Sharp fixed-offset" in distance_maps.notes[3]
    assert "maxSharpAngle limiting" in distance_maps.notes[3]
    assert (
        "default 3D signed fixed/variable Type::Offset, sharp max-angle, fixed/variable shell Z restore/one-pass default relaxation"
        in distance_maps.notes[3]
    )
    assert "explicit relaxIterations" in distance_maps.notes[3]
    assert "constant/custom source-Z restore plus callable zCallback output/index/origin context" in distance_maps.notes[3]
    assert (
        "positive closed fixed/variable non-intersection, closed fixed zero-offset identity indicesMap/origin output, plus negative and shell-inner closed fixed/variable intersection indicesMap/origin output"
        in distance_maps.notes[3]
    )
    assert "closed fixed zero-offset identity indicesMap/origin output" in distance_maps.notes[3]
    assert "open fixed bent/zig and variable bent/zig round-end indicesMap/origin output" in distance_maps.notes[3]
    assert "broader intersection index maps remain future parity items" in distance_maps.notes[3]
    assert "closed clockwise signed Type::Offset round-corner fixed-offset" in distance_maps.notes[3]
    assert "open EndType::Round/Cut fixed-offset" in distance_maps.notes[3]
    assert (
        "open fixed cut-end connected collinear seam-preserving axis/non-axis plus axis/non-axis shifted parallel global-outline composition, axis-aligned perpendicular crossing, horizontal/vertical/non-axis touching-chain including horizontal direction variants, direction-reversed vertical and diagonal origin maps, and first-direction-reversed vertical/diagonal outline ordering, axis/non-axis overlapping-parallel, and axis/non-axis collinear-overlap plus direction-reversed horizontal collinear-overlap including first-source and both-reversed ordering, vertical direction variants, diagonal direction variants, and three-segment horizontal/vertical/diagonal collinear-overlap chains including diagonal chain direction variants global-outline indicesMap/origin output"
        in distance_maps.notes[3]
    )
    assert "closed clockwise signed variable-offset Type::Offset round/sharp-corner" in distance_maps.notes[3]
    assert (
        "positive fixed/variable including unequal-variable and mixed-signed Type::Offset final-outline self-overlap remap with indicesMap intersections"
        in distance_maps.notes[3]
    )
    assert "signed variable-offset Type::Shell round/sharp-corner" in distance_maps.notes[3]
    assert "empty negative-shell output" in distance_maps.notes[3]
    assert "open variable-offset EndType::Cut" in distance_maps.notes[3]
    assert "closed signed fixed-offset Type::Shell" in distance_maps.notes[3]
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_default_3d_z_restore_relaxation_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_variable_shell_3d_z_restore_relaxation_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_variable_negative_offset_3d_z_restore_relaxation_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_variable_sharp_max_angle_3d_z_restore_relaxation_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core tests::offset_contours_matches_meshlib_closed_variable_mixed_signed_offset_contract -- --exact --nocapture --test-threads=1"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core tests::offset_contours_exposes_meshlib_mixed_signed_variable_index_map_contract -- --exact --nocapture --test-threads=1"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_closed_variable_mixed_signed_offset_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_mixed_signed_variable_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_restore_z_relax_iterations"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_constant_z_callback_mode"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_custom_z_callback_mode"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_callable_z_callback_context"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_restore_z_relax_iterations -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_constant_z_callback_mode -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_custom_z_callback_mode -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_exposes_meshlib_callable_z_callback_context -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_positive_round_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_negative_intersection_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core tests::offset_contours_exposes_meshlib_zero_offset_identity_index_map_contract -- --exact --nocapture --test-threads=1"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_zero_offset_identity_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_positive_variable_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_negative_variable_intersection_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_fixed_shell_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_variable_shell_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_round_end_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_round_end_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_exposes_meshlib_open_variable_zig_round_end_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_variable_zig_round_end_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_perpendicular_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_perpendicular_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_touching_horizontal_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_touching_horizontal_direction_variants_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_vertical_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_vertical_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_diagonal_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_diagonal_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_touching_horizontal_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_horizontal_direction_variants_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_vertical_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_vertical_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_diagonal_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_diagonal_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract -q"
        in distance_maps.validation_gates
    )
    assert "sharp-corner Z-restore coverage" not in distance_maps.notes[3]
    assert "closed variable-offset negative-signed shell mode" not in distance_maps.notes[3]
    assert "complex sharp max-angle branches" not in distance_maps.notes[3]
    assert "variable-offset sharp-corner mode" not in distance_maps.notes[3]
    assert "variable-offset sharp-corner shell mode" not in distance_maps.notes[3]
    assert "object_lines_from_mrlines" in distance_maps.notes[4]
    assert "object_lines_from_ply" in distance_maps.notes[5]
    assert "ASCII PLY" in distance_maps.notes[5]
    assert "big-endian" in distance_maps.notes[5]
    assert "color" in distance_maps.notes[5]
    assert (
        "magic-line whitespace, format-version whitespace, minor punctuation-suffix tolerance and alpha-suffix rejection, format-line, element-line, and property-line trailing-token tolerance plus element-count alpha or underscore suffix rejection and property-name prefix suffix tolerance"
        in distance_maps.notes[5]
    )
    assert "end_header trailing-whitespace handling" in distance_maps.notes[5]
    assert "strict header directive, leading keyword whitespace, and identifier validation" in distance_maps.notes[5]
    assert "strict scalar type alias validation" in distance_maps.notes[5]
    assert "Vector3f coordinate narrowing and source scalar conversion" in distance_maps.notes[5]
    assert "scalar-to-int edge endpoint conversion plus ASCII row integer-prefix suffix, narrow integer wrapping, and unsigned scalar sign-cast handling" in distance_maps.notes[5]
    assert "invalid edge skipping" in distance_maps.notes[5]
    assert "edge elements without vertex1/vertex2 skipping" in distance_maps.notes[5]
    assert "r/g/b short-name color precedence over red/green/blue" in distance_maps.notes[5]
    assert "scalar-to-uchar" in distance_maps.notes[5]
    assert "integer wrapping" in distance_maps.notes[5]
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ply_import_prefers_meshlib_rgb_short_names_over_long_color_names -q"
        in distance_maps.validation_gates
    )
    assert "unneeded list-property skipping" in distance_maps.notes[5]
    assert "MeshLib-style binary list-count scalar conversion" in distance_maps.notes[5]
    assert "vertex-only point payloads" in distance_maps.notes[5]
    assert "MeshLib per-vertex PLY UV import aliases" in distance_maps.notes[5]
    assert "TextureFile comment metadata" in distance_maps.notes[5]
    assert "miniply-style leading/trailing comment whitespace trimming" in distance_maps.notes[5]
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_format_version_tuple -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_format_minor_prefix_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_format_minor_alpha_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_format_minor_underscore_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_magic -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_format_line_tokens -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_element_line_tokens -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_element_count_alpha_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_meshlib_element_count_underscore_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_property_line_tokens -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_leading_header_keyword_whitespace_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_spaced_format_version_tuple -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_end_header -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_unknown_header_directives_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_casts_coordinates_to_vector3f_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_wraps_narrow_vertex_coordinates_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_casts_float_edge_indices_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_wraps_narrow_edge_indices_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_unneeded_vertex_property -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_skipped_element -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_last_integer_prefix_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_skips_meshlib_unsigned_negative_edge_endpoint -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_casts_float_vertex_colors_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_wraps_integer_vertex_colors_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_ignores_unneeded_list_properties_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_meshlib_property_name_prefix_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_non_identifier_property_names_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_rejects_float64_type_alias_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_ignores_invalid_edges_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_ascii_ply_import_skips_edge_elements_without_meshlib_vertex_properties"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_binary_ply_import_skips_edge_elements_without_meshlib_vertex_properties"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_skips_edge_elements_without_meshlib_vertex_properties -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_binary_ply_import_skips_edge_elements_without_meshlib_vertex_properties -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_accepts_vertex_only_files_like_meshlib -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_ascii_ply_import_trims_meshlib_texturefile_comment_trailing_spaces"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_preserves_meshlib_uv_and_texture_comment -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_ascii_ply_import_trims_meshlib_texturefile_comment_trailing_spaces -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_pts_import_accepts_meshlib_trailing_point_fields -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_pts_import_accepts_meshlib_last_coordinate_prefix_suffix -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_line_polyline_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_line_and_polyline_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_accepts_meshlib_compact_signed_points_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_polygon_rect_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_polygon_and_rect_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_circle_ellipse_sampling_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_circle_and_ellipse_sampling_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_rounded_rect_sampling_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_rounded_rect_sampling_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_linear_path_commands_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_linear_path_commands_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_curve_path_commands_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_curve_path_commands_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_arc_path_commands_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_arc_path_commands_y_flip -q"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core object_lines_svg_import_matches_meshlib_transform_attributes_y_flip"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_object_lines_svg_import_matches_meshlib_transform_attributes_y_flip -q"
        in distance_maps.validation_gates
    )
    assert "object_lines_from_pts" in distance_maps.notes[6]
    assert "trailing point-field tolerance" in distance_maps.notes[6]
    assert "last-coordinate numeric-prefix suffix tolerance" in distance_maps.notes[6]
    assert "object_lines_from_svg" in distance_maps.notes[6]
    assert "line/polyline/polygon/circle/ellipse/simple-rounded-rect/path-command/transform import" in distance_maps.notes[6]
    assert "compact signed polyline/polygon points" in distance_maps.notes[6]
    assert "post-parse Y-axis flip" in distance_maps.notes[6]
    assert "SVG transforms remain future parity coverage" not in distance_maps.notes[6]
    assert "distance_map_to_iso_segments" in distance_maps.notes[7]
    assert "distance_map_merge" in distance_maps.notes[8]
    assert "distance_map_contour_boolean" in distance_maps.notes[9]
    assert "distance_map_from_tiff" in distance_maps.notes[10]
    assert "distance_map_to_tiff" in distance_maps.notes[11]
    assert "parse_gcode_paths" in distance_maps.notes[12]
    assert "strtof command-value narrowing" in distance_maps.notes[12]
    assert "leading command-value whitespace" in distance_maps.notes[12]
    assert "special, and hexadecimal float tokens" in distance_maps.notes[12]
    assert "hexadecimal float tokens" in distance_maps.notes[12]
    assert "no-motion feedrateMax updates" in distance_maps.notes[12]
    assert "zero-idle feedrate post-pass rewriting" in distance_maps.notes[12]
    assert "radius-only G2/G3 no-op handling" in distance_maps.notes[12]
    assert "G28 home zero-length idle actions" in distance_maps.notes[12]
    assert "MeshLib-style arc radius-mismatch warning formatting" in distance_maps.notes[12]
    assert (
        "cargo test -p zennah-geometry-core gcode_command_values_match_meshlib_strtof_narrowing"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_command_values_accept_meshlib_strtof_special_float_tokens"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_command_values_accept_meshlib_strtof_hex_float_tokens"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_command_values_accept_meshlib_strtof_leading_whitespace"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_arc_radius_mismatch_warning_matches_meshlib_to_string_float_format"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_radius_only_arc_matches_meshlib_no_motion_feedrate_contract"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_feedrate_only_frame_updates_meshlib_feedrate_max_without_segments"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_zero_idle_feedrate_is_rewritten_to_meshlib_final_feedrate_max"
        in distance_maps.validation_gates
    )
    assert (
        "cargo test -p zennah-geometry-core gcode_g28_at_home_emits_meshlib_zero_length_idle_action"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_command_narrowing -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_special_float_tokens -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_hex_float_tokens -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_strtof_leading_whitespace -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_arc_radius_mismatch_warning_format -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_radius_only_arc_no_motion_feedrate_contract tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_feedrate_only_frame_without_segments -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_zero_idle_feedrate_post_pass -q"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_parse_gcode_paths_matches_meshlib_g28_at_home_zero_length_idle_action -q"
        in distance_maps.validation_gates
    )
    assert "GcodeMachineSettings MeshLib JSON import/export" in distance_maps.notes[13]
    assert "CNCMachineSettings::saveToJson/loadFromJson-style Axes Order" in distance_maps.notes[13]
    assert "inactive-axis omission" in distance_maps.notes[13]
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_gcode_machine_settings_exports_meshlib_cnc_json_contract tests/test_geometry_sdk_gcode.py::test_gcode_machine_settings_imports_meshlib_cnc_json_contract -q"
        in distance_maps.validation_gates
    )
    assert "load_gcode_source" in distance_maps.notes[14]
    assert "CRLF frame carriage-return preservation" in distance_maps.notes[14]
    assert (
        "cargo test -p zennah-geometry-core gcode_source_file_preserves_meshlib_crlf_frame_carriage_returns"
        in distance_maps.validation_gates
    )
    assert (
        "uv run --extra dev pytest tests/test_geometry_sdk_gcode.py::test_gcode_source_file_preserves_meshlib_crlf_frame_carriage_returns -q"
        in distance_maps.validation_gates
    )
    assert "UV/TextureFile metadata" in distance_maps.notes[15]
    assert "SVG line/polyline/polygon/circle/ellipse/simple-rounded-rect/path-command/transform workflows are Rust-backed" in distance_maps.notes[15]
    assert "SVG transforms" not in distance_maps.notes[15]
    assert "ObjectLines texture image loading/rendering/export" in distance_maps.notes[15]
    assert "ObjectLines PLY UV, texture" not in distance_maps.notes[15]

    capability = capabilities["distance-map-from-mesh"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_from_mesh"]

    capability = capabilities["distance-map-contours"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_from_contours"]

    capability = capabilities["object-lines-from-contours"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_from_contours"]

    capability = capabilities["object-lines-to-contours"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_to_contours"]

    capability = capabilities["offset-contours"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["offset_contours", "offset_contours_with_origins"]

    capability = capabilities["object-lines-load-mrlines"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_from_mrlines"]

    capability = capabilities["object-lines-save-mrlines"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_to_mrlines"]

    capability = capabilities["object-lines-load-ply"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_from_ply"]
    assert "edge elements without vertex1/vertex2 skipping" in capability["notes"][0]

    capability = capabilities["object-lines-save-ply"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_to_ply"]

    capability = capabilities["object-lines-load-pts"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_from_pts"]
    assert "trailing point-field tolerance" in capability["notes"][0]
    assert "last-coordinate numeric-prefix suffix tolerance" in capability["notes"][0]

    capability = capabilities["object-lines-load-svg"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_from_svg"]
    assert "compact signed <polyline>/<polygon> points" in capability["notes"][0]

    capability = capabilities["object-lines-save-pts"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_to_pts"]

    capability = capabilities["object-lines-save-dxf"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["object_lines_to_dxf"]

    capability = capabilities["distance-map-iso-lines"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_to_iso_segments"]

    capability = capabilities["distance-map-merge"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_merge"]

    capability = capabilities["distance-map-contour-boolean"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_contour_boolean"]

    capability = capabilities["distance-map-from-tiff"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_from_tiff"]

    capability = capabilities["distance-map-to-tiff"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["distance_map_to_tiff"]

    capability = capabilities["gcode-parse-paths"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["parse_gcode_paths"]
    assert "strtof command-value narrowing" in capability["notes"][0]
    assert "leading command-value whitespace" in capability["notes"][0]
    assert "special, and hexadecimal float tokens" in capability["notes"][0]
    assert "hexadecimal float tokens" in capability["notes"][0]
    assert "no-motion feedrateMax updates" in capability["notes"][0]
    assert "zero-idle feedrate post-pass rewriting" in capability["notes"][0]
    assert "radius-only G2/G3 no-op handling" in capability["notes"][0]
    assert "G28 home zero-length idle actions" in capability["notes"][0]
    assert "MeshLib-style arc radius-mismatch warning formatting" in capability["notes"][0]

    capability = capabilities["gcode-load-source"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["load_gcode_source"]
    assert "CRLF carriage returns" in capability["notes"][0]

    capability = capabilities["gcode-write-source"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["write_gcode_source"]

    capability = capabilities["gcode-parse-file-paths"]
    assert capability["rust_backed"] is True
    assert capability["sdk_operations"] == ["parse_gcode_file_paths"]
    assert "CRLF carriage-return frame preservation" in capability["notes"][0]
