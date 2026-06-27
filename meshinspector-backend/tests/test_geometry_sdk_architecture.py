from __future__ import annotations

import ast
import json
import re
from pathlib import Path


BACKEND_ROOT = Path(__file__).resolve().parents[1]
FRONTEND_ROOT = BACKEND_ROOT.parent / "meshinspector-frontend"
SDK_ROOT = BACKEND_ROOT / "geometry_sdk"
RUST_CORE_ROOT = BACKEND_ROOT / "geometry-rs" / "crates" / "zennah-geometry-core" / "src"
RUST_PY_ROOT = BACKEND_ROOT / "geometry-rs" / "crates" / "zennah-geometry-py" / "src"


def _line_count(path: Path) -> int:
    return len(path.read_text().splitlines())


def _imported_modules(path: Path) -> set[str]:
    tree = ast.parse(path.read_text(), filename=str(path))
    modules: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            modules.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            modules.add(node.module.split(".")[0])
            modules.add(node.module)
    return modules


def _assigned_literal(path: Path, name: str):
    tree = ast.parse(path.read_text(), filename=str(path))
    for node in tree.body:
        if not isinstance(node, ast.Assign):
            continue
        if any(isinstance(target, ast.Name) and target.id == name for target in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(f"{name} assignment not found in {path}")


def _class_methods(path: Path, class_name: str) -> set[str]:
    tree = ast.parse(path.read_text(), filename=str(path))
    for node in tree.body:
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            return {
                child.name
                for child in node.body
                if isinstance(child, ast.FunctionDef) and not child.name.startswith("_")
            }
    raise AssertionError(f"{class_name} class not found in {path}")


def _function_source(path: Path, name: str) -> str:
    source = path.read_text()
    tree = ast.parse(source, filename=str(path))
    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef) and node.name == name:
            segment = ast.get_source_segment(source, node)
            if segment is None:
                raise AssertionError(f"{name} source not found in {path}")
            return segment
    raise AssertionError(f"{name} function not found in {path}")


def test_rust_core_keeps_facades_and_modules_bounded() -> None:
    assert _line_count(RUST_CORE_ROOT / "lib.rs") <= 80
    assert _line_count(RUST_CORE_ROOT / "spatial.rs") <= 500

    for path in RUST_CORE_ROOT.rglob("*.rs"):
        if path.name == "tests.rs":
            continue
        assert _line_count(path) <= 700, f"{path.relative_to(RUST_CORE_ROOT)} should stay modular"


def test_rust_python_bindings_keep_facades_and_modules_bounded() -> None:
    assert _line_count(RUST_PY_ROOT / "lib.rs") <= 80

    for path in RUST_PY_ROOT.rglob("*.rs"):
        if path.name == "lib.rs":
            continue
        assert _line_count(path) <= 350, f"{path.relative_to(RUST_PY_ROOT)} should stay domain-scoped"


def test_python_rust_accelerator_keeps_import_stable_facade() -> None:
    accelerators_root = SDK_ROOT / "accelerators"
    assert _line_count(accelerators_root / "rust.py") <= 120

    for path in accelerators_root.glob("_rust_*.py"):
        assert _line_count(path) <= 450, f"{path.name} should stay domain-scoped"


def test_rust_owned_accelerator_slice_has_no_python_fallback_mode() -> None:
    accelerators_root = SDK_ROOT / "accelerators"
    rust_owned_modules = sorted(path.name for path in accelerators_root.glob("_rust_*.py"))
    forbidden_snippets = [
        'mode == "python"',
        'mode != "rust"',
        "kernel is None",
        "return None\n    if _common._rs is None",
    ]

    for module_name in rust_owned_modules:
        source = (accelerators_root / module_name).read_text(encoding="utf-8")
        leaked = [snippet for snippet in forbidden_snippets if snippet in source]
        assert leaked == [], f"{module_name} still exposes Python fallback mode snippets: {leaked}"


def test_rust_accelerator_modules_never_silently_fall_back_after_kernel_lookup() -> None:
    accelerators_root = SDK_ROOT / "accelerators"
    offenders = []
    for path in accelerators_root.glob("_rust_*.py"):
        source = path.read_text(encoding="utf-8")
        if "if kernel is None:" in source:
            offenders.append(path.name)

    assert offenders == []


def test_third_party_geometry_engines_stay_behind_adapters() -> None:
    allowed_roots = {
        SDK_ROOT / "adapters",
        SDK_ROOT / "io",
        SDK_ROOT / "testing",
    }
    forbidden = {"meshlib", "trimesh", "scipy"}

    for path in SDK_ROOT.rglob("*.py"):
        if any(path.is_relative_to(root) for root in allowed_roots):
            continue
        imports = _imported_modules(path)
        leaked = forbidden.intersection(imports)
        assert not leaked, f"{path.relative_to(SDK_ROOT)} imports adapter-only engines: {sorted(leaked)}"


def test_versioned_ui_operations_route_through_sdk_facade() -> None:
    path = BACKEND_ROOT / "services" / "operations.py"
    imports = _imported_modules(path)
    assert not {"meshlib", "trimesh", "scipy"}.intersection(imports)

    source = path.read_text()
    assert "default_sdk.basic_repair(" in source
    assert "default_sdk.service_fill_holes(" in source
    assert "default_sdk.resize_ring(" in source
    assert "default_sdk.service_hollow(" in source
    assert "default_sdk.protected_hollow_mesh(" in source
    assert "default_sdk.adaptive_hollow_to_weight(" in source
    assert "default_sdk.adaptive_protected_hollow_to_weight(" in source
    assert "default_sdk.global_thicken(" in source
    assert "default_sdk.local_thicken_to_minimum(" in source
    assert "default_sdk.local_scoop(" in source
    assert "default_sdk.smooth(" in source
    assert "default_sdk.service_compare_field(" in source
    assert "default_sdk.service_compare(" in source
    assert "default_sdk.save_compare_npz(" in source
    assert "np.savez" not in source


def test_hollow_operations_normalize_ui_protect_regions_to_available_manifest_ids() -> None:
    source = (BACKEND_ROOT / "services" / "operations.py").read_text()

    assert "def _available_region_ids(" in source
    assert "protect_region_ids = _available_region_ids(" in source
    assert "list(request.protect_regions)" not in source
    assert "request.protect_regions," not in source


def test_manufacturability_snapshot_routes_through_sdk_facade() -> None:
    path = BACKEND_ROOT / "services" / "manufacturability.py"
    imports = _imported_modules(path)
    assert not {"meshlib", "trimesh", "scipy", "services.health", "services.measure_ring", "services.regions", "services.thickness_meshlib"}.intersection(imports)

    source = path.read_text()
    assert "default_sdk.load_mesh(" in source
    assert "default_sdk.stats(" in source
    assert "default_sdk.service_health(" in source
    assert "default_sdk.measure_ring(" in source
    assert "default_sdk.service_thickness(" in source
    assert "default_sdk.detect_ring_regions(" in source
    assert "default_sdk.save_thickness_npz(" in source


def test_versioned_artifact_generation_uses_sdk_conversion_boundary() -> None:
    conversion_path = BACKEND_ROOT / "services" / "sdk_conversion.py"
    assert conversion_path.exists()

    conversion_source = conversion_path.read_text()
    conversion_imports = _imported_modules(conversion_path)
    assert not {"meshlib", "trimesh", "scipy"}.intersection(conversion_imports)
    assert "default_sdk.load_mesh(" in conversion_source
    assert "default_sdk.save_mesh(" in conversion_source

    for relative_path in ["services/ingest.py", "services/operations.py"]:
        source = (BACKEND_ROOT / relative_path).read_text()
        assert "services.convert" not in source
        assert "services.sdk_conversion" in source


def test_versioned_api_routes_do_not_import_mesh_engines() -> None:
    path = BACKEND_ROOT / "api" / "routers" / "versions.py"
    imports = _imported_modules(path)
    assert not {"meshlib", "trimesh", "scipy"}.intersection(imports)


def test_versioned_api_routes_do_not_own_geometry_file_parsers() -> None:
    source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()

    forbidden_route_helpers = {
        "_ply_header_and_data",
        "_point_cloud_ply_layout",
        "_point_cloud_property_indices",
        "_load_point_cloud_document",
        "_save_point_cloud_document",
    }
    leaked = sorted(name for name in forbidden_route_helpers if f"def {name}" in source)
    assert leaked == []


def test_versioned_scalar_overlay_routes_through_sdk_facade() -> None:
    path = BACKEND_ROOT / "api" / "routers" / "versions.py"
    source = path.read_text()
    imports = _imported_modules(path)

    assert "numpy" not in imports
    assert "default_sdk.thickness_overlay_payload(" in source
    assert "default_sdk.compare_overlay_payload(" in source
    assert "np." not in source


def test_scalar_overlay_artifact_payloads_cross_rust_boundary() -> None:
    source = (SDK_ROOT / "analysis" / "artifacts.py").read_text()

    assert "_rust_analysis.scalar_overlay_payload(" in source
    assert "np.where(" not in source
    assert "np.min(" not in source
    assert "np.max(" not in source
    assert "np.mean(" not in source


def test_voxel_value_ranges_cross_rust_boundary() -> None:
    for relative_path in [
        "voxel/conversion.py",
        "voxel/rendering.py",
    ]:
        source = (SDK_ROOT / relative_path).read_text()
        assert "_rust_voxel.voxel_value_range(" in source
        assert "np.min(" not in source
        assert "np.max(" not in source


def test_sdf_grid_coordinate_helpers_cross_rust_boundary() -> None:
    source = (SDK_ROOT / "types.py").read_text()
    points_source = _function_source(SDK_ROOT / "types.py", "points")
    point_to_grid_source = _function_source(SDK_ROOT / "types.py", "point_to_grid")

    assert "from geometry_sdk.accelerators import _rust_sdf" in source
    assert "_rust_sdf.sdf_grid_points(" in points_source
    assert "_rust_sdf.sdf_points_to_grid(" in point_to_grid_source

    for forbidden in [
        "np.arange(",
        "np.meshgrid(",
        "np.stack(",
        "/ self.voxel_size_mm",
    ]:
        assert forbidden not in points_source
        assert forbidden not in point_to_grid_source


def test_sdf_grid_bounds_planning_crosses_rust_boundary() -> None:
    source = (SDK_ROOT / "accelerators" / "_rust_sdf.py").read_text()
    in_bounds_source = _function_source(
        SDK_ROOT / "accelerators" / "_rust_sdf.py",
        "sample_sdf_grid_in_bounds",
    )
    aligned_source = _function_source(
        SDK_ROOT / "accelerators" / "_rust_sdf.py",
        "sample_aligned_sdf_grids",
    )

    assert '_require_rust_kernel("sample_sdf_grid_in_bounds")' in in_bounds_source
    assert '_require_rust_kernel("combine_bounding_boxes")' in aligned_source
    assert '_require_rust_kernel("sdf_grid_values")' not in in_bounds_source

    for forbidden in [
        "np.ceil(",
        "np.maximum(",
        "np.min(",
        "np.max(",
        "np.vstack(",
    ]:
        assert forbidden not in source


def test_services_package_does_not_eager_import_legacy_geometry_modules() -> None:
    path = BACKEND_ROOT / "services" / "__init__.py"
    source = path.read_text()
    assert "from .convert" not in source
    assert "from .analyze" not in source
    assert "from .hollow" not in source
    assert "from .resize" not in source
    assert "from .repair" not in source


def test_services_root_no_longer_carries_retired_geometry_engines() -> None:
    retired_modules = {
        "analyze.py",
        "convert.py",
        "health.py",
        "hollow.py",
        "measure_ring.py",
        "regions.py",
        "repair.py",
        "resize.py",
        "thickness.py",
        "thickness_meshlib.py",
    }
    services_root = BACKEND_ROOT / "services"
    still_present = sorted(path.name for path in services_root.glob("*.py") if path.name in retired_modules)
    assert still_present == []

    forbidden_imports = {
        "meshlib",
        "trimesh",
        "scipy",
        "services.convert",
        "services.health",
        "services.hollow",
        "services.measure_ring",
        "services.regions",
        "services.repair",
        "services.resize",
        "services.thickness",
        "services.thickness_meshlib",
    }
    for path in services_root.glob("*.py"):
        imports = _imported_modules(path)
        leaked = forbidden_imports.intersection(imports)
        assert not leaked, f"{path.relative_to(BACKEND_ROOT)} imports retired geometry engines: {sorted(leaked)}"


def test_active_compat_analyze_route_uses_sdk_facade_for_mesh_counts() -> None:
    path = BACKEND_ROOT / "api" / "routes" / "analyze.py"
    imports = _imported_modules(path)
    assert not {"meshlib", "trimesh", "scipy", "services.convert"}.intersection(imports)
    assert "default_sdk.load_mesh(" in path.read_text()


def test_active_frontend_viewer_mounts_official_meshlib_workbench_runtime() -> None:
    active_sources = [
        FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx",
        FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts",
        FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts",
        FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts",
    ]
    combined_source = "\n".join(path.read_text() for path in active_sources)
    assert "MeshLibWorkbenchHost" in combined_source
    assert "useMeshLibWorkbenchManifest" in combined_source
    assert "getMeshLibWorkbenchManifest" in combined_source
    assert "MeshLibWorkbenchManifest" in combined_source
    assert "meshlib-workbench" in combined_source


def test_frontend_has_single_authoritative_tool_inspector_surface() -> None:
    assert not (FRONTEND_ROOT / "src" / "features" / "editor" / "panels" / "AdvancedEditPanel.tsx").exists()

    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    assert "ToolInspector" in viewer_page
    assert "AdvancedEditPanel" not in viewer_page


def test_frontend_public_assets_ship_official_meshlib_workbench_runtime() -> None:
    workbench_root = FRONTEND_ROOT / "public" / "meshlib-workbench"
    runtime_root = workbench_root / "runtime"
    assert (workbench_root / "index.html").exists()
    assert (workbench_root / "bridge.js").exists()
    assert (runtime_root / "manifest.json").exists()
    assert (runtime_root / "ViewerApp.js").exists()
    assert (runtime_root / "ViewerApp.wasm").exists()


def test_official_meshlib_runtime_precreates_required_logo_resource_dirs() -> None:
    wasm_loader = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "wasm_loader.js").read_text()

    assert "function ensureMeshLibLogoDirectories()" in wasm_loader
    assert "Module.FS_createPath('/resource', 'logos', true, true);" in wasm_loader
    assert "Module.FS_createPath('/resource/logos', 'X1', true, true);" in wasm_loader
    assert "Module.FS_createPath('/resource/logos', 'X3', true, true);" in wasm_loader
    assert "preRun: [ensureMeshLibLogoDirectories]" in wasm_loader


def test_official_meshlib_runtime_prefers_rust_scene_archive_over_flat_mesh() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "manifest?.meshlib_scene_mru_url" in runtime_bootstrap
    assert (
        "return manifest?.meshlib_scene_mru_url || manifest?.normalized_mesh_url || "
        "manifest?.preview_high_url || manifest?.preview_low_url || null;" in runtime_bootstrap
    )
    assert "return 'mru';" in runtime_bootstrap
    assert runtime_bootstrap.index("manifest?.meshlib_scene_mru_url") < runtime_bootstrap.index("manifest?.normalized_mesh_url")


def test_backend_product_routes_advertise_meshlib_workbench_runtime() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    operations_source = (BACKEND_ROOT / "api" / "routers" / "operations.py").read_text()
    assert "MeshLibWorkbenchManifest" in versions_source
    assert '"/versions/{version_id}/meshlib-workbench"' in versions_source
    assert '"/versions/{version_id}/interactive-commit"' in operations_source
    assert "supports_interactive_commit" in versions_source


def test_meshlib_workbench_manifest_exposes_command_level_rust_capabilities() -> None:
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    workspace_command_ids = sorted(set(re.findall(r"id: '([^']+)'", tool_registry)))
    assert len(workspace_command_ids) >= 20

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "MeshLibWorkbenchCommandCapability" in schemas
    assert "command_capabilities: list[MeshLibWorkbenchCommandCapability]" in schemas
    assert "MeshLibWorkbenchCommandCapability" in frontend_types
    assert "command_capabilities: MeshLibWorkbenchCommandCapability[]" in frontend_types
    assert "WORKBENCH_COMMAND_CAPABILITIES" in versions_source
    assert "supports_workspace_commands" in versions_source
    assert "command_capabilities=" in versions_source
    assert "meshinspectorWorkbenchCommandCount" in bridge
    assert "meshinspectorWorkbenchCommandCount" in runtime_bootstrap
    assert "meshinspectorWorkbenchRuntimeTools" in bridge
    assert "meshinspectorWorkbenchRuntimeTools" in runtime_bootstrap

    missing = [command_id for command_id in workspace_command_ids if f'"command_id": "{command_id}"' not in versions_source]
    assert missing == []

    for marker in [
        '"runtime_tool_id": "select_mark_region"',
        '"runtime_tool_id": "thicken_brush"',
        '"runtime_tool_id": "scoop_brush"',
        '"runtime_tool_id": "smooth_brush"',
        '"runtime_tool_id": "measure_inspect"',
        '"endpoint_url_key": "selection_endpoint_url"',
        '"endpoint_url_key": "brush_endpoint_url"',
        '"endpoint_url_key": "measurement_endpoint_url"',
        '"sdk_operations": ["section_contour"]',
        '"sdk_operations": ["mesh_healer_diagnostics", "hole_fill_plan_diagnostics", "repeated_hole_boundary_vertices_diagnostics", "hole_complicating_faces_diagnostics", "remove_hole_complicating_faces", "short_edge_diagnostics", "degenerate_face_diagnostics", "multiple_edge_diagnostics", "repair_multiple_edges", "repair_nonmanifold_edges", "duplicate_nonmanifold_vertices", "duplicate_multi_hole_vertices", "not_smooth_face_diagnostics", "find_disoriented_faces", "flip_normals", "crease_edge_diagnostics", "crease_repair_plan_diagnostics", "fix_mesh_creases", "unite_close_vertices", "basic_repair", "service_fill_holes", "prune_small_components", "tunnel_diagnostics", "detect_tunnel_faces", "eliminate_tunnels", "fix_self_intersections_relax", "rebuild_via_sdf"]',
        '"sdk_operations": ["resize_ring"]',
        '"sdk_operations": ["adaptive_protected_hollow_to_weight"]',
        '"sdk_operations": ["protected_hollow_mesh"]',
        '"sdk_operations": ["protected_hollow_mesh", "plan_drain_holes", "apply_drain_holes_voxel"]',
        '"sdk_operations": ["local_thicken_to_minimum"]',
        '"sdk_operations": ["local_scoop"]',
        '"sdk_operations": ["smooth"]',
        '"sdk_operations": ["service_compare_field", "service_compare"]',
        '"sdk_operations": ["distance_map_from_mesh"]',
        '"sdk_operations": ["distance_map_from_contours"]',
        '"sdk_operations": ["object_lines_from_contours"]',
        '"sdk_operations": ["object_lines_to_contours"]',
        '"sdk_operations": ["object_lines_from_mrlines"]',
        '"sdk_operations": ["object_lines_to_mrlines"]',
        '"sdk_operations": ["object_lines_from_ply"]',
        '"sdk_operations": ["object_lines_to_ply"]',
        '"sdk_operations": ["object_lines_from_pts"]',
        '"sdk_operations": ["object_lines_to_pts"]',
        '"sdk_operations": ["object_lines_to_dxf"]',
        '"sdk_operations": ["distance_map_to_iso_segments"]',
        '"sdk_operations": ["distance_map_merge"]',
        '"sdk_operations": ["distance_map_contour_boolean"]',
        '"sdk_operations": ["distance_map_from_tiff"]',
        '"sdk_operations": ["distance_map_to_tiff"]',
        '"sdk_operations": ["parse_gcode_paths"]',
        '"sdk_operations": ["load_gcode_source"]',
        '"sdk_operations": ["write_gcode_source"]',
        '"sdk_operations": ["parse_gcode_file_paths"]',
    ]:
        assert marker in versions_source


def test_official_workbench_plugin_assets_expose_parity_inventory_tools() -> None:
    expected_tabs = {
        "Home",
        "View",
        "Select",
        "Mesh Repair",
        "Mesh Edit",
        "Inspect / Features",
        "Compare / Report",
        "Point Cloud",
        "CT / Voxels",
        "Distance Maps / Lines / G-code",
        "Automation",
    }
    source_items_path = BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json"
    source_ui_path = BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.ui.json"
    public_items_path = FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json"
    public_ui_path = FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.ui.json"

    source_items = json.loads(source_items_path.read_text())
    source_ui = json.loads(source_ui_path.read_text())
    public_items = json.loads(public_items_path.read_text())
    public_ui = json.loads(public_ui_path.read_text())

    assert public_items == source_items
    assert public_ui == source_ui
    assert {tab["Name"] for tab in source_ui.get("Tabs", [])} >= expected_tabs

    item_by_feature_id = {
        item.get("OfficialFeatureId"): item
        for item in source_items.get("Items", [])
        if item.get("OfficialFeatureId")
    }
    inventory = _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "OFFICIAL_PARITY_INVENTORY")
    inventory_by_id = {feature["official_feature_id"]: feature for feature in inventory}
    assert set(item_by_feature_id) >= set(inventory_by_id)

    for feature_id, feature in inventory_by_id.items():
        item = item_by_feature_id[feature_id]
        assert item["ParityStatus"] == feature["status"]
        assert item["MeshLibSourcePaths"] == feature["meshlib_source_paths"]
        assert item["ValidationGates"] == feature["validation_gates"]
        if feature["status"] == "missing":
            assert item["Enabled"] is False
            assert item["MissingBackendOperation"] is True
        else:
            assert item["RustOwnerModules"] == feature["rust_owner_modules"]
            assert item.get("BridgeModules", []) == feature.get("bridge_modules", [])
            assert all(module.startswith("geometry-rs/") for module in item["RustOwnerModules"])
            assert all(not module.startswith("geometry-rs/") for module in item.get("BridgeModules", []))


def test_enabled_official_workbench_items_have_host_dispatch_path() -> None:
    source_items_path = BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json"
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    host_source = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    runtime_source = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    source_items = json.loads(source_items_path.read_text())
    workspace_command_ids = set(re.findall(r"id: '([^']+)'", tool_registry))
    host_alias_ids = set(re.findall(r"['\"]([^'\"]+)['\"]:\s*'[^']+'", host_source))
    runtime_alias_ids = set(re.findall(r"['\"]([^'\"]+)['\"]:\s*'[^']+'", runtime_source))

    missing_dispatch_paths = []
    for item in source_items.get("Items", []):
        command_id = item.get("CommandId")
        if not command_id or item.get("Enabled") is not True:
            continue
        if command_id.startswith("runtime-"):
            continue
        if command_id in workspace_command_ids or command_id in host_alias_ids or command_id in runtime_alias_ids:
            continue
        missing_dispatch_paths.append(command_id)

    assert sorted(missing_dispatch_paths) == []


def test_gcode_file_workbench_commands_are_registered_for_host_execution() -> None:
    expected_commands = {
        "gcode-load-source": "gcode_load_source_endpoint_url",
        "gcode-write-source": "gcode_write_source_endpoint_url",
        "gcode-parse-file-paths": "gcode_parse_file_paths_endpoint_url",
    }
    manifest_paths = [
        BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json",
        FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json",
    ]
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    host_source = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    runtime_source = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    capability_by_id = {
        capability["command_id"]: capability
        for capability in _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "WORKBENCH_COMMAND_CAPABILITIES")
    }

    for command_id, endpoint_key in expected_commands.items():
        assert f"id: '{command_id}'" in tool_registry
        assert f"case '{command_id}':" in viewer_page
        assert f"'{command_id}': '{command_id}'" in host_source
        assert f"'{command_id}': '{command_id}'" in runtime_source
        assert capability_by_id[command_id]["endpoint_url_key"] == endpoint_key

    for manifest_path in manifest_paths:
        manifest = json.loads(manifest_path.read_text())
        items_by_command = {item.get("CommandId"): item for item in manifest.get("Items", [])}
        for command_id in expected_commands:
            assert items_by_command[command_id]["RustBacked"] is True


def test_enabled_rust_owned_workbench_commands_are_advertised_as_rust_backed() -> None:
    manifest_paths = [
        BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json",
        FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json",
        BACKEND_ROOT.parent / "meshlib-workbench" / "build-wasm" / "html" / "assets" / "MeshInspectorWorkbenchPlugin.items.json",
    ]
    capability_by_id = {
        capability["command_id"]: capability
        for capability in _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "WORKBENCH_COMMAND_CAPABILITIES")
    }

    for manifest_path in manifest_paths:
        manifest = json.loads(manifest_path.read_text())
        missing_rust_flag = []
        for item in manifest.get("Items", []):
            command_id = item.get("CommandId")
            if not command_id or item.get("Enabled") is not True or item.get("MissingBackendOperation") is True:
                continue
            owner_modules = item.get("RustOwnerModules") or []
            has_rust_owner = any(module.startswith("geometry-rs/") for module in owner_modules)
            backend_is_rust_backed = capability_by_id.get(command_id, {}).get("rust_backed") is True
            if (has_rust_owner or backend_is_rust_backed) and item.get("RustBacked") is not True:
                missing_rust_flag.append(command_id)

        assert sorted(missing_rust_flag) == []


def test_sdk_only_official_workbench_items_are_disabled_until_product_endpoint_exists() -> None:
    source_items_path = BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json"
    source_items = json.loads(source_items_path.read_text())

    capability_by_id = {
        capability["command_id"]: capability
        for capability in _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "WORKBENCH_COMMAND_CAPABILITIES")
    }
    direct_frontend_commands = {"upload-new", "download-stl", "wireframe", "snapshots", "version-history", "restore-branch", "job-activity"}

    falsely_enabled = []
    for item in source_items.get("Items", []):
        command_id = item.get("CommandId")
        if not command_id or item.get("Enabled") is not True:
            continue
        capability = capability_by_id.get(command_id)
        if command_id in direct_frontend_commands:
            continue
        if capability and (capability.get("endpoint_url_key") or capability.get("group") == "runtime"):
            continue
        falsely_enabled.append(command_id)

    assert sorted(falsely_enabled) == []


def test_runtime_bridge_projects_official_parity_inventory_into_meshlib_ui_state() -> None:
    for runtime_path in [
        FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js",
        FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js",
    ]:
        source = runtime_path.read_text()
        assert "official_parity_inventory" in source
        assert "function officialParityInventory(" in source
        assert "function officialWorkbenchTools(" in source
        assert "function isOfficialParityToolEnabled(" in source
        assert "meshinspectorWorkbenchOfficialParityFeatureCount" in source
        assert "meshinspectorWorkbenchOfficialParityMissingCount" in source
        assert "meshinspectorWorkbenchOfficialParityGroups" in source
        assert "meshinspectorWorkbenchDisabledFeatureIds" in source
        assert "missing_backend_operation" in source


def test_official_workbench_cpp_plugin_registers_manifest_tools_with_disabled_placeholders() -> None:
    items_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    plugin_source = (BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.cpp").read_text()
    item_names = {
        str(item.get("Name"))
        for item in items_manifest.get("Items", [])
        if item.get("OfficialFeatureId") or item.get("RelatedOfficialFeatureId")
    }
    disabled_item_names = {
        str(item.get("Name"))
        for item in items_manifest.get("Items", [])
        if item.get("MissingBackendOperation") is True
    }

    assert item_names
    assert disabled_item_names
    assert "class DisabledOfficialParityToolBase" in plugin_source

    for item_name in sorted(item_names):
        assert f'"{item_name}"' in plugin_source

    for item_name in sorted(disabled_item_names):
        assert re.search(rf'DisabledOfficialParityToolBase\(\s*"{re.escape(item_name)}"', plugin_source)

    registered_items = set(re.findall(r"MR_REGISTER_RIBBON_ITEM\(\s*([A-Za-z0-9_]+)\s*\)", plugin_source))
    for expected_class in [
        "FileSceneViewerTool",
        "MeshHealerTool",
        "MeshEditSimplifyTool",
        "BooleanCollisionTool",
        "OffsetShellTool",
        "CompareReportTool",
        "PointCloudIcpTool",
        "VoxelsCtSdfTool",
        "DistanceMapsLinesGcodeTool",
        "AutomationPluginApiTool",
    ]:
        assert expected_class in registered_items


def test_official_workbench_offset_contours_surface_lists_current_meshlib_direction_variant_parity() -> None:
    manifest_paths = [
        BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json",
        FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json",
        BACKEND_ROOT.parent / "meshlib-workbench" / "build-wasm" / "html" / "assets" / "MeshInspectorWorkbenchPlugin.items.json",
    ]
    capability_by_id = {
        capability["command_id"]: capability
        for capability in _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "WORKBENCH_COMMAND_CAPABILITIES")
    }
    expected_gates = {
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract",
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract",
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract",
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract",
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract",
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract",
        "cargo test -p zennah-geometry-core offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract",
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract",
        "cargo test -p zennah-geometry-core offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract -q",
        "uv run --extra dev pytest tests/test_geometry_sdk_distance_map.py::test_offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract -q",
    }
    tooltip_markers = [
        "direction-reversed horizontal collinear-overlap",
        "first-source and both-reversed ordering",
        "vertical direction variants",
        "diagonal direction variants",
        "three-segment horizontal/vertical/diagonal collinear-overlap chains",
        "diagonal chain direction variants",
        "global-outline indicesMap/origin output",
    ]

    capability = capability_by_id["offset-contours"]
    for marker in tooltip_markers:
        assert marker in capability["notes"][0]

    for manifest_path in manifest_paths:
        manifest = json.loads(manifest_path.read_text())
        item = next(entry for entry in manifest.get("Items", []) if entry.get("CommandId") == "offset-contours")
        assert expected_gates <= set(item["ValidationGates"])
        for marker in tooltip_markers:
            assert marker in item["Tooltip"]

    plugin_source = (BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.cpp").read_text()
    for marker in tooltip_markers:
        assert marker in plugin_source


def test_meshlib_workbench_rust_capabilities_reference_geometry_sdk_facade_methods() -> None:
    capabilities = _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "WORKBENCH_COMMAND_CAPABILITIES")
    sdk_methods = _class_methods(SDK_ROOT / "engine.py", "GeometrySDK")

    rust_capabilities = [capability for capability in capabilities if capability.get("rust_backed") is True]
    assert rust_capabilities, "Workbench manifest should advertise Rust-backed capabilities"

    missing_operations: dict[str, list[str]] = {}
    for capability in rust_capabilities:
        sdk_operations = capability.get("sdk_operations") or []
        assert sdk_operations, f"{capability['command_id']} is rust_backed but has no sdk_operations"
        missing = [operation for operation in sdk_operations if operation not in sdk_methods]
        if missing:
            missing_operations[capability["command_id"]] = missing

    assert missing_operations == {}


def test_meshlib_workbench_rust_capabilities_cross_rust_boundary_modules() -> None:
    capabilities = _assigned_literal(BACKEND_ROOT / "api" / "routers" / "versions.py", "WORKBENCH_COMMAND_CAPABILITIES")
    rust_operations = {
        operation
        for capability in capabilities
        if capability.get("rust_backed") is True
        for operation in capability.get("sdk_operations") or []
    }

    direct_operation_markers: dict[str, list[tuple[Path, str, list[str]]]] = {
        "adaptive_protected_hollow_to_weight": [
            (
                SDK_ROOT / "jewelry" / "hollow.py",
                "adaptive_protected_hollow_to_weight",
                ["_rust_hollow.adaptive_protected_hollow_to_weight("],
            ),
        ],
        "apply_brush_strokes": [
            (SDK_ROOT / "deform" / "brushes.py", "apply_brush_strokes", ["_rust_brushes.apply_brush_strokes("]),
        ],
        "apply_drain_holes_voxel": [
            (
                SDK_ROOT / "jewelry" / "hollow.py",
                "apply_drain_holes_voxel",
                ["drain_hole_cutters_mesh(", "voxel_boolean_mesh("],
            ),
            (
                SDK_ROOT / "jewelry" / "hollow.py",
                "drain_hole_cutters_mesh",
                ["_rust_hollow.drain_hole_cutters_mesh("],
            ),
            (SDK_ROOT / "voxel" / "mesh_ops.py", "voxel_boolean_mesh", ["_rust_mesh_ops.voxel_boolean_mesh("]),
        ],
        "basic_repair": [
            (SDK_ROOT / "repair" / "basic.py", "basic_repair", ["_require_rust(", "rust.basic_repair("]),
        ],
        "mesh_healer_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "mesh_healer_diagnostics",
                ["_require_rust(", "rust.mesh_healer_diagnostics("],
            ),
        ],
        "find_disoriented_faces": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "find_disoriented_faces",
                ["_require_rust(", "rust.find_disoriented_faces("],
            ),
        ],
        "flip_normals": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "flip_normals",
                ["_require_rust(", "rust.flip_normals("],
            ),
        ],
        "tunnel_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "tunnel_diagnostics",
                ["_require_rust(", "rust.tunnel_diagnostics("],
            ),
        ],
        "detect_tunnel_faces": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "detect_tunnel_faces",
                ["_require_rust(", "rust.detect_tunnel_faces("],
            ),
        ],
        "eliminate_tunnels": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "eliminate_tunnels",
                ["_require_rust(", "rust.eliminate_tunnels("],
            ),
        ],
        "hole_fill_plan_diagnostics": [
            (
                SDK_ROOT / "repair" / "holes.py",
                "hole_fill_plan_diagnostics",
                ["_require_rust(", "rust.hole_fill_plan_diagnostics("],
            ),
        ],
        "repeated_hole_boundary_vertices_diagnostics": [
            (
                SDK_ROOT / "repair" / "holes.py",
                "repeated_hole_boundary_vertices_diagnostics",
                ["_require_rust(", "rust.repeated_hole_boundary_vertices_diagnostics("],
            ),
        ],
        "hole_complicating_faces_diagnostics": [
            (
                SDK_ROOT / "repair" / "holes.py",
                "hole_complicating_faces_diagnostics",
                ["_require_rust(", "rust.hole_complicating_faces_diagnostics("],
            ),
        ],
        "remove_hole_complicating_faces": [
            (
                SDK_ROOT / "repair" / "holes.py",
                "remove_hole_complicating_faces",
                ["_require_rust(", "rust.remove_hole_complicating_faces("],
            ),
        ],
        "extract_selected_faces_as_mesh": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "extract_selected_faces_as_mesh",
                ["_rust_mesh.extract_selected_faces_as_mesh("],
            ),
        ],
        "expand_face_selection_to_components": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "expand_face_selection_to_components",
                ["_rust_mesh.expand_face_selection_to_components("],
            ),
        ],
        "apply_meshlib_selection_modifier": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "apply_meshlib_selection_modifier",
                ["_rust_mesh.apply_meshlib_selection_modifier("],
            ),
        ],
        "meshlib_select_scene_objects": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "meshlib_select_scene_objects",
                ["_rust_mesh.meshlib_select_scene_objects("],
            ),
        ],
        "graph_cut_select_region": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "graph_cut_select_region",
                ["_rust_mesh.graph_cut_select_region("],
            ),
        ],
        "graph_cut_select_region_auto_not_region": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "graph_cut_select_region_auto_not_region",
                ["_rust_mesh.graph_cut_select_region_auto_not_region("],
            ),
        ],
        "point_cloud_extract_selected_points_as_object": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_extract_selected_points_as_object",
                ["rust.point_cloud_extract_selected_points_as_object("],
            ),
        ],
        "point_cloud_pick_by_ray": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_pick_by_ray",
                ["rust.point_cloud_pick_by_ray("],
            ),
        ],
        "point_cloud_select_by_screen_brush": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_select_by_screen_brush",
                ["rust.point_cloud_select_by_screen_brush("],
            ),
        ],
        "point_cloud_select_by_screen_polygon": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_select_by_screen_polygon",
                ["rust.point_cloud_select_by_screen_polygon("],
            ),
        ],
        "point_cloud_select_by_screen_rect": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_select_by_screen_rect",
                ["rust.point_cloud_select_by_screen_rect("],
            ),
        ],
        "select_boundary_edges": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_boundary_edges",
                ["_rust_mesh.select_boundary_edges("],
            ),
        ],
        "select_boundary_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_boundary_faces",
                ["_rust_mesh.select_boundary_faces("],
            ),
        ],
        "select_camera_facing_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_camera_facing_faces",
                ["_rust_mesh.select_camera_facing_faces("],
            ),
        ],
        "select_crease_edges": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_crease_edges",
                ["_rust_smoothness.crease_edge_diagnostics("],
            ),
        ],
        "select_degenerate_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_degenerate_faces",
                ["_rust_repair.select_degenerate_faces("],
            ),
        ],
        "select_face_by_ray": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_face_by_ray",
                ["first_ray_hit("],
            ),
            (
                SDK_ROOT / "spatial" / "raycast.py",
                "first_ray_hit",
                ["_rust_raycast.first_ray_hit("],
            ),
        ],
        "select_faces_by_area": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_faces_by_area",
                ["_rust_mesh.select_faces_by_area("],
            ),
        ],
        "select_faces_by_screen_brush": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_faces_by_screen_brush",
                ["_rust_mesh.select_faces_by_screen_brush("],
            ),
        ],
        "select_faces_by_screen_polygon": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_faces_by_screen_polygon",
                ["_rust_mesh.select_faces_by_screen_polygon("],
            ),
        ],
        "select_faces_by_screen_rect": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_faces_by_screen_rect",
                ["_rust_mesh.select_faces_by_screen_rect("],
            ),
        ],
        "select_inside_part_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_inside_part_faces",
                ["_rust_mesh.select_inside_part_faces("],
            ),
        ],
        "select_largest_component_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_largest_component_faces",
                ["_rust_mesh.select_largest_component_faces("],
            ),
        ],
        "select_not_smooth_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_not_smooth_faces",
                ["_rust_repair.select_not_smooth_faces("],
            ),
        ],
        "select_outer_layer_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_outer_layer_faces",
                ["_rust_mesh.select_outer_layer_faces("],
            ),
        ],
        "select_overhang_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_overhang_faces",
                ["_rust_mesh.select_overhang_faces("],
            ),
        ],
        "select_overlapping_faces": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_overlapping_faces",
                ["_rust_mesh.select_overlapping_faces("],
            ),
        ],
        "select_short_edges": [
            (
                SDK_ROOT / "core" / "mesh.py",
                "select_short_edges",
                ["_rust_repair.select_short_edges("],
            ),
        ],
        "self_intersecting_faces": [
            (
                SDK_ROOT / "spatial" / "intersections.py",
                "self_intersecting_faces",
                ["_rust_intersections.self_intersecting_faces("],
            ),
        ],
            "closest_points_on_mesh": [
                (
                    SDK_ROOT / "spatial" / "closest_point.py",
                    "closest_points_on_mesh",
                    ["_rust_closest_point.closest_points_on_mesh("],
                ),
            ],
            "feature_pair_measurements": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "feature_pair_measurements",
                    ["_rust_features.feature_pair_measurements("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_features.py",
                    "feature_pair_measurements",
                    ['_require_core_kernel("feature_pair_measurements")'],
                ),
            ],
            "mesh_geodesic_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_path",
                    ["_rust_geodesic.mesh_geodesic_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_geodesic_path",
                    ['_require_core_kernel("mesh_geodesic_path")'],
                ),
            ],
            "mesh_geodesic_polyline_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_polyline_path",
                    ["_rust_geodesic.mesh_geodesic_polyline_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_geodesic_polyline_path",
                    ['_require_core_kernel("mesh_geodesic_polyline_path")'],
                ),
            ],
                "mesh_cut_measure_contours": [
                    (
                        SDK_ROOT / "core" / "mesh.py",
                        "mesh_cut_measure_contours",
                        ["_rust_geodesic.mesh_cut_measure_contours("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_cut_measure_contours",
                        ['_require_core_kernel("mesh_cut_measure_contours")'],
                    ),
                ],
                "mesh_cut_measure_edge_path_topology_cut": [
                    (
                        SDK_ROOT / "core" / "mesh.py",
                        "mesh_cut_measure_edge_path_topology_cut",
                        ["_rust_geodesic.mesh_cut_measure_edge_path_topology_cut("],
                    ),
                    (
                        SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                        "mesh_cut_measure_edge_path_topology_cut",
                        ['_require_core_kernel("mesh_cut_measure_edge_path_topology_cut")'],
                    ),
                ],
                "mesh_geodesic_quadrangle_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_quadrangle_path",
                    ["_rust_geodesic.mesh_geodesic_quadrangle_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_geodesic_quadrangle_path",
                    ['_require_core_kernel("mesh_geodesic_quadrangle_path")'],
                ),
            ],
            "mesh_planar_triangle_strip_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_planar_triangle_strip_path",
                    ["_rust_geodesic.mesh_planar_triangle_strip_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_planar_triangle_strip_path",
                    ['_require_core_kernel("mesh_planar_triangle_strip_path")'],
                ),
            ],
            "mesh_surface_edge_point_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_surface_edge_point_path",
                    ["_rust_geodesic.mesh_surface_edge_point_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_surface_edge_point_path",
                    ['_require_core_kernel("mesh_surface_edge_point_path")'],
                ),
            ],
            "mesh_geodesic_edge_point_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_edge_point_path",
                    ["_rust_geodesic.mesh_geodesic_edge_point_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_geodesic_edge_point_path",
                    ['_require_core_kernel("mesh_geodesic_edge_point_path")'],
                ),
            ],
            "mesh_triangle_strip_unfolded_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_triangle_strip_unfolded_path",
                    ["_rust_geodesic.mesh_triangle_strip_unfolded_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_triangle_strip_unfolded_path",
                    ['_require_core_kernel("mesh_triangle_strip_unfolded_path")'],
                ),
            ],
            "mesh_steepest_descent_triangle_step": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_steepest_descent_triangle_step",
                    ["_rust_geodesic.mesh_steepest_descent_triangle_step("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_steepest_descent_triangle_step",
                    ['_require_core_kernel("mesh_steepest_descent_triangle_step")'],
                ),
            ],
            "mesh_steepest_descent_edge_step": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_steepest_descent_edge_step",
                    ["_rust_geodesic.mesh_steepest_descent_edge_step("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_steepest_descent_edge_step",
                    ['_require_core_kernel("mesh_steepest_descent_edge_step")'],
                ),
            ],
            "mesh_steepest_descent_vertex_step": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_steepest_descent_vertex_step",
                    ["_rust_geodesic.mesh_steepest_descent_vertex_step("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_steepest_descent_vertex_step",
                    ['_require_core_kernel("mesh_steepest_descent_vertex_step")'],
                ),
            ],
            "mesh_steepest_descent_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_steepest_descent_path",
                    ["_rust_geodesic.mesh_steepest_descent_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_steepest_descent_path",
                    ['_require_core_kernel("mesh_steepest_descent_path")'],
                ),
            ],
            "mesh_fast_marching_surface_path": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_fast_marching_surface_path",
                    ["_rust_fast_marching.mesh_fast_marching_surface_path("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_fast_marching.py",
                    "mesh_fast_marching_surface_path",
                    ['_require_core_kernel("mesh_fast_marching_surface_path")'],
                ),
            ],
            "mesh_fast_marching_surface_path_tri_points": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_fast_marching_surface_path_tri_points",
                    ["_rust_fast_marching.mesh_fast_marching_surface_path_tri_points("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_fast_marching.py",
                    "mesh_fast_marching_surface_path_tri_points",
                    ['_require_core_kernel("mesh_fast_marching_surface_path_tri_points")'],
                ),
            ],
            "mesh_surface_path_tri_points": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_surface_path_tri_points",
                    ["_rust_fast_marching.mesh_surface_path_tri_points("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_fast_marching.py",
                    "mesh_surface_path_tri_points",
                    ['_require_core_kernel("mesh_surface_path_tri_points")'],
                ),
            ],
            "mesh_geodesic_distance_field": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_distance_field",
                    ["_rust_geodesic.mesh_geodesic_distance_field("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic_surface.py",
                    "mesh_geodesic_distance_field",
                    ['_require_core_kernel("mesh_geodesic_distance_field")'],
                ),
            ],
            "mesh_closest_surface_path_targets": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_closest_surface_path_targets",
                    ["_rust_geodesic.mesh_closest_surface_path_targets("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic_surface.py",
                    "mesh_closest_surface_path_targets",
                    ['_require_core_kernel("mesh_closest_surface_path_targets")'],
                ),
            ],
            "mesh_surface_distance_seed_vertices": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_surface_distance_seed_vertices",
                    ["_rust_geodesic.mesh_surface_distance_seed_vertices("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic_surface.py",
                    "mesh_surface_distance_seed_vertices",
                    ['_require_core_kernel("mesh_surface_distance_seed_vertices")'],
                ),
            ],
            "mesh_geodesic_iso_region": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_iso_region",
                    ["_rust_geodesic.mesh_geodesic_iso_region("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic_surface.py",
                    "mesh_geodesic_iso_region",
                    ['_require_core_kernel("mesh_geodesic_iso_region")'],
                ),
            ],
            "mesh_geodesic_extreme_edges": [
                (
                    SDK_ROOT / "core" / "mesh.py",
                    "mesh_geodesic_extreme_edges",
                    ["_rust_geodesic.mesh_geodesic_extreme_edges("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_geodesic.py",
                    "mesh_geodesic_extreme_edges",
                    ['_require_core_kernel("mesh_geodesic_extreme_edges")'],
                ),
            ],
            "exact_mesh_intersections": [
                (
                    SDK_ROOT / "spatial" / "intersections.py",
                    "exact_mesh_intersections",
                    ["_rust_intersections.exact_mesh_intersections("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_spatial.py",
                    "exact_mesh_intersections",
                    ['_require_rust_kernel("exact_mesh_intersections")'],
                ),
            ],
            "exact_boolean_mesh": [
                (
                    SDK_ROOT / "spatial" / "boolean.py",
                    "exact_boolean_mesh",
                    ["_rust_mesh_ops.exact_boolean_mesh("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                    "exact_boolean_mesh",
                    ['_require_rust_kernel("exact_boolean_mesh")'],
                ),
            ],
            "detect_ring_regions": [
                (SDK_ROOT / "jewelry" / "regions.py", "detect_ring_regions", ["_rust_jewelry.detect_ring_regions("]),
            ],
        "local_scoop": [
            (SDK_ROOT / "deform" / "local.py", "local_scoop", ["_rust_local_deform.local_offset_vertices("]),
        ],
        "local_thicken_to_minimum": [
            (
                SDK_ROOT / "deform" / "local.py",
                "local_thicken_to_minimum",
                ["_rust_local_deform.local_thicken_to_minimum_vertices("],
            ),
        ],
        "plan_drain_holes": [
            (SDK_ROOT / "jewelry" / "hollow.py", "plan_drain_holes", ["_rust_hollow.plan_drain_holes("]),
        ],
        "prune_small_components": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "prune_small_components",
                ["_require_rust(", "rust.prune_small_components("],
            ),
        ],
        "rebuild_via_sdf": [
            (
                SDK_ROOT / "repair" / "voxel.py",
                "rebuild_via_sdf",
                ["_rust_repair.rebuild_via_sdf("],
            ),
        ],
        "fix_self_intersections_relax": [
            (
                SDK_ROOT / "repair" / "self_intersections.py",
                "fix_self_intersections_relax",
                ["_rust_repair.fix_self_intersections_relax("],
            ),
        ],
        "short_edge_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "short_edge_diagnostics",
                ["_require_rust(", "rust.short_edge_diagnostics("],
            ),
        ],
        "degenerate_face_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "degenerate_face_diagnostics",
                ["_require_rust(", "rust.degenerate_face_diagnostics("],
            ),
        ],
        "multiple_edge_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "multiple_edge_diagnostics",
                ["_require_rust(", "rust.multiple_edge_diagnostics("],
            ),
        ],
        "repair_multiple_edges": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "repair_multiple_edges",
                ["_require_rust(", "rust.repair_multiple_edges("],
            ),
        ],
        "repair_nonmanifold_edges": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "repair_nonmanifold_edges",
                ["_require_rust(", "rust.repair_nonmanifold_edges("],
            ),
        ],
        "duplicate_nonmanifold_vertices": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "duplicate_nonmanifold_vertices",
                ["_require_rust(", "rust.duplicate_nonmanifold_vertices("],
            ),
        ],
        "duplicate_multi_hole_vertices": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "duplicate_multi_hole_vertices",
                ["_require_rust(", "rust.duplicate_multi_hole_vertices("],
            ),
        ],
        "not_smooth_face_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "not_smooth_face_diagnostics",
                ["_require_rust(", "rust.not_smooth_face_diagnostics("],
            ),
        ],
        "crease_edge_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "crease_edge_diagnostics",
                ["_require_rust(", "rust.crease_edge_diagnostics("],
            ),
        ],
        "crease_repair_plan_diagnostics": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "crease_repair_plan_diagnostics",
                ["_require_rust(", "rust.crease_repair_plan_diagnostics("],
            ),
        ],
        "fix_mesh_creases": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "fix_mesh_creases",
                ["_require_rust(", "rust.fix_mesh_creases("],
            ),
        ],
        "unite_close_vertices": [
            (
                SDK_ROOT / "repair" / "basic.py",
                "unite_close_vertices",
                ["_require_rust(", "rust.unite_close_vertices("],
            ),
        ],
        "pairwise_point_to_point_icp": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "pairwise_point_to_point_icp",
                ["_require_rust(", "rust.pairwise_point_to_point_icp("],
            ),
        ],
        "pairwise_point_to_plane_icp": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "pairwise_point_to_plane_icp",
                ["_require_rust(", "rust.pairwise_point_to_plane_icp("],
            ),
        ],
        "multiway_point_to_point_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_point_to_point_icp",
                ["_require_rust(", "rust.multiway_point_to_point_icp("],
            ),
        ],
        "multiway_point_to_plane_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_point_to_plane_icp",
                ["_require_rust(", "rust.multiway_point_to_plane_icp("],
            ),
        ],
        "multiway_combined_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_combined_icp",
                ["_require_rust(", "rust.multiway_combined_icp("],
            ),
        ],
        "multiway_all_object_point_to_point_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_all_object_point_to_point_icp",
                ["_require_rust(", "rust.multiway_all_object_point_to_point_icp("],
            ),
        ],
        "multiway_all_object_point_to_plane_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_all_object_point_to_plane_icp",
                ["_require_rust(", "rust.multiway_all_object_point_to_plane_icp("],
            ),
        ],
        "multiway_all_object_combined_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_all_object_combined_icp",
                ["_require_rust(", "rust.multiway_all_object_combined_icp("],
            ),
        ],
        "multiway_sequential_cascade_point_to_point_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_sequential_cascade_point_to_point_icp",
                ["_require_rust(", "rust.multiway_sequential_cascade_point_to_point_icp("],
            ),
        ],
        "multiway_sequential_cascade_point_to_plane_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_sequential_cascade_point_to_plane_icp",
                ["_require_rust(", "rust.multiway_sequential_cascade_point_to_plane_icp("],
            ),
        ],
        "multiway_sequential_cascade_combined_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_sequential_cascade_combined_icp",
                ["_require_rust(", "rust.multiway_sequential_cascade_combined_icp("],
            ),
        ],
        "multiway_aabb_cascade_point_to_point_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_aabb_cascade_point_to_point_icp",
                ["_require_rust(", "rust.multiway_aabb_cascade_point_to_point_icp("],
            ),
        ],
        "multiway_aabb_cascade_point_to_plane_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_aabb_cascade_point_to_plane_icp",
                ["_require_rust(", "rust.multiway_aabb_cascade_point_to_plane_icp("],
            ),
        ],
        "multiway_aabb_cascade_combined_icp": [
            (
                SDK_ROOT / "point_cloud" / "multiway.py",
                "multiway_aabb_cascade_combined_icp",
                ["_require_rust(", "rust.multiway_aabb_cascade_combined_icp("],
            ),
        ],
        "point_cloud_grid_sample": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_grid_sample",
                ["_require_rust(", "rust.point_cloud_grid_sample_indices("],
            ),
        ],
        "point_cloud_uniform_sample": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_uniform_sample",
                ["_require_rust(", "rust.point_cloud_uniform_sample_indices("],
            ),
        ],
        "point_cloud_nearest_projections": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_nearest_projections",
                ["_require_rust(", "rust.point_cloud_nearest_projections("],
            ),
        ],
        "point_cloud_n_closest_neighbors": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_n_closest_neighbors",
                ["_require_rust(", "rust.point_cloud_n_closest_neighbors("],
            ),
        ],
        "point_cloud_project_to_mesh": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_project_to_mesh",
                ["_require_rust(", "rust.point_cloud_project_to_mesh("],
            ),
        ],
        "point_cloud_two_closest_points": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_two_closest_points",
                ["_require_rust(", "rust.point_cloud_two_closest_points("],
            ),
        ],
        "point_cloud_neighbors_in_radius": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_neighbors_in_radius",
                ["_require_rust(", "rust.point_cloud_neighbors_in_radius("],
            ),
        ],
        "point_cloud_local_neighbor_fan": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_local_neighbor_fan",
                ["_require_rust(", "rust.point_cloud_local_neighbor_fan("],
            ),
        ],
        "point_cloud_local_fan_triangles": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_local_fan_triangles",
                ["_require_rust(", "rust.point_cloud_local_fan_triangles("],
            ),
        ],
        "point_cloud_local_triangulation_repetitions": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_local_triangulation_repetitions",
                ["_require_rust(", "rust.point_cloud_local_triangulation_repetitions("],
            ),
        ],
        "point_cloud_triangulate_candidate_mesh": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_triangulate_candidate_mesh",
                ["_require_rust(", "rust.point_cloud_triangulate_candidate_mesh("],
            ),
        ],
        "point_cloud_triangulate_cleaned_candidate_mesh": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_triangulate_cleaned_candidate_mesh",
                ["_require_rust(", "rust.point_cloud_triangulate_cleaned_candidate_mesh("],
            ),
        ],
        "point_cloud_triangulate_topology_candidate_mesh": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_triangulate_topology_candidate_mesh",
                ["_require_rust(", "rust.point_cloud_triangulate_topology_candidate_mesh("],
            ),
        ],
        "point_cloud_triangulate_filled_candidate_mesh": [
            (
                SDK_ROOT / "point_cloud" / "icp.py",
                "point_cloud_triangulate_filled_candidate_mesh",
                ["_require_rust(", "rust.point_cloud_triangulate_filled_candidate_mesh("],
            ),
        ],
        "distance_map_from_contours": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_from_contours",
                ["_require_rust(", "rust.distance_map_from_contours("],
            ),
        ],
        "distance_map_from_mesh": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_from_mesh",
                ["_require_rust(", "rust.distance_map_from_mesh("],
            ),
        ],
        "object_lines_from_contours": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_from_contours",
                ["_require_rust(", "rust.object_lines_from_contours("],
            ),
        ],
        "object_lines_to_contours": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_to_contours",
                ["_require_rust(", "rust.object_lines_to_contours("],
            ),
        ],
        "object_lines_from_mrlines": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_from_mrlines",
                ["_require_rust(", "rust.object_lines_from_mrlines("],
            ),
        ],
        "object_lines_to_mrlines": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_to_mrlines",
                ["_require_rust(", "rust.object_lines_to_mrlines("],
            ),
        ],
        "object_lines_from_ply": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_from_ply",
                ["_require_rust(", "rust.object_lines_from_ply("],
            ),
        ],
        "object_lines_to_ply": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_to_ply",
                ["_require_rust(", "rust.object_lines_to_ply("],
            ),
        ],
        "object_lines_from_pts": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_from_pts",
                ["_require_rust(", "rust.object_lines_from_pts("],
            ),
        ],
        "object_lines_to_pts": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_to_pts",
                ["_require_rust(", "rust.object_lines_to_pts("],
            ),
        ],
        "object_lines_to_dxf": [
            (
                SDK_ROOT / "distance_map" / "lines.py",
                "object_lines_to_dxf",
                ["_require_rust(", "rust.object_lines_to_dxf("],
            ),
        ],
        "distance_map_to_iso_segments": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_to_iso_segments",
                ["_require_rust(", "rust.distance_map_to_iso_segments("],
            ),
        ],
        "distance_map_merge": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_merge",
                ["_require_rust(", "rust.distance_map_merge("],
            ),
        ],
        "distance_map_contour_boolean": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_contour_boolean",
                ["_require_rust(", "rust.distance_map_contour_boolean("],
            ),
        ],
        "distance_map_from_tiff": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_from_tiff",
                ["_require_rust(", "rust.distance_map_from_tiff("],
            ),
        ],
        "distance_map_to_tiff": [
            (
                SDK_ROOT / "distance_map" / "contours.py",
                "distance_map_to_tiff",
                ["rust.distance_map_to_tiff("],
            ),
        ],
            "parse_gcode_paths": [
                (
                    SDK_ROOT / "gcode" / "paths.py",
                    "parse_gcode_paths",
                    ["_require_rust(", "rust.parse_gcode_paths("],
                ),
            ],
            "sample_sdf_grid": [
                (
                    SDK_ROOT / "voxel" / "sdf.py",
                    "sample_sdf_grid",
                    ["_rust_sdf.sample_sdf_grid("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_sdf.py",
                    "sample_sdf_grid_in_bounds",
                    ['_require_rust_kernel("sample_sdf_grid_in_bounds")'],
                ),
            ],
            "sdf_occupancy": [
                (
                    SDK_ROOT / "voxel" / "sdf.py",
                    "sdf_occupancy",
                    ["_rust_sdf.sdf_occupancy("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_sdf.py",
                    "sdf_occupancy",
                    ['_require_rust_kernel("sdf_occupancy")'],
                ),
            ],
            "estimate_sdf_volume": [
                (
                    SDK_ROOT / "voxel" / "sdf.py",
                    "estimate_sdf_volume",
                    ["_rust_sdf.estimate_sdf_volume("],
                ),
                (
                    SDK_ROOT / "accelerators" / "_rust_sdf.py",
                    "estimate_sdf_volume",
                    ['_require_rust_kernel("estimate_sdf_volume")'],
                ),
            ],
                "extract_sdf_isosurface": [
                    (
                        SDK_ROOT / "voxel" / "marching.py",
                        "extract_marching_tetrahedra",
                        ["_rust_marching.extract_marching_tetrahedra("],
                    ),
                ],
                    "voxel_boolean_mesh": [
                        (
                            SDK_ROOT / "voxel" / "mesh_ops.py",
                            "voxel_boolean_mesh",
                            ["_rust_mesh_ops.voxel_boolean_mesh("],
                        ),
                        (
                            SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                            "voxel_boolean_mesh",
                            ['_require_rust_kernel("voxel_boolean_mesh")'],
                        ),
                    ],
                    "voxel_offset_mesh": [
                        (
                            SDK_ROOT / "voxel" / "mesh_ops.py",
                            "voxel_offset_mesh",
                            ["_rust_mesh_ops.voxel_offset_mesh("],
                        ),
                        (
                            SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                            "voxel_offset_mesh",
                            ['_require_rust_kernel("voxel_offset_mesh")'],
                        ),
                    ],
                    "voxel_shell_mesh": [
                        (
                            SDK_ROOT / "voxel" / "mesh_ops.py",
                            "voxel_shell_mesh",
                            ["_rust_mesh_ops.voxel_shell_mesh("],
                        ),
                        (
                            SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                            "voxel_shell_mesh",
                            ['_require_rust_kernel("voxel_shell_mesh")'],
                        ),
                    ],
                    "voxel_thicken_mesh": [
                        (
                            SDK_ROOT / "voxel" / "mesh_ops.py",
                            "voxel_thicken_mesh",
                            ["_rust_mesh_ops.voxel_thicken_mesh("],
                        ),
                        (
                            SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                            "voxel_thicken_mesh",
                            ['_require_rust_kernel("voxel_thicken_mesh")'],
                        ),
                    ],
                        "voxel_weighted_shell_mesh": [
                            (
                                SDK_ROOT / "voxel" / "mesh_ops.py",
                                "voxel_weighted_shell_mesh",
                                ["_rust_mesh_ops.voxel_weighted_shell_mesh("],
                        ),
                        (
                            SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                            "voxel_weighted_shell_mesh",
                                ['_require_rust_kernel("voxel_weighted_shell_mesh")'],
                            ),
                        ],
                        "voxel_partial_offset_mesh": [
                            (
                                SDK_ROOT / "voxel" / "mesh_ops.py",
                                "voxel_partial_offset_mesh",
                                ["_rust_mesh_ops.voxel_partial_offset_mesh("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_mesh_ops.py",
                                "voxel_partial_offset_mesh",
                                ['_require_rust_kernel("voxel_partial_offset_mesh")'],
                            ),
                        ],
                        "offset_verts_mesh": [
                            (
                                SDK_ROOT / "mesh_edit" / "__init__.py",
                                "offset_verts_mesh",
                                ["_rust_mesh_edit.offset_verts_mesh("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_mesh_edit.py",
                                "offset_verts_mesh",
                                ['_require_rust_kernel("offset_verts_mesh")'],
                            ),
                            ],
                            "decimate_mesh": [
                                (
                                    SDK_ROOT / "mesh_edit" / "__init__.py",
                                    "decimate_mesh",
                                    ["_rust_mesh_edit.decimate_mesh("],
                                ),
                                (
                                    SDK_ROOT / "accelerators" / "_rust_mesh_edit.py",
                                    "decimate_mesh",
                                    ['_require_rust_kernel("decimate_mesh")'],
                                ),
                            ],
                            "offset_contours": [
                            (
                                SDK_ROOT / "distance_map" / "lines.py",
                                "offset_contours",
                                ["rust.offset_contours("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_lines.py",
                                "offset_contours",
                                ['_require_rust_kernel("offset_contours")'],
                            ),
                        ],
                        "offset_contours_with_origins": [
                            (
                                SDK_ROOT / "distance_map" / "lines.py",
                                "offset_contours_with_origins",
                                ["rust.offset_contours_with_origins("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_lines.py",
                                "offset_contours_with_origins",
                                ['_require_rust_kernel("offset_contours_with_origins")'],
                            ),
                        ],
                        "object_lines_from_svg": [
                            (
                                SDK_ROOT / "distance_map" / "lines.py",
                                "object_lines_from_svg",
                                ["rust.object_lines_from_svg("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_lines.py",
                                "object_lines_from_svg",
                                ['_require_rust_kernel("object_lines_from_svg")'],
                            ),
                        ],
                        "load_raw_voxels": [
                            (
                                SDK_ROOT / "voxel" / "raw.py",
                                "load_raw_voxels",
                                ["_rust_voxel.load_raw_voxels("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_raw.py",
                                "load_raw_voxels",
                                ['_require_rust_kernel("load_raw_voxels")'],
                            ),
                        ],
                        "load_raw_voxels_auto": [
                            (
                                SDK_ROOT / "voxel" / "raw.py",
                                "load_raw_voxels_auto",
                                ["_rust_voxel.load_raw_voxels_auto("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_raw.py",
                                "load_raw_voxels_auto",
                                ['_require_rust_kernel("load_raw_voxels_auto")'],
                            ),
                        ],
                        "load_tiff_voxels_dir": [
                            (
                                SDK_ROOT / "voxel" / "raw.py",
                                "load_tiff_voxels_dir",
                                ["_rust_voxel.load_tiff_voxels_dir("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_raw.py",
                                "load_tiff_voxels_dir",
                                ['_require_rust_kernel("load_tiff_voxels_dir")'],
                            ),
                        ],
                        "voxel_binary_values": [
                            (
                                SDK_ROOT / "voxel" / "ops.py",
                                "voxel_binary_values",
                                ["_rust_voxel.voxel_binary_values_required("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_ops.py",
                                "voxel_binary_values_required",
                                ['_require_rust_kernel("voxel_binary_values")'],
                            ),
                        ],
                        "voxel_binary_iso_value": [
                            (
                                SDK_ROOT / "voxel" / "ops.py",
                                "voxel_binary_iso_value",
                                ["_rust_voxel.voxel_binary_iso_value("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_ops.py",
                                "voxel_binary_iso_value",
                                ['_require_rust_kernel("voxel_binary_iso_value")'],
                            ),
                        ],
                        "voxel_default_iso_value": [
                            (
                                SDK_ROOT / "voxel" / "raw.py",
                                "voxel_default_iso_value",
                                ["_rust_voxel.voxel_default_iso_value("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_ops.py",
                                "voxel_default_iso_value",
                                ['_require_rust_kernel("voxel_default_iso_value")'],
                            ),
                        ],
                        "voxel_to_mesh_simple": [
                            (
                                SDK_ROOT / "voxel" / "conversion.py",
                                "voxel_to_mesh_simple",
                                ["_rust_voxel.voxel_to_mesh_simple_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_conversion.py",
                                "voxel_to_mesh_simple_values",
                                ['_require_rust_kernel("voxel_to_mesh_simple_values")'],
                            ),
                        ],
                        "voxel_to_mesh_dual": [
                            (
                                SDK_ROOT / "voxel" / "conversion.py",
                                "voxel_to_mesh_dual",
                                ["_rust_voxel.voxel_to_mesh_dual_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_conversion.py",
                                "voxel_to_mesh_dual_values",
                                ['_require_rust_kernel("voxel_to_mesh_dual_values_with_settings")'],
                            ),
                        ],
                        "voxel_move_mesh_to_max_deriv": [
                            (
                                SDK_ROOT / "voxel" / "conversion.py",
                                "voxel_move_mesh_to_max_deriv",
                                ["_rust_voxel.voxel_move_mesh_to_max_deriv_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_conversion.py",
                                "voxel_move_mesh_to_max_deriv_values",
                                ['_require_rust_kernel("voxel_move_mesh_to_max_deriv_values")'],
                            ),
                        ],
                        "voxel_to_mesh_smart": [
                            (
                                SDK_ROOT / "voxel" / "conversion.py",
                                "voxel_to_mesh_smart",
                                ["_rust_voxel.voxel_to_mesh_smart_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_conversion.py",
                                "voxel_to_mesh_smart_values",
                                ['_require_rust_kernel("voxel_to_mesh_smart_values")'],
                            ),
                        ],
                        "voxel_path": [
                            (
                                SDK_ROOT / "voxel" / "path.py",
                                "voxel_path",
                                ["_rust_voxel.voxel_path_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_sampling.py",
                                "voxel_path_values",
                                ['_require_rust_kernel("voxel_path_values")'],
                            ),
                        ],
                        "voxel_path_build_four": [
                            (
                                SDK_ROOT / "voxel" / "path.py",
                                "voxel_path_build_four",
                                ["_rust_voxel.voxel_path_build_four_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_sampling.py",
                                "voxel_path_build_four_values",
                                ['_require_rust_kernel("voxel_path_build_four_values")'],
                            ),
                        ],
                        "voxel_slice": [
                            (
                                SDK_ROOT / "voxel" / "slice.py",
                                "voxel_slice",
                                ["_rust_voxel.voxel_slice_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_sampling.py",
                                "voxel_slice_values",
                                ['_require_rust_kernel("voxel_slice_values")'],
                            ),
                        ],
                        "voxel_line_graph": [
                            (
                                SDK_ROOT / "voxel" / "line_graph.py",
                                "voxel_line_graph",
                                ["_rust_voxel.voxel_line_graph_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_sampling.py",
                                "voxel_line_graph_values",
                                ['_require_rust_kernel("voxel_line_graph_values")'],
                            ),
                        ],
                        "voxel_active_box": [
                            (
                                SDK_ROOT / "voxel" / "active_box.py",
                                "voxel_active_box",
                                ["_rust_voxel.voxel_active_box_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_sampling.py",
                                "voxel_active_box_values",
                                ['_require_rust_kernel("voxel_active_box_values")'],
                            ),
                        ],
                        "voxel_volume_render_data": [
                            (
                                SDK_ROOT / "voxel" / "rendering.py",
                                "voxel_volume_render_data",
                                ["_rust_voxel.voxel_volume_render_data_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_rendering.py",
                                "voxel_volume_render_data_values",
                                ['_require_rust_kernel("voxel_volume_render_data_values")'],
                            ),
                        ],
                        "voxel_volume_render_lut": [
                            (
                                SDK_ROOT / "voxel" / "rendering.py",
                                "voxel_volume_render_lut",
                                ["_rust_voxel.voxel_volume_render_lut_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_rendering.py",
                                "voxel_volume_render_lut_values",
                                ['_require_rust_kernel("voxel_volume_render_lut_values")'],
                            ),
                        ],
                        "voxel_volume_render_ray": [
                            (
                                SDK_ROOT / "voxel" / "rendering.py",
                                "voxel_volume_render_ray",
                                ["_rust_voxel.voxel_volume_render_ray_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_rendering.py",
                                "voxel_volume_render_ray_values",
                                ['_require_rust_kernel("voxel_volume_render_ray_values")'],
                            ),
                        ],
                        "voxel_segmentation": [
                            (
                                SDK_ROOT / "voxel" / "segmentation.py",
                                "voxel_segmentation",
                                ["_rust_voxel.voxel_segmentation_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_segmentation.py",
                                "voxel_segmentation_values",
                                ['_require_rust_kernel("voxel_segmentation_values")'],
                            ),
                        ],
                        "voxel_segmentation_mesh": [
                            (
                                SDK_ROOT / "voxel" / "segmentation.py",
                                "voxel_segmentation_mesh",
                                ["_rust_voxel.voxel_segmentation_mesh_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_segmentation.py",
                                "voxel_segmentation_mesh_values",
                                ['_require_rust_kernel("voxel_segmentation_mesh_values")'],
                            ),
                        ],
                        "voxel_mask_to_mesh": [
                            (
                                SDK_ROOT / "voxel" / "segmentation.py",
                                "voxel_mask_to_mesh",
                                ["_rust_voxel.voxel_mask_to_mesh_values("],
                            ),
                            (
                                SDK_ROOT / "accelerators" / "_rust_voxel_segmentation.py",
                                "voxel_mask_to_mesh_values",
                                ['_require_rust_kernel("voxel_mask_to_mesh_values")'],
                            ),
                        ],
                        "load_gcode_source": [
                        (
                            SDK_ROOT / "gcode" / "paths.py",
                "load_gcode_source",
                ["_require_rust(", "rust.load_gcode_source("],
            ),
        ],
        "write_gcode_source": [
            (
                SDK_ROOT / "gcode" / "paths.py",
                "write_gcode_source",
                ["rust.write_gcode_source("],
            ),
        ],
        "parse_gcode_file_paths": [
            (
                SDK_ROOT / "gcode" / "paths.py",
                "parse_gcode_file_paths",
                ["_require_rust(", "rust.parse_gcode_file_paths("],
            ),
        ],
        "protected_hollow_mesh": [
            (SDK_ROOT / "jewelry" / "hollow.py", "protected_hollow_mesh", ["_rust_hollow.protected_hollow_mesh("]),
        ],
        "resize_ring": [
            (SDK_ROOT / "deform" / "resize.py", "resize_ring", ["_require_rust(", "rust.resize_ring_vertices("]),
        ],
        "section_contour": [
            (SDK_ROOT / "analysis" / "section.py", "section_contour", ["_rust_analysis.section_contour("]),
        ],
        "service_compare": [
            (SDK_ROOT / "analysis" / "compare.py", "service_compare_summary", ["_rust_compare.service_compare_summary("]),
        ],
        "service_compare_field": [
            (SDK_ROOT / "analysis" / "compare.py", "service_compare_distances", ["_rust_compare.service_compare_distances("]),
        ],
        "service_fill_holes": [
            (SDK_ROOT / "repair" / "holes.py", "service_fill_holes", ["_require_rust(", "rust.service_fill_holes("]),
        ],
        "service_health": [
            (SDK_ROOT / "analysis" / "health.py", "service_mesh_health", ["_rust_health.service_mesh_health("]),
        ],
        "smooth": [
            (
                SDK_ROOT / "deform" / "local.py",
                "smooth",
                ["_rust_local_deform.taubin_smooth_vertices(", "_rust_local_deform.smooth_vertices_with_falloff("],
            ),
        ],
        "subdivide_mesh": [
            (
                SDK_ROOT / "mesh_edit" / "__init__.py",
                "subdivide_mesh",
                ["_rust_mesh_edit.subdivide_mesh("],
            ),
        ],
        "make_delone_edge_flips": [
            (
                SDK_ROOT / "mesh_edit" / "__init__.py",
                "make_delone_edge_flips",
                ["_rust_mesh_edit.make_delone_edge_flips("],
            ),
            (
                SDK_ROOT / "accelerators" / "_rust_mesh_edit.py",
                "make_delone_edge_flips",
                ['_require_rust_kernel("make_delone_edge_flips")'],
            ),
        ],
    }
    artifact_operation_markers = {
        "thickness_overlay_payload": [
            (SDK_ROOT / "analysis" / "artifacts.py", "thickness_overlay_payload", ["load_thickness_npz("]),
            (
                BACKEND_ROOT / "services" / "manufacturability.py",
                "compute_manufacturability_snapshot",
                ["default_sdk.service_thickness(", "default_sdk.save_thickness_npz("],
            ),
        ],
    }

    unmapped = sorted(rust_operations - set(direct_operation_markers) - set(artifact_operation_markers))
    assert unmapped == []

    missing_markers: dict[str, list[str]] = {}
    for operation in sorted(rust_operations):
        marker_groups = direct_operation_markers.get(operation) or artifact_operation_markers.get(operation) or []
        for path, function_name, markers in marker_groups:
            source = _function_source(path, function_name)
            missing = [marker for marker in markers if marker not in source]
            if missing:
                missing_markers.setdefault(operation, []).extend(f"{path.relative_to(BACKEND_ROOT)}:{marker}" for marker in missing)

    assert missing_markers == {}


def test_hollow_drain_workspace_commands_advertise_rust_drain_hole_support() -> None:
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    operations_source = (BACKEND_ROOT / "services" / "operations.py").read_text()
    sdk_engine = (SDK_ROOT / "engine.py").read_text()

    assert "case 'prepare-casting'" in tool_inspector
    assert "case 'hollow-drains'" in tool_inspector
    assert "add_drain_holes: true" in tool_inspector

    assert "default_sdk.plan_drain_holes(" in operations_source
    assert "default_sdk.apply_drain_holes_voxel(" in operations_source
    assert "def plan_drain_holes(" in sdk_engine
    assert "def apply_drain_holes_voxel(" in sdk_engine

    for command_id in ["prepare-casting", "hollow-drains"]:
        command_index = versions_source.index(f'"command_id": "{command_id}"')
        next_command_index = versions_source.find('"command_id": "', command_index + 1)
        block = versions_source[command_index: next_command_index if next_command_index != -1 else len(versions_source)]
        assert '"endpoint_url_key": "hollow_endpoint_url"' in block
        assert '"rust_backed": True' in block
        assert '"protected_hollow_mesh"' in block
        assert '"plan_drain_holes"' in block
        assert '"apply_drain_holes_voxel"' in block


def test_workbench_host_absolutizes_command_capability_endpoint_urls_for_runtime() -> None:
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "command_capabilities: manifest.command_capabilities.map" in host
    assert "endpoint_url: absolutize(capability.endpoint_url)" in host
    assert "meshinspectorWorkbenchBackendEndpointCount" in bridge
    assert "meshinspectorWorkbenchRelativeEndpointCount" in bridge
    assert "meshinspectorWorkbenchBackendEndpointCount" in runtime_bootstrap
    assert "meshinspectorWorkbenchRelativeEndpointCount" in runtime_bootstrap


def test_subdivide_mesh_official_workbench_command_is_reachable_from_frontend() -> None:
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_models = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "export interface SubdivideRequestV2" in frontend_types
    assert "smooth_mode: boolean" in frontend_types
    assert "min_sharp_dihedral_angle: number" in frontend_types
    assert "region_faces?: number[]" in frontend_types
    assert "not_flippable_edges?: [number, number][]" in frontend_types
    assert "max_deviation_after_flip?: number | null" in frontend_types
    assert "max_angle_change_after_flip?: number | null" in frontend_types
    assert "critical_tri_aspect_ratio_flip?: number | null" in frontend_types
    assert "export async function submitSubdivide" in frontend_models
    assert 'fetchApi(`/api/versions/${versionId}/subdivide`' in frontend_models
    assert "useSubdivideOperation = createOperationMutation<SubdivideRequestV2>(submitSubdivide)" in frontend_hooks
    assert "| 'subdivide-mesh'" in workspace_types
    assert "id: 'subdivide-mesh'" in tool_registry
    assert "contextualToolId: 'subdivide-mesh'" in tool_registry
    assert "case 'subdivide-mesh'" in tool_inspector
    assert "onSubdivide" in tool_inspector
    assert "subdivideRegionFaces" in tool_inspector
    assert "subdivideNotFlippableEdges" in tool_inspector
    assert "subdivideMaxDeviationAfterFlip" in tool_inspector
    assert "subdivideMaxAngleChangeAfterFlip" in tool_inspector
    assert "subdivideCriticalTriAspectRatioFlip" in tool_inspector
    assert "region_faces: parseIndexList(drafts.subdivideRegionFaces)" in tool_inspector
    assert "not_flippable_edges: parseEdgePairs(drafts.subdivideNotFlippableEdges)" in tool_inspector
    assert "max_deviation_after_flip:" in tool_inspector
    assert "max_angle_change_after_flip:" in tool_inspector
    assert "critical_tri_aspect_ratio_flip:" in tool_inspector
    assert "subdivideMutation = useSubdivideOperation()" in viewer_page
    assert "subdivideRequestFromWorkbenchPayload" in viewer_page
    assert "'region_faces', 'regionFaces', 'face_region', 'faceRegion'" in viewer_page
    assert "'not_flippable_edges', 'notFlippableEdges', 'protected_edges', 'protectedEdges'" in viewer_page
    assert "'max_deviation_after_flip', 'maxDeviationAfterFlip', 'maxDeviation', 'deviation'" in viewer_page
    assert "'max_angle_change_after_flip', 'maxAngleChangeAfterFlip', 'max_angle_change', 'maxAngleChange'" in viewer_page
    assert "'critical_tri_aspect_ratio_flip', 'criticalAspectRatioFlip', 'critical_tri_aspect_ratio', 'criticalTriAspectRatio'" in viewer_page
    assert "subdivideMutation.mutateAsync({ versionId, params: request })" in viewer_page
    assert "'Subdivide Mesh': 'subdivide-mesh'" in runtime_bootstrap
    assert "'SubdivideMeshTool': 'subdivide-mesh'" in runtime_bootstrap
    assert "commandId: 'subdivide-mesh'" in runtime_bootstrap


def test_make_delone_official_workbench_command_is_reachable_from_frontend() -> None:
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_models = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    assert "export interface MakeDeloneRequestV2" in frontend_types
    assert "num_iters: number" in frontend_types
    assert "region_faces?: number[]" in frontend_types
    assert "max_deviation_after_flip?: number | null" in frontend_types
    assert "max_angle_change?: number | null" in frontend_types
    assert "critical_tri_aspect_ratio?: number | null" in frontend_types
    assert "not_flippable_edges?: [number, number][]" in frontend_types
    assert "vert_region?: number[]" in frontend_types
    assert "export async function submitMakeDelone" in frontend_models
    assert 'fetchApi(`/api/versions/${versionId}/make-delone`' in frontend_models
    assert "useMakeDeloneOperation = createOperationMutation<MakeDeloneRequestV2>(submitMakeDelone)" in frontend_hooks
    assert "| 'make-delone'" in workspace_types
    assert "id: 'make-delone'" in tool_registry
    assert "contextualToolId: 'make-delone'" in tool_registry
    assert "case 'make-delone'" in tool_inspector
    assert "onMakeDelone" in tool_inspector
    assert "makeDeloneMaxDeviationAfterFlip" in tool_inspector
    assert "makeDeloneMaxAngleChange" in tool_inspector
    assert "makeDeloneCriticalTriAspectRatio" in tool_inspector
    assert "makeDeloneNotFlippableEdges" in tool_inspector
    assert "makeDeloneVertRegion" in tool_inspector
    assert "makeDeloneMutation = useMakeDeloneOperation()" in viewer_page
    assert "makeDeloneRequestFromWorkbenchPayload" in viewer_page
    assert "max_deviation_after_flip" in viewer_page
    assert "max_angle_change" in viewer_page
    assert "critical_tri_aspect_ratio" in viewer_page
    assert "not_flippable_edges" in viewer_page
    assert "'protectedEdges'" in viewer_page
    assert "vert_region" in viewer_page
    assert "'vertRegion'" in viewer_page
    assert "makeDeloneMutation.mutateAsync({ versionId, params: request })" in viewer_page
    assert "'Make Delone': 'make-delone'" in runtime_bootstrap
    assert "'MakeDeloneTool': 'make-delone'" in runtime_bootstrap
    assert "commandId: 'make-delone'" in runtime_bootstrap
    assert "'Make Delone': 'make-delone'" in host
    assert "MakeDeloneTool: 'make-delone'" in host


def test_decimate_mesh_official_workbench_command_is_reachable_from_frontend() -> None:
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_models = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    assert "export interface DecimateRequestV2" in frontend_types
    assert "strategy: 'minimize_error' | 'shortest_edge_first'" in frontend_types
    assert "target_face_count?: number | null" in frontend_types
    assert "target_face_ratio?: number | null" in frontend_types
    assert "subdivide_parts: number" in frontend_types
    assert "decimate_between_parts: boolean" in frontend_types
    assert "max_triangle_aspect_ratio: number" in frontend_types
    assert "max_bd_shift?: number | null" in frontend_types
    assert "stabilizer: number" in frontend_types
    assert "region_faces?: number[]" in frontend_types
    assert "not_flippable_edges?: [number, number][]" in frontend_types
    assert "collapse_near_not_flippable: boolean" in frontend_types
    assert "angle_weighted_dist_to_plane: boolean" in frontend_types
    assert "touch_near_bd_edges: boolean" in frontend_types
    assert "touch_bd_verts: boolean" in frontend_types
    assert "metadata?: Record<string, unknown>" in frontend_types
    assert "export async function submitDecimate" in frontend_models
    assert 'fetchApi(`/api/versions/${versionId}/decimate`' in frontend_models
    assert "useDecimateOperation = createOperationMutation<DecimateRequestV2>(submitDecimate)" in frontend_hooks
    assert "| 'decimate-mesh'" in workspace_types
    assert "id: 'decimate-mesh'" in tool_registry
    assert "contextualToolId: 'decimate-mesh'" in tool_registry
    assert "case 'decimate-mesh'" in tool_inspector
    assert '<option value="minimize_error">Minimize Error</option>' in tool_inspector
    assert 'label="Target Faces"' in tool_inspector
    assert 'label="Target %"' in tool_inspector
    assert 'label="Parallel Algorithm"' in tool_inspector
    assert 'label="Subdivide Parts"' in tool_inspector
    assert 'label="Max Tri Aspect"' in tool_inspector
    assert 'label="Max Boundary Shift"' in tool_inspector
    assert 'label="Stabilizer"' in tool_inspector
    assert "Region Faces" in tool_inspector
    assert "Not Flippable Edges" in tool_inspector
    assert 'label="Collapse Near Protected"' in tool_inspector
    assert 'label="Angle Weighted Planes"' in tool_inspector
    assert 'label="Touch Boundary Edges"' in tool_inspector
    assert 'label="Touch Boundary Verts"' in tool_inspector
    assert "onDecimate" in tool_inspector
    assert "decimateMutation = useDecimateOperation()" in viewer_page
    assert "decimateRequestFromWorkbenchPayload" in viewer_page
    assert "target_face_count:" in viewer_page
    assert "'target_face_count', 'targetFaceCount', 'target_triangles', 'targetTriangles'" in viewer_page
    assert "target_face_ratio:" in viewer_page
    assert "'target_face_ratio', 'targetFaceRatio', 'target_percentage', 'targetPercentage'" in viewer_page
    assert "hasExplicitTargetFaces = hasAnyPayloadKey(payload, targetFaceKeys)" in viewer_page
    assert "hasExplicitTargetRatio = hasAnyPayloadKey(payload, targetRatioKeys)" in viewer_page
    assert "hasExplicitTargetRatio ? 0 : fallbackTargetFaces" in viewer_page
    assert "hasExplicitTargetFaces ? 0 : fallbackTargetPercent > 0" in viewer_page
    assert "subdivide_parts:" in viewer_page
    assert "'subdivide_parts', 'subdivideParts', 'parallel_parts', 'parallelParts'" in viewer_page
    assert "decimate_between_parts:" in viewer_page
    assert "'decimate_between_parts', 'decimateBetweenParts'" in viewer_page
    assert "region_faces: integerListFromPayload" in viewer_page
    assert "'region_faces', 'regionFaces', 'face_region', 'faceRegion'" in viewer_page
    assert "metadata: recordFromUnknown(payload.metadata)" in viewer_page
    assert "stabilizer:" in viewer_page
    assert "numberFromPayload(payload, ['stabilizer', 'qemStabilizer']" in viewer_page
    assert "angle_weighted_dist_to_plane: booleanFromPayload" in viewer_page
    assert "'angle_weighted_dist_to_plane', 'angleWeightedDistToPlane', 'angle_weighted_planes', 'angleWeightedPlanes'" in viewer_page
    assert "decimateMutation.mutateAsync({ versionId, params: request })" in viewer_page
    assert "'Decimate Mesh': 'decimate-mesh'" in runtime_bootstrap
    assert "'DecimateMeshTool': 'decimate-mesh'" in runtime_bootstrap
    assert "WORKBENCH_CANVAS_COMMAND_OVERLAYS" in runtime_bootstrap
    assert "label: 'Decimate Mesh'" in runtime_bootstrap
    assert "commandId: 'decimate-mesh'" in runtime_bootstrap
    assert "meshinspector-workbench-command-overlay" in runtime_bootstrap
    assert "{ tab: 'mesh-edit', minX: 526, maxX: 604" in runtime_bootstrap
    assert "'Decimate Mesh': 'decimate-mesh'" in host
    assert "DecimateMeshTool: 'decimate-mesh'" in host


def test_workbench_decimate_defaults_are_deletion_capped_not_target_based() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    decimate_payloads = re.findall(
        r"commandId: 'decimate-mesh'.*?payload:\s*\{(?P<payload>.*?)\},\s*options:",
        runtime_bootstrap,
        flags=re.DOTALL,
    )

    assert decimate_payloads
    for payload in decimate_payloads:
        assert "target_face_count" not in payload
        assert "targetFaceCount" not in payload
        assert "target_face_ratio" not in payload
        assert "targetFaceRatio" not in payload
        assert "target_percentage" not in payload
        assert "targetPercentage" not in payload
        assert "max_deleted_vertices" in payload
        assert "max_deleted_faces" in payload


def test_workbench_command_execute_false_is_a_hard_stop() -> None:
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert "function explicitBooleanFromPayload" in viewer_page
    assert "explicitBooleanFromPayload(invocation.options, ['execute', 'auto_execute', 'submit'])" in viewer_page
    assert "explicitBooleanFromPayload(invocation.payload, ['execute', 'auto_execute', 'submit'])" in viewer_page
    assert "optionExecuteFlag === false || payloadExecuteFlag === false" in viewer_page


def test_mesh_cut_measure_path_official_workbench_command_uses_rust_topology_endpoint() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    plugin_manifest = json.loads((BACKEND_ROOT.parent / "meshlib-workbench" / "MeshInspectorWorkbenchPlugin.items.json").read_text())
    runtime_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    workbench_cases = (FRONTEND_ROOT / "e2e" / "fixtures" / "workbenchCommandCases.ts").read_text()

    assert '"command_id": "mesh-cut-measure-path"' in versions_source
    assert '"endpoint_url_key": "mesh_cut_measure_topology_endpoint_url"' in versions_source
    assert '"mesh_geodesic_polyline_path"' in versions_source
    assert '"mesh_cut_measure_contours"' in versions_source
    assert '"mesh_cut_measure_edge_path_topology_cut"' in versions_source
    assert '"object_lines_from_contours"' in versions_source

    for manifest in (plugin_manifest, runtime_manifest):
        item = next(
            (entry for entry in manifest.get("Items", []) if entry.get("CommandId") == "mesh-cut-measure-path"),
            None,
        )
        assert item is not None
        assert item["Name"] == "Mesh Cut & Measure Path"
        assert item["MissingBackendOperation"] is False
        assert item["EndpointUrlKey"] == "mesh_cut_measure_topology_endpoint_url"
        assert "MR::buildShortestPath" in item["Tooltip"]
        assert "MR::convertSurfacePathsToMeshContours / MR::cutMesh" in item["Tooltip"]
        assert "topology endpoint" in item["Tooltip"]

    assert "| 'mesh-cut-measure-path'" in workspace_types
    assert "id: 'mesh-cut-measure-path'" in tool_registry
    assert "contextualToolId: 'mesh-cut-measure-path'" in tool_registry
    assert "case 'mesh-cut-measure-path'" in viewer_page
    assert "measureInspectRequestFromWorkbenchPayload(workbenchRequest)" in viewer_page
    assert "'Mesh Cut & Measure Path': 'mesh-cut-measure-path'" in host
    assert "MeshCutMeasurePathTool: 'mesh-cut-measure-path'" in host
    assert "commandId: 'mesh-cut-measure-path'" in workbench_cases
    assert "endpointKey: 'mesh_cut_measure_topology_endpoint_url'" in workbench_cases
    assert "control_vertices: [0, 1]" in workbench_cases


def test_official_gcode_path_parser_routes_through_versioned_rust_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "G-code Path Parser" in item_names
    parser_item = next(item for item in plugin_manifest.get("Items", []) if item.get("CommandId") == "gcode-parse-paths")
    assert parser_item["RustBacked"] is True

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert '"/versions/{version_id}/gcode/parse-paths"' in versions_source
    assert '"command_id": "gcode-parse-paths"' in versions_source
    assert '"endpoint_url_key": "gcode_parse_paths_endpoint_url"' in versions_source
    assert '"sdk_operations": ["parse_gcode_paths"]' in versions_source
    assert "default_sdk.parse_gcode_paths(" in versions_source
    assert "GcodeParsePathsRequest" in schemas
    assert "GcodeParsePathsResponse" in schemas
    assert "GcodeParsePathsRequest" in frontend_types
    assert "GcodeParsePathsResponse" in frontend_types
    assert "submitGcodeParsePaths(" in frontend_api
    assert "useGcodeParsePathsOperation" in frontend_hooks
    assert "| 'gcode-parse-paths'" in workspace_types
    assert "gcodeSource: string" in workspace_types
    assert "id: 'gcode-parse-paths'" in tool_registry
    assert "label: 'G-code Path Parser'" in tool_registry
    assert "contextualToolId: 'gcode-parse-paths'" in tool_registry
    assert "case 'gcode-parse-paths'" in tool_inspector
    assert "onGcodeParse" in tool_inspector
    assert "gcodeParseResult" in tool_inspector
    assert "gcodeParseMutation = useGcodeParsePathsOperation()" in viewer_page
    assert "setGcodeParseResult" in viewer_page
    assert "case 'gcode-parse-paths'" in viewer_page
    assert "gcodeRequestFromWorkbenchPayload" in viewer_page


def test_official_mesh_to_voxels_sdf_routes_through_versioned_rust_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Mesh to Voxels / SDF" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    engine = (SDK_ROOT / "engine.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert '"/versions/{version_id}/voxels/mesh-to-sdf"' in versions_source
    assert '"command_id": "mesh-to-voxels-sdf"' in versions_source
    assert '"endpoint_url_key": "voxelize_mesh_endpoint_url"' in versions_source
    assert '"sdk_operations": ["sample_sdf_grid", "sdf_occupancy", "estimate_sdf_volume", "extract_sdf_isosurface"]' in versions_source
    assert "default_sdk.sample_sdf_grid(" in versions_source
    assert "default_sdk.sdf_occupancy(" in versions_source
    assert "default_sdk.estimate_sdf_volume(" in versions_source
    assert "default_sdk.extract_sdf_isosurface(" in versions_source
    assert "def sdf_occupancy(" in engine
    assert "def estimate_sdf_volume(" in engine
    assert "MeshToVoxelsSdfRequest" in schemas
    assert "MeshToVoxelsSdfResponse" in schemas
    assert "MeshToVoxelsSdfRequest" in frontend_types
    assert "MeshToVoxelsSdfResponse" in frontend_types
    assert "submitMeshToVoxelsSdf(" in frontend_api
    assert "useMeshToVoxelsSdfOperation" in frontend_hooks
    assert "| 'mesh-to-voxels-sdf'" in workspace_types
    assert "voxelSizeMm: number" in workspace_types
    assert "id: 'mesh-to-voxels-sdf'" in tool_registry
    assert "label: 'Mesh to Voxels / SDF'" in tool_registry
    assert "contextualToolId: 'mesh-to-voxels-sdf'" in tool_registry
    assert "case 'mesh-to-voxels-sdf'" in tool_inspector
    assert "onMeshToVoxelsSdf" in tool_inspector
    assert "meshToVoxelsResult" in tool_inspector
    assert "meshToVoxelsMutation = useMeshToVoxelsSdfOperation()" in viewer_page
    assert "setMeshToVoxelsResult" in viewer_page
    assert "case 'mesh-to-voxels-sdf'" in viewer_page
    assert "meshToVoxelsRequestFromWorkbenchPayload" in viewer_page


def test_official_collision_detection_routes_through_versioned_rust_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Collision Detection" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    engine = (SDK_ROOT / "engine.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert '"/versions/{version_id}/collision/detect"' in versions_source
    assert '"command_id": "collision-detect"' in versions_source
    assert '"endpoint_url_key": "collision_endpoint_url"' in versions_source
    assert '"sdk_operations": ["exact_mesh_intersections"]' in versions_source
    assert "default_sdk.exact_mesh_intersections(" in versions_source
    assert "def exact_mesh_intersections(" in engine
    assert "CollisionDetectRequest" in schemas
    assert "CollisionDetectResponse" in schemas
    assert "CollisionDetectRequest" in frontend_types
    assert "CollisionDetectResponse" in frontend_types
    assert "submitCollisionDetect(" in frontend_api
    assert "useCollisionDetectOperation" in frontend_hooks
    assert "| 'collision-detect'" in workspace_types
    assert "collisionTargetVersionId: string" in workspace_types
    assert "id: 'collision-detect'" in tool_registry
    assert "label: 'Collision Detection'" in tool_registry
    assert "contextualToolId: 'collision-detect'" in tool_registry
    assert "case 'collision-detect'" in tool_inspector
    assert "onCollisionDetect" in tool_inspector
    assert "collisionResult" in tool_inspector
    assert "collisionMutation = useCollisionDetectOperation()" in viewer_page
    assert "setCollisionResult" in viewer_page
    assert "case 'collision-detect'" in viewer_page
    assert "collisionRequestFromWorkbenchPayload" in viewer_page
    assert "'Collision Detection': 'collision-detect'" in runtime_bootstrap
    assert "'CollisionDetectionTool': 'collision-detect'" in runtime_bootstrap


def test_official_exact_boolean_routes_through_versioned_rust_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Exact Boolean" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    engine = (SDK_ROOT / "engine.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert '"/versions/{version_id}/boolean/exact"' in versions_source
    assert '"command_id": "exact-boolean"' in versions_source
    assert '"endpoint_url_key": "exact_boolean_endpoint_url"' in versions_source
    assert '"sdk_operations": ["exact_boolean_mesh"]' in versions_source
    assert "default_sdk.exact_boolean_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.exact_boolean_mesh, ...)
    assert "def exact_boolean_mesh(" in engine
    assert "ExactBooleanRequest" in schemas
    assert "ExactBooleanResponse" in schemas
    assert "ExactBooleanRequest" in frontend_types
    assert "ExactBooleanResponse" in frontend_types
    assert "submitExactBoolean(" in frontend_api
    assert "useExactBooleanOperation" in frontend_hooks
    assert "| 'exact-boolean'" in workspace_types
    assert "booleanTargetVersionId: string" in workspace_types
    assert "booleanOperation:" in workspace_types
    assert "id: 'exact-boolean'" in tool_registry
    assert "label: 'Exact Boolean'" in tool_registry
    assert "contextualToolId: 'exact-boolean'" in tool_registry
    assert "case 'exact-boolean'" in tool_inspector
    assert "onExactBoolean" in tool_inspector
    assert "exactBooleanResult" in tool_inspector
    assert "exactBooleanMutation = useExactBooleanOperation()" in viewer_page
    assert "setExactBooleanResult" in viewer_page
    assert "case 'exact-boolean'" in viewer_page
    assert "exactBooleanRequestFromWorkbenchPayload" in viewer_page
    assert "'Exact Boolean': 'exact-boolean'" in runtime_bootstrap
    assert "'ExactBooleanTool': 'exact-boolean'" in runtime_bootstrap


def test_official_voxel_boolean_routes_through_versioned_rust_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Voxel Boolean" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    engine = (SDK_ROOT / "engine.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert '"/versions/{version_id}/boolean/voxel"' in versions_source
    assert '"command_id": "voxel-boolean"' in versions_source
    assert '"endpoint_url_key": "voxel_boolean_endpoint_url"' in versions_source
    assert '"sdk_operations": ["voxel_boolean_mesh"]' in versions_source
    assert "default_sdk.voxel_boolean_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.voxel_boolean_mesh, ...)
    assert "def voxel_boolean_mesh(" in engine
    assert "VoxelBooleanRequest" in schemas
    assert "VoxelBooleanResponse" in schemas
    assert "VoxelBooleanRequest" in frontend_types
    assert "VoxelBooleanResponse" in frontend_types
    assert "submitVoxelBoolean(" in frontend_api
    assert "useVoxelBooleanOperation" in frontend_hooks
    assert "| 'voxel-boolean'" in workspace_types
    assert "voxelBooleanTargetVersionId: string" in workspace_types
    assert "voxelBooleanOperation:" in workspace_types
    assert "id: 'voxel-boolean'" in tool_registry
    assert "label: 'Voxel Boolean'" in tool_registry
    assert "contextualToolId: 'voxel-boolean'" in tool_registry
    assert "case 'voxel-boolean'" in tool_inspector
    assert "onVoxelBoolean" in tool_inspector
    assert "voxelBooleanResult" in tool_inspector
    assert "voxelBooleanMutation = useVoxelBooleanOperation()" in viewer_page
    assert "setVoxelBooleanResult" in viewer_page
    assert "case 'voxel-boolean'" in viewer_page
    assert "voxelBooleanRequestFromWorkbenchPayload" in viewer_page
    assert "'Voxel Boolean': 'voxel-boolean'" in runtime_bootstrap
    assert "'VoxelBooleanTool': 'voxel-boolean'" in runtime_bootstrap


def test_official_offset_shell_routes_through_versioned_rust_sdk_endpoints() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Offset Mesh" in item_names
    assert "Shell Mesh" in item_names
    assert "Thickening" in item_names
    assert "Weighted Shell" in item_names
    assert "Partial Offset" in item_names
    assert "Expand/Shrink" in item_names
    assert "Shrink/Expand" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    engine = (SDK_ROOT / "engine.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    assert '"/versions/{version_id}/offset/voxel"' in versions_source
    assert '"/versions/{version_id}/shell/voxel"' in versions_source
    assert '"/versions/{version_id}/offset/thicken"' in versions_source
    assert '"/versions/{version_id}/offset/weighted-shell"' in versions_source
    assert '"/versions/{version_id}/offset/partial"' in versions_source
    assert '"/versions/{version_id}/offset/verts"' in versions_source
    assert '"/versions/{version_id}/offset/expand-shrink"' in versions_source
    assert '"/versions/{version_id}/offset/shrink-expand"' in versions_source
    assert '"command_id": "offset-mesh"' in versions_source
    assert '"command_id": "shell-mesh"' in versions_source
    assert '"command_id": "thicken-mesh"' in versions_source
    assert '"command_id": "weighted-shell"' in versions_source
    assert '"command_id": "partial-offset"' in versions_source
    assert '"command_id": "offset-verts"' in versions_source
    assert '"command_id": "expand-shrink"' in versions_source
    assert '"command_id": "shrink-expand"' in versions_source
    assert '"endpoint_url_key": "offset_mesh_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "shell_mesh_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "thicken_mesh_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "weighted_shell_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "partial_offset_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "offset_verts_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "expand_shrink_endpoint_url"' in versions_source
    assert '"endpoint_url_key": "shrink_expand_endpoint_url"' in versions_source
    assert '"sdk_operations": ["voxel_offset_mesh"]' in versions_source
    assert '"sdk_operations": ["voxel_shell_mesh"]' in versions_source
    assert '"sdk_operations": ["voxel_thicken_mesh"]' in versions_source
    assert '"sdk_operations": ["voxel_weighted_shell_mesh"]' in versions_source
    assert '"sdk_operations": ["voxel_partial_offset_mesh"]' in versions_source
    assert '"sdk_operations": ["offset_verts_mesh"]' in versions_source
    assert "default_sdk.voxel_offset_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.voxel_offset_mesh, ...)
    assert "default_sdk.voxel_shell_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.voxel_shell_mesh, ...)
    assert "default_sdk.voxel_thicken_mesh(" in versions_source
    assert "default_sdk.voxel_weighted_shell_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.voxel_weighted_shell_mesh, ...)
    assert "default_sdk.voxel_partial_offset_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.voxel_partial_offset_mesh, ...)
    assert "default_sdk.offset_verts_mesh" in versions_source  # threadpool-wrapped: run_in_threadpool(default_sdk.offset_verts_mesh, ...)
    assert "def voxel_offset_mesh(" in engine
    assert "def voxel_shell_mesh(" in engine
    assert "def voxel_thicken_mesh(" in engine
    assert "def voxel_weighted_shell_mesh(" in engine
    assert "def voxel_partial_offset_mesh(" in engine
    assert "def offset_verts_mesh(" in engine
    assert "OffsetMeshRequest" in schemas
    assert "ShellMeshRequest" in schemas
    assert "ThickenMeshRequest" in schemas
    assert "WeightedShellRequest" in schemas
    assert "PartialOffsetRequest" in schemas
    assert "OffsetVertsRequest" in schemas
    assert "OffsetSmoothingRequest" in schemas
    assert "OffsetShellMeshResponse" in schemas
    assert "OffsetMeshRequest" in frontend_types
    assert "ShellMeshRequest" in frontend_types
    assert "ThickenMeshRequest" in frontend_types
    assert "WeightedShellRequest" in frontend_types
    assert "PartialOffsetRequest" in frontend_types
    assert "OffsetVertsRequest" in frontend_types
    assert "OffsetSmoothingRequest" in frontend_types
    assert "OffsetShellMeshResponse" in frontend_types
    assert "submitOffsetMesh(" in frontend_api
    assert "submitShellMesh(" in frontend_api
    assert "submitThickenMesh(" in frontend_api
    assert "submitWeightedShell(" in frontend_api
    assert "submitPartialOffset(" in frontend_api
    assert "submitOffsetVerts(" in frontend_api
    assert "submitExpandShrink(" in frontend_api
    assert "submitShrinkExpand(" in frontend_api
    assert "useOffsetMeshOperation" in frontend_hooks
    assert "useShellMeshOperation" in frontend_hooks
    assert "useThickenMeshOperation" in frontend_hooks
    assert "useWeightedShellOperation" in frontend_hooks
    assert "usePartialOffsetOperation" in frontend_hooks
    assert "useOffsetVertsOperation" in frontend_hooks
    assert "useExpandShrinkOperation" in frontend_hooks
    assert "useShrinkExpandOperation" in frontend_hooks
    assert "| 'offset-mesh'" in workspace_types
    assert "| 'shell-mesh'" in workspace_types
    assert "| 'thicken-mesh'" in workspace_types
    assert "| 'weighted-shell'" in workspace_types
    assert "| 'partial-offset'" in workspace_types
    assert "| 'offset-verts'" in workspace_types
    assert "| 'expand-shrink'" in workspace_types
    assert "| 'shrink-expand'" in workspace_types
    assert "offsetDistanceMm: number" in workspace_types
    assert "shellWallThicknessMm: number" in workspace_types
    assert "thickenMeshThicknessMm: number" in workspace_types
    assert "weightedShellOffsetMm: number" in workspace_types
    assert "partialOffsetDistanceMm: number" in workspace_types
    assert "offsetVertsDistanceMm: number" in workspace_types
    assert "expandShrinkDistanceMm: number" in workspace_types
    assert "shrinkExpandDistanceMm: number" in workspace_types
    assert "id: 'offset-mesh'" in tool_registry
    assert "id: 'shell-mesh'" in tool_registry
    assert "id: 'thicken-mesh'" in tool_registry
    assert "id: 'weighted-shell'" in tool_registry
    assert "id: 'partial-offset'" in tool_registry
    assert "id: 'offset-verts'" in tool_registry
    assert "id: 'expand-shrink'" in tool_registry
    assert "id: 'shrink-expand'" in tool_registry
    assert "label: 'Offset Mesh'" in tool_registry
    assert "label: 'Shell Mesh'" in tool_registry
    assert "label: 'Thickening'" in tool_registry
    assert "label: 'Weighted Shell'" in tool_registry
    assert "label: 'Partial Offset'" in tool_registry
    assert "label: 'Offset Verts'" in tool_registry
    assert "label: 'Expand/Shrink'" in tool_registry
    assert "label: 'Shrink/Expand'" in tool_registry
    assert "contextualToolId: 'offset-mesh'" in tool_registry
    assert "contextualToolId: 'shell-mesh'" in tool_registry
    assert "contextualToolId: 'thicken-mesh'" in tool_registry
    assert "contextualToolId: 'weighted-shell'" in tool_registry
    assert "contextualToolId: 'partial-offset'" in tool_registry
    assert "contextualToolId: 'offset-verts'" in tool_registry
    assert "contextualToolId: 'expand-shrink'" in tool_registry
    assert "contextualToolId: 'shrink-expand'" in tool_registry
    assert "case 'offset-mesh'" in tool_inspector
    assert "case 'shell-mesh'" in tool_inspector
    assert "case 'thicken-mesh'" in tool_inspector
    assert "case 'weighted-shell'" in tool_inspector
    assert "case 'partial-offset'" in tool_inspector
    assert "case 'offset-verts'" in tool_inspector
    assert "case 'expand-shrink'" in tool_inspector
    assert "case 'shrink-expand'" in tool_inspector
    assert "onOffsetMesh" in tool_inspector
    assert "onShellMesh" in tool_inspector
    assert "onThickenMesh" in tool_inspector
    assert "onWeightedShell" in tool_inspector
    assert "onPartialOffset" in tool_inspector
    assert "onOffsetVerts" in tool_inspector
    assert "onExpandShrink" in tool_inspector
    assert "onShrinkExpand" in tool_inspector
    assert "offsetShellResult" in tool_inspector
    assert "offsetMeshMutation = useOffsetMeshOperation()" in viewer_page
    assert "shellMeshMutation = useShellMeshOperation()" in viewer_page
    assert "thickenMeshMutation = useThickenMeshOperation()" in viewer_page
    assert "weightedShellMutation = useWeightedShellOperation()" in viewer_page
    assert "partialOffsetMutation = usePartialOffsetOperation()" in viewer_page
    assert "offsetVertsMutation = useOffsetVertsOperation()" in viewer_page
    assert "expandShrinkMutation = useExpandShrinkOperation()" in viewer_page
    assert "shrinkExpandMutation = useShrinkExpandOperation()" in viewer_page
    assert "setOffsetShellResult" in viewer_page
    assert "case 'offset-mesh'" in viewer_page
    assert "case 'shell-mesh'" in viewer_page
    assert "case 'thicken-mesh'" in viewer_page
    assert "case 'weighted-shell'" in viewer_page
    assert "case 'partial-offset'" in viewer_page
    assert "case 'offset-verts'" in viewer_page
    assert "case 'expand-shrink'" in viewer_page
    assert "case 'shrink-expand'" in viewer_page
    assert "offsetMeshRequestFromWorkbenchPayload" in viewer_page
    assert "shellMeshRequestFromWorkbenchPayload" in viewer_page
    assert "thickenMeshRequestFromWorkbenchPayload" in viewer_page
    assert "weightedShellRequestFromWorkbenchPayload" in viewer_page
    assert "partialOffsetRequestFromWorkbenchPayload" in viewer_page
    assert "offsetVertsRequestFromWorkbenchPayload" in viewer_page
    assert "offsetSmoothingRequestFromWorkbenchPayload" in viewer_page
    assert "'Offset Mesh': 'offset-verts'" in runtime_bootstrap
    assert "'Shell Mesh': 'shell-mesh'" in runtime_bootstrap
    assert "'Thickening': 'thicken-mesh'" in runtime_bootstrap
    assert "'Weighted Shell': 'weighted-shell'" in runtime_bootstrap
    assert "'Partial Offset': 'partial-offset'" in runtime_bootstrap
    assert "'Offset Verts': 'offset-verts'" in runtime_bootstrap
    assert "'Expand/Shrink': 'expand-shrink'" in runtime_bootstrap
    assert "'Shrink/Expand': 'shrink-expand'" in runtime_bootstrap
    assert "'OffsetMeshTool': 'offset-verts'" in runtime_bootstrap
    assert "'ShellMeshTool': 'shell-mesh'" in runtime_bootstrap
    assert "'ThickeningTool': 'thicken-mesh'" in runtime_bootstrap
    assert "'WeightedShellTool': 'weighted-shell'" in runtime_bootstrap
    assert "'PartialOffsetTool': 'partial-offset'" in runtime_bootstrap
    assert "'OffsetVertsTool': 'offset-verts'" in runtime_bootstrap
    assert "'ExpandShrinkTool': 'expand-shrink'" in runtime_bootstrap
    assert "'ShrinkExpandTool': 'shrink-expand'" in runtime_bootstrap
    assert "'Offset Mesh': 'offset-verts'" in host
    assert "OffsetMeshTool: 'offset-verts'" in host
    assert "'Thickening': 'thicken-mesh'" in host
    assert "ThickeningTool: 'thicken-mesh'" in host
    assert "'Weighted Shell': 'weighted-shell'" in host
    assert "WeightedShellTool: 'weighted-shell'" in host
    assert "'Partial Offset': 'partial-offset'" in host
    assert "PartialOffsetTool: 'partial-offset'" in host
    assert "'Offset Verts': 'offset-verts'" in host
    assert "OffsetVertsTool: 'offset-verts'" in host


def test_official_workbench_backend_jobs_enter_viewer_job_tracker() -> None:
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()

    assert "job: responsePayload" in runtime_bootstrap
    assert "import type { JobResponse" in host
    assert "onJobSubmitted?: (job: JobResponse) => void" in host
    assert "job?: JobResponse" in host
    assert "event.data.payload?.job?.id" in host
    assert "onJobSubmitted?.(event.data.payload.job)" in host
    assert "queryKey: ['version-jobs', committedVersionId]" in host
    assert "onJobSubmitted={trackWorkbenchJob}" in viewer_page
    assert "const trackWorkbenchJob = useCallback((job: JobResponse)" in viewer_page
    assert "setActiveJobId(job.id)" in viewer_page
    assert "queryClient.invalidateQueries({ queryKey: ['version-jobs', variables.versionId] });" in frontend_hooks
    assert "queryClient.invalidateQueries({ queryKey: ['version-jobs', versionId] });" in viewer_page
    assert "queryClient.invalidateQueries({ queryKey: ['version-jobs', nextVersionId] });" in viewer_page


def test_official_workbench_commit_uploads_route_through_sdk_boundary() -> None:
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    operations_router = (BACKEND_ROOT / "api" / "routers" / "operations.py").read_text()
    worker_runtime = (BACKEND_ROOT / "workers" / "runtime.py").read_text()
    operation_service = (BACKEND_ROOT / "services" / "operations.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()

    assert "submitInteractiveCommit(" in frontend_api
    assert "FormData" in frontend_api
    assert "InteractiveCommitRequest" in frontend_types
    assert "useInteractiveCommitOperation" in frontend_hooks
    assert "submit_interactive_commit" in operations_router
    assert "InteractiveCommitRequest.model_validate" in worker_runtime
    assert 'operation_type == "interactive_commit"' in worker_runtime
    assert "run_interactive_commit_operation" in operation_service
    assert "from services.sdk_conversion import to_glb, to_ply, to_stl" in operation_service
    assert "services.convert" not in operation_service
    assert "INTERACTIVE_COMMIT" in schemas


def test_official_workbench_save_exports_can_commit_back_to_versioned_backend() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    compose_html = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "compose_html.js").read_text()
    io_files = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "io_files.js").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    operation_service = (BACKEND_ROOT / "services" / "operations.py").read_text()

    assert "window.MeshInspectorWorkbenchBridge" in runtime_bootstrap
    assert "commitSavedFile" in runtime_bootstrap
    assert "manifest.commit_endpoint_url" in runtime_bootstrap
    assert "meshlib_workbench_export" in runtime_bootstrap
    assert "meshlib-workbench:commit-complete" in runtime_bootstrap
    assert "dataset.meshinspectorWorkbenchBridge" in runtime_bootstrap
    assert compose_html.index('"runtime_bootstrap.js"') < compose_html.index('"wasm_loader.js"')

    assert "MeshInspectorWorkbenchBridge.commitSavedFile" in io_files
    assert "meshlib-workbench:commit-complete" in bridge
    assert "meshlib_workbench_export" in schemas
    assert '"meshlib_workbench_export": "interactive_commit"' in operation_service
    assert "meshlib-workbench:commit-complete" in host
    assert "queryClient.invalidateQueries({ queryKey: ['version'" in host
    assert "queryClient.invalidateQueries({ queryKey: ['viewer-manifest'" in host
    assert "queryClient.invalidateQueries({ queryKey: ['meshlib-workbench'" in host


def test_meshlib_runtime_web_requests_support_backend_multipart_contracts() -> None:
    web_request = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "web_request.js").read_text()
    operations_router = (BACKEND_ROOT / "api" / "routers" / "operations.py").read_text()

    assert 'request_json: str = Form(...)' in operations_router
    assert 'mesh_file: UploadFile = File(...)' in operations_router
    assert "var web_req_add_formdata_text" in web_request
    assert ".formdata.append(name, value);" in web_request
    assert "var web_req_add_formdata_file" in web_request
    assert "new Blob([content], { type: contentType })" in web_request
    assert "encodeURIComponent(" in web_request
    assert "web_req_build_url(" in web_request
    assert ".ody" not in web_request
    assert "web_req_ctxs[ctxId].body" in web_request


def test_official_measure_inspect_tool_routes_through_versioned_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Measure / Inspect" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    workspace_types = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "types.ts").read_text()
    tool_registry = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "toolRegistry.ts").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    assert '"/versions/{version_id}/measure-inspect"' in versions_source
    assert '"command_id": "measure-inspect"' in versions_source
    assert '"runtime_tool_id": "measure_inspect"' in versions_source
    assert (
        '"sdk_operations": ["closest_points_on_mesh", "feature_pair_measurements", "mesh_geodesic_path", "mesh_geodesic_polyline_path", "mesh_cut_measure_contours", "mesh_geodesic_quadrangle_path", "mesh_fast_marching_surface_path", "mesh_fast_marching_surface_path_tri_points", "mesh_surface_path_tri_points", "object_lines_from_contours", "mesh_geodesic_distance_field", "mesh_closest_surface_path_targets", "mesh_surface_distance_seed_vertices", "mesh_geodesic_iso_region", "mesh_geodesic_extreme_edges", "thickness_overlay_payload"]'
        in versions_source
    )
    assert "default_sdk.closest_points_on_mesh(" in versions_source
    assert "default_sdk.feature_pair_measurements(" in versions_source
    assert "default_sdk.mesh_geodesic_path(" in versions_source
    assert "default_sdk.mesh_geodesic_polyline_path(" in versions_source
    assert "default_sdk.mesh_cut_measure_contours(" in versions_source
    assert "default_sdk.mesh_geodesic_quadrangle_path(" in versions_source
    assert "mesh_fast_marching_surface_path" in versions_source
    assert "mesh_fast_marching_surface_path_tri_points" in versions_source
    assert "mesh_surface_path_tri_points" in versions_source
    assert "default_sdk.object_lines_from_contours(" in versions_source
    assert "default_sdk.mesh_geodesic_distance_field(" in versions_source
    assert "default_sdk.mesh_surface_distance_seed_vertices(" in versions_source
    assert "default_sdk.mesh_geodesic_iso_region(" in versions_source
    assert "default_sdk.mesh_geodesic_extreme_edges(" in versions_source
    assert "default_sdk.thickness_overlay_payload(" in versions_source
    assert "numpy" not in _imported_modules(BACKEND_ROOT / "api" / "routers" / "versions.py")
    assert "MeasureInspectRequest" in schemas
    assert "MeasureInspectResponse" in schemas
    assert "MeasureInspectFeaturePrimitive" in schemas
    assert "MeasureInspectFeaturePairResult" in schemas
    assert "measurement_endpoint_url" in schemas
    assert "mesh_cut_measure_topology_endpoint_url" in schemas
    assert "supports_measure_inspect" in versions_source
    assert "control_vertices" in schemas
    assert "close_path" in schemas
    assert "include_refined_surface_path" in schemas
    assert "cut_contours" in schemas
    assert "surface_path_refinement" in schemas
    assert "submitMeasureInspect(" in frontend_api
    assert "MeasureInspectRequest" in frontend_types
    assert "MeasureInspectResponse" in frontend_types
    assert "metric?: 'euclidean' | 'geodesic'" in frontend_types
    assert "control_vertices?: number[]" in frontend_types
    assert "close_path?: boolean" in frontend_types
    assert "include_refined_surface_path?: boolean" in frontend_types
    assert "control_vertex_indices" in frontend_types
    assert "closed_path" in frontend_types
    assert "path_object_lines" in frontend_types
    assert "path_point_normals" in frontend_types
    assert "path_object_points" in frontend_types
    assert "cut_contours" in frontend_types
    assert "surface_path_refinement" in frontend_types
    assert "MeasureInspectFeaturePrimitive" in frontend_types
    assert "feature_pairs: MeasureInspectFeaturePairResult[]" in frontend_types
    assert "distance: MeasureInspectFeatureDistanceResult" in frontend_types
    assert "center_distance: MeasureInspectFeatureDistanceResult" in frontend_types
    assert "MeasureInspectFeatureIntersectionResult" in frontend_types
    assert "intersections: MeasureInspectFeatureIntersectionResult[]" in frontend_types
    assert "leg_lengths_mm" in frontend_types
    assert "path_vertex_indices" in frontend_types
    assert "surface_distance" in frontend_types
    assert "MeasureInspectSurfaceDistanceResult" in frontend_types
    assert "iso_value_mm" in frontend_types
    assert "iso_segments" in frontend_types
    assert "include_extreme_edges" in frontend_types
    assert "ridge_edges" in frontend_types
    assert "gorge_edges" in frontend_types
    assert "seed_edges" in frontend_types
    assert "seed_face_ids" in frontend_types
    assert "useMeasureInspectOperation" in frontend_hooks
    assert "| 'measure-inspect'" in workspace_types
    assert "id: 'measure-inspect'" in tool_registry
    assert "label: 'Measure / Inspect'" in tool_registry
    assert "contextualToolId: 'measure-inspect'" in tool_registry
    assert "case 'measure-inspect'" in tool_inspector
    assert "onMeasureInspect" in tool_inspector
    assert "measureInspectMutation = useMeasureInspectOperation()" in viewer_page
    assert "setMeasureInspectResult" in viewer_page
    assert "measureInspect" in runtime_bootstrap
    assert "manifest.measurement_endpoint_url" in runtime_bootstrap
    assert runtime_bootstrap.count("commandId: 'runtime-measure-inspect'") >= 2
    assert runtime_bootstrap.count("points: [[0, 0, 0], [5, 5, 5]]") >= 2
    assert re.search(
        r"commandId: 'runtime-measure-inspect'.{0,260}"
        r"points: \[\[0, 0, 0\], \[5, 5, 5\]\].{0,180}"
        r"options: \{ execute: true \}",
        runtime_bootstrap,
        re.S,
    )
    assert "meshCutMeasureTopology" in runtime_bootstrap
    assert "manifest.mesh_cut_measure_topology_endpoint_url" in runtime_bootstrap
    assert "function normalizeMeasureInspectPayload(params = {}, options = {})" in runtime_bootstrap
    assert "appendPoint(points, params.point_world)" in runtime_bootstrap
    assert "appendPoint(points, params.world_point)" in runtime_bootstrap
    assert "appendPair(pointPairs, params)" in runtime_bootstrap
    assert "appendFeature(features, params.features)" in runtime_bootstrap
    assert "appendFeaturePair(featurePairs, params.feature_pairs)" in runtime_bootstrap
    assert "feature_pairs: featurePairs" in runtime_bootstrap
    assert "control_vertices" in runtime_bootstrap
    assert "close_path" in runtime_bootstrap
    assert "include_refined_surface_path" in runtime_bootstrap
    assert "point_pairs: pointPairs" in runtime_bootstrap
    assert "distance_metric" in viewer_page
    assert "start_vertex" in viewer_page
    assert "control_vertices" in viewer_page
    assert "close_path" in viewer_page
    assert "include_refined_surface_path" in viewer_page
    assert "surface_distance" in viewer_page
    assert "seed_vertex" in viewer_page
    assert "seed_edges" in viewer_page
    assert "iso_value_mm" in viewer_page
    assert "include_extreme_edges" in viewer_page
    assert "surface_distance: surfaceDistance" in runtime_bootstrap
    assert "include_extreme_edges" in runtime_bootstrap
    assert "meshlib-workbench:measure-complete" in runtime_bootstrap
    assert "meshlib-workbench:measure-failed" in runtime_bootstrap
    assert "meshlib-workbench:measure-complete" in bridge
    assert "meshlib-workbench:measure-failed" in bridge
    assert "meshlib-workbench:measure-complete" in host
    assert "meshlib-workbench:measure-failed" in host
    assert "'runtime-measure-inspect': 'measure-inspect'" in host


def test_official_select_mark_region_tool_persists_selection_payloads_without_mesh_uploads() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert "Select / Mark Region" in item_names

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    assert '"/versions/{version_id}/selection-commit"' in versions_source
    assert "SelectionCommitRequest" in schemas
    assert "SelectionCommitResponse" in schemas
    assert "resolved_counts" in schemas
    assert "selection_endpoint_url" in schemas
    assert "supports_selection_commit" in versions_source
    assert "meshlib_selection_json" in versions_source
    assert "_resolve_selection_vertex_ids(" in versions_source
    assert "register_file_artifact(" in versions_source
    assert "submitSelectionCommit(" in frontend_api
    assert "SelectionCommitRequest" in frontend_types
    assert "SelectionCommitResponse" in frontend_types
    assert "resolved_counts" in frontend_types
    assert "useSelectionCommitOperation" in frontend_hooks
    assert "commitSelection" in runtime_bootstrap
    assert "manifest.selection_endpoint_url" in runtime_bootstrap
    assert "meshlib-workbench:selection-complete" in runtime_bootstrap
    assert "meshlib-workbench:selection-complete" in bridge
    assert "meshlib-workbench:selection-complete" in host


def test_official_local_edit_brush_tools_replay_through_rust_sdk_endpoint() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert {"Thicken Brush", "Scoop Brush", "Smooth Brush"}.issubset(item_names)

    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    operations_router = (BACKEND_ROOT / "api" / "routers" / "operations.py").read_text()
    worker_runtime = (BACKEND_ROOT / "workers" / "runtime.py").read_text()
    operation_service = (BACKEND_ROOT / "services" / "operations.py").read_text()
    schemas = (BACKEND_ROOT / "domain" / "schemas.py").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_types = (FRONTEND_ROOT / "src" / "lib" / "api" / "types.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    assert "brush_endpoint_url" in schemas
    assert 'INTERACTIVE_BRUSH_REPLAY = "interactive_brush_replay"' in schemas
    assert "BrushReplayRequest" in schemas
    assert "BrushReplayStroke" in schemas
    assert "supports_brush_replay" in versions_source
    assert '"/versions/{version_id}/brush-replay"' in operations_router
    assert '"interactive_brush_replay"' in operations_router
    assert "BrushReplayRequest.model_validate(payload)" in worker_runtime
    assert "run_interactive_brush_replay_operation" in worker_runtime
    assert "run_interactive_brush_replay_operation" in operation_service
    assert "default_sdk.apply_brush_strokes(" in operation_service
    assert "BrushStroke(" in operation_service
    assert "interactive_brush_payload_json" in operation_service
    assert "submitBrushReplay(" in frontend_api
    assert "BrushReplayRequest" in frontend_types
    assert "useBrushReplayOperation" in frontend_hooks
    assert "commitBrushStrokes" in runtime_bootstrap
    assert "manifest.brush_endpoint_url" in runtime_bootstrap
    assert "function brushSelectionPayload(stroke = {})" in runtime_bootstrap
    assert "function numericBrushOption(stroke, keys, fallback)" in runtime_bootstrap
    assert "amount_mm: numericBrushOption(stroke, ['amount_mm', 'depth_mm', 'target_thickness_mm'], 0.15)" in runtime_bootstrap
    assert "falloff_mm: numericBrushOption(stroke, ['falloff_mm', 'brush_radius_mm', 'radius_mm'], 1.5)" in runtime_bootstrap
    assert "selection: normalizeSelectionPayload(brushSelectionPayload(stroke))" in runtime_bootstrap
    assert "brush_points_world: nested.brush_points_world ?? stroke.brush_points_world" in runtime_bootstrap
    assert "region_ids: nested.region_ids ?? stroke.region_ids" in runtime_bootstrap
    assert "meshlib-workbench:brush-complete" in runtime_bootstrap
    assert "meshlib-workbench:brush-complete" in bridge
    assert "meshlib-workbench:brush-complete" in host


def test_official_runtime_exposes_sdk_command_dispatcher_to_meshlib_tools() -> None:
    plugin_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    item_names = {str(item.get("Name")) for item in plugin_manifest.get("Items", [])}
    assert {
        "Select / Mark Region",
        "Thicken Brush",
        "Scoop Brush",
        "Smooth Brush",
        "Measure / Inspect",
    }.issubset(item_names)

    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "const WORKBENCH_TOOL_COMMAND_ALIASES" in runtime_bootstrap
    for alias in [
        "'Select / Mark Region': 'runtime-select-mark-region'",
        "'select_mark_region': 'runtime-select-mark-region'",
        "'Thicken Brush': 'runtime-thicken-brush'",
        "'thicken_brush': 'runtime-thicken-brush'",
        "'Scoop Brush': 'runtime-scoop-brush'",
        "'scoop_brush': 'runtime-scoop-brush'",
        "'Smooth Brush': 'runtime-smooth-brush'",
        "'smooth_brush': 'runtime-smooth-brush'",
        "'Measure / Inspect': 'runtime-measure-inspect'",
        "'measure_inspect': 'runtime-measure-inspect'",
    ]:
        assert alias in runtime_bootstrap

    assert "function findWorkbenchCommandCapability(commandId)" in runtime_bootstrap
    assert "async function dispatchWorkbenchCommand(commandId, payload = {}, options = {})" in runtime_bootstrap
    assert "capability.endpoint_url_key === 'selection_endpoint_url'" in runtime_bootstrap
    assert "return commitSelection(payload.selection || payload, {" in runtime_bootstrap
    assert "capability.endpoint_url_key === 'brush_endpoint_url'" in runtime_bootstrap
    assert "return commitBrushStrokes(strokes, {" in runtime_bootstrap
    assert "capability.endpoint_url_key === 'measurement_endpoint_url'" in runtime_bootstrap
    assert "return measureInspect(payload, {" in runtime_bootstrap
    assert "capability.endpoint_url_key === 'mesh_cut_measure_topology_endpoint_url'" in runtime_bootstrap
    assert "return meshCutMeasureTopology(payload, {" in runtime_bootstrap
    assert "dispatchCommand: dispatchWorkbenchCommand" in runtime_bootstrap
    assert "window.meshinspectorWorkbenchDispatchCommand = dispatchWorkbenchCommand" in runtime_bootstrap
    assert "window.meshinspectorWorkbenchBridge = window.MeshInspectorWorkbenchBridge" in runtime_bootstrap


def test_official_meshlib_plugin_ribbon_clicks_activate_host_tools_without_wasm_null_callbacks() -> None:
    items_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.items.json").read_text()
    )
    ui_manifest = json.loads(
        (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "assets" / "MeshInspectorWorkbenchPlugin.ui.json").read_text()
    )
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()

    item_names = {str(item.get("Name")) for item in items_manifest.get("Items", [])}
    ui_items_by_tab = {
        str(tab.get("Name")): {
            str(item.get("Name"))
            for group in tab.get("Groups", [])
            for item in group.get("List", [])
        }
        for tab in ui_manifest.get("Tabs", [])
    }
    expected_runtime_to_host = {
        "runtime-select-mark-region": "regions",
        "runtime-thicken-brush": "runtime-thicken-brush",
        "runtime-scoop-brush": "runtime-scoop-brush",
        "runtime-smooth-brush": "runtime-smooth-brush",
        "runtime-measure-inspect": "measure-inspect",
    }

    assert {
        "Select / Mark Region",
        "Thicken Brush",
        "Scoop Brush",
        "Smooth Brush",
        "Measure / Inspect",
    }.issubset(item_names)
    assert ui_items_by_tab["Select"] == {"Select / Mark Region"}
    assert ui_items_by_tab["Mesh Edit"] >= {
        "Offset / Shell",
        "Boolean / Collision",
        "Mesh Edit / Simplify",
        "Select / Mark Region",
        "Thicken Brush",
        "Scoop Brush",
        "Smooth Brush",
    }
    assert ui_items_by_tab["Inspect / Features"] == {"Measure / Inspect"}

    assert "const WORKBENCH_CANVAS_COMMAND_HITBOXES" in runtime_bootstrap
    assert "'selection': [" in runtime_bootstrap
    assert "label: 'Select / Mark Region'" in runtime_bootstrap
    assert "minX: 180" in runtime_bootstrap
    assert "maxX: 360" in runtime_bootstrap
    assert "const WORKBENCH_CANVAS_POINTER_EVENTS" in runtime_bootstrap
    assert "function installWorkbenchCanvasCommandBridge()" in runtime_bootstrap
    assert "function handleWorkbenchCanvasClick(event)" in runtime_bootstrap
    assert "[data-meshinspector-workbench-accessible-command]" in runtime_bootstrap
    assert "for (const eventName of WORKBENCH_CANVAS_POINTER_EVENTS)" in runtime_bootstrap
    for event_name in ["'pointerdown'", "'mousedown'", "'mouseup'", "'click'"]:
        assert event_name in runtime_bootstrap
    assert "window.addEventListener(eventName, handleWorkbenchCanvasClick, true)" in runtime_bootstrap
    assert "event.stopImmediatePropagation()" in runtime_bootstrap
    assert "lastWorkbenchCanvasCommandDispatch" in runtime_bootstrap
    assert "void dispatchWorkbenchCommand(command.commandId, resolveWorkbenchCommandPayload(command), command.options ?? {})" in runtime_bootstrap
    assert "function shouldForwardRuntimeWorkbenchToolCommand(payload = {}, options = {})" in runtime_bootstrap
    assert "const forwarded = forwardHostWorkbenchCommand(capability, payload, options)" in runtime_bootstrap
    assert "Submitted through host bridge" in runtime_bootstrap
    assert "return forwarded" in runtime_bootstrap

    for runtime_command_id, workspace_command_id in expected_runtime_to_host.items():
        assert f"commandId: '{runtime_command_id}'" in runtime_bootstrap
        assert f"'{runtime_command_id}': '{workspace_command_id}'" in host


def test_official_runtime_canvas_tab_hitboxes_match_meshlib_ribbon_tabs() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    ranges = {
        match.group(1): tuple(int(match.group(index)) for index in range(2, 6))
        for match in re.finditer(
            r"\{ tab: '([^']+)', minX: (\d+), maxX: (\d+), minY: (\d+), maxY: (\d+) \}",
            runtime_bootstrap,
        )
    }

    assert ranges["selection"] == (294, 366, 0, 36)
    assert ranges["mesh-edit"] == (526, 604, 0, 36)
    assert ranges["inspect-features"] == (604, 700, 0, 36)
    assert ranges["mesh-edit"][1] == ranges["inspect-features"][0]

    assert "label: 'MeshLib Select tab'" in runtime_bootstrap


def test_official_runtime_forwards_noninteractive_workspace_commands_to_host() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()
    bridge = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "bridge.js").read_text()
    host = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "MeshLibWorkbenchHost.tsx").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    for command_id in [
        '"repair"',
        '"fit-size"',
        '"reduce-weight"',
        '"prepare-casting"',
        '"make-manufacturable"',
        '"resize"',
        '"protected-hollow"',
        '"hollow-drains"',
        '"thicken-violations"',
        '"regions"',
        '"compare-versions"',
        '"version-history"',
        '"job-activity"',
    ]:
        assert command_id in versions_source

    assert "function forwardHostWorkbenchCommand(capability, payload = {}, options = {})" in runtime_bootstrap
    assert "function isRuntimeWorkbenchToolCommand(capability)" in runtime_bootstrap
    assert "capability.endpoint_url_key === 'selection_endpoint_url' && isRuntimeWorkbenchToolCommand(capability)" in runtime_bootstrap
    assert "meshlib-workbench:host-command" in runtime_bootstrap
    assert "return forwardHostWorkbenchCommand(capability, payload, options)" in runtime_bootstrap
    assert "'meshlib-workbench:host-command'" in bridge
    assert "export type WorkbenchHostCommandPayload" in host
    assert "payload: Record<string, unknown>" in host
    assert "options: Record<string, unknown>" in host
    assert "onWorkspaceCommand?: (command: WorkbenchHostCommandPayload) => void" in host
    assert "event.data?.type === 'meshlib-workbench:host-command'" in host
    assert "payload: recordFromUnknown(hostCommand.payload)" in host
    assert "options: recordFromUnknown(hostCommand.options)" in host
    assert "onWorkspaceCommand(command)" in host
    assert "type WorkbenchCommandInvocation" in viewer_page
    assert "function requestPayloadFromWorkbenchCommand" in viewer_page
    assert "function shouldExecuteWorkbenchCommand" in viewer_page
    assert "const onWorkbenchHostCommand = (command: WorkbenchHostCommandPayload)" in viewer_page
    assert "onCommandSelect(command.commandId, {" in viewer_page
    assert "payload: command.payload" in viewer_page
    assert "options: command.options" in viewer_page
    assert "onWorkspaceCommand={onWorkbenchHostCommand}" in viewer_page


def test_official_inspect_workspace_commands_apply_payload_to_sdk_queries() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    for marker in [
        '"command_id": "section"',
        '"endpoint_url_key": "section_endpoint_url"',
        '"sdk_operations": ["section_contour"]',
        '"command_id": "heatmap"',
        '"endpoint_url_key": "thickness_overlay_url"',
        '"sdk_operations": ["thickness_overlay_payload"]',
        '"command_id": "regions"',
        '"endpoint_url_key": "selection_endpoint_url"',
        '"sdk_operations": ["detect_ring_regions", "closest_points_on_mesh"]',
        '"command_id": "distance-map-from-mesh"',
        '"sdk_operations": ["distance_map_from_mesh"]',
        '"command_id": "distance-map-contours"',
        '"sdk_operations": ["distance_map_from_contours"]',
        '"command_id": "distance-map-iso-lines"',
        '"sdk_operations": ["distance_map_to_iso_segments"]',
        '"command_id": "distance-map-merge"',
        '"sdk_operations": ["distance_map_merge"]',
        '"command_id": "distance-map-contour-boolean"',
        '"sdk_operations": ["distance_map_contour_boolean"]',
        '"command_id": "distance-map-from-tiff"',
        '"sdk_operations": ["distance_map_from_tiff"]',
        '"command_id": "distance-map-to-tiff"',
        '"sdk_operations": ["distance_map_to_tiff"]',
    ]:
        assert marker in versions_source

    assert "function vectorFromPayloadKeys(payload: Record<string, unknown>, keys: string[])" in viewer_page
    assert "function sectionPlaneConstantFromWorkbenchPayload(" in viewer_page
    assert "const selectedRegionIdsFromWorkbench = stringListFromPayload(" in viewer_page
    assert "['selected_region_ids', 'region_ids', 'regions_selected', 'regions']" in viewer_page
    assert "const selectedRegionIdFromWorkbench = stringFromPayload(workbenchState, ['selected_region_id', 'region_id', 'region'])" in viewer_page
    assert "const sectionAxisFromWorkbench = vectorFromPayloadKeys(workbenchState, ['plane_axis', 'section_axis', 'axis', 'manual_axis'])" in viewer_page
    assert "setSectionConstant(sectionPlaneConstantFromWorkbenchPayload(workbenchState, sectionConstant, sectionAxis));" in viewer_page
    assert "setHeatmapEnabled(booleanFromPayload(workbenchState, ['enabled', 'heatmap_enabled', 'show'], true));" in viewer_page
    assert "setRegionOverlayEnabled(booleanFromPayload(workbenchState, ['enabled', 'region_overlay_enabled', 'show'], true));" in viewer_page
    assert "setSelectedRegionIds(selectedRegionIdsFromWorkbench);" in viewer_page
    assert "setSelectedRegionId(selectedRegionIdFromWorkbench ?? selectedRegionIdsFromWorkbench[0] ?? selectedRegionId);" in viewer_page


def test_official_restore_branch_command_executes_version_branch_payloads() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()

    assert '"command_id": "restore-branch"' in versions_source
    assert '"endpoint_url_key": "branch_endpoint_url"' in versions_source
    assert "branchVersion(" in frontend_api
    assert "useBranchVersion()" in frontend_hooks
    assert "function sourceVersionIdFromWorkbenchPayload(" in viewer_page
    for marker in [
        "'source_version_id'",
        "'restore_version_id'",
        "'branch_version_id'",
        "'target_version_id'",
        "'open_version_id'",
        "'history_version_id'",
        "'version_id'",
    ]:
        assert marker in viewer_page
    assert "case 'restore-branch': {" in viewer_page
    assert "const sourceVersionId = sourceVersionIdFromWorkbenchPayload(workbenchState, versionId);" in viewer_page
    assert "operation_label: stringFromPayload(workbenchState, ['operation_label', 'label']) ?? `Restore Branch from ${sourceVersionId}`" in viewer_page
    assert "branchVersionMutation.mutateAsync({" in viewer_page
    assert "versionId: sourceVersionId" in viewer_page
    assert "setVersionId(nextVersion.id)" in viewer_page


def test_official_version_history_command_opens_payload_version() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()

    assert '"command_id": "version-history"' in versions_source
    assert '"endpoint_url_key": "model_versions_endpoint_url"' in versions_source
    assert "getModelVersions(" in frontend_api
    assert "useModelVersions(" in frontend_hooks
    assert "function versionHistoryVersionIdFromWorkbenchPayload(" in viewer_page
    assert "['open_version_id', 'history_version_id', 'target_version_id', 'version_id']" in viewer_page
    assert "case 'version-history': {" in viewer_page
    assert "const historyVersionId = versionHistoryVersionIdFromWorkbenchPayload(workbenchState);" in viewer_page
    assert "onOpenVersion(historyVersionId);" in viewer_page
    assert "setReviewPane('history');" in viewer_page
    assert "setRightDockTab('review');" in viewer_page


def test_official_compare_command_executes_payload_aliases_and_disable() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()

    assert '"command_id": "compare-versions"' in versions_source
    assert '"endpoint_url_key": "compare_endpoint_url"' in versions_source
    assert '"sdk_operations": ["service_compare_field", "service_compare"]' in versions_source
    assert "getCompareOverlay(" in frontend_api
    assert "getCompareSummary(" in frontend_api
    assert "useCompareOverlay(" in frontend_hooks
    assert "useCompareSummary(" in frontend_hooks
    assert "function compareVersionIdFromWorkbenchPayload(" in viewer_page
    assert "['other_version_id', 'compare_version_id', 'compare_target_version_id', 'target_version_id', 'version_id']" in viewer_page
    assert "function shouldDisableCompareFromWorkbenchPayload(" in viewer_page
    assert "['disable', 'off', 'close', 'clear', 'reset']" in viewer_page
    assert "booleanFromPayload(payload, ['enabled', 'compare_enabled', 'show'], true) === false" in viewer_page
    assert "case 'compare-versions': {" in viewer_page
    assert "if (shouldDisableCompareFromWorkbenchPayload(workbenchState))" in viewer_page
    assert "onRequestCompare(null);" in viewer_page
    assert "const otherVersionId = compareVersionIdFromWorkbenchPayload(workbenchState);" in viewer_page
    assert "onCompareVersion(otherVersionId);" in viewer_page


def test_official_wireframe_command_honors_explicit_payload_state() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert '"command_id": "wireframe"' in versions_source
    assert "Frontend-only topology overlay over the loaded MeshLib viewport mesh." in versions_source
    assert "case 'wireframe': {" in viewer_page
    assert "setWireframe(booleanFromPayload(workbenchState, ['enabled', 'wireframe_enabled', 'show'], true));" in viewer_page
    assert "setActiveTool('wireframe', 'inspect');" in viewer_page
    assert "setRightDockTab('tool');" in viewer_page


def test_official_state_commands_consume_workbench_options_as_fallback() -> None:
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert "function statePayloadFromWorkbenchCommand(invocation?: WorkbenchCommandInvocation)" in viewer_page
    assert "...(invocation?.options ?? {})" in viewer_page
    assert "...requestPayload" in viewer_page
    assert "const workbenchState = statePayloadFromWorkbenchCommand(invocation);" in viewer_page
    assert "sectionContourParamsFromWorkbenchPayload(\n            workbenchState," in viewer_page
    assert "const jobId = jobIdFromWorkbenchPayload(workbenchState);" in viewer_page
    assert "setWireframe(booleanFromPayload(workbenchState, ['enabled', 'wireframe_enabled', 'show'], true));" in viewer_page
    assert "if (shouldDisableCompareFromWorkbenchPayload(workbenchState))" in viewer_page
    assert "const otherVersionId = compareVersionIdFromWorkbenchPayload(workbenchState);" in viewer_page
    assert "const historyVersionId = versionHistoryVersionIdFromWorkbenchPayload(workbenchState);" in viewer_page
    assert "const sourceVersionId = sourceVersionIdFromWorkbenchPayload(workbenchState, versionId);" in viewer_page
    assert "const selectedRegionIdsFromWorkbench = stringListFromPayload(\n            workbenchState," in viewer_page
    assert "const snapshotName = stringFromPayload(workbenchState, ['snapshot_name', 'name', 'label']);" in viewer_page


def test_official_download_stl_command_uses_manifest_artifact_endpoint() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert '"command_id": "download-stl"' in versions_source
    assert '"endpoint_url_key": "artifact_endpoint_url"' in versions_source
    assert '"artifact_endpoint_url": f"/api/artifacts/{manufacturing_stl.id}" if manufacturing_stl else None' in versions_source
    assert "function downloadUrlFromWorkbenchInvocation(" in viewer_page
    assert "invocation?.endpointUrl" in viewer_page
    assert "stringFromPayload(payload, ['artifact_url', 'download_url', 'url'])" in viewer_page
    assert "case 'download-stl': {" in viewer_page
    assert "const downloadUrl = downloadUrlFromWorkbenchInvocation(invocation, currentStlArtifact ? getArtifactUrl(currentStlArtifact.id) : null);" in viewer_page
    assert "window.open(downloadUrl, '_blank', 'noopener,noreferrer')" in viewer_page


def test_official_job_activity_command_tracks_payload_job_id() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()

    assert '"command_id": "job-activity"' in versions_source
    assert '"endpoint_url_key": "jobs_endpoint_url"' in versions_source
    assert "useVersionJobs(" in frontend_hooks
    assert "function jobIdFromWorkbenchPayload(" in viewer_page
    assert "['job_id', 'active_job_id', 'id']" in viewer_page
    assert "case 'job-activity': {" in viewer_page
    assert "const jobId = jobIdFromWorkbenchPayload(workbenchState);" in viewer_page
    assert "setActiveJobId(jobId);" in viewer_page
    assert "setRightDockTab('activity');" in viewer_page


def test_official_snapshots_command_executes_save_and_load_payloads() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()

    assert '"command_id": "snapshots"' in versions_source
    assert '"endpoint_url_key": "inspection_snapshots_endpoint_url"' in versions_source
    assert "getInspectionSnapshots(" in frontend_api
    assert "createInspectionSnapshot(" in frontend_api
    assert "useInspectionSnapshots(" in frontend_hooks
    assert "useCreateInspectionSnapshot()" in frontend_hooks
    assert "InspectionSnapshotState" in viewer_page
    assert "function inspectionSnapshotStateFromWorkbenchPayload(" in viewer_page
    assert "function findInspectionSnapshotForWorkbenchPayload(" in viewer_page
    assert "case 'snapshots': {" in viewer_page
    assert "const snapshotName = stringFromPayload(workbenchState, ['snapshot_name', 'name', 'label']);" in viewer_page
    assert "const snapshotToLoad = findInspectionSnapshotForWorkbenchPayload(" in viewer_page
    assert "onLoadInspection(snapshotToLoad);" in viewer_page
    assert "params: inspectionSnapshotStateFromWorkbenchPayload(" in viewer_page
    assert "setActiveTool('snapshots', 'inspect');" in viewer_page
    assert "setRightDockTab('tool');" in viewer_page


def test_official_export_section_command_fetches_sdk_contour_before_svg_download() -> None:
    versions_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()

    assert '"command_id": "export-section"' in versions_source
    assert '"endpoint_url_key": "section_endpoint_url"' in versions_source
    assert '"sdk_operations": ["section_contour"]' in versions_source
    assert "import { getSectionContour }" in viewer_page
    assert "function sectionContourParamsFromWorkbenchPayload(" in viewer_page
    assert "function sectionSvgFromContour(contour: SectionContourPayload)" in viewer_page
    assert "function downloadSectionContourSvg(contour: SectionContourPayload, sourceVersionId: string)" in viewer_page
    assert "case 'export-section': {" in viewer_page
    assert "const sectionParams = sectionContourParamsFromWorkbenchPayload(" in viewer_page
    assert "void getSectionContour(versionId, sectionParams).then((contour) => {" in viewer_page
    assert "downloadSectionContourSvg(contour, versionId);" in viewer_page
    assert "selected_region_ids: selectedRegionIdsFromWorkbench.length > 0 ? selectedRegionIdsFromWorkbench : fallbackRegionIds" in viewer_page


def test_official_runtime_command_lookup_accepts_meshlib_command_labels() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "const capabilityLabel = normalizeWorkbenchCommandId(capability.label)" in runtime_bootstrap
    assert "capabilityLabel === normalizedCommandId" in runtime_bootstrap


def test_official_runtime_command_lookup_prioritizes_runtime_aliases_over_workspace_tools() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "const exactCommandMatch = capabilities.find((capability) => {" in runtime_bootstrap
    assert "const runtimeLabelMatch = capabilities.find((capability) => {" in runtime_bootstrap
    assert "const runtimeToolMatch = capabilities.find((capability) => {" in runtime_bootstrap
    assert "const workspaceToolMatch = capabilities.find((capability) => {" in runtime_bootstrap
    assert "isRuntimeWorkbenchToolCommand(capability) && capabilityToolId === normalizedCommandId" in runtime_bootstrap
    assert "return runtimeToolMatch || workspaceToolMatch || null;" in runtime_bootstrap
    assert runtime_bootstrap.index("const exactCommandMatch") < runtime_bootstrap.index("const runtimeToolMatch")
    assert runtime_bootstrap.index("const runtimeToolMatch") < runtime_bootstrap.index("const workspaceToolMatch")


def test_meshlib_runtime_unload_settings_save_is_guarded() -> None:
    runtime_config = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "config.js").read_text()

    assert "window.onbeforeunload" in runtime_config
    assert "emsForceSettingsSave" in runtime_config
    assert "typeof Module === 'undefined' || typeof Module.ccall !== 'function'" in runtime_config
    assert "try {" in runtime_config
    assert "catch (error)" in runtime_config
    assert "MeshLib runtime settings save skipped on unload" in runtime_config


def test_frontend_activity_panel_consumes_version_scoped_job_history() -> None:
    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    activity_panel = (FRONTEND_ROOT / "src" / "features" / "editor" / "panels" / "JobActivityPanel.tsx").read_text()

    assert "getVersionJobs(" in frontend_api
    assert "`/api/versions/${versionId}/jobs`" in frontend_api
    assert "useVersionJobs(" in frontend_hooks
    assert "const versionJobsQuery = useVersionJobs(versionId)" in viewer_page
    assert "jobHistory={versionJobsQuery.data ?? []}" in viewer_page
    assert "jobHistory: JobResponse[]" in activity_panel
    assert "Recent Jobs" in activity_panel


def test_section_tool_uses_versioned_sdk_section_endpoint() -> None:
    backend_source = (BACKEND_ROOT / "api" / "routers" / "versions.py").read_text()
    assert '"/versions/{version_id}/section"' in backend_source
    assert "default_sdk.section_contour(" in backend_source

    frontend_api = (FRONTEND_ROOT / "src" / "lib" / "api" / "models.ts").read_text()
    assert "getSectionContour(" in frontend_api
    assert "/section?" in frontend_api

    frontend_hooks = (FRONTEND_ROOT / "src" / "hooks" / "useModelProcessing.ts").read_text()
    assert "useSectionContour(" in frontend_hooks

    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    assert "useSectionContour(" in viewer_page


def test_viewer_section_overlay_consumes_server_sdk_contour() -> None:
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    assert "sectionContour={activeSectionContour}" in viewer_page
    assert "onSectionContourChange" not in viewer_page

    viewer_engine = (FRONTEND_ROOT / "src" / "features" / "editor" / "viewer" / "ViewerEngine.tsx").read_text()
    assert "computeSliceStats" not in viewer_engine
    assert "onSectionContourChange" not in viewer_engine
    assert "contour={sectionContour}" in viewer_engine


def test_official_runtime_section_overlay_consumes_server_sdk_contour() -> None:
    runtime_bootstrap = (FRONTEND_ROOT / "public" / "meshlib-workbench" / "runtime" / "runtime_bootstrap.js").read_text()

    assert "function renderSectionContourOverlay(result)" in runtime_bootstrap
    assert "projectSectionPoint(segment?.start, origin, uAxis, vAxis)" in runtime_bootstrap
    assert "document.documentElement.dataset.meshinspectorWorkbenchSectionOverlay = 'ready'" in runtime_bootstrap
    assert "data-meshinspector-section-segment" in runtime_bootstrap
    assert "renderSectionContourOverlay(result)" in runtime_bootstrap


def test_section_svg_export_uses_server_sdk_projection_basis() -> None:
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    assert "createPlaneBasis" not in viewer_page
    assert "function sectionSvgFromContour(contour: SectionContourPayload)" in viewer_page
    assert "contour.plane_u_axis" in viewer_page
    assert "contour.plane_v_axis" in viewer_page
    assert "contour.projected_bounds_min" in viewer_page
    assert "contour.projected_bounds_max" in viewer_page


def test_compare_overlay_autosubmit_waits_for_cache_state() -> None:
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    assert "compareCacheQuery.isPending" in viewer_page
    assert "compareCacheQuery.isFetching" in viewer_page
    assert "compareMutation.isPending" in viewer_page


def test_frontend_selected_region_mutations_follow_manifest_allowed_operations() -> None:
    viewer_page = (FRONTEND_ROOT / "src" / "app" / "viewer" / "page.tsx").read_text()
    tool_inspector = (FRONTEND_ROOT / "src" / "features" / "editor" / "workspace" / "ToolInspector.tsx").read_text()

    assert "getSelectedRegionOperationAvailability(" in viewer_page
    assert "getBatchRegionOperationAvailability(" in viewer_page
    assert "allowed_operations.includes(operation)" in viewer_page
    assert "getSelectedRegionOperationReason(" in tool_inspector
    assert "getBatchRegionOperationReason(" in tool_inspector
    assert "allowed_operations.includes(operation)" in tool_inspector


def test_rust_extension_import_stays_at_accelerator_boundary() -> None:
    direct_importers = []
    for path in SDK_ROOT.rglob("*.py"):
        if "geometry_sdk._zennah_geometry_rs" in path.read_text():
            direct_importers.append(path.relative_to(SDK_ROOT).as_posix())

    assert direct_importers == ["accelerators/_rust_common.py"]


def test_public_geometry_feature_modules_cross_rust_accelerator_boundary() -> None:
    allowed_non_algorithm_modules = {
        "__init__.py",
        "engine.py",
        "types.py",
        "materials.py",
        "adapters/__init__.py",
        "adapters/meshlib_reference.py",
        "analysis/__init__.py",
        "analysis/artifacts.py",
        "core/__init__.py",
        "deform/__init__.py",
        "distance_map/__init__.py",
        "gcode/__init__.py",
        "io/__init__.py",
        "io/trimesh_adapter.py",
        "jewelry/__init__.py",
        "point_cloud/__init__.py",
        "repair/__init__.py",
        "spatial/__init__.py",
        "testing/__init__.py",
        "testing/fixtures.py",
        "testing/goldens.py",
        "testing/openvdb.py",
        "testing/parity.py",
        "testing/performance.py",
        "testing/uploaded_fragments.py",
        "voxel/__init__.py",
    }
    rust_boundary_markers = {
        "_rust",
        "accelerators import rust",
        "from geometry_sdk.accelerators",
        "import geometry_sdk.accelerators",
    }

    missing_rust_boundary = []
    for path in sorted(SDK_ROOT.rglob("*.py")):
        relative_path = path.relative_to(SDK_ROOT).as_posix()
        if relative_path.startswith("accelerators/") or relative_path in allowed_non_algorithm_modules:
            continue
        source = path.read_text()
        if not any(marker in source for marker in rust_boundary_markers):
            missing_rust_boundary.append(relative_path)

    assert missing_rust_boundary == []


def test_rust_owned_python_modules_stay_thin_wrappers() -> None:
    rust_owned = [
        SDK_ROOT / "analysis" / "compare.py",
        SDK_ROOT / "analysis" / "health.py",
        SDK_ROOT / "analysis" / "manufacturability.py",
        SDK_ROOT / "analysis" / "section.py",
        SDK_ROOT / "analysis" / "stats.py",
        SDK_ROOT / "analysis" / "thickness.py",
        SDK_ROOT / "core" / "mesh.py",
        SDK_ROOT / "deform" / "_distance.py",
        SDK_ROOT / "deform" / "brushes.py",
        SDK_ROOT / "deform" / "local.py",
        SDK_ROOT / "deform" / "resize.py",
        SDK_ROOT / "deform" / "thicken.py",
        SDK_ROOT / "jewelry" / "hollow.py",
        SDK_ROOT / "jewelry" / "regions.py",
        SDK_ROOT / "jewelry" / "ring_measurement.py",
        SDK_ROOT / "materials.py",
        SDK_ROOT / "repair" / "basic.py",
        SDK_ROOT / "repair" / "holes.py",
        SDK_ROOT / "repair" / "voxel.py",
        SDK_ROOT / "spatial" / "aabb_tree.py",
        SDK_ROOT / "spatial" / "closest_point.py",
        SDK_ROOT / "spatial" / "intersections.py",
        SDK_ROOT / "spatial" / "raycast.py",
        SDK_ROOT / "spatial" / "signed_distance.py",
        SDK_ROOT / "voxel" / "extract.py",
        SDK_ROOT / "voxel" / "marching.py",
        SDK_ROOT / "voxel" / "mesh_ops.py",
        SDK_ROOT / "voxel" / "ops.py",
        SDK_ROOT / "voxel" / "refine.py",
        SDK_ROOT / "voxel" / "sdf.py",
    ]
    forbidden_imports = {"math", "numpy", "geometry_sdk.core.mesh"}

    for path in rust_owned:
        imports = _imported_modules(path)
        leaked = forbidden_imports.intersection(imports)
        assert not leaked, f"{path.relative_to(SDK_ROOT)} should delegate geometry math to Rust, not {sorted(leaked)}"
