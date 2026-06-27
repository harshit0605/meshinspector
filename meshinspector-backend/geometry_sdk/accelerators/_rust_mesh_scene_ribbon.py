from __future__ import annotations

from typing import Any

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


_SCENE_RIBBON_OPERATION_LABELS = {
    "select_all": "Ribbon Scene Select all",
    "selectAll": "Ribbon Scene Select all",
    "Ribbon Scene Select all": "Ribbon Scene Select all",
    "unselect_all": "Ribbon Scene Unselect all",
    "unselectAll": "Ribbon Scene Unselect all",
    "Ribbon Scene Unselect all": "Ribbon Scene Unselect all",
    "show_all": "Ribbon Scene Show all",
    "showAll": "Ribbon Scene Show all",
    "Ribbon Scene Show all": "Ribbon Scene Show all",
    "hide_all": "Ribbon Scene Hide all",
    "hideAll": "Ribbon Scene Hide all",
    "Ribbon Scene Hide all": "Ribbon Scene Hide all",
    "show_only_previous": "Ribbon Scene Show only previous",
    "show_only_prev": "Ribbon Scene Show only previous",
    "showOnlyPrevious": "Ribbon Scene Show only previous",
    "showOnlyPrev": "Ribbon Scene Show only previous",
    "Ribbon Scene Show only previous": "Ribbon Scene Show only previous",
    "show_only_next": "Ribbon Scene Show only next",
    "showOnlyNext": "Ribbon Scene Show only next",
    "Ribbon Scene Show only next": "Ribbon Scene Show only next",
    "sort_by_name": "Ribbon Scene Sort by name",
    "sortByName": "Ribbon Scene Sort by name",
    "Ribbon Scene Sort by name": "Ribbon Scene Sort by name",
    "remove_selected": "Ribbon Scene Remove selected objects",
    "remove_selected_objects": "Ribbon Scene Remove selected objects",
    "Ribbon Scene Remove selected objects": "Ribbon Scene Remove selected objects",
}


_SCENE_COLLECTION_KEYS = (
    "scene_objects",
    "scene_group_objects",
    "scene_line_objects",
    "scene_point_objects",
    "scene_distance_map_objects",
    "scene_feature_objects",
    "scene_voxel_objects",
)


def _scene_collection(metadata: dict[str, Any], key: str) -> list[dict[str, Any]]:
    value = metadata.get(key)
    return value if isinstance(value, list) else []


def _scene_collections(mesh: MeshDocument) -> tuple[
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
]:
    return tuple(_scene_collection(mesh.metadata, key) for key in _SCENE_COLLECTION_KEYS)  # type: ignore[return-value]


def _update_scene_collection_metadata(metadata: dict[str, Any], payload: dict[str, Any]) -> None:
    for key in _SCENE_COLLECTION_KEYS:
        objects = payload[key]
        metadata[key] = objects
        metadata[f"{key[:-1]}_count"] = len(objects)
    if "scene_child_order" in payload:
        metadata["scene_child_order"] = payload["scene_child_order"]


def _require_any_scene_collection(mesh: MeshDocument, operation: str) -> tuple[
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
    list[dict[str, Any]],
]:
    collections = _scene_collections(mesh)
    if not any(collections):
        raise ValueError(f"MRU scene {operation} requires at least one scene object collection")
    return collections


def meshlib_apply_scene_ribbon_action(
    mesh: MeshDocument,
    *,
    action: str,
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_apply_scene_ribbon_action")
    (
        scene_objects,
        scene_group_objects,
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
    ) = _require_any_scene_collection(mesh, "ribbon actions")
    root_key = str(mesh.metadata.get("root_key") or "0_Root")
    payload: dict[str, Any] = kernel(
        scene_objects,
        root_key,
        str(action),
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
        scene_group_objects,
    )
    metadata = dict(mesh.metadata)
    _update_scene_collection_metadata(metadata, payload)
    metadata.update(
        {
            "affected_scene_object_keys": payload["affected_object_keys"],
            "selected_scene_object_keys": payload["selected_object_keys"],
            "visible_scene_object_keys": payload["visible_object_keys"],
            "removed_scene_object_keys": payload["removed_object_keys"],
            "scene_child_order": payload["scene_child_order"],
            "meshlib_operation": _SCENE_RIBBON_OPERATION_LABELS.get(str(action), str(action)),
            "meshlib_reference": "MR::RibbonSceneButtons",
            "meshlib_source": "MeshLib/source/MRCommonPlugins/ViewerButtons/MRRibbonSceneButtons.cpp;MeshLib/source/MRViewer/MRSceneObjectsListDrawer.cpp;MeshLib/source/MRMesh/MRObject.cpp",
        }
    )
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_group_scene_objects(
    mesh: MeshDocument,
    *,
    group_key: str,
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_group_scene_objects")
    (
        scene_objects,
        scene_group_objects,
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
    ) = _require_any_scene_collection(mesh, "grouping")
    root_key = str(mesh.metadata.get("root_key") or "0_Root")
    payload: dict[str, Any] = kernel(
        scene_objects,
        root_key,
        str(group_key),
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
        scene_group_objects,
    )
    metadata = dict(mesh.metadata)
    _update_scene_collection_metadata(metadata, payload)
    metadata.update(
        {
            "affected_scene_object_keys": payload["affected_object_keys"],
            "selected_scene_object_keys": payload["selected_object_keys"],
            "visible_scene_object_keys": payload["visible_object_keys"],
            "removed_scene_object_keys": payload["removed_object_keys"],
            "scene_child_order": payload["scene_child_order"],
            "meshlib_operation": "Scene Group",
            "meshlib_reference": "MR::RibbonMenu::drawGroupUngroupButton",
            "meshlib_source": "MeshLib/source/MRViewer/MRRibbonMenu.cpp;MeshLib/source/MRMesh/MRObject.cpp",
        }
    )
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_ungroup_scene_objects(mesh: MeshDocument) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_ungroup_scene_objects")
    (
        scene_objects,
        scene_group_objects,
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
    ) = _scene_collections(mesh)
    if not scene_group_objects:
        raise ValueError("MRU scene ungroup requires scene_group_objects metadata")
    root_key = str(mesh.metadata.get("root_key") or "0_Root")
    payload: dict[str, Any] = kernel(
        scene_objects,
        root_key,
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
        scene_group_objects,
    )
    metadata = dict(mesh.metadata)
    _update_scene_collection_metadata(metadata, payload)
    metadata.update(
        {
            "affected_scene_object_keys": payload["affected_object_keys"],
            "selected_scene_object_keys": payload["selected_object_keys"],
            "visible_scene_object_keys": payload["visible_object_keys"],
            "removed_scene_object_keys": payload["removed_object_keys"],
            "scene_child_order": payload["scene_child_order"],
            "meshlib_operation": "Scene Ungroup",
            "meshlib_reference": "MR::RibbonMenu::drawGroupUngroupButton",
            "meshlib_source": "MeshLib/source/MRViewer/MRRibbonMenu.cpp;MeshLib/source/MRViewer/MRSceneReorder.cpp;MeshLib/source/MRMesh/MRObject.cpp",
        }
    )
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_rename_scene_object(
    mesh: MeshDocument,
    *,
    object_key: str,
    object_name: str,
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_rename_scene_object")
    (
        scene_objects,
        scene_group_objects,
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
    ) = _require_any_scene_collection(mesh, "object rename")
    payload: dict[str, Any] = kernel(
        scene_objects,
        str(object_key),
        str(object_name),
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_feature_objects,
        scene_voxel_objects,
        scene_group_objects,
    )
    metadata = dict(mesh.metadata)
    _update_scene_collection_metadata(metadata, payload)
    metadata.update(
        {
            "meshlib_operation": "Ribbon Scene Rename",
            "meshlib_reference": "MR::ImGuiMenu::tryRenameSelectedObject;MR::Object::name",
            "meshlib_source": "MeshLib/source/MRCommonPlugins/ViewerButtons/MRRibbonSceneButtons.cpp;MeshLib/source/MRViewer/ImGuiMenu.cpp;MeshLib/source/MRMesh/MRObject.h",
        }
    )
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )
