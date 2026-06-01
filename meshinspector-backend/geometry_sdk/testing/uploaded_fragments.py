"""Portable fixtures derived from local uploaded pipeline outputs."""

from __future__ import annotations

from dataclasses import asdict
import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import numpy as np

from geometry_sdk.analysis.health import compute_mesh_health
from geometry_sdk.analysis.stats import compute_mesh_stats
from geometry_sdk.core.mesh import connected_face_components
from geometry_sdk.io.trimesh_adapter import load_mesh
from geometry_sdk.testing.goldens import GOLDEN_DIR, load_golden
from geometry_sdk.types import MeshDocument


FRAGMENT_DIR = GOLDEN_DIR / "uploaded_fragments"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def extract_component_by_size_rank(mesh: MeshDocument, *, rank: int) -> MeshDocument:
    if rank < 1:
        raise ValueError("rank must be 1-based")
    components = connected_face_components(mesh)
    if rank > len(components):
        raise ValueError(f"rank {rank} is outside {len(components)} connected components")
    ordered_components = sorted(components, key=lambda component: (-len(component), min(component)))
    face_ids = np.asarray(ordered_components[rank - 1], dtype=np.int64)
    faces = mesh.faces[face_ids]
    used_vertices, inverse = np.unique(faces.reshape(-1), return_inverse=True)
    fragment = MeshDocument(
        vertices=mesh.vertices[used_vertices],
        faces=inverse.reshape(-1, 3),
        metadata={
            "source": "uploaded_processed_component",
            "component_rank_by_size": rank,
            "source_face_count": mesh.face_count,
            "source_vertex_count": mesh.vertex_count,
        },
    )
    return fragment


def save_npz_mesh(mesh: MeshDocument, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(
        path,
        vertices=mesh.vertices.astype(np.float64, copy=False),
        faces=mesh.faces.astype(np.int64, copy=False),
        metadata=json.dumps(mesh.metadata, sort_keys=True),
    )
    return path


def load_npz_mesh(path: Path) -> MeshDocument:
    with np.load(path) as artifact:
        metadata: dict[str, Any] = {}
        if "metadata" in artifact:
            metadata = json.loads(str(artifact["metadata"].item()))
        return MeshDocument(
            vertices=np.asarray(artifact["vertices"], dtype=np.float64),
            faces=np.asarray(artifact["faces"], dtype=np.int64),
            metadata=metadata,
        )


def fragment_payload(fragment_path: Path) -> dict[str, Any]:
    mesh = load_npz_mesh(fragment_path)
    return {
        "path": str(fragment_path),
        "sha256": sha256_file(fragment_path),
        "bytes": fragment_path.stat().st_size,
        "mesh": {"vertices": mesh.vertex_count, "faces": mesh.face_count},
        "sdk_stats": asdict(compute_mesh_stats(mesh)),
        "sdk_health": asdict(compute_mesh_health(mesh)),
        "metadata": dict(mesh.metadata),
    }


def build_fragments(*, repo_root: Path, manifest_name: str = "uploaded_sample_reference_v1.json") -> dict[str, Any]:
    manifest = load_golden(manifest_name)
    output: dict[str, Any] = {}
    for sample_name, sample in manifest["samples"].items():
        processed = sample["processed"]
        packaged = processed.get("packaged_fragment")
        if not packaged:
            continue
        source_path = repo_root / processed["path"]
        if not source_path.exists():
            raise FileNotFoundError(source_path)
        mesh = load_mesh(source_path)
        fragment = extract_component_by_size_rank(mesh, rank=int(packaged["component_rank_by_size"]))
        relative_fragment_path = Path(packaged["path"])
        fragment_path = repo_root / relative_fragment_path
        save_npz_mesh(fragment, fragment_path)
        output[sample_name] = fragment_payload(fragment_path)
    return output


def _main() -> None:
    parser = argparse.ArgumentParser(description="Build portable uploaded-sample mesh fragments.")
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[3])
    args = parser.parse_args()
    payload = build_fragments(repo_root=args.repo_root)
    print(json.dumps(payload, indent=2, sort_keys=True))


if __name__ == "__main__":
    _main()
