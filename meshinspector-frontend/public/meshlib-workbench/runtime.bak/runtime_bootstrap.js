(function () {
    let hostPayload = null;
    let runtimeReady = false;
    let loadStarted = false;
    let activeWorkbenchCanvasTab = 'prepare';
    let workbenchCanvasBridgeInstalled = false;
    let workbenchCanvasCommandOverlay = null;
    let workbenchCanvasAccessibilityOverlay = null;
    let workbenchResultPanel = null;
    let workbenchSectionOverlay = null;
    let lastWorkbenchCanvasCommandDispatch = { commandId: null, at: 0 };

    function signalReady() {
        window.parent?.postMessage({ type: 'meshlib-workbench:ready' }, window.location.origin);
    }

    function extensionForPayload(manifest) {
        if (manifest?.meshlib_scene_mru_url) {
            return 'mru';
        }
        if (manifest?.normalized_mesh_url) {
            return 'ply';
        }
        if (manifest?.preview_high_url || manifest?.preview_low_url) {
            return 'glb';
        }
        return 'ply';
    }

    function meshUrlForPayload(manifest) {
        return manifest?.meshlib_scene_mru_url || manifest?.normalized_mesh_url || manifest?.preview_high_url || manifest?.preview_low_url || null;
    }

    function runtimeFilename(filename) {
        const fallback = 'meshlib-workbench-export.ply';
        if (!filename) {
            return fallback;
        }
        const parts = String(filename).split('/');
        return parts[parts.length - 1] || fallback;
    }

    function postHostMessage(type, payload) {
        window.parent?.postMessage({ type, payload }, window.location.origin);
    }

    function formatWorkbenchMetric(value, suffix = '', precision = 3) {
        if (typeof value !== 'number' || !Number.isFinite(value)) {
            return 'n/a';
        }
        return `${value.toFixed(precision)}${suffix}`;
    }

    function ensureWorkbenchResultPanel() {
        if (workbenchResultPanel?.isConnected) {
            return workbenchResultPanel;
        }
        const panel = document.createElement('section');
        panel.setAttribute('aria-live', 'polite');
        panel.setAttribute('aria-label', 'MeshInspector Rust result');
        panel.dataset.meshinspectorWorkbenchResultPanel = 'ready';
        Object.assign(panel.style, {
            position: 'fixed',
            top: '166px',
            right: '18px',
            zIndex: '2147483645',
            width: 'min(320px, calc(100vw - 36px))',
            maxHeight: 'calc(100vh - 210px)',
            overflow: 'auto',
            border: '1px solid rgba(148, 163, 184, 0.34)',
            borderRadius: '8px',
            background: 'rgba(8, 12, 21, 0.88)',
            boxShadow: '0 18px 48px rgba(0, 0, 0, 0.34)',
            color: '#e5e7eb',
            font: '12px/1.45 Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
            padding: '12px',
            pointerEvents: 'none',
            backdropFilter: 'blur(12px)',
        });
        document.body.appendChild(panel);
        workbenchResultPanel = panel;
        return panel;
    }

    function renderWorkbenchResultPanel({ title, status = 'Rust-backed REST result', rows = [], detail = '' }) {
        const panel = ensureWorkbenchResultPanel();
        panel.replaceChildren();

        const eyebrow = document.createElement('div');
        eyebrow.textContent = status;
        Object.assign(eyebrow.style, {
            color: '#93c5fd',
            fontSize: '10px',
            fontWeight: '700',
            letterSpacing: '0.14em',
            textTransform: 'uppercase',
        });
        panel.appendChild(eyebrow);

        const heading = document.createElement('div');
        heading.textContent = title;
        Object.assign(heading.style, {
            color: '#f8fafc',
            fontSize: '14px',
            fontWeight: '700',
            marginTop: '5px',
        });
        panel.appendChild(heading);

        if (rows.length > 0) {
            const grid = document.createElement('div');
            Object.assign(grid.style, {
                display: 'grid',
                gridTemplateColumns: 'minmax(0, 1fr) auto',
                gap: '6px 12px',
                marginTop: '10px',
            });
            for (const row of rows) {
                const label = document.createElement('span');
                label.textContent = row.label;
                label.style.color = '#94a3b8';
                const value = document.createElement('strong');
                value.textContent = row.value;
                Object.assign(value.style, {
                    color: '#f8fafc',
                    fontWeight: '600',
                    textAlign: 'right',
                });
                grid.append(label, value);
            }
            panel.appendChild(grid);
        }

        if (detail) {
            const note = document.createElement('p');
            note.textContent = detail;
            Object.assign(note.style, {
                color: '#cbd5e1',
                margin: '10px 0 0',
            });
            panel.appendChild(note);
        }
    }

    function renderMeasureInspectResult(result) {
        const firstPoint = Array.isArray(result?.points) ? result.points[0] : null;
        const firstPair = Array.isArray(result?.point_pairs) ? result.point_pairs[0] : null;
        const rows = [
            { label: 'Version', value: String(result?.version_id || hostPayload?.manifest?.version_id || 'active') },
            { label: 'Point probes', value: String(Array.isArray(result?.points) ? result.points.length : 0) },
            { label: 'Point pairs', value: String(Array.isArray(result?.point_pairs) ? result.point_pairs.length : 0) },
        ];
        if (firstPoint) {
            rows.push(
                { label: 'First distance', value: formatWorkbenchMetric(firstPoint.distance_to_surface_mm, ' mm') },
                { label: 'Local thickness', value: formatWorkbenchMetric(firstPoint.local_thickness_mm, ' mm') },
                { label: 'Closest face', value: String(firstPoint.face_index ?? 'n/a') },
            );
        }
        if (firstPair) {
            rows.push({ label: 'Pair length', value: formatWorkbenchMetric(firstPair.distance_mm, ' mm') });
        }
        renderWorkbenchResultPanel({
            title: 'Measure Dimensions',
            status: 'Rust-backed REST result',
            rows,
        });
    }

    function ensureWorkbenchSectionOverlay() {
        if (workbenchSectionOverlay?.isConnected) {
            return workbenchSectionOverlay;
        }
        const overlay = document.createElement('div');
        overlay.dataset.meshinspectorSectionOverlay = 'empty';
        overlay.setAttribute('aria-label', 'MeshInspector section contour preview');
        Object.assign(overlay.style, {
            position: 'fixed',
            left: '24px',
            bottom: '24px',
            width: 'min(340px, calc(100vw - 48px))',
            aspectRatio: '16 / 10',
            zIndex: '2147483644',
            pointerEvents: 'none',
            border: '1px solid rgba(148, 163, 184, 0.36)',
            borderRadius: '8px',
            background: 'rgba(2, 6, 23, 0.78)',
            boxShadow: '0 18px 48px rgba(0, 0, 0, 0.34)',
            backdropFilter: 'blur(10px)',
            overflow: 'hidden',
            display: 'none',
        });
        document.body.appendChild(overlay);
        workbenchSectionOverlay = overlay;
        return overlay;
    }

    function clearWorkbenchSectionOverlay(state = 'empty') {
        const overlay = ensureWorkbenchSectionOverlay();
        overlay.replaceChildren();
        overlay.style.display = state === 'ready' ? 'block' : 'none';
        overlay.dataset.meshinspectorSectionOverlay = state;
        document.documentElement.dataset.meshinspectorWorkbenchSectionOverlay = state;
        document.documentElement.dataset.meshinspectorWorkbenchSectionSegmentCount = '0';
        document.documentElement.dataset.meshinspectorWorkbenchSectionContourCount = '0';
    }

    function sectionVector(value, length) {
        if (!Array.isArray(value) || value.length < length) {
            return null;
        }
        const vector = value.slice(0, length).map((item) => Number(item));
        return vector.every((item) => Number.isFinite(item)) ? vector : null;
    }

    function projectSectionPoint(point, origin, uAxis, vAxis) {
        const source = sectionVector(point, 3);
        if (!source) {
            return null;
        }
        const relative = [
            source[0] - origin[0],
            source[1] - origin[1],
            source[2] - origin[2],
        ];
        return [
            relative[0] * uAxis[0] + relative[1] * uAxis[1] + relative[2] * uAxis[2],
            relative[0] * vAxis[0] + relative[1] * vAxis[1] + relative[2] * vAxis[2],
        ];
    }

    function sectionProjectedBounds(result, projectedSegments) {
        const explicitMin = sectionVector(result?.projected_bounds_min, 2);
        const explicitMax = sectionVector(result?.projected_bounds_max, 2);
        if (explicitMin && explicitMax && explicitMax[0] > explicitMin[0] && explicitMax[1] > explicitMin[1]) {
            return { min: explicitMin, max: explicitMax };
        }

        const points = projectedSegments.flatMap((segment) => [segment.start, segment.end]);
        if (points.length === 0) {
            return null;
        }
        return {
            min: [
                Math.min(...points.map((point) => point[0])),
                Math.min(...points.map((point) => point[1])),
            ],
            max: [
                Math.max(...points.map((point) => point[0])),
                Math.max(...points.map((point) => point[1])),
            ],
        };
    }

    function renderSectionContourOverlay(result) {
        const rawSegments = Array.isArray(result?.segments) ? result.segments : [];
        const origin = sectionVector(result?.plane_origin, 3);
        const uAxis = sectionVector(result?.plane_u_axis, 3);
        const vAxis = sectionVector(result?.plane_v_axis, 3);
        if (!origin || !uAxis || !vAxis || rawSegments.length === 0) {
            clearWorkbenchSectionOverlay('empty');
            return;
        }

        const projectedSegments = [];
        for (const segment of rawSegments) {
            const start = projectSectionPoint(segment?.start, origin, uAxis, vAxis);
            const end = projectSectionPoint(segment?.end, origin, uAxis, vAxis);
            if (start && end) {
                projectedSegments.push({ start, end, selected: Boolean(segment?.selected_region_hit) });
            }
        }
        const bounds = sectionProjectedBounds(result, projectedSegments);
        if (!bounds || projectedSegments.length === 0) {
            clearWorkbenchSectionOverlay('empty');
            return;
        }

        const overlay = ensureWorkbenchSectionOverlay();
        const width = 340;
        const height = 212.5;
        const padding = 22;
        const spanU = Math.max(bounds.max[0] - bounds.min[0], 1e-6);
        const spanV = Math.max(bounds.max[1] - bounds.min[1], 1e-6);
        const scale = Math.min((width - padding * 2) / spanU, (height - padding * 2) / spanV);
        const drawWidth = spanU * scale;
        const drawHeight = spanV * scale;
        const offsetX = (width - drawWidth) / 2;
        const offsetY = (height - drawHeight) / 2;
        const mapPoint = (point) => [
            offsetX + (point[0] - bounds.min[0]) * scale,
            height - (offsetY + (point[1] - bounds.min[1]) * scale),
        ];

        const svgNamespace = 'http://www.w3.org/2000/svg';
        const svg = document.createElementNS(svgNamespace, 'svg');
        svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
        svg.setAttribute('role', 'img');
        svg.setAttribute('aria-label', 'Section contour');
        Object.assign(svg.style, {
            display: 'block',
            width: '100%',
            height: '100%',
        });

        const rect = document.createElementNS(svgNamespace, 'rect');
        rect.setAttribute('x', String(offsetX));
        rect.setAttribute('y', String(height - offsetY - drawHeight));
        rect.setAttribute('width', String(drawWidth));
        rect.setAttribute('height', String(drawHeight));
        rect.setAttribute('fill', 'rgba(15, 23, 42, 0.42)');
        rect.setAttribute('stroke', 'rgba(148, 163, 184, 0.36)');
        rect.setAttribute('stroke-width', '1');
        svg.appendChild(rect);

        for (const segment of projectedSegments) {
            const [x1, y1] = mapPoint(segment.start);
            const [x2, y2] = mapPoint(segment.end);
            const line = document.createElementNS(svgNamespace, 'line');
            line.dataset.meshinspectorSectionSegment = 'true';
            line.setAttribute('data-meshinspector-section-segment', 'true');
            line.setAttribute('x1', x1.toFixed(3));
            line.setAttribute('y1', y1.toFixed(3));
            line.setAttribute('x2', x2.toFixed(3));
            line.setAttribute('y2', y2.toFixed(3));
            line.setAttribute('stroke', segment.selected ? '#f59e0b' : '#38bdf8');
            line.setAttribute('stroke-width', segment.selected ? '2.8' : '2.2');
            line.setAttribute('stroke-linecap', 'round');
            svg.appendChild(line);
        }

        overlay.replaceChildren(svg);
        overlay.style.display = 'block';
        overlay.dataset.meshinspectorSectionOverlay = 'ready';
        document.documentElement.dataset.meshinspectorWorkbenchSectionOverlay = 'ready';
        document.documentElement.dataset.meshinspectorWorkbenchSectionSegmentCount = String(projectedSegments.length);
        document.documentElement.dataset.meshinspectorWorkbenchSectionContourCount = String(result?.contour_count ?? 0);
    }

    function renderSectionContourResult(result) {
        renderWorkbenchResultPanel({
            title: 'Section Slice',
            status: 'Rust-backed REST result',
            rows: [
                { label: 'Version', value: String(result?.version_id || hostPayload?.manifest?.version_id || 'active') },
                { label: 'Contours', value: String(result?.contour_count ?? 0) },
                { label: 'Segments', value: String(result?.segment_count ?? 0) },
                { label: 'Perimeter', value: formatWorkbenchMetric(result?.perimeter_mm, ' mm', 2) },
                { label: 'Width', value: formatWorkbenchMetric(result?.width_mm, ' mm', 2) },
                { label: 'Depth', value: formatWorkbenchMetric(result?.depth_mm, ' mm', 2) },
            ],
        });
        renderSectionContourOverlay(result);
    }

    function countPayloadItems(value, key) {
        const count = value?.[key];
        if (typeof count === 'number' && Number.isFinite(count)) {
            return count;
        }
        return 0;
    }

    function renderSelectionCommitResult(result) {
        const selectionCounts = result?.selection_counts || {};
        const resolvedCounts = result?.resolved_counts || {};
        renderWorkbenchResultPanel({
            title: 'Select / Mark Region',
            status: 'Rust-backed REST result',
            rows: [
                { label: 'Version', value: String(result?.version_id || hostPayload?.manifest?.version_id || 'active') },
                { label: 'Artifact', value: String(result?.artifact_id || 'n/a') },
                { label: 'Faces', value: String(countPayloadItems(selectionCounts, 'face_ids')) },
                { label: 'Regions', value: String(countPayloadItems(selectionCounts, 'region_ids')) },
                { label: 'Resolved vertices', value: String(countPayloadItems(resolvedCounts, 'vertex_ids')) },
                { label: 'Object version', value: String(result?.selected_object_version_id || 'none') },
            ],
        });
    }

    function searchParamsForWindow(targetWindow) {
        try {
            const href = targetWindow?.location?.href;
            return href ? new URL(href).searchParams : null;
        } catch (_error) {
            return null;
        }
    }

    function activeWorkbenchSearchParams() {
        return searchParamsForWindow(window.top) || searchParamsForWindow(window.parent) || searchParamsForWindow(window);
    }

    function splitWorkbenchListValue(value) {
        return String(value || '')
            .split(',')
            .map((item) => item.trim())
            .filter(Boolean);
    }

    function parseWorkbenchIntegerListValue(value) {
        return splitWorkbenchListValue(value)
            .map((item) => Number.parseInt(item, 10))
            .filter((item) => Number.isInteger(item) && item >= 0);
    }

    function numberFromWorkbenchParams(params, keys, fallback) {
        for (const key of keys) {
            const rawValue = params?.get(key);
            if (rawValue !== null && rawValue !== undefined && rawValue !== '') {
                const parsed = Number(rawValue);
                if (Number.isFinite(parsed)) {
                    return parsed;
                }
            }
        }
        return fallback;
    }

    function integerListFromWorkbenchParams(params, keys) {
        for (const key of keys) {
            const parsed = parseWorkbenchIntegerListValue(params?.get(key));
            if (parsed.length > 0) {
                return parsed;
            }
        }
        return [];
    }

    function manifestRegionEntries() {
        return Array.isArray(hostPayload?.manifest?.region_manifest) ? hostPayload.manifest.region_manifest : [];
    }

    function activeWorkbenchRegionId() {
        const params = activeWorkbenchSearchParams();
        const directRegion = params?.get('region') || params?.get('selected_region_id') || params?.get('region_id');
        if (directRegion) {
            return directRegion;
        }
        const selectedRegions = splitWorkbenchListValue(params?.get('regions_selected') || params?.get('selected_region_ids') || params?.get('region_ids'));
        if (selectedRegions.length > 0) {
            return selectedRegions[0];
        }
        const regions = manifestRegionEntries();
        const innerBand = regions.find((region) =>
            region?.region_id === 'inner_band' &&
            Array.isArray(region.allowed_operations) &&
            region.allowed_operations.includes('scoop') &&
            Number(region.vertex_count || 0) > 0,
        );
        const editableRegion = regions.find((region) =>
            Array.isArray(region?.allowed_operations) &&
            region.allowed_operations.includes('scoop') &&
            Number(region.vertex_count || 0) > 0,
        );
        return innerBand?.region_id || editableRegion?.region_id || regions[0]?.region_id || 'inner_band';
    }

    function activeWorkbenchRegionIds() {
        const params = activeWorkbenchSearchParams();
        const selectedRegions = splitWorkbenchListValue(params?.get('regions_selected') || params?.get('selected_region_ids') || params?.get('region_ids'));
        return selectedRegions.length > 0 ? selectedRegions : [activeWorkbenchRegionId()];
    }

    function defaultBrushSelectionPayload() {
        return {
            mode: 'faces',
            vertex_ids: [],
            face_ids: [0],
            region_ids: [],
            brush_points_world: [],
            metadata: {
                selector: 'meshlib_canvas_face_selection',
                source: 'meshlib_canvas_plugin_overlay',
            },
        };
    }

    function defaultSubdividePayload(source) {
        const params = activeWorkbenchSearchParams();
        const queryFaces = integerListFromWorkbenchParams(
            params,
            ['subdivide_faces', 'faces_selected', 'selected_faces'],
        );
        return {
            max_edge_len: numberFromWorkbenchParams(
                params,
                ['subdivide_max_edge_len', 'max_edge_len', 'edge_length_mm'],
                0.04,
            ),
            max_edge_splits: Math.max(1, Math.round(numberFromWorkbenchParams(
                params,
                ['subdivide_max_edge_splits', 'max_edge_splits'],
                4,
            ))),
            region_faces: queryFaces.length > 0 ? queryFaces : [1],
            subdivide_border: true,
            curvature_priority: 0,
            project_on_original_mesh: false,
            smooth_mode: false,
            min_sharp_dihedral_angle_degrees: 30,
            max_tri_aspect_ratio: 12,
            metadata: {
                source,
                selection_source: queryFaces.length > 0 ? 'url_selected_faces' : 'validation_long_edge_region',
            },
        };
    }

    function resolveWorkbenchCommandPayload(command) {
        if (typeof command.payload === 'function') {
            try {
                return command.payload() || {};
            } catch (error) {
                postHostMessage('meshlib-workbench:command-failed', {
                    command_id: command.commandId,
                    error: error instanceof Error ? error.message : 'MeshLib workbench command payload failed',
                });
                return {};
            }
        }
        return command.payload ?? {};
    }

    const WORKBENCH_TOOL_COMMAND_ALIASES = Object.freeze({
        'Select / Mark Region': 'runtime-select-mark-region',
        'select_mark_region': 'runtime-select-mark-region',
        'RegionMarkTool': 'runtime-select-mark-region',
        'Selection to Object': 'runtime-selection-to-object',
        'selection_to_object': 'runtime-selection-to-object',
        'SelectionToObjectTool': 'runtime-selection-to-object',
        'Thicken Brush': 'runtime-thicken-brush',
        'thicken_brush': 'runtime-thicken-brush',
        'ThickenBrushTool': 'runtime-thicken-brush',
        'Scoop Brush': 'runtime-scoop-brush',
        'scoop_brush': 'runtime-scoop-brush',
        'ScoopBrushTool': 'runtime-scoop-brush',
        'Smooth Brush': 'runtime-smooth-brush',
        'smooth_brush': 'runtime-smooth-brush',
        'SmoothBrushTool': 'runtime-smooth-brush',
        'Decimate Mesh': 'decimate-mesh',
        'decimate_mesh': 'decimate-mesh',
        'DecimateMeshTool': 'decimate-mesh',
        'Subdivide Mesh': 'subdivide-mesh',
        'subdivide_mesh': 'subdivide-mesh',
        'SubdivideMeshTool': 'subdivide-mesh',
        'Make Delone': 'make-delone',
        'make_delone': 'make-delone',
        'MakeDeloneTool': 'make-delone',
        'Measure / Inspect': 'runtime-measure-inspect',
        'measure_inspect': 'runtime-measure-inspect',
        'MeasureInspectTool': 'runtime-measure-inspect',
        'Mesh Cut & Measure Path': 'mesh-cut-measure-path',
        'mesh_cut_measure_path': 'mesh-cut-measure-path',
        'mesh_cut_and_measure': 'mesh-cut-measure-path',
        'MeshCutMeasurePathTool': 'mesh-cut-measure-path',
        'Mesh to Voxels / SDF': 'mesh-to-voxels-sdf',
        'mesh_to_voxels_sdf': 'mesh-to-voxels-sdf',
        'MeshToVoxelsSdfTool': 'mesh-to-voxels-sdf',
        'Offset Mesh': 'offset-verts',
        'offset_mesh': 'offset-verts',
        'OffsetMeshTool': 'offset-verts',
        'Shell Mesh': 'shell-mesh',
        'shell_mesh': 'shell-mesh',
        'ShellMeshTool': 'shell-mesh',
        'Thickening': 'thicken-mesh',
        'thicken_mesh': 'thicken-mesh',
        'ThickeningTool': 'thicken-mesh',
        'Weighted Shell': 'weighted-shell',
        'weighted_shell': 'weighted-shell',
        'WeightedShellTool': 'weighted-shell',
        'Partial Offset': 'partial-offset',
        'partial_offset': 'partial-offset',
        'PartialOffsetTool': 'partial-offset',
        'Offset Verts': 'offset-verts',
        'offset_verts': 'offset-verts',
        'OffsetVertsTool': 'offset-verts',
        'Offset Contours': 'offset-contours',
        'offset_contours': 'offset-contours',
        'OffsetContoursTool': 'offset-contours',
        'Contour Distance Map': 'distance-map-contours',
        'Distance Map From Contours': 'distance-map-contours',
        'distance_map_from_contours': 'distance-map-contours',
        'DistanceMapContoursTool': 'distance-map-contours',
        'Distance Map Iso-Lines': 'distance-map-iso-lines',
        'Distance Map Iso Lines': 'distance-map-iso-lines',
        'distance_map_to_iso_segments': 'distance-map-iso-lines',
        'DistanceMapIsoLinesTool': 'distance-map-iso-lines',
        'Distance Map Merge': 'distance-map-merge',
        'distance_map_merge': 'distance-map-merge',
        'DistanceMapMergeTool': 'distance-map-merge',
        'Contour Boolean From Distance Maps': 'distance-map-contour-boolean',
        'Contour Boolean': 'distance-map-contour-boolean',
        'distance_map_contour_boolean': 'distance-map-contour-boolean',
        'DistanceMapContourBooleanTool': 'distance-map-contour-boolean',
        'ObjectLines From Contours': 'object-lines-from-contours',
        'object_lines_from_contours': 'object-lines-from-contours',
        'ObjectLinesFromContoursTool': 'object-lines-from-contours',
        'ObjectLines To Contours': 'object-lines-to-contours',
        'object_lines_to_contours': 'object-lines-to-contours',
        'ObjectLinesToContoursTool': 'object-lines-to-contours',
        'Point Cloud / ICP': 'point-cloud-icp',
        'point-cloud-icp': 'point-cloud-icp',
        'point_cloud_icp': 'point-cloud-icp',
        'PointCloudIcpTool': 'point-cloud-icp',
        'G-code Path Parser': 'gcode-parse-paths',
        'gcode-parse-paths': 'gcode-parse-paths',
        'gcode_parse_paths': 'gcode-parse-paths',
        'GcodeParsePathsTool': 'gcode-parse-paths',
        'Load G-code Source': 'gcode-load-source',
        'gcode-load-source': 'gcode-load-source',
        'gcode_load_source': 'gcode-load-source',
        'GcodeLoadSourceTool': 'gcode-load-source',
        'Write G-code Source': 'gcode-write-source',
        'gcode-write-source': 'gcode-write-source',
        'gcode_write_source': 'gcode-write-source',
        'GcodeWriteSourceTool': 'gcode-write-source',
        'Parse G-code File Paths': 'gcode-parse-file-paths',
        'gcode-parse-file-paths': 'gcode-parse-file-paths',
        'gcode_parse_file_paths': 'gcode-parse-file-paths',
        'GcodeParseFilePathsTool': 'gcode-parse-file-paths',
        'Expand/Shrink': 'expand-shrink',
        'expand_shrink': 'expand-shrink',
        'ExpandShrinkTool': 'expand-shrink',
        'Shrink/Expand': 'shrink-expand',
        'shrink_expand': 'shrink-expand',
        'ShrinkExpandTool': 'shrink-expand',
        'Exact Boolean': 'exact-boolean',
        'exact_boolean': 'exact-boolean',
        'ExactBooleanTool': 'exact-boolean',
        'Voxel Boolean': 'voxel-boolean',
        'voxel_boolean': 'voxel-boolean',
        'VoxelBooleanTool': 'voxel-boolean',
        'Binary Operations': 'voxel-binary-operations',
        'voxel-binary-operations': 'voxel-binary-operations',
        'voxel_binary_operations': 'voxel-binary-operations',
        'BinaryOperationsTool': 'voxel-binary-operations',
        'Voxels Line Graph': 'voxel-line-graph',
        'voxel-line-graph': 'voxel-line-graph',
        'voxel_line_graph': 'voxel-line-graph',
        'VoxelsLineGraphTool': 'voxel-line-graph',
        'Set Active Voxel Box': 'voxel-active-box',
        'voxel-active-box': 'voxel-active-box',
        'voxel_active_box': 'voxel-active-box',
        'SetActiveVoxelBoxTool': 'voxel-active-box',
        'Voxels Slice': 'voxel-slice',
        'voxel-slice': 'voxel-slice',
        'voxel_slice': 'voxel-slice',
        'VoxelsSliceTool': 'voxel-slice',
        'Voxels Segmentation': 'voxel-segmentation',
        'voxel-segmentation': 'voxel-segmentation',
        'voxel_segmentation': 'voxel-segmentation',
        'VoxelsSegmentationTool': 'voxel-segmentation',
        'Voxels Mask to Mesh': 'voxel-mask-to-mesh',
        'voxel-mask-to-mesh': 'voxel-mask-to-mesh',
        'voxel_mask_to_mesh': 'voxel-mask-to-mesh',
        'VoxelsMaskToMeshTool': 'voxel-mask-to-mesh',
        'Voxels Path': 'voxel-path',
        'voxel-path': 'voxel-path',
        'voxel_path': 'voxel-path',
        'VoxelsPathTool': 'voxel-path',
        'Voxels Path Build Four': 'voxel-path-build-four',
        'voxel-path-build-four': 'voxel-path-build-four',
        'voxel_path_build_four': 'voxel-path-build-four',
        'VoxelsPathBuildFourTool': 'voxel-path-build-four',
        'Voxels to Mesh Simple': 'voxel-to-mesh-simple',
        'voxel-to-mesh-simple': 'voxel-to-mesh-simple',
        'voxel_to_mesh_simple': 'voxel-to-mesh-simple',
        'VoxelsToMeshSimpleTool': 'voxel-to-mesh-simple',
        'Voxels to Mesh Smart': 'voxel-to-mesh-smart',
        'voxel-to-mesh-smart': 'voxel-to-mesh-smart',
        'voxel_to_mesh_smart': 'voxel-to-mesh-smart',
        'VoxelsToMeshSmartTool': 'voxel-to-mesh-smart',
        'Voxels Volume Rendering Data': 'voxel-volume-render-data',
        'voxel-volume-render-data': 'voxel-volume-render-data',
        'voxel_volume_render_data': 'voxel-volume-render-data',
        'VoxelsVolumeRenderingDataTool': 'voxel-volume-render-data',
        'Voxels Volume Rendering LUT': 'voxel-volume-render-lut',
        'voxel-volume-render-lut': 'voxel-volume-render-lut',
        'voxel_volume_render_lut': 'voxel-volume-render-lut',
        'VoxelsVolumeRenderingLutTool': 'voxel-volume-render-lut',
        'Voxels Volume Rendering Ray': 'voxel-volume-render-ray',
        'voxel-volume-render-ray': 'voxel-volume-render-ray',
        'voxel_volume_render_ray': 'voxel-volume-render-ray',
        'VoxelsVolumeRenderingRayTool': 'voxel-volume-render-ray',
        'Collision Detection': 'collision-detect',
        'collision_detection': 'collision-detect',
        'CollisionDetectionTool': 'collision-detect',
    });

    const WORKBENCH_CANVAS_COMMAND_HITBOXES = Object.freeze({
        'mesh-edit': [
            {
                commandId: 'decimate-mesh',
                minX: 420,
                maxX: 540,
                minY: 36,
                maxY: 124,
                payload: {
                    strategy: 'minimize_error',
                    max_error: 1000.0,
                    max_deleted_vertices: 100000,
                    max_deleted_faces: 100000,
                    pack_mesh: true,
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                },
                options: { execute: true },
            },
            {
                commandId: 'subdivide-mesh',
                minX: 540,
                maxX: 660,
                minY: 36,
                maxY: 124,
                payload: () => defaultSubdividePayload('meshlib_canvas_plugin_button'),
                options: { execute: true },
            },
            {
                commandId: 'make-delone',
                minX: 660,
                maxX: 770,
                minY: 36,
                maxY: 124,
                payload: {
                    num_iters: 1,
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                },
                options: { execute: true },
            },
            {
                commandId: 'runtime-thicken-brush',
                minX: 770,
                maxX: 890,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    strokes: [
                        {
                            selection: defaultBrushSelectionPayload(),
                            amount_mm: 0.04,
                            falloff_mm: 1.5,
                            iterations: 1,
                            strength: 0.35,
                            metadata: { source: 'meshlib_canvas_plugin_button' },
                        },
                    ],
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'runtime-smooth-brush',
                minX: 890,
                maxX: 1010,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    strokes: [
                        {
                            selection: defaultBrushSelectionPayload(),
                            amount_mm: 0,
                            falloff_mm: 1.5,
                            iterations: 1,
                            strength: 0.35,
                            metadata: { source: 'meshlib_canvas_plugin_button' },
                        },
                    ],
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'scoop',
                minX: 1010,
                maxX: 1128,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    region_id: activeWorkbenchRegionId(),
                    depth_mm: 0.05,
                    falloff_mm: 1.5,
                    keep_min_thickness_mm: 0.6,
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'runtime-scoop-brush',
                minX: 1128,
                maxX: 1248,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    strokes: [
                        {
                            selection: defaultBrushSelectionPayload(),
                            amount_mm: 0.04,
                            falloff_mm: 1.5,
                            iterations: 1,
                            strength: 0.35,
                            metadata: { source: 'meshlib_canvas_plugin_button' },
                        },
                    ],
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'protected-hollow',
                minX: 1248,
                maxX: 1390,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    mode: 'fixed_thickness',
                    material: 'gold_18k',
                    wall_thickness_mm: 0.8,
                    processing_mode: 'interactive',
                    min_allowed_thickness_mm: 0.6,
                    protect_regions: ['head', 'gem_seat', 'ornament_relief', 'inner_band'],
                    add_drain_holes: false,
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                }),
                options: { execute: true },
            },
        ],
        'inspect-features': [
            {
                commandId: 'runtime-measure-inspect',
                minX: 0,
                maxX: 140,
                minY: 36,
                maxY: 124,
                payload: {
                    points: [[0, 0, 0], [5, 5, 5]],
                    include_local_thickness: true,
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                },
                options: { execute: true },
            },
            {
                commandId: 'section',
                minX: 140,
                maxX: 280,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    section_enabled: true,
                    section_constant: 0,
                    plane_axis: [0, 0, 1],
                    selected_region_ids: activeWorkbenchRegionIds(),
                    metadata: { source: 'meshlib_canvas_plugin_button' },
                }),
                options: { execute: true },
            },
        ],
    });

    const WORKBENCH_CANVAS_COMMAND_OVERLAYS = Object.freeze({
        'selection': [
            {
                commandId: 'runtime-select-mark-region',
                label: 'Select / Mark Region',
                minX: 180,
                maxX: 360,
                minY: 36,
                maxY: 124,
                payload: {
                    selection: {
                        mode: 'faces',
                        face_ids: [0, 1, 2, 3, 4, 5, 6, 7],
                        metadata: {
                            selector: 'meshlib_canvas_face_selection',
                            source: 'meshlib_canvas_plugin_overlay',
                        },
                    },
                    label: 'Workbench face selection',
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
        ],
        'mesh-edit': [
            {
                commandId: 'decimate-mesh',
                label: 'Decimate Mesh',
                minX: 420,
                maxX: 540,
                minY: 36,
                maxY: 124,
                payload: {
                    strategy: 'minimize_error',
                    max_error: 1000.0,
                    max_deleted_vertices: 100000,
                    max_deleted_faces: 100000,
                    pack_mesh: true,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'subdivide-mesh',
                label: 'Subdivide Mesh',
                minX: 540,
                maxX: 660,
                minY: 36,
                maxY: 124,
                payload: () => defaultSubdividePayload('meshlib_canvas_plugin_overlay'),
                options: { execute: true },
            },
            {
                commandId: 'make-delone',
                label: 'Make Delone',
                minX: 660,
                maxX: 770,
                minY: 36,
                maxY: 124,
                payload: {
                    num_iters: 1,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'runtime-thicken-brush',
                label: 'Thicken (Quick)',
                minX: 770,
                maxX: 890,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    strokes: [
                        {
                            selection: defaultBrushSelectionPayload(),
                            amount_mm: 0.04,
                            falloff_mm: 1.5,
                            iterations: 1,
                            strength: 0.35,
                            metadata: { source: 'meshlib_canvas_plugin_overlay' },
                        },
                    ],
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'runtime-smooth-brush',
                label: 'Smooth (Quick)',
                minX: 890,
                maxX: 1010,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    strokes: [
                        {
                            selection: defaultBrushSelectionPayload(),
                            amount_mm: 0,
                            falloff_mm: 1.5,
                            iterations: 1,
                            strength: 0.35,
                            metadata: { source: 'meshlib_canvas_plugin_overlay' },
                        },
                    ],
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'scoop',
                label: 'Scoop Region',
                minX: 1010,
                maxX: 1128,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    region_id: activeWorkbenchRegionId(),
                    depth_mm: 0.05,
                    falloff_mm: 1.5,
                    keep_min_thickness_mm: 0.6,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'runtime-scoop-brush',
                label: 'Scoop (Quick)',
                minX: 1128,
                maxX: 1248,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    strokes: [
                        {
                            selection: defaultBrushSelectionPayload(),
                            amount_mm: 0.04,
                            falloff_mm: 1.5,
                            iterations: 1,
                            strength: 0.35,
                            metadata: { source: 'meshlib_canvas_plugin_overlay' },
                        },
                    ],
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'protected-hollow',
                label: 'Protected Hollow',
                minX: 1248,
                maxX: 1390,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    mode: 'fixed_thickness',
                    material: 'gold_18k',
                    wall_thickness_mm: 0.8,
                    min_allowed_thickness_mm: 0.6,
                    protect_regions: ['head', 'gem_seat', 'ornament_relief', 'inner_band'],
                    add_drain_holes: false,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                }),
                options: { execute: true },
            },
            {
                commandId: 'repair',
                label: 'Auto Repair',
                minX: 240,
                maxX: 350,
                minY: 124,
                maxY: 200,
                payload: { metadata: { source: 'meshlib_canvas_plugin_overlay' } },
                options: { execute: true },
            },
            {
                commandId: 'resize',
                label: 'Resize',
                minX: 350,
                maxX: 450,
                minY: 124,
                maxY: 200,
                payload: {
                    target_ring_size_us: 5,
                    axis_mode: 'auto',
                    preserve_head: true,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'reduce-weight',
                label: 'Reduce Weight',
                minX: 450,
                maxX: 590,
                minY: 124,
                maxY: 200,
                payload: {
                    mode: 'target_weight',
                    processing_mode: 'interactive',
                    material: 'gold_18k',
                    target_weight_g: 3,
                    min_allowed_thickness_mm: 0.6,
                    protect_regions: [],
                    add_drain_holes: false,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'prepare-casting',
                label: 'Prepare Casting',
                minX: 590,
                maxX: 740,
                minY: 124,
                maxY: 200,
                payload: {
                    mode: 'fixed_thickness',
                    processing_mode: 'interactive',
                    material: 'gold_18k',
                    wall_thickness_mm: 0.8,
                    min_allowed_thickness_mm: 0.6,
                    protect_regions: [],
                    add_drain_holes: true,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'offset-verts',
                label: 'Offset Mesh',
                minX: 740,
                maxX: 860,
                minY: 124,
                maxY: 200,
                payload: {
                    offset_mm: 0.1,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'shell-mesh',
                label: 'Shell Mesh',
                minX: 860,
                maxX: 980,
                minY: 124,
                maxY: 200,
                payload: {
                    wall_thickness_mm: 0.6,
                    voxel_size_mm: 0.4,
                    padding_mm: 1.2,
                    refine: true,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'thicken-mesh',
                label: 'Thicken Mesh',
                minX: 980,
                maxX: 1110,
                minY: 124,
                maxY: 200,
                payload: {
                    thickness_mm: 0.4,
                    voxel_size_mm: 0.4,
                    padding_mm: 1,
                    refine: true,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
        ],
        'inspect-features': [
            {
                commandId: 'runtime-measure-inspect',
                label: 'Measure Dimensions',
                minX: 0,
                maxX: 160,
                minY: 36,
                maxY: 124,
                payload: {
                    points: [[0, 0, 0], [5, 5, 5]],
                    include_local_thickness: true,
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                },
                options: { execute: true },
            },
            {
                commandId: 'section',
                label: 'Section Slice',
                minX: 160,
                maxX: 300,
                minY: 36,
                maxY: 124,
                payload: () => ({
                    section_enabled: true,
                    section_constant: 0,
                    plane_axis: [0, 0, 1],
                    selected_region_ids: activeWorkbenchRegionIds(),
                    metadata: { source: 'meshlib_canvas_plugin_overlay' },
                }),
                options: { execute: true },
            },
        ],
    });

    const WORKBENCH_CANVAS_TAB_HITBOXES = Object.freeze([
        { tab: 'prepare', minX: 160, maxX: 294, minY: 0, maxY: 36 },
        { tab: 'selection', minX: 294, maxX: 366, minY: 0, maxY: 36 },
        { tab: 'mesh-edit', minX: 526, maxX: 604, minY: 0, maxY: 36 },
        { tab: 'inspect-features', minX: 604, maxX: 700, minY: 0, maxY: 36 },
    ]);

    const WORKBENCH_CANVAS_ACCESSIBILITY_CONTROLS = Object.freeze([
        {
            id: 'tab-view',
            label: 'MeshLib View tab',
            left: 242,
            top: 0,
            width: 52,
            height: 36,
            clicks: [{ x: 322, y: 18 }],
        },
        {
            id: 'tab-modify',
            label: 'MeshLib Modify tab',
            left: 410,
            top: 0,
            width: 66,
            height: 36,
            clicks: [{ x: 565, y: 18 }],
        },
        {
            id: 'tab-select',
            label: 'MeshLib Select tab',
            left: 294,
            top: 0,
            width: 72,
            height: 36,
            clicks: [{ x: 330, y: 18 }],
        },
        {
            id: 'tab-inspect',
            label: 'MeshLib Inspect tab',
            left: 476,
            top: 0,
            width: 72,
            height: 36,
            clicks: [{ x: 652, y: 18 }],
        },
        {
            id: 'front-view',
            label: 'MeshLib Front View',
            left: 76,
            top: 36,
            width: 86,
            height: 88,
            clicks: [
                { x: 322, y: 18 },
                { x: 104, y: 78 },
            ],
        },
    ]);

    const WORKBENCH_CANVAS_POINTER_EVENTS = Object.freeze(['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']);

    function normalizeWorkbenchCommandId(commandId) {
        if (!commandId) {
            return null;
        }
        const normalized = String(commandId).trim();
        return WORKBENCH_TOOL_COMMAND_ALIASES[normalized] || normalized;
    }

    function workbenchCommandCapabilities() {
        return Array.isArray(hostPayload?.manifest?.command_capabilities) ? hostPayload.manifest.command_capabilities : [];
    }

    function officialParityInventory() {
        return Array.isArray(hostPayload?.manifest?.official_parity_inventory) ? hostPayload.manifest.official_parity_inventory : [];
    }

    function commandCapabilityById() {
        const capabilities = workbenchCommandCapabilities();
        return new Map(capabilities.map((capability) => [capability.command_id, capability]));
    }

    function isProductReadyWorkbenchCapability(capability) {
        if (!capability) {
            return false;
        }
        if (capability.endpoint_url || capability.endpoint_url_key) {
            return true;
        }
        if (capability.group === 'runtime' && capability.runtime_tool_id) {
            return true;
        }
        return capability.rust_backed !== true;
    }

    function isOfficialParityToolEnabled(feature) {
        if (!feature || feature.status === 'missing') {
            return false;
        }
        if (feature.status === 'implemented') {
            return true;
        }
        const backendCommandIds = Array.isArray(feature.backend_command_ids) ? feature.backend_command_ids : [];
        const hostedToolIds = Array.isArray(feature.hosted_tool_ids) ? feature.hosted_tool_ids : [];
        if (hostedToolIds.length > 0) {
            return true;
        }
        const capabilities = commandCapabilityById();
        return backendCommandIds.some((commandId) => isProductReadyWorkbenchCapability(capabilities.get(commandId)));
    }

    function officialWorkbenchTools() {
        return officialParityInventory().map((feature) => ({
            official_feature_id: feature.official_feature_id,
            label: feature.label,
            group: feature.group,
            status: feature.status,
            enabled: isOfficialParityToolEnabled(feature),
            disabled_reason: isOfficialParityToolEnabled(feature) ? null : 'missing_backend_operation',
            backend_command_ids: Array.isArray(feature.backend_command_ids) ? feature.backend_command_ids : [],
            hosted_tool_ids: Array.isArray(feature.hosted_tool_ids) ? feature.hosted_tool_ids : [],
            rust_owner_modules: Array.isArray(feature.rust_owner_modules) ? feature.rust_owner_modules : [],
        }));
    }

    function markOfficialParityInventory() {
        const inventory = officialParityInventory();
        const tools = officialWorkbenchTools();
        const disabledFeatureIds = tools
            .filter((tool) => !tool.enabled)
            .map((tool) => tool.official_feature_id)
            .filter(Boolean)
            .sort();
        const groups = Array.from(new Set(inventory.map((feature) => feature.group).filter(Boolean))).sort();

        document.documentElement.dataset.meshinspectorWorkbenchOfficialParityFeatureCount = String(inventory.length);
        document.documentElement.dataset.meshinspectorWorkbenchOfficialParityMissingCount = String(
            tools.filter((tool) => tool.disabled_reason === 'missing_backend_operation').length,
        );
        document.documentElement.dataset.meshinspectorWorkbenchOfficialParityGroups = groups.join(',');
        document.documentElement.dataset.meshinspectorWorkbenchDisabledFeatureIds = disabledFeatureIds.join(',');
    }

    function findWorkbenchCommandCapability(commandId) {
        const normalizedCommandId = normalizeWorkbenchCommandId(commandId);
        if (!normalizedCommandId) {
            return null;
        }
        const capabilities = workbenchCommandCapabilities();
        const exactCommandMatch = capabilities.find((capability) => {
            const capabilityCommandId = normalizeWorkbenchCommandId(capability.command_id);
            return capabilityCommandId === normalizedCommandId;
        });
        if (exactCommandMatch) {
            return exactCommandMatch;
        }

        const runtimeLabelMatch = capabilities.find((capability) => {
            const capabilityLabel = normalizeWorkbenchCommandId(capability.label);
            return isRuntimeWorkbenchToolCommand(capability) && capabilityLabel === normalizedCommandId;
        });
        if (runtimeLabelMatch) {
            return runtimeLabelMatch;
        }

        const runtimeToolMatch = capabilities.find((capability) => {
            const capabilityToolId = normalizeWorkbenchCommandId(capability.runtime_tool_id);
            return isRuntimeWorkbenchToolCommand(capability) && capabilityToolId === normalizedCommandId;
        });
        const workspaceToolMatch = capabilities.find((capability) => {
            const capabilityToolId = normalizeWorkbenchCommandId(capability.runtime_tool_id);
            const capabilityLabel = normalizeWorkbenchCommandId(capability.label);
            return capabilityToolId === normalizedCommandId || capabilityLabel === normalizedCommandId;
        });
        return runtimeToolMatch || workspaceToolMatch || null;
    }

    function workbenchCanvasPointFromEvent(event) {
        const targetElement = event.target instanceof Element ? event.target : null;
        const canvas = targetElement?.closest('canvas') || document.querySelector('canvas');
        const rect = canvas?.getBoundingClientRect();
        return {
            x: event.clientX - (rect?.left || 0),
            y: event.clientY - (rect?.top || 0),
        };
    }

    function hitTestWorkbenchCanvas(point, hitboxes) {
        return hitboxes.find((hitbox) =>
            point.x >= hitbox.minX &&
            point.x < hitbox.maxX &&
            point.y >= hitbox.minY &&
            point.y < hitbox.maxY,
        ) || null;
    }

    function dispatchWorkbenchCanvasPointerSequence(x, y) {
        const canvas = document.querySelector('canvas');
        const rect = canvas?.getBoundingClientRect();
        if (!canvas || !rect) {
            return;
        }

        const clientX = rect.left + x;
        const clientY = rect.top + y;
        const eventInit = {
            bubbles: true,
            cancelable: true,
            view: window,
            button: 0,
            buttons: 1,
            clientX,
            clientY,
            screenX: window.screenX + clientX,
            screenY: window.screenY + clientY,
        };

        for (const eventName of WORKBENCH_CANVAS_POINTER_EVENTS) {
            const EventCtor = eventName.startsWith('pointer') && typeof PointerEvent === 'function' ? PointerEvent : MouseEvent;
            const buttons = ['pointerup', 'mouseup', 'click'].includes(eventName) ? 0 : 1;
            canvas.dispatchEvent(new EventCtor(eventName, { ...eventInit, buttons }));
        }
    }

    function trackWorkbenchCanvasTabClick(point) {
        if (point.y < 0 || point.y > 36) {
            return;
        }
        const tabHit = hitTestWorkbenchCanvas(point, WORKBENCH_CANVAS_TAB_HITBOXES);
        activeWorkbenchCanvasTab = tabHit?.tab || null;
        syncWorkbenchCanvasCommandOverlay();
    }

    function ensureWorkbenchCanvasCommandOverlay() {
        if (workbenchCanvasCommandOverlay) {
            return workbenchCanvasCommandOverlay;
        }

        const overlay = document.createElement('div');
        overlay.id = 'meshinspector-workbench-command-overlay';
        overlay.setAttribute('aria-label', 'MeshInspector hosted MeshLib commands');
        Object.assign(overlay.style, {
            position: 'fixed',
            inset: '0',
            zIndex: '20',
            pointerEvents: 'none',
            fontFamily: 'Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        });
        document.body.appendChild(overlay);
        workbenchCanvasCommandOverlay = overlay;
        return overlay;
    }

    function ensureWorkbenchCanvasAccessibilityOverlay() {
        if (workbenchCanvasAccessibilityOverlay) {
            return workbenchCanvasAccessibilityOverlay;
        }

        const overlay = document.createElement('div');
        overlay.id = 'meshinspector-workbench-accessibility-overlay';
        overlay.setAttribute('aria-label', 'MeshLib canvas accessibility controls');
        Object.assign(overlay.style, {
            position: 'fixed',
            inset: '0',
            zIndex: '30',
            pointerEvents: 'none',
            fontFamily: 'Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        });

        for (const control of WORKBENCH_CANVAS_ACCESSIBILITY_CONTROLS) {
            const button = document.createElement('button');
            button.type = 'button';
            button.textContent = control.label;
            button.setAttribute('aria-label', control.label);
            button.dataset.meshinspectorWorkbenchUiControl = control.id;
            Object.assign(button.style, {
                position: 'fixed',
                left: `${control.left}px`,
                top: `${control.top}px`,
                width: `${control.width}px`,
                height: `${control.height}px`,
                border: '0',
                padding: '0',
                opacity: '0.01',
                background: 'transparent',
                color: 'transparent',
                cursor: 'pointer',
                pointerEvents: 'auto',
            });
            button.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                control.clicks.forEach((point, index) => {
                    window.setTimeout(() => dispatchWorkbenchCanvasPointerSequence(point.x, point.y), index * 90);
                });
            });
            overlay.appendChild(button);
        }

        document.body.appendChild(overlay);
        workbenchCanvasAccessibilityOverlay = overlay;
        return overlay;
    }

    function syncWorkbenchCanvasAccessibilityCommands(commands) {
        const overlay = ensureWorkbenchCanvasAccessibilityOverlay();
        overlay
            .querySelectorAll('[data-meshinspector-workbench-accessible-command]')
            .forEach((button) => button.remove());

        for (const command of commands) {
            const capability = findWorkbenchCommandCapability(command.commandId);
            if (!capability) {
                continue;
            }

            const button = document.createElement('button');
            button.type = 'button';
            button.textContent = command.label;
            button.setAttribute('aria-label', command.label);
            button.title = `${command.label} (Rust-backed)`;
            button.dataset.meshinspectorWorkbenchAccessibleCommand = command.commandId;
            Object.assign(button.style, {
                position: 'fixed',
                left: `${command.minX}px`,
                top: `${command.minY + 10}px`,
                width: `${Math.max(96, command.maxX - command.minX - 12)}px`,
                height: '46px',
                border: '0',
                padding: '0',
                opacity: '0.01',
                background: 'transparent',
                color: 'transparent',
                cursor: 'pointer',
                pointerEvents: 'auto',
            });
            button.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                void dispatchWorkbenchCommand(command.commandId, resolveWorkbenchCommandPayload(command), command.options ?? {});
            });
            overlay.appendChild(button);
        }
    }

    const WORKBENCH_RIBBON_TAB_SECTIONS = Object.freeze({
        'prepare': [
            { caption: 'Prepare', commandIds: ['repair', 'resize', 'reduce-weight', 'prepare-casting', 'protected-hollow'] },
        ],
        'mesh-edit': [
            { caption: 'Simplify', commandIds: ['decimate-mesh', 'subdivide-mesh', 'make-delone'] },
            { caption: 'Sculpt', commandIds: ['runtime-thicken-brush', 'runtime-smooth-brush', 'scoop', 'runtime-scoop-brush'] },
            { caption: 'Offset', commandIds: ['offset-verts', 'shell-mesh', 'thicken-mesh'] },
        ],
        'selection': [
            { caption: 'Selection', commandIds: ['runtime-select-mark-region'] },
        ],
        'inspect-features': [
            { caption: 'Measure', commandIds: ['runtime-measure-inspect', 'section'] },
        ],
    });
    const WORKBENCH_RIBBON_STRIP_LEFT = Object.freeze({
        'prepare': 565,
        'mesh-edit': 420,
        'selection': 190,
        'inspect-features': 250,
    });

    function workbenchOverlayCommandPool() {
        const pool = new Map();
        for (const commands of Object.values(WORKBENCH_CANVAS_COMMAND_OVERLAYS)) {
            for (const command of commands) {
                pool.set(command.commandId, command);
            }
        }
        return pool;
    }

    function syncWorkbenchCanvasCommandOverlay() {
        const overlay = ensureWorkbenchCanvasCommandOverlay();
        const sections = activeWorkbenchCanvasTab ? (WORKBENCH_RIBBON_TAB_SECTIONS[activeWorkbenchCanvasTab] || []) : [];
        const pool = workbenchOverlayCommandPool();
        const renderedCommands = [];
        overlay.replaceChildren();

        const strip = document.createElement('div');
        Object.assign(strip.style, {
            position: 'fixed',
            left: `${WORKBENCH_RIBBON_STRIP_LEFT[activeWorkbenchCanvasTab] ?? 420}px`,
            top: '36px',
            height: '64px',
            display: 'flex',
            alignItems: 'stretch',
            pointerEvents: 'none',
        });

        for (const [sectionIndex, section] of sections.entries()) {
            const commands = section.commandIds
                .map((commandId) => pool.get(commandId))
                .filter((command) => command && findWorkbenchCommandCapability(command.commandId));
            if (commands.length === 0) {
                continue;
            }

            const sectionBox = document.createElement('div');
            Object.assign(sectionBox.style, {
                display: 'flex',
                flexDirection: 'column',
                padding: '2px 10px 0',
                borderLeft: sectionIndex > 0 ? '1px solid rgba(255, 255, 255, 0.08)' : '0',
            });

            const buttonRow = document.createElement('div');
            Object.assign(buttonRow.style, {
                display: 'flex',
                alignItems: 'stretch',
                gap: '2px',
                flex: '1 1 auto',
            });

            for (const command of commands) {
                renderedCommands.push(command);
                const button = document.createElement('button');
                button.type = 'button';
                button.textContent = command.label;
                button.title = `${command.label} (Rust-backed)`;
                button.setAttribute('aria-label', command.label);
                button.dataset.meshinspectorWorkbenchCommand = command.commandId;
                Object.assign(button.style, {
                    border: '0',
                    borderRadius: '4px',
                    background: 'transparent',
                    color: '#d6d6dc',
                    cursor: 'pointer',
                    font: '500 12px/15px Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
                    padding: '0 12px',
                    maxWidth: '102px',
                    whiteSpace: 'normal',
                    textAlign: 'center',
                    pointerEvents: 'auto',
                });
                button.addEventListener('mouseenter', () => {
                    button.style.background = 'rgba(255, 255, 255, 0.08)';
                });
                button.addEventListener('mouseleave', () => {
                    button.style.background = 'transparent';
                });
                button.addEventListener('click', (event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    void dispatchWorkbenchCommand(command.commandId, resolveWorkbenchCommandPayload(command), command.options ?? {});
                });
                buttonRow.appendChild(button);
            }

            const caption = document.createElement('div');
            caption.textContent = section.caption;
            Object.assign(caption.style, {
                font: '400 10.5px/14px Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
                color: '#8e8e96',
                textAlign: 'center',
                letterSpacing: '0.05em',
                paddingTop: '2px',
            });

            sectionBox.appendChild(buttonRow);
            sectionBox.appendChild(caption);
            strip.appendChild(sectionBox);
        }

        overlay.appendChild(strip);
        overlay.hidden = renderedCommands.length === 0;
        document.documentElement.dataset.meshinspectorWorkbenchCanvasOverlayTab = activeWorkbenchCanvasTab || '';
        document.documentElement.dataset.meshinspectorWorkbenchCanvasOverlayCommands = renderedCommands
            .map((command) => command.commandId)
            .join(',');
        syncWorkbenchCanvasAccessibilityCommands(renderedCommands);
    }

    function shouldDispatchWorkbenchCanvasCommand(command, event) {
        if (!['pointerup', 'mouseup', 'click'].includes(event.type)) {
            return false;
        }
        const now = performance.now();
        if (lastWorkbenchCanvasCommandDispatch.commandId === command.commandId && now - lastWorkbenchCanvasCommandDispatch.at < 250) {
            return false;
        }
        lastWorkbenchCanvasCommandDispatch = { commandId: command.commandId, at: now };
        return true;
    }

    function handleWorkbenchCanvasClick(event) {
        if (event.defaultPrevented || event.button !== 0) {
            return;
        }
        if (
            typeof event.target?.closest === 'function' &&
            event.target.closest('[data-meshinspector-workbench-command], [data-meshinspector-workbench-accessible-command]')
        ) {
            return;
        }
        const point = workbenchCanvasPointFromEvent(event);
        trackWorkbenchCanvasTabClick(point);
        const activeCommands = activeWorkbenchCanvasTab ? WORKBENCH_CANVAS_COMMAND_HITBOXES[activeWorkbenchCanvasTab] : null;
        if (!activeCommands) {
            return;
        }
        const command = hitTestWorkbenchCanvas(point, activeCommands);
        if (!command) {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation();
        if (shouldDispatchWorkbenchCanvasCommand(command, event)) {
            void dispatchWorkbenchCommand(command.commandId, resolveWorkbenchCommandPayload(command), command.options ?? {});
        }
    }

    function installWorkbenchCanvasCommandBridge() {
        if (workbenchCanvasBridgeInstalled) {
            return;
        }
        workbenchCanvasBridgeInstalled = true;
        ensureWorkbenchCanvasAccessibilityOverlay();
        for (const eventName of WORKBENCH_CANVAS_POINTER_EVENTS) {
            window.addEventListener(eventName, handleWorkbenchCanvasClick, true);
        }
        document.documentElement.dataset.meshinspectorWorkbenchCanvasCommandBridge = 'ready';
    }

    function markCommandCapabilities() {
        const capabilities = workbenchCommandCapabilities();
        const endpointUrls = capabilities.map((capability) => capability.endpoint_url).filter(Boolean);
        const backendEndpointCount = endpointUrls.filter((endpointUrl) => /^https?:\/\//.test(endpointUrl)).length;
        const relativeEndpointCount = endpointUrls.length - backendEndpointCount;
        const runtimeTools = Array.from(
            new Set(capabilities.map((capability) => capability.runtime_tool_id).filter(Boolean)),
        ).sort();
        document.documentElement.dataset.meshinspectorWorkbenchCommandCount = String(capabilities.length);
        document.documentElement.dataset.meshinspectorWorkbenchBackendEndpointCount = String(backendEndpointCount);
        document.documentElement.dataset.meshinspectorWorkbenchRelativeEndpointCount = String(relativeEndpointCount);
        document.documentElement.dataset.meshinspectorWorkbenchRuntimeTools = runtimeTools.join(',');
        markOfficialParityInventory();
    }

    async function commitSavedFile(filename, content, options = {}) {
        const manifest = hostPayload?.manifest;
        if (!manifest?.commit_endpoint_url || !content) {
            return false;
        }

        const exportFilename = runtimeFilename(filename);
        const request = {
            tool_id: options.tool_id || 'meshlib_workbench_export',
            operation_label: options.operation_label || 'MeshLib Workbench Export',
            selection: null,
            preserve_detail: true,
            metadata: {
                source: 'meshlib_workbench_save',
                filename: exportFilename,
                runtime_version_id: manifest.version_id,
                ...(options.metadata || {}),
            },
        };
        const formData = new FormData();
        formData.append('request_json', JSON.stringify(request));
        formData.append('mesh_file', new Blob([content], { type: 'application/octet-stream' }), exportFilename);

        try {
            const response = await fetch(manifest.commit_endpoint_url, {
                method: 'POST',
                body: formData,
            });
            const responsePayload = await response.json().catch(() => ({}));
            if (!response.ok) {
                postHostMessage('meshlib-workbench:commit-failed', {
                    version_id: manifest.version_id,
                    filename: exportFilename,
                    status: response.status,
                    error: responsePayload.detail || response.statusText,
                });
                return false;
            }
            postHostMessage('meshlib-workbench:commit-complete', {
                version_id: manifest.version_id,
                filename: exportFilename,
                job: responsePayload,
            });
            return true;
        } catch (error) {
            postHostMessage('meshlib-workbench:commit-failed', {
                version_id: manifest.version_id,
                filename: exportFilename,
                error: error instanceof Error ? error.message : 'MeshLib workbench commit failed',
            });
            return false;
        }
    }

    function normalizeSelectionPayload(selection = {}) {
        return {
            mode: selection.mode || 'brush',
            vertex_ids: Array.isArray(selection.vertex_ids) ? selection.vertex_ids : [],
            face_ids: Array.isArray(selection.face_ids) ? selection.face_ids : [],
            region_ids: Array.isArray(selection.region_ids) ? selection.region_ids : [],
            brush_points_world: Array.isArray(selection.brush_points_world) ? selection.brush_points_world : [],
            metadata: selection.metadata && typeof selection.metadata === 'object' ? selection.metadata : {},
        };
    }

    function brushSelectionPayload(stroke = {}) {
        const nested = stroke.selection && typeof stroke.selection === 'object' ? stroke.selection : {};
        return {
            mode: nested.mode ?? stroke.mode,
            vertex_ids: nested.vertex_ids ?? stroke.vertex_ids,
            face_ids: nested.face_ids ?? stroke.face_ids,
            region_ids: nested.region_ids ?? stroke.region_ids,
            brush_points_world: nested.brush_points_world ?? stroke.brush_points_world,
            metadata: {
                ...(stroke.selection_metadata || {}),
                ...(nested.metadata || {}),
            },
        };
    }

    function numericBrushOption(stroke, keys, fallback) {
        for (const key of keys) {
            const value = stroke?.[key];
            if (Number.isFinite(value)) {
                return value;
            }
            if (typeof value === 'string' && value.trim() !== '') {
                const parsed = Number(value);
                if (Number.isFinite(parsed)) {
                    return parsed;
                }
            }
        }
        return fallback;
    }

    async function commitSelection(selection, options = {}) {
        const manifest = hostPayload?.manifest;
        if (!manifest?.selection_endpoint_url) {
            renderWorkbenchResultPanel({
                title: 'Select / Mark Region',
                status: 'Rust endpoint unavailable',
                detail: 'The active workbench manifest did not advertise a selection endpoint.',
            });
            return null;
        }

        const normalizedSelection = normalizeSelectionPayload(selection);
        const request = {
            tool_id: 'select_mark_region',
            operation_label: options.operation_label || 'Select / Mark Region',
            selection: normalizedSelection,
            label: options.label || null,
            create_object: options.create_object === true || options.createObject === true,
            metadata: {
                source: 'meshlib_workbench_selection',
                runtime_version_id: manifest.version_id,
                ...(options.metadata || {}),
            },
        };

        try {
            renderWorkbenchResultPanel({
                title: 'Select / Mark Region',
                status: 'Calling Rust REST endpoint',
                rows: [
                    { label: 'Faces', value: String(normalizedSelection.face_ids.length) },
                    { label: 'Regions', value: String(normalizedSelection.region_ids.length) },
                    { label: 'Brush points', value: String(normalizedSelection.brush_points_world.length) },
                ],
            });
            const response = await fetch(manifest.selection_endpoint_url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
            });
            const responsePayload = await response.json().catch(() => ({}));
            if (!response.ok) {
                renderWorkbenchResultPanel({
                    title: 'Select / Mark Region',
                    status: 'Rust endpoint failed',
                    detail: responsePayload.detail || response.statusText,
                });
                postHostMessage('meshlib-workbench:selection-failed', {
                    version_id: manifest.version_id,
                    status: response.status,
                    error: responsePayload.detail || response.statusText,
                });
                return null;
            }
            renderSelectionCommitResult(responsePayload);
            postHostMessage('meshlib-workbench:selection-complete', {
                version_id: responsePayload.selected_object_version_id || manifest.version_id,
                source_version_id: manifest.version_id,
                selected_object_version_id: responsePayload.selected_object_version_id || null,
                selection: responsePayload,
            });
            return responsePayload;
        } catch (error) {
            renderWorkbenchResultPanel({
                title: 'Select / Mark Region',
                status: 'Rust endpoint failed',
                detail: error instanceof Error ? error.message : 'MeshLib workbench selection commit failed',
            });
            postHostMessage('meshlib-workbench:selection-failed', {
                version_id: manifest.version_id,
                error: error instanceof Error ? error.message : 'MeshLib workbench selection commit failed',
            });
            return null;
        }
    }

    function normalizeBrushStroke(stroke = {}) {
        return {
            tool_id: stroke.tool_id || 'smooth_brush',
            selection: normalizeSelectionPayload(brushSelectionPayload(stroke)),
            amount_mm: numericBrushOption(stroke, ['amount_mm', 'depth_mm', 'target_thickness_mm'], 0.15),
            falloff_mm: numericBrushOption(stroke, ['falloff_mm', 'brush_radius_mm', 'radius_mm'], 1.5),
            iterations: Number.isFinite(stroke.iterations) ? Math.max(1, Math.round(stroke.iterations)) : 1,
            strength: Number.isFinite(stroke.strength) ? Math.min(1, Math.max(0, stroke.strength)) : 0.5,
            metadata: stroke.metadata && typeof stroke.metadata === 'object' ? stroke.metadata : {},
        };
    }

    async function commitBrushStrokes(strokes, options = {}) {
        const manifest = hostPayload?.manifest;
        if (!manifest?.brush_endpoint_url) {
            return null;
        }
        const normalizedStrokes = Array.isArray(strokes) ? strokes.map(normalizeBrushStroke) : [];
        if (!normalizedStrokes.length) {
            return null;
        }

        const request = {
            operation_label: options.operation_label || 'MeshLib Brush Replay',
            strokes: normalizedStrokes,
            metadata: {
                source: 'meshlib_workbench_brush_replay',
                runtime_version_id: manifest.version_id,
                ...(options.metadata || {}),
            },
        };

        try {
            const response = await fetch(manifest.brush_endpoint_url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
            });
            const responsePayload = await response.json().catch(() => ({}));
            if (!response.ok) {
                postHostMessage('meshlib-workbench:brush-failed', {
                    version_id: manifest.version_id,
                    status: response.status,
                    error: responsePayload.detail || response.statusText,
                });
                return null;
            }
            postHostMessage('meshlib-workbench:brush-complete', {
                version_id: manifest.version_id,
                job: responsePayload,
            });
            return responsePayload;
        } catch (error) {
            postHostMessage('meshlib-workbench:brush-failed', {
                version_id: manifest.version_id,
                error: error instanceof Error ? error.message : 'MeshLib workbench brush replay failed',
            });
            return null;
        }
    }

    function pointTuple(value) {
        if (!Array.isArray(value) || value.length < 3) {
            return null;
        }
        const point = value.slice(0, 3).map((coordinate) => Number(coordinate));
        return point.every(Number.isFinite) ? point : null;
    }

    function measureMetric(value) {
        if (typeof value !== 'string') {
            return null;
        }
        const normalized = value.trim().toLowerCase();
        if (['geodesic', 'surface', 'surface_distance', 'surface-distance'].includes(normalized)) {
            return 'geodesic';
        }
        if (['euclidean', 'linear', 'straight', 'straight_line', 'straight-line'].includes(normalized)) {
            return 'euclidean';
        }
        return null;
    }

    function nonnegativeInteger(value) {
        const parsed = Number(value);
        return Number.isInteger(parsed) && parsed >= 0 ? parsed : null;
    }

    function positiveNumber(value) {
        const parsed = Number(value);
        return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
    }

    function nonnegativeNumber(value) {
        const parsed = Number(value);
        return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
    }

    function nonnegativeIntegerList(value) {
        if (!Array.isArray(value)) {
            return [];
        }
        return value
            .map((item) => nonnegativeInteger(item))
            .filter((item) => item !== null);
    }

    function nonnegativeEdgePairs(value) {
        if (!Array.isArray(value)) {
            return [];
        }
        return value
            .map((item) => {
                if (!Array.isArray(item) || item.length < 2) {
                    return null;
                }
                const first = nonnegativeInteger(item[0]);
                const second = nonnegativeInteger(item[1]);
                return first !== null && second !== null && first !== second ? [first, second] : null;
            })
            .filter((item) => item !== null);
    }

    function appendPoint(points, value) {
        const directPoint = pointTuple(value);
        if (directPoint) {
            points.push(directPoint);
            return;
        }
        if (!Array.isArray(value)) {
            return;
        }
        for (const item of value) {
            const point = pointTuple(item);
            if (point) {
                points.push(point);
            }
        }
    }

    function appendPair(pointPairs, value) {
        if (!value) {
            return;
        }
        if (Array.isArray(value)) {
            if (value.length >= 2) {
                const start = pointTuple(value[0]);
                const end = pointTuple(value[1]);
                if (start && end) {
                    pointPairs.push({ start, end });
                    return;
                }
            }
            for (const item of value) {
                appendPair(pointPairs, item);
            }
            return;
        }
        if (typeof value !== 'object') {
            return;
        }
        const start = pointTuple(value.start || value.from || value.p0 || value.a);
        const end = pointTuple(value.end || value.to || value.p1 || value.b);
        if (start && end) {
            const metric = measureMetric(value.metric ?? value.distance_metric ?? value.distanceMetric);
            const startVertex = nonnegativeInteger(value.start_vertex ?? value.startVertex ?? value.from_vertex ?? value.fromVertex);
            const endVertex = nonnegativeInteger(value.end_vertex ?? value.endVertex ?? value.to_vertex ?? value.toVertex);
            const controlVertices = nonnegativeIntegerList(
                value.control_vertices ??
                    value.controlVertices ??
                    value.control_vertex_indices ??
                    value.controlVertexIndices ??
                    value.path_vertices ??
                    value.pathVertices ??
                    value.polyline_vertices ??
                    value.polylineVertices,
            );
            const closePath =
                booleanWorkbenchFlag(value, [
                    'close_path',
                    'closePath',
                    'closed_path',
                    'closedPath',
                    'closed',
                    'is_closed',
                    'isClosed',
                ]) === true;
            const includeRefinedSurfacePath =
                booleanWorkbenchFlag(value, [
                    'include_refined_surface_path',
                    'includeRefinedSurfacePath',
                    'refine_surface_path',
                    'refineSurfacePath',
                ]) === true;
            const maxPathLen = positiveNumber(
                value.geodesic_max_path_len_mm ??
                    value.max_path_len_mm ??
                    value.maxPathLenMm ??
                    value.max_path_length_mm ??
                    value.maxPathLengthMm,
            );
            pointPairs.push({
                start,
                end,
                label: typeof value.label === 'string' ? value.label : null,
                ...(metric ? { metric } : {}),
                ...(startVertex !== null ? { start_vertex: startVertex } : {}),
                ...(endVertex !== null ? { end_vertex: endVertex } : {}),
                ...(controlVertices.length ? { control_vertices: controlVertices } : {}),
                ...(closePath ? { close_path: true } : {}),
                ...(includeRefinedSurfacePath ? { include_refined_surface_path: true } : {}),
                ...(maxPathLen !== null ? { geodesic_max_path_len_mm: maxPathLen } : {}),
            });
        }
    }

    function featureKind(value) {
        if (typeof value !== 'string') {
            return null;
        }
        const normalized = value.toLowerCase().replace(/[_\s-]+/g, '_');
        if (['point', 'sphere', 'line', 'plane', 'circle', 'cylinder'].includes(normalized)) {
            return normalized;
        }
        return null;
    }

    function appendFeature(features, value) {
        if (!value) {
            return;
        }
        if (Array.isArray(value)) {
            for (const item of value) {
                appendFeature(features, item);
            }
            return;
        }
        if (typeof value !== 'object') {
            return;
        }
        const kind = featureKind(value.kind ?? value.type ?? value.primitive_type ?? value.primitiveType);
        const center = pointTuple(value.center ?? value.point ?? value.origin ?? value.position);
        if (!kind || !center) {
            return;
        }
        const direction = pointTuple(value.direction ?? value.normal ?? value.axis);
        const radius = nonnegativeNumber(value.radius_mm ?? value.radiusMm ?? value.radius);
        const length = nonnegativeNumber(value.length_mm ?? value.lengthMm ?? value.length);
        features.push({
            feature_id: String(value.feature_id ?? value.featureId ?? value.id ?? `feature_${features.length}`),
            kind,
            center,
            ...(direction ? { direction } : {}),
            ...(radius !== null ? { radius_mm: radius } : {}),
            ...(length !== null ? { length_mm: length } : {}),
        });
    }

    function appendFeaturePair(featurePairs, value) {
        if (!value) {
            return;
        }
        if (Array.isArray(value)) {
            if (value.length >= 2 && typeof value[0] === 'string' && typeof value[1] === 'string') {
                featurePairs.push({ first_feature_id: value[0], second_feature_id: value[1] });
                return;
            }
            for (const item of value) {
                appendFeaturePair(featurePairs, item);
            }
            return;
        }
        if (typeof value !== 'object') {
            return;
        }
        const first = value.first_feature_id ?? value.firstFeatureId ?? value.from_feature_id ?? value.fromFeatureId ?? value.a;
        const second = value.second_feature_id ?? value.secondFeatureId ?? value.to_feature_id ?? value.toFeatureId ?? value.b;
        if (typeof first === 'string' && typeof second === 'string') {
            featurePairs.push({
                first_feature_id: first,
                second_feature_id: second,
                label: typeof value.label === 'string' ? value.label : null,
            });
        }
    }

    function surfaceDistancePayload(params = {}) {
        const nested =
            params.surface_distance && typeof params.surface_distance === 'object'
                ? params.surface_distance
                : params.surfaceDistance && typeof params.surfaceDistance === 'object'
                  ? params.surfaceDistance
                  : null;
        const source = nested || params;
        const seed = pointTuple(source.seed ?? source.seed_point ?? source.seedPoint ?? source.point ?? source.point_world ?? source.world_point);
        const seedVertex = nonnegativeInteger(source.seed_vertex ?? source.seedVertex);
        const seedVertices = nonnegativeIntegerList(source.seed_vertices ?? source.seedVertices ?? source.source_vertices ?? source.sourceVertices);
        const seedEdges = nonnegativeEdgePairs(source.seed_edges ?? source.seedEdges ?? source.source_edges ?? source.sourceEdges ?? source.selected_edges ?? source.selectedEdges);
        const seedFaceIds = nonnegativeIntegerList(source.seed_face_ids ?? source.seedFaceIds ?? source.source_face_ids ?? source.sourceFaceIds ?? source.selected_face_ids ?? source.selectedFaceIds);
        const maxDistance = positiveNumber(source.max_distance_mm ?? source.maxDistanceMm ?? source.max_path_len_mm ?? source.maxPathLenMm);
        const isoValue = nonnegativeNumber(source.iso_value_mm ?? source.isoValueMm ?? source.iso_value ?? source.isoValue ?? source.value);
        const requested = Boolean(
            nested ||
                seed ||
                seedVertex !== null ||
                seedVertices.length ||
                seedEdges.length ||
                seedFaceIds.length ||
                isoValue !== null ||
                params.surface_distance === true ||
                params.surfaceDistance === true,
        );
        if (!requested) {
            return null;
        }
        return {
            ...(seed ? { seed } : {}),
            ...(seedVertex !== null ? { seed_vertex: seedVertex } : {}),
            ...(seedVertices.length ? { seed_vertices: seedVertices } : {}),
            ...(seedEdges.length ? { seed_edges: seedEdges } : {}),
            ...(seedFaceIds.length ? { seed_face_ids: seedFaceIds } : {}),
            ...(maxDistance !== null ? { max_distance_mm: maxDistance } : {}),
            ...(isoValue !== null ? { iso_value_mm: isoValue } : {}),
            include_distances: source.include_distances ?? source.includeDistances ?? true,
            include_iso_segments: source.include_iso_segments ?? source.includeIsoSegments ?? true,
            include_extreme_edges: source.include_extreme_edges ?? source.includeExtremeEdges ?? false,
        };
    }

    function normalizeMeasureInspectPayload(params = {}, options = {}) {
        const points = [];
        const pointPairs = [];
        const features = [];
        const featurePairs = [];
        const surfaceDistance = surfaceDistancePayload(params);
        const metric =
            measureMetric(params.metric ?? params.distance_metric ?? params.distanceMetric) ??
            (params.geodesic === true || params.surface_distance === true || params.surfaceDistance === true ? 'geodesic' : null);
        appendPoint(points, params.points);
        appendPoint(points, params.point);
        appendPoint(points, params.point_world);
        appendPoint(points, params.world_point);
        appendPoint(points, params.points_world);
        appendPoint(points, params.world_points);
        appendPoint(points, params.position);
        appendPair(pointPairs, params.point_pairs);
        appendPair(pointPairs, params.pairs);
        appendPair(pointPairs, params.distance_pairs);
        appendPair(pointPairs, params.segments);
        appendPair(pointPairs, params);
        appendFeature(features, params.features);
        appendFeature(features, params.feature_primitives);
        appendFeature(features, params.featurePrimitives);
        appendFeaturePair(featurePairs, params.feature_pairs);
        appendFeaturePair(featurePairs, params.featurePairs);
        appendFeaturePair(featurePairs, params.measurement_feature_pairs);
        appendFeaturePair(featurePairs, params.measurementFeaturePairs);
        if (metric) {
            for (const pair of pointPairs) {
                pair.metric ??= metric;
            }
        }
        return {
            points,
            point_pairs: pointPairs,
            features,
            feature_pairs: featurePairs,
            surface_distance: surfaceDistance,
            include_local_thickness: params.include_local_thickness ?? options.include_local_thickness ?? true,
        };
    }

    async function measureInspect(params = {}, options = {}) {
        const manifest = hostPayload?.manifest;
        if (!manifest?.measurement_endpoint_url) {
            renderWorkbenchResultPanel({
                title: 'Measure Dimensions',
                status: 'Rust endpoint unavailable',
                detail: 'The active workbench manifest did not advertise a measurement endpoint.',
            });
            return null;
        }
        try {
            const request = normalizeMeasureInspectPayload(params, options);
            renderWorkbenchResultPanel({
                title: 'Measure Dimensions',
                status: 'Calling Rust REST endpoint',
                rows: [
                    { label: 'Point probes', value: String(request.points.length) },
                    { label: 'Point pairs', value: String(request.point_pairs.length) },
                ],
            });
            const response = await fetch(manifest.measurement_endpoint_url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
            });
            const responsePayload = await response.json().catch(() => ({}));
            if (!response.ok) {
                postHostMessage('meshlib-workbench:measure-failed', {
                    version_id: manifest.version_id,
                    status: response.status,
                    error: responsePayload.detail || response.statusText,
                });
                return null;
            }
            renderMeasureInspectResult(responsePayload);
            postHostMessage('meshlib-workbench:measure-complete', {
                version_id: manifest.version_id,
                measurement: responsePayload,
            });
            return responsePayload;
        } catch (error) {
            renderWorkbenchResultPanel({
                title: 'Measure Dimensions',
                status: 'Rust endpoint failed',
                detail: error instanceof Error ? error.message : 'MeshLib workbench measure / inspect failed',
            });
            postHostMessage('meshlib-workbench:measure-failed', {
                version_id: manifest.version_id,
                error: error instanceof Error ? error.message : 'MeshLib workbench measure / inspect failed',
            });
            return null;
        }
    }

    function normalizeSectionContourPayload(params = {}) {
        const rawAxis = pointTuple(params.plane_axis ?? params.section_axis ?? params.axis ?? params.manual_axis) || [0, 0, 1];
        const magnitude = Math.hypot(rawAxis[0], rawAxis[1], rawAxis[2]) || 1;
        const selectedRegionIds = Array.isArray(params.selected_region_ids)
            ? params.selected_region_ids.filter((item) => typeof item === 'string' && item)
            : Array.isArray(params.regions)
              ? params.regions.filter((item) => typeof item === 'string' && item)
              : activeWorkbenchRegionIds();
        const sectionConstant = Number(
            params.section_constant ??
                params.plane ??
                params.plane_offset ??
                params.offset_mm ??
                params.section_offset_mm ??
                0,
        );
        return {
            section_constant: Number.isFinite(sectionConstant) ? sectionConstant : 0,
            plane_axis: [rawAxis[0] / magnitude, rawAxis[1] / magnitude, rawAxis[2] / magnitude],
            selected_region_ids: selectedRegionIds,
        };
    }

    async function sectionContourInspect(capability, params = {}) {
        if (!capability?.endpoint_url) {
            clearWorkbenchSectionOverlay('unavailable');
            renderWorkbenchResultPanel({
                title: 'Section Slice',
                status: 'Rust endpoint unavailable',
                detail: 'The active workbench manifest did not advertise a section endpoint.',
            });
            return null;
        }
        const request = normalizeSectionContourPayload(params);
        try {
            const url = new URL(capability.endpoint_url, window.location.origin);
            url.searchParams.set('section_constant', String(request.section_constant));
            url.searchParams.set('axis_x', String(request.plane_axis[0]));
            url.searchParams.set('axis_y', String(request.plane_axis[1]));
            url.searchParams.set('axis_z', String(request.plane_axis[2]));
            if (request.selected_region_ids.length > 0) {
                url.searchParams.set('selected_region_ids', request.selected_region_ids.join(','));
            }
            renderWorkbenchResultPanel({
                title: 'Section Slice',
                status: 'Calling Rust REST endpoint',
                rows: [
                    { label: 'Plane offset', value: `${request.section_constant.toFixed(2)} mm` },
                    { label: 'Axis', value: request.plane_axis.map((item) => item.toFixed(2)).join(', ') },
                ],
            });
            const response = await fetch(url.toString(), { method: 'GET' });
            const responsePayload = await response.json().catch(() => ({}));
            if (!response.ok) {
                clearWorkbenchSectionOverlay('error');
                renderWorkbenchResultPanel({
                    title: 'Section Slice',
                    status: 'Rust endpoint failed',
                    detail: responsePayload.detail || response.statusText,
                });
                postHostMessage('meshlib-workbench:command-failed', {
                    command_id: capability.command_id,
                    status: response.status,
                    error: responsePayload.detail || response.statusText,
                });
                return null;
            }
            renderSectionContourResult(responsePayload);
            postHostMessage('meshlib-workbench:section-complete', {
                version_id: hostPayload?.manifest?.version_id || null,
                section: responsePayload,
            });
            return responsePayload;
        } catch (error) {
            clearWorkbenchSectionOverlay('error');
            renderWorkbenchResultPanel({
                title: 'Section Slice',
                status: 'Rust endpoint failed',
                detail: error instanceof Error ? error.message : 'MeshLib workbench section contour failed',
            });
            postHostMessage('meshlib-workbench:command-failed', {
                command_id: capability.command_id,
                error: error instanceof Error ? error.message : 'MeshLib workbench section contour failed',
            });
            return null;
        }
    }

    function meshCutMeasureTopologyPayloadHasItems(payload = {}) {
        return arrayPayloadHasItems(payload.control_vertices) ||
            arrayPayloadHasItems(payload.controlVertices) ||
            arrayPayloadHasItems(payload.path_vertex_indices) ||
            arrayPayloadHasItems(payload.pathVertexIndices);
    }

    function normalizeMeshCutMeasureTopologyPayload(params = {}, options = {}) {
        const rawControls = params.control_vertices ||
            params.controlVertices ||
            params.path_vertex_indices ||
            params.pathVertexIndices ||
            options.control_vertices ||
            options.controlVertices ||
            [];
        const controlVertices = Array.isArray(rawControls)
            ? rawControls.map((index) => Number.parseInt(index, 10)).filter((index) => Number.isFinite(index))
            : [];
        return {
            control_vertices: controlVertices,
            close_path: Boolean(params.close_path ?? params.closePath ?? options.close_path ?? options.closePath ?? false),
            max_path_len_mm: params.max_path_len_mm ?? params.maxPathLenMm ?? options.max_path_len_mm ?? options.maxPathLenMm ?? null,
            operation_label: options.operation_label || params.operation_label || params.operationLabel || 'Mesh Cut & Measure Topology',
        };
    }

    async function meshCutMeasureTopology(params = {}, options = {}) {
        const manifest = hostPayload?.manifest;
        if (!manifest?.mesh_cut_measure_topology_endpoint_url) {
            return null;
        }
        try {
            const request = normalizeMeshCutMeasureTopologyPayload(params, options);
            const response = await fetch(manifest.mesh_cut_measure_topology_endpoint_url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
            });
            const responsePayload = await response.json().catch(() => ({}));
            if (!response.ok) {
                postHostMessage('meshlib-workbench:mesh-cut-measure-topology-failed', {
                    version_id: manifest.version_id,
                    status: response.status,
                    error: responsePayload.detail || response.statusText,
                });
                return null;
            }
            postHostMessage('meshlib-workbench:mesh-cut-measure-topology-complete', {
                version_id: manifest.version_id,
                cut: responsePayload,
            });
            return responsePayload;
        } catch (error) {
            postHostMessage('meshlib-workbench:mesh-cut-measure-topology-failed', {
                version_id: manifest.version_id,
                error: error instanceof Error ? error.message : 'MeshLib workbench Mesh Cut & Measure topology export failed',
            });
            return null;
        }
    }

    function forwardHostWorkbenchCommand(capability, payload = {}, options = {}) {
        const forwarded = {
            version_id: hostPayload?.manifest?.version_id || null,
            command_id: capability.command_id,
            label: capability.label,
            group: capability.group,
            runtime_tool_id: capability.runtime_tool_id,
            endpoint_url_key: capability.endpoint_url_key,
            endpoint_url: capability.endpoint_url,
            rust_backed: capability.rust_backed === true,
            sdk_operations: Array.isArray(capability.sdk_operations) ? capability.sdk_operations : [],
            payload,
            options,
        };
        postHostMessage('meshlib-workbench:host-command', forwarded);
        return {
            status: 'forwarded',
            command_id: capability.command_id,
            endpoint_url_key: capability.endpoint_url_key || null,
        };
    }

    function isRuntimeWorkbenchToolCommand(capability) {
        return Boolean(capability?.runtime_tool_id) ||
            capability?.group === 'runtime' ||
            String(capability?.command_id || '').startsWith('runtime-');
    }

    function booleanWorkbenchFlag(source = {}, keys = []) {
        for (const key of keys) {
            const value = source?.[key];
            if (typeof value === 'boolean') {
                return value;
            }
            if (typeof value === 'string') {
                const normalized = value.toLowerCase();
                if (['1', 'true', 'yes', 'on'].includes(normalized)) {
                    return true;
                }
                if (['0', 'false', 'no', 'off'].includes(normalized)) {
                    return false;
                }
            }
        }
        return null;
    }

    function arrayPayloadHasItems(value) {
        return Array.isArray(value) && value.length > 0;
    }

    function selectionPayloadHasSelector(selection = {}) {
        const selector = typeof selection.metadata?.selector === 'string' ? selection.metadata.selector : '';
        return new Set([
            'area_faces',
            'boundary_edges',
            'boundary_faces',
            'camera_facing_faces',
            'crease_edges',
            'degenerate_faces',
            'graph_cut_region',
            'inside_part_faces',
            'largest_component',
            'not_smooth_faces',
            'outer_layer_faces',
            'overhang_faces',
            'overlapping_faces',
            'pick_face',
            'screen_lasso_faces',
            'screen_rect_faces',
            'self_intersections',
            'short_edges',
        ]).has(selector);
    }

    function selectionPayloadHasItems(payload = {}) {
        const selection = payload.selection && typeof payload.selection === 'object' ? payload.selection : payload;
        return arrayPayloadHasItems(selection.vertex_ids) ||
            arrayPayloadHasItems(selection.face_ids) ||
            arrayPayloadHasItems(selection.region_ids) ||
            arrayPayloadHasItems(selection.brush_points_world) ||
            selectionPayloadHasSelector(selection);
    }

    function brushPayloadHasItems(payload = {}) {
        if (Array.isArray(payload.strokes)) {
            return payload.strokes.some((stroke) => selectionPayloadHasItems(stroke));
        }
        if (payload.stroke && typeof payload.stroke === 'object') {
            return selectionPayloadHasItems(payload.stroke);
        }
        return selectionPayloadHasItems(payload);
    }

    function measurePayloadHasItems(payload = {}) {
        return arrayPayloadHasItems(payload.points) ||
            arrayPayloadHasItems(payload.point) ||
            arrayPayloadHasItems(payload.point_world) ||
            arrayPayloadHasItems(payload.world_point) ||
            arrayPayloadHasItems(payload.points_world) ||
            arrayPayloadHasItems(payload.world_points) ||
            arrayPayloadHasItems(payload.point_pairs) ||
            arrayPayloadHasItems(payload.pairs) ||
            arrayPayloadHasItems(payload.distance_pairs) ||
            arrayPayloadHasItems(payload.segments);
    }

    function runtimeCommandPayloadHasItems(capability, payload = {}) {
        switch (capability.endpoint_url_key) {
            case 'selection_endpoint_url':
                return selectionPayloadHasItems(payload);
            case 'brush_endpoint_url':
                return brushPayloadHasItems(payload);
            case 'measurement_endpoint_url':
                return measurePayloadHasItems(payload);
            case 'mesh_cut_measure_topology_endpoint_url':
                return meshCutMeasureTopologyPayloadHasItems(payload);
            default:
                return false;
        }
    }

    function runtimeWorkbenchToolExecuteFlag(payload = {}, options = {}) {
        return booleanWorkbenchFlag(options, ['execute', 'auto_execute', 'submit'])
            ?? booleanWorkbenchFlag(payload, ['execute', 'auto_execute', 'submit']);
    }

    function shouldForwardRuntimeWorkbenchToolCommand(payload = {}, options = {}) {
        const executeFlag = runtimeWorkbenchToolExecuteFlag(payload, options);
        if (executeFlag === true) {
            return false;
        }
        if (executeFlag === false) {
            return true;
        }
        return false;
    }

    async function dispatchWorkbenchCommand(commandId, payload = {}, options = {}) {
        const capability = findWorkbenchCommandCapability(commandId);
        if (!capability) {
            postHostMessage('meshlib-workbench:command-failed', {
                command_id: commandId,
                error: 'MeshLib workbench command capability was not advertised by the backend manifest',
            });
            return null;
        }

        const metadata = {
            source: 'meshlib_workbench_command_dispatch',
            command_id: capability.command_id,
            runtime_tool_id: capability.runtime_tool_id,
            ...(payload.metadata || {}),
            ...(options.metadata || {}),
        };

        const runtimeExecuteFlag = runtimeWorkbenchToolExecuteFlag(payload, options);
        if (
            isRuntimeWorkbenchToolCommand(capability) &&
            runtimeExecuteFlag !== true &&
            (shouldForwardRuntimeWorkbenchToolCommand(payload, options) || !runtimeCommandPayloadHasItems(capability, payload))
        ) {
            return forwardHostWorkbenchCommand(capability, payload, {
                ...options,
                execute: false,
                metadata,
            });
        }

        if (capability.endpoint_url_key === 'selection_endpoint_url' && isRuntimeWorkbenchToolCommand(capability)) {
            return commitSelection(payload.selection || payload, {
                ...options,
                label: options.label || payload.label || null,
                operation_label: options.operation_label || capability.label || 'Select / Mark Region',
                create_object: options.create_object === true ||
                    options.createObject === true ||
                    payload.create_object === true ||
                    payload.createObject === true ||
                    capability.create_object === true,
                metadata,
            });
        }

        if (capability.endpoint_url_key === 'brush_endpoint_url' && isRuntimeWorkbenchToolCommand(capability)) {
            const rawStrokes = Array.isArray(payload.strokes) ? payload.strokes : [payload.stroke || payload];
            const strokes = rawStrokes.filter(Boolean).map((stroke) => ({
                ...stroke,
                tool_id: stroke.tool_id || capability.runtime_tool_id,
                metadata: {
                    ...(stroke.metadata || {}),
                    ...metadata,
                },
            }));
            return commitBrushStrokes(strokes, {
                ...options,
                operation_label: options.operation_label || capability.label || 'MeshLib Brush Replay',
                metadata,
            });
        }

        if (capability.endpoint_url_key === 'measurement_endpoint_url' && isRuntimeWorkbenchToolCommand(capability)) {
            return measureInspect(payload, {
                ...options,
                operation_label: options.operation_label || capability.label || 'Measure / Inspect',
                metadata,
            });
        }

        if (capability.command_id === 'section' && capability.endpoint_url_key === 'section_endpoint_url') {
            forwardHostWorkbenchCommand(capability, payload, {
                ...options,
                execute: false,
                operation_label: options.operation_label || capability.label || 'Section Slice',
                metadata,
            });
            return sectionContourInspect(capability, payload);
        }

        if (capability.endpoint_url_key === 'mesh_cut_measure_topology_endpoint_url' && isRuntimeWorkbenchToolCommand(capability)) {
            return meshCutMeasureTopology(payload, {
                ...options,
                operation_label: options.operation_label || capability.label || 'Mesh Cut & Measure Topology',
                metadata,
            });
        }

        if (runtimeExecuteFlag !== true) {
            return forwardHostWorkbenchCommand(capability, payload, options);
        }

        const forwarded = forwardHostWorkbenchCommand(capability, payload, options);
        if (runtimeExecuteFlag === true) {
            renderWorkbenchResultPanel({
                title: capability.label || capability.command_id,
                status: 'Submitted through host bridge',
                rows: [
                    { label: 'Command', value: capability.command_id },
                    { label: 'Endpoint', value: capability.endpoint_url_key || 'host' },
                    { label: 'Rust backed', value: capability.rust_backed === true ? 'yes' : 'no' },
                ],
                detail: 'The official MeshLib command was sent to the app host. Job/result evidence is recorded by the backend.',
            });
        }
        return forwarded;
    }

    window.MeshInspectorWorkbenchBridge = {
        get manifest() {
            return hostPayload?.manifest || null;
        },
        get runtimeManifest() {
            return hostPayload?.runtimeManifest || null;
        },
        commitSavedFile,
        commitSelection,
        commitBrushStrokes,
        measureInspect,
        meshCutMeasureTopology,
        dispatchCommand: dispatchWorkbenchCommand,
        findCommandCapability: findWorkbenchCommandCapability,
        officialWorkbenchTools,
    };
    window.meshinspectorWorkbenchBridge = window.MeshInspectorWorkbenchBridge;
    window.meshinspectorWorkbenchDispatchCommand = dispatchWorkbenchCommand;
    installWorkbenchCanvasCommandBridge();
    document.documentElement.dataset.meshinspectorWorkbenchBridge = 'ready';

    async function bootFromPayload() {
        if (loadStarted || !runtimeReady || !hostPayload?.manifest) {
            return;
        }

        const manifest = hostPayload.manifest;
        const meshUrl = meshUrlForPayload(manifest);
        if (!meshUrl) {
            signalReady();
            return;
        }

        if (typeof emplace_file_in_local_FS_and_open !== 'function') {
            console.error('MeshLib runtime loaded without emplace_file_in_local_FS_and_open');
            signalReady();
            return;
        }

        loadStarted = true;
        try {
            const response = await fetch(meshUrl, { credentials: 'same-origin' });
            if (!response.ok) {
                throw new Error(`Failed to fetch runtime mesh (${response.status})`);
            }
            const bytes = new Uint8Array(await response.arrayBuffer());
            const filename = `active-version.${extensionForPayload(manifest)}`;
            emplace_file_in_local_FS_and_open(filename, bytes, function () {
                signalReady();
            });
        } catch (error) {
            console.error('Failed to load active mesh into MeshLib runtime', error);
            signalReady();
        }
    }

    const previousPostWasmLoad = window.postWasmLoad;
    window.postWasmLoad = function () {
        if (typeof previousPostWasmLoad === 'function') {
            previousPostWasmLoad();
        }
        runtimeReady = true;
        void bootFromPayload();
    };

    window.addEventListener('message', function (event) {
        if (event.origin !== window.location.origin) {
            return;
        }
        if (event.data?.type !== 'meshlib-workbench:init') {
            return;
        }
        hostPayload = event.data.payload;
        markCommandCapabilities();
        syncWorkbenchCanvasCommandOverlay();
        void bootFromPayload();
    });
})();
