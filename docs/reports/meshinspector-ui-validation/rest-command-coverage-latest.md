# MeshInspector REST Workbench Coverage

Generated: 2026-06-12T21:33:56.759Z

Manifest: `.codex/ui-validation/current-workbench-manifest.json`
Evidence root: `.codex/ui-validation`

## Summary

- Rust-backed REST commands: 80/80 passing
- Untested commands: 0
- Failed commands: 0
- Guarded-only commands: 0
- SDK operations covered by at least one passing command: 171/171

## Non-Passing Commands

None.

## Passing Evidence Index

| Command | Group | Evidence |
|---|---|---|
| `export-section` | file | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `repair` | prepare | `.codex/ui-validation/official-ui-missing-mesh-edit-runtime/partial-results.json` |
| `fit-size` | prepare | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `reduce-weight` | prepare | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `prepare-casting` | prepare | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `make-manufacturable` | prepare | `.codex/ui-validation/official-ui-make-manufacturable-visual-ready-timeout-fixed/partial-results.json` |
| `resize` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `protected-hollow` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `offset-mesh` | modify | `.codex/ui-validation/official-ui-voxel-cube-visual-ready-after-guard/partial-results.json` |
| `shell-mesh` | modify | `.codex/ui-validation/official-ui-voxel-cube-visual-ready-after-guard/partial-results.json` |
| `thicken-mesh` | modify | `.codex/ui-validation/official-ui-offset-boolean-inspect-visual-ready/partial-results.json` |
| `weighted-shell` | modify | `.codex/ui-validation/official-ui-voxel-cube-visual-ready-after-guard/partial-results.json` |
| `partial-offset` | modify | `.codex/ui-validation/official-ui-voxel-cube-visual-ready-after-guard/partial-results.json` |
| `offset-verts` | modify | `.codex/ui-validation/official-ui-offset-boolean-inspect-visual-ready/partial-results.json` |
| `expand-shrink` | modify | `.codex/ui-validation/official-ui-voxel-cube-visual-ready-after-guard/partial-results.json` |
| `shrink-expand` | modify | `.codex/ui-validation/official-ui-voxel-cube-visual-ready-after-guard/partial-results.json` |
| `hollow-drains` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `thicken-violations` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `thicken-region` | modify | `.codex/ui-validation/official-ui-thicken-region-after-local-deform-fix/partial-results.json` |
| `batch-thicken` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `scoop` | modify | `.codex/ui-validation/official-ui-scoop-after-local-deform-fix/partial-results.json` |
| `smooth` | modify | `.codex/ui-validation/official-ui-smooth-after-local-deform-fix/partial-results.json` |
| `batch-smooth` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `decimate-mesh` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `subdivide-mesh` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `make-delone` | modify | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `section` | inspect | `.codex/ui-validation/official-ui-section-regions-exact-ids/partial-results.json` |
| `heatmap` | inspect | `.codex/ui-validation/official-ui-tail/partial-results.json` |
| `regions` | inspect | `.codex/ui-validation/official-ui-regions-exact-id-after-runtime-tool-fix/partial-results.json` |
| `measure-inspect` | inspect | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `mesh-cut-measure-path` | inspect | `.codex/ui-validation/official-ui-offset-boolean-inspect-visual-ready/partial-results.json` |
| `compare-versions` | review | `.codex/ui-validation/official-ui-compare-cube/partial-results.json` |
| `point-cloud-icp` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-contours` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-from-contours` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-to-contours` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `offset-contours` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-load-mrlines` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-save-mrlines` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-load-ply` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-save-ply` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-load-pts` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-load-svg` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-save-pts` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `object-lines-save-dxf` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-from-mesh` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-iso-lines` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-merge` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-contour-boolean` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-from-tiff` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `distance-map-to-tiff` | inspect | `.codex/ui-validation/official-ui-distance-lines-pointcloud-batch/partial-results.json` |
| `gcode-parse-paths` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `mesh-to-voxels-sdf` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-binary-operations` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `open-raw-voxels` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `open-voxels-from-tiff` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-slice` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-line-graph` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-active-box` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-volume-render-data` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-volume-render-lut` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-volume-render-ray` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-segmentation` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-mask-to-mesh` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-to-mesh-simple` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-to-mesh-smart` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-path` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-path-build-four` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `voxel-boolean` | inspect | `.codex/ui-validation/official-ui-boolean-cube-visual-ready-after-guard/partial-results.json` |
| `collision-detect` | inspect | `.codex/ui-validation/official-ui-tail/partial-results.json` |
| `exact-boolean` | inspect | `.codex/ui-validation/official-ui-boolean-cube-visual-ready-after-guard/partial-results.json` |
| `gcode-load-source` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `gcode-write-source` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `gcode-parse-file-paths` | inspect | `.codex/ui-validation/official-ui-sweep/partial-results.json` |
| `runtime-select-mark-region` | runtime | `.codex/ui-validation/official-ui-selection/partial-results.json` |
| `runtime-selection-to-object` | runtime | `.codex/ui-validation/official-ui-selection/partial-results.json` |
| `runtime-thicken-brush` | runtime | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `runtime-scoop-brush` | runtime | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `runtime-smooth-brush` | runtime | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
| `runtime-measure-inspect` | runtime | `.codex/ui-validation/official-ui-core-customer-workflows-visual-ready/partial-results.json` |
