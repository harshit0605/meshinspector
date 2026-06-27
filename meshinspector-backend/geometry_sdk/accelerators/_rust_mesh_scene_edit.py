from __future__ import annotations

from typing import Any

import numpy as np

from geometry_sdk.accelerators import _rust_common as _common
from geometry_sdk.types import MeshDocument


def _require_core_kernel(name: str):
    if _common._rs is None:
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs is not installed")
    if not hasattr(_common._rs, name):
        raise RuntimeError(f"Rust kernel {name} is required, but _zennah_geometry_rs does not expose it")
    return getattr(_common._rs, name)


def _scene_objects_for_edit(mesh: MeshDocument, operation: str) -> list[dict[str, Any]]:
    scene_objects = mesh.metadata.get("scene_objects")
    if not isinstance(scene_objects, list) or not scene_objects:
        raise ValueError(f"mesh.metadata['scene_objects'] is required for MRU scene object {operation}")
    return scene_objects


def _scene_objects_for_transform(mesh: MeshDocument) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    scene_objects = mesh.metadata.get("scene_objects")
    if not isinstance(scene_objects, list):
        scene_objects = []
    scene_feature_objects = mesh.metadata.get("scene_feature_objects")
    if not isinstance(scene_feature_objects, list):
        scene_feature_objects = []
    if not scene_objects and not scene_feature_objects:
        raise ValueError(
            "mesh.metadata['scene_objects'] or mesh.metadata['scene_feature_objects'] "
            "is required for MRU scene object transform editing"
        )
    return scene_objects, scene_feature_objects


def _scene_feature_objects_for_edit(mesh: MeshDocument, operation: str) -> list[dict[str, Any]]:
    scene_feature_objects = mesh.metadata.get("scene_feature_objects")
    if not isinstance(scene_feature_objects, list) or not scene_feature_objects:
        raise ValueError(
            f"mesh.metadata['scene_feature_objects'] is required for MRU FeatureObject {operation}"
        )
    return scene_feature_objects


def _update_child_order_metadata(metadata: dict[str, Any], payload: dict[str, Any]) -> None:
    if "scene_child_order" in payload:
        metadata["scene_child_order"] = payload["scene_child_order"]


def meshlib_transform_scene_object(
    mesh: MeshDocument,
    *,
    object_key: str,
    xf: dict[str, Any],
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_transform_scene_object")
    scene_objects, scene_feature_objects = _scene_objects_for_transform(mesh)
    payload: dict[str, Any] = kernel(
        mesh.vertices,
        scene_objects,
        str(object_key),
        xf,
        scene_feature_objects,
    )
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "scene_objects": payload["scene_objects"],
            "scene_object_count": len(payload["scene_objects"]),
            "scene_feature_objects": payload.get("scene_feature_objects", scene_feature_objects),
            "scene_feature_object_count": len(
                payload.get("scene_feature_objects", scene_feature_objects)
            ),
            "meshlib_operation": "MR::Object::setXf/MR::FeatureObject::setXf",
            "meshlib_reference": "MR::Object::setXf;MR::FeatureObject::setXf",
            "meshlib_source": (
                "MeshLib/source/MRMesh/MRObject.cpp;"
                "MeshLib/source/MRMesh/MRFeatureObject.cpp"
            ),
        }
    )
    _update_child_order_metadata(metadata, payload)
    return MeshDocument(
        vertices=np.asarray(payload["vertices"], dtype=np.float64).reshape((-1, 3)),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_reparent_scene_object(
    mesh: MeshDocument,
    *,
    object_key: str,
    new_parent_key: str,
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_reparent_scene_object")
    scene_objects = _scene_objects_for_edit(mesh, "reparent editing")
    root_key = str(mesh.metadata.get("root_key") or "0_Root")
    payload: dict[str, Any] = kernel(
        scene_objects,
        root_key,
        str(object_key),
        str(new_parent_key),
    )
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "scene_objects": payload["scene_objects"],
            "scene_object_count": len(payload["scene_objects"]),
            "meshlib_operation": "MR::Object::addChild",
            "meshlib_reference": "MR::Object::addChild",
            "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp",
        }
    )
    _update_child_order_metadata(metadata, payload)
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_set_scene_object_state(
    mesh: MeshDocument,
    *,
    object_key: str,
    visibility_mask: int | None = None,
    visible: bool | None = None,
    selected: bool | None = None,
    locked: bool | None = None,
    parent_locked: bool | None = None,
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_set_scene_object_state")
    scene_objects, scene_feature_objects = _scene_objects_for_transform(mesh)
    if visibility_mask is not None and visible is not None:
        raise ValueError("Pass either visibility_mask or visible, not both")
    if visible is not None:
        visibility_mask = 0xFFFF_FFFF if visible else 0
    try:
        payload: dict[str, Any] = kernel(
            scene_objects,
            str(object_key),
            visibility_mask,
            selected,
            locked,
            parent_locked,
            scene_feature_objects,
        )
    except TypeError:
        if scene_feature_objects:
            raise
        payload = kernel(
            scene_objects,
            str(object_key),
            visibility_mask,
            selected,
            locked,
            parent_locked,
        )
    operation = "MR::Object::setVisible" if visibility_mask is not None else "MR::Object::setLocked"
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "scene_objects": payload["scene_objects"],
            "scene_object_count": len(payload["scene_objects"]),
            "scene_feature_objects": payload.get("scene_feature_objects", scene_feature_objects),
            "scene_feature_object_count": len(
                payload.get("scene_feature_objects", scene_feature_objects)
            ),
            "meshlib_operation": operation,
            "meshlib_reference": "MR::Object::setVisible;MR::Object::setLocked;MR::Object::setParentLocked",
            "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp",
        }
    )
    _update_child_order_metadata(metadata, payload)
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_select_scene_objects(
    mesh: MeshDocument,
    *,
    object_keys,
    mode: str = "select_one",
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_select_scene_objects")
    scene_objects, scene_feature_objects = _scene_objects_for_transform(mesh)
    try:
        payload: dict[str, Any] = kernel(
            scene_objects,
            [str(key) for key in object_keys],
            str(mode),
            scene_feature_objects,
        )
    except TypeError:
        if scene_feature_objects:
            raise
        payload = kernel(
            scene_objects,
            [str(key) for key in object_keys],
            str(mode),
        )
    operation = (
        "MR::NameTagSelectionMode::toggle"
        if str(mode) in {"toggle", "primary_ctrl", "primaryCtrl", "ctrl"}
        else "MR::NameTagSelectionMode::selectOne"
    )
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "scene_objects": payload["scene_objects"],
            "scene_object_count": len(payload["scene_objects"]),
            "scene_feature_objects": payload.get("scene_feature_objects", scene_feature_objects),
            "scene_feature_object_count": len(
                payload.get("scene_feature_objects", scene_feature_objects)
            ),
            "selected_scene_object_keys": payload["selected_object_keys"],
            "meshlib_operation": operation,
            "meshlib_reference": "MR::NameTagSelectionMode::selectOne;MR::NameTagSelectionMode::toggle",
            "meshlib_source": "MeshLib/source/MRViewer/ImGuiMenu.cpp;MeshLib/source/MRCommonPlugins/Selectors/MRSelectObjectByClick.cpp",
        }
    )
    _update_child_order_metadata(metadata, payload)
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_set_scene_feature_object_visualize_property(
    mesh: MeshDocument,
    *,
    object_key: str,
    property: str,
    viewport_mask: int,
    dimension_name: str | None = None,
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_set_scene_feature_object_visualize_property")
    scene_feature_objects = _scene_feature_objects_for_edit(mesh, "visualize-property editing")
    payload: dict[str, Any] = kernel(
        scene_feature_objects,
        str(object_key),
        str(property),
        int(viewport_mask),
        dimension_name,
    )
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "scene_feature_objects": payload["scene_feature_objects"],
            "scene_feature_object_count": len(payload["scene_feature_objects"]),
            "meshlib_operation": "MR::FeatureObject::setVisualizePropertyMask",
            "meshlib_reference": (
                "MR::FeatureObject::setVisualizePropertyMask;"
                "MR::FeatureObject::serializeFields_"
            ),
            "meshlib_source": (
                "MeshLib/source/MRMesh/MRFeatureObject.cpp;"
                "MeshLib/source/MRViewer/ImGuiMenu.cpp"
            ),
        }
    )
    _update_child_order_metadata(metadata, payload)
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )


def meshlib_scene_feature_object_render_payload(
    mesh: MeshDocument,
    *,
    viewport_mask: int = 0xFFFF_FFFF,
    circle_segments: int = 64,
) -> dict[str, Any]:
    kernel = _require_core_kernel("meshlib_scene_feature_object_render_payload")
    scene_feature_objects = _scene_feature_objects_for_edit(mesh, "render-payload generation")
    return dict(
        kernel(
            scene_feature_objects,
            int(viewport_mask),
            int(circle_segments),
        )
    )


def meshlib_reorder_scene_children(
    mesh: MeshDocument,
    *,
    parent_key: str,
    ordered_child_keys: list[str],
) -> MeshDocument:
    kernel = _require_core_kernel("meshlib_reorder_scene_children")
    scene_objects = _scene_objects_for_edit(mesh, "reorder editing")
    root_key = str(mesh.metadata.get("root_key") or "0_Root")
    payload: dict[str, Any] = kernel(
        scene_objects,
        root_key,
        str(parent_key),
        [str(key) for key in ordered_child_keys],
    )
    metadata = dict(mesh.metadata)
    metadata.update(
        {
            "scene_objects": payload["scene_objects"],
            "scene_object_count": len(payload["scene_objects"]),
            "meshlib_operation": "MR::ChangeSceneObjectsOrder",
            "meshlib_reference": "MR::ChangeSceneObjectsOrder",
            "meshlib_source": "MeshLib/source/MRMesh/MRChangeSceneObjectsOrder.h",
        }
    )
    _update_child_order_metadata(metadata, payload)
    return MeshDocument(
        vertices=mesh.vertices.copy(),
        faces=mesh.faces.copy(),
        unit=mesh.unit,
        metadata=metadata,
    )
