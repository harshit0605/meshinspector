from __future__ import annotations

import ast
from pathlib import Path


BACKEND_ROOT = Path(__file__).resolve().parents[1]
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


def test_rust_extension_import_stays_at_accelerator_boundary() -> None:
    direct_importers = []
    for path in SDK_ROOT.rglob("*.py"):
        if "geometry_sdk._zennah_geometry_rs" in path.read_text():
            direct_importers.append(path.relative_to(SDK_ROOT).as_posix())

    assert direct_importers == ["accelerators/_rust_common.py"]


def test_rust_owned_python_modules_stay_thin_wrappers() -> None:
    rust_owned = [
        SDK_ROOT / "analysis" / "compare.py",
        SDK_ROOT / "analysis" / "health.py",
        SDK_ROOT / "analysis" / "manufacturability.py",
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
