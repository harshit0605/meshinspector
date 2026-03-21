'use client';

import { Suspense, useEffect, useMemo, useRef, useState } from 'react';
import { Canvas, ThreeEvent, useFrame, useLoader, useThree } from '@react-three/fiber';
import { Bounds, Environment, Html, OrbitControls, useBounds, useGLTF } from '@react-three/drei';
import * as THREE from 'three';
import { PLYLoader } from 'three/examples/jsm/loaders/PLYLoader.js';
import { acceleratedRaycast, computeBoundsTree, disposeBoundsTree } from 'three-mesh-bvh';
import type { ScalarOverlayResponse, SectionContourPayload } from '@/lib/api/types';

type RegionPayload = {
  regions: Array<{
    region_id: string;
    label: string;
    vertex_indices: number[];
  }>;
};

const REGION_COLORS: Record<string, string> = {
  inner_band: '#38bdf8',
  outer_band: '#22c55e',
  head: '#f59e0b',
  ornament_relief: '#f43f5e',
  unknown: '#71717a',
};

(THREE.BufferGeometry.prototype as unknown as { computeBoundsTree?: () => void }).computeBoundsTree = computeBoundsTree;
(THREE.BufferGeometry.prototype as unknown as { disposeBoundsTree?: () => void }).disposeBoundsTree = disposeBoundsTree;
(THREE.Mesh.prototype as unknown as { raycast?: THREE.Mesh['raycast'] }).raycast = acceleratedRaycast;

function FitScene() {
  const bounds = useBounds();
  useEffect(() => {
    bounds.refresh().clip().fit();
  }, [bounds]);
  return null;
}

function Loading() {
  return (
    <Html center>
      <div className="rounded-lg border border-zinc-700 bg-zinc-900/90 px-4 py-2 text-sm text-zinc-200">
        Loading mesh...
      </div>
    </Html>
  );
}

function useJsonPayload<T>(url?: string | null) {
  const [payload, setPayload] = useState<T | null>(null);

  useEffect(() => {
    let active = true;
    setPayload(null);
    if (!url) {
      return;
    }

    void fetch(url)
      .then((response) => response.json())
      .then((data: T) => {
        if (active) {
          setPayload(data);
        }
      })
      .catch(() => {
        if (active) {
          setPayload(null);
        }
      });

    return () => {
      active = false;
    };
  }, [url]);

  return payload;
}

function createClippingPlane(sectionEnabled: boolean) {
  return new THREE.Plane(new THREE.Vector3(0, 1, 0), sectionEnabled ? 0 : 0);
}

function colorForScalar(value: number, overlay: ScalarOverlayResponse) {
  if (overlay.overlay_type === 'compare') {
    const scale = Math.max(Math.abs(overlay.min_value), Math.abs(overlay.max_value), 1e-6);
    const normalized = Math.max(-1, Math.min(1, value / scale));
    if (normalized >= 0) {
      return new THREE.Color().setRGB(1.0, 0.3 + 0.4 * (1 - normalized), 0.2 + 0.4 * (1 - normalized));
    }
    return new THREE.Color().setRGB(0.2 + 0.4 * (1 + normalized), 0.5 + 0.3 * (1 + normalized), 1.0);
  }

  const min = overlay.min_value;
  const max = Math.max(overlay.max_value, min + 1e-6);
  const t = Math.max(0, Math.min(1, (value - min) / (max - min)));
  const low = new THREE.Color('#ef4444');
  const mid = new THREE.Color('#f59e0b');
  const high = new THREE.Color('#22c55e');
  return t < 0.5 ? low.lerp(mid, t / 0.5) : mid.lerp(high, (t - 0.5) / 0.5);
}

function useVertexToRegion(payload: RegionPayload | null) {
  return useMemo(() => {
    if (!payload) {
      return new Map<number, string>();
    }
    const map = new Map<number, string>();
    for (const region of payload.regions) {
      for (const vertexIndex of region.vertex_indices) {
        if (!map.has(vertexIndex)) {
          map.set(vertexIndex, region.region_id);
        }
      }
    }
    return map;
  }, [payload]);
}

function computeSliceStats(
  geometry: THREE.BufferGeometry,
  regionPayload: RegionPayload | null,
  selectedRegionIds: string[],
  sectionConstant: number,
): SectionContourPayload {
  const vertexToRegion = new Map<number, string>();
  for (const region of regionPayload?.regions ?? []) {
    for (const vertexIndex of region.vertex_indices) {
      if (!vertexToRegion.has(vertexIndex)) {
        vertexToRegion.set(vertexIndex, region.region_id);
      }
    }
  }

  const position = geometry.getAttribute('position');
  const index = geometry.getIndex();
  const selectedRegionSet = new Set(selectedRegionIds);
  const epsilon = 1e-5;
  const segmentKeys = new Set<string>();
  const endpointKeys = new Set<string>();
  const adjacency = new Map<string, Set<string>>();
  const segments: Array<{ start: [number, number, number]; end: [number, number, number]; selected_region_hit: boolean }> = [];
  let segmentCount = 0;
  let selectedRegionSegmentCount = 0;
  let perimeterMm = 0;
  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minZ = Number.POSITIVE_INFINITY;
  let maxZ = Number.NEGATIVE_INFINITY;

  const quantizePoint = (point: THREE.Vector3) =>
    [point.x, point.y, point.z].map((value) => Math.round(value / epsilon)).join(':');

  const addAdjacency = (a: string, b: string) => {
    if (!adjacency.has(a)) adjacency.set(a, new Set());
    if (!adjacency.has(b)) adjacency.set(b, new Set());
    adjacency.get(a)?.add(b);
    adjacency.get(b)?.add(a);
  };

  const edgeIntersections = (aIndex: number, bIndex: number): THREE.Vector3[] => {
    const a = new THREE.Vector3(position.getX(aIndex), position.getY(aIndex), position.getZ(aIndex));
    const b = new THREE.Vector3(position.getX(bIndex), position.getY(bIndex), position.getZ(bIndex));
    const da = a.y - sectionConstant;
    const db = b.y - sectionConstant;

    if (Math.abs(da) <= epsilon && Math.abs(db) <= epsilon) {
      return [];
    }
    if (Math.abs(da) <= epsilon) {
      return [a];
    }
    if (Math.abs(db) <= epsilon) {
      return [b];
    }
    if (da * db > 0) {
      return [];
    }
    const t = da / (da - db);
    return [a.lerp(b, t)];
  };

  const triangleCount = index ? index.count / 3 : position.count / 3;
  for (let triangleIndex = 0; triangleIndex < triangleCount; triangleIndex += 1) {
    const aIndex = index ? index.getX(triangleIndex * 3) : triangleIndex * 3;
    const bIndex = index ? index.getX(triangleIndex * 3 + 1) : triangleIndex * 3 + 1;
    const cIndex = index ? index.getX(triangleIndex * 3 + 2) : triangleIndex * 3 + 2;

    const intersections = [
      ...edgeIntersections(aIndex, bIndex),
      ...edgeIntersections(bIndex, cIndex),
      ...edgeIntersections(cIndex, aIndex),
    ];
    const uniquePoints = new Map<string, THREE.Vector3>();
    for (const point of intersections) {
      uniquePoints.set(quantizePoint(point), point);
    }
    const points = Array.from(uniquePoints.values());
    if (points.length !== 2) {
      continue;
    }

    const [p0, p1] = points;
    const key0 = quantizePoint(p0);
    const key1 = quantizePoint(p1);
    const segmentKey = [key0, key1].sort().join('|');
    if (segmentKeys.has(segmentKey)) {
      continue;
    }
    segmentKeys.add(segmentKey);
    endpointKeys.add(key0);
    endpointKeys.add(key1);
    addAdjacency(key0, key1);
    segmentCount += 1;
    perimeterMm += p0.distanceTo(p1);
    minX = Math.min(minX, p0.x, p1.x);
    maxX = Math.max(maxX, p0.x, p1.x);
    minZ = Math.min(minZ, p0.z, p1.z);
    maxZ = Math.max(maxZ, p0.z, p1.z);

    const triangleRegions = [aIndex, bIndex, cIndex]
      .map((vertexIndex) => vertexToRegion.get(vertexIndex))
      .filter((value): value is string => !!value);
    if (triangleRegions.some((regionId) => selectedRegionSet.has(regionId))) {
      selectedRegionSegmentCount += 1;
    }
    segments.push({
      start: [p0.x, p0.y, p0.z],
      end: [p1.x, p1.y, p1.z],
      selected_region_hit: triangleRegions.some((regionId) => selectedRegionSet.has(regionId)),
    });
  }

  let contourCount = 0;
  const visited = new Set<string>();
  for (const endpoint of endpointKeys) {
    if (visited.has(endpoint)) {
      continue;
    }
    contourCount += 1;
    const stack = [endpoint];
    while (stack.length) {
      const current = stack.pop()!;
      if (visited.has(current)) {
        continue;
      }
      visited.add(current);
      for (const next of adjacency.get(current) ?? []) {
        if (!visited.has(next)) {
          stack.push(next);
        }
      }
    }
  }

  return {
    section_constant: sectionConstant,
    contour_count: contourCount,
    segment_count: segmentCount,
    selected_region_segment_count: selectedRegionSegmentCount,
    perimeter_mm: segmentCount > 0 ? perimeterMm : null,
    width_mm: segmentCount > 0 ? maxX - minX : null,
    depth_mm: segmentCount > 0 ? maxZ - minZ : null,
    bounds_min: segmentCount > 0 ? [minX, sectionConstant, minZ] : null,
    bounds_max: segmentCount > 0 ? [maxX, sectionConstant, maxZ] : null,
    segments,
  };
}

function RegionPickMesh({
  geometry,
  regionPayload,
  onRegionPick,
}: {
  geometry: THREE.BufferGeometry;
  regionPayload: RegionPayload | null;
  onRegionPick?: (regionId: string, additive?: boolean) => void;
}) {
  const vertexToRegion = useVertexToRegion(regionPayload);

  const onPointerDown = (event: ThreeEvent<PointerEvent>) => {
    if (!event.face || !onRegionPick) {
      return;
    }
    const candidates = [event.face.a, event.face.b, event.face.c];
    const counts = new Map<string, number>();
    for (const vertexIndex of candidates) {
      const regionId = vertexToRegion.get(vertexIndex);
      if (!regionId) continue;
      counts.set(regionId, (counts.get(regionId) ?? 0) + 1);
    }
    const picked = Array.from(counts.entries()).sort((a, b) => b[1] - a[1])[0]?.[0];
    if (picked) {
      event.stopPropagation();
      onRegionPick(picked, event.shiftKey || event.ctrlKey || event.metaKey);
    }
  };

  return (
    <mesh geometry={geometry} onPointerDown={onPointerDown} renderOrder={1}>
      <meshBasicMaterial transparent opacity={0.001} depthWrite={false} />
    </mesh>
  );
}

function ScalarOverlay({
  geometry,
  overlay,
  sectionEnabled,
  sectionConstant,
}: {
  geometry: THREE.BufferGeometry;
  overlay: ScalarOverlayResponse | null;
  sectionEnabled: boolean;
  sectionConstant: number;
}) {
  const { gl } = useThree();
  const clippingPlane = useMemo(() => createClippingPlane(sectionEnabled), [sectionEnabled]);
  const overlayGeometry = useMemo(() => {
    if (!overlay) {
      return null;
    }
    const clone = geometry.clone();
    const positionCount = clone.attributes.position.count;
    const colors = new Float32Array(positionCount * 3);
    for (let index = 0; index < positionCount; index += 1) {
      const color = colorForScalar(overlay.values[index] ?? 0, overlay);
      colors[index * 3] = color.r;
      colors[index * 3 + 1] = color.g;
      colors[index * 3 + 2] = color.b;
    }
    clone.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    (clone as THREE.BufferGeometry & { computeBoundsTree?: () => void }).computeBoundsTree?.();
    return clone;
  }, [geometry, overlay]);

  useEffect(() => {
    gl.localClippingEnabled = sectionEnabled;
  }, [gl, sectionEnabled]);

  useFrame(() => {
    clippingPlane.constant = sectionConstant;
  });

  useEffect(() => () => overlayGeometry?.dispose(), [overlayGeometry]);

  if (!overlayGeometry || !overlay) {
    return null;
  }

  return (
    <mesh geometry={overlayGeometry} renderOrder={2}>
      <meshStandardMaterial
        vertexColors
        transparent
        opacity={0.58}
        depthWrite={false}
        polygonOffset
        polygonOffsetFactor={-1}
        clippingPlanes={sectionEnabled ? [clippingPlane] : []}
      />
    </mesh>
  );
}

function SectionContourOverlay({
  contour,
  sectionEnabled,
}: {
  contour: SectionContourPayload | null;
  sectionEnabled: boolean;
}) {
  const contourGeometry = useMemo(() => {
    if (!sectionEnabled || !contour?.segments.length) {
      return null;
    }
    const points = contour.segments.flatMap((segment) => [...segment.start, ...segment.end]);
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(points, 3));
    return geometry;
  }, [contour, sectionEnabled]);

  const dimensionGeometry = useMemo(() => {
    if (!sectionEnabled || !contour?.bounds_min || !contour.bounds_max) {
      return null;
    }
    const [minX, y, minZ] = contour.bounds_min;
    const [maxX, , maxZ] = contour.bounds_max;
    const points = [
      minX, y, minZ,
      maxX, y, minZ,
      maxX, y, minZ,
      maxX, y, maxZ,
    ];
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(points, 3));
    return geometry;
  }, [contour, sectionEnabled]);

  useEffect(() => () => contourGeometry?.dispose(), [contourGeometry]);
  useEffect(() => () => dimensionGeometry?.dispose(), [dimensionGeometry]);

  if (!sectionEnabled || !contourGeometry) {
    return null;
  }

  return (
    <>
      <lineSegments geometry={contourGeometry} renderOrder={4}>
        <lineBasicMaterial color="#f8fafc" transparent opacity={0.95} />
      </lineSegments>
      {dimensionGeometry ? (
        <lineSegments geometry={dimensionGeometry} renderOrder={5}>
          <lineBasicMaterial color="#f59e0b" transparent opacity={0.8} />
        </lineSegments>
      ) : null}
    </>
  );
}

function RegionOverlay({
  geometry,
  regionPayload,
  selectedRegionId,
  selectedRegionIds,
  enabled,
  sectionEnabled,
  sectionConstant,
}: {
  geometry: THREE.BufferGeometry;
  regionPayload: RegionPayload | null;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  enabled: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
}) {
  const { gl } = useThree();
  const clippingPlane = useMemo(() => createClippingPlane(sectionEnabled), [sectionEnabled]);
  const overlayGeometry = useMemo(() => {
    if (!regionPayload || !enabled || selectedRegionIds.length === 0) {
      return null;
    }

    const clone = geometry.clone();
    const positionCount = clone.attributes.position.count;
    const colors = new Float32Array(positionCount * 3);
    for (let index = 0; index < positionCount; index += 1) {
      colors[index * 3] = 0.12;
      colors[index * 3 + 1] = 0.12;
      colors[index * 3 + 2] = 0.15;
    }
    for (const region of regionPayload.regions) {
      if (!selectedRegionIds.includes(region.region_id)) {
        continue;
      }
      const baseTint = new THREE.Color(REGION_COLORS[region.region_id] || '#60a5fa');
      const tint = region.region_id === selectedRegionId ? baseTint.clone().offsetHSL(0, 0, 0.1) : baseTint;
      for (const vertexIndex of region.vertex_indices) {
        if (vertexIndex < 0 || vertexIndex >= positionCount) continue;
        colors[vertexIndex * 3] = tint.r;
        colors[vertexIndex * 3 + 1] = tint.g;
        colors[vertexIndex * 3 + 2] = tint.b;
      }
    }
    clone.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    (clone as THREE.BufferGeometry & { computeBoundsTree?: () => void }).computeBoundsTree?.();
    return clone;
  }, [enabled, geometry, regionPayload, selectedRegionId, selectedRegionIds]);

  useEffect(() => {
    gl.localClippingEnabled = sectionEnabled;
  }, [gl, sectionEnabled]);

  useFrame(() => {
    clippingPlane.constant = sectionConstant;
  });

  useEffect(() => () => overlayGeometry?.dispose(), [overlayGeometry]);

  if (!overlayGeometry) {
    return null;
  }

  return (
    <mesh geometry={overlayGeometry} renderOrder={3}>
      <meshStandardMaterial
        vertexColors
        transparent
        opacity={0.42}
        depthWrite={false}
        polygonOffset
        polygonOffsetFactor={-2}
        clippingPlanes={sectionEnabled ? [clippingPlane] : []}
      />
    </mesh>
  );
}

function OverlayLayer({
  normalizedMeshUrl,
  regionArtifactUrl,
  scalarOverlay,
  regionOverlayEnabled,
  selectedRegionId,
  selectedRegionIds,
  sectionEnabled,
  sectionConstant,
  onRegionPick,
  onSectionContourChange,
}: {
  normalizedMeshUrl?: string | null;
  regionArtifactUrl?: string | null;
  scalarOverlay: ScalarOverlayResponse | null;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  sectionEnabled: boolean;
  sectionConstant: number;
  onRegionPick?: (regionId: string, additive?: boolean) => void;
  onSectionContourChange?: (payload: SectionContourPayload | null) => void;
}) {
  const geometry = useLoader(PLYLoader, normalizedMeshUrl || '');
  const regionPayload = useJsonPayload<RegionPayload>(regionArtifactUrl);
  const contourPayload = useMemo(
    () => computeSliceStats(geometry, regionPayload, selectedRegionIds, sectionConstant),
    [geometry, regionPayload, sectionConstant, selectedRegionIds],
  );

  useEffect(() => {
    geometry.computeVertexNormals();
    (geometry as THREE.BufferGeometry & { computeBoundsTree?: () => void }).computeBoundsTree?.();
  }, [geometry]);

  useEffect(() => {
    if (!onSectionContourChange) {
      return;
    }
    onSectionContourChange(contourPayload);
  }, [contourPayload, onSectionContourChange]);

  useEffect(() => () => onSectionContourChange?.(null), [onSectionContourChange]);

  return (
    <>
      <RegionPickMesh geometry={geometry} regionPayload={regionPayload} onRegionPick={onRegionPick} />
      <ScalarOverlay geometry={geometry} overlay={scalarOverlay} sectionEnabled={sectionEnabled} sectionConstant={sectionConstant} />
      <SectionContourOverlay contour={contourPayload} sectionEnabled={sectionEnabled} />
      <RegionOverlay
        geometry={geometry}
        regionPayload={regionPayload}
        selectedRegionId={selectedRegionId}
        selectedRegionIds={selectedRegionIds}
        enabled={regionOverlayEnabled}
        sectionEnabled={sectionEnabled}
        sectionConstant={sectionConstant}
      />
    </>
  );
}

function MeshModel({
  lowUrl,
  highUrl,
  wireframe,
  sectionEnabled,
  sectionConstant,
  normalizedMeshUrl,
  regionArtifactUrl,
  regionOverlayEnabled,
  selectedRegionId,
  selectedRegionIds,
  scalarOverlay,
  onRegionPick,
  onSectionContourChange,
}: {
  lowUrl: string;
  highUrl?: string | null;
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  normalizedMeshUrl?: string | null;
  regionArtifactUrl?: string | null;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  scalarOverlay: ScalarOverlayResponse | null;
  onRegionPick?: (regionId: string, additive?: boolean) => void;
  onSectionContourChange?: (payload: SectionContourPayload | null) => void;
}) {
  const [useHigh, setUseHigh] = useState(false);
  const model = useGLTF(useHigh && highUrl ? highUrl : lowUrl);
  const groupRef = useRef<THREE.Group>(null);
  const { gl } = useThree();
  const clippingPlane = useMemo(() => createClippingPlane(sectionEnabled), [sectionEnabled]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      if (highUrl) setUseHigh(true);
    }, 1200);
    return () => window.clearTimeout(timer);
  }, [highUrl]);

  useEffect(() => {
    gl.localClippingEnabled = sectionEnabled;
    model.scene.traverse((child) => {
      if (!(child instanceof THREE.Mesh)) return;
      child.geometry.computeVertexNormals();
      (child.geometry as THREE.BufferGeometry & { computeBoundsTree?: () => void }).computeBoundsTree?.();
      const materials = Array.isArray(child.material) ? child.material : [child.material];
      materials.forEach((material) => {
        if (
          material instanceof THREE.MeshStandardMaterial ||
          material instanceof THREE.MeshPhysicalMaterial ||
          material instanceof THREE.MeshBasicMaterial
        ) {
          material.wireframe = wireframe;
          material.clippingPlanes = sectionEnabled ? [clippingPlane] : [];
          material.clipShadows = true;
          material.needsUpdate = true;
        }
      });
    });
  }, [clippingPlane, gl, model.scene, sectionEnabled, wireframe]);

  useFrame(() => {
    clippingPlane.constant = sectionConstant;
  });

  return (
    <group ref={groupRef}>
      <primitive object={model.scene} />
      {normalizedMeshUrl && (
        <OverlayLayer
          normalizedMeshUrl={normalizedMeshUrl}
          regionArtifactUrl={regionArtifactUrl}
          scalarOverlay={scalarOverlay}
          regionOverlayEnabled={regionOverlayEnabled}
          selectedRegionId={selectedRegionId}
          selectedRegionIds={selectedRegionIds}
          sectionEnabled={sectionEnabled}
          sectionConstant={sectionConstant}
          onRegionPick={onRegionPick}
          onSectionContourChange={onSectionContourChange}
        />
      )}
    </group>
  );
}

export default function ViewerEngine({
  lowUrl,
  highUrl,
  wireframe,
  sectionEnabled,
  sectionConstant,
  normalizedMeshUrl,
  regionArtifactUrl,
  regionOverlayEnabled,
  selectedRegionId,
  selectedRegionIds,
  scalarOverlay,
  onRegionPick,
  onSectionContourChange,
}: {
  lowUrl: string;
  highUrl?: string | null;
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  normalizedMeshUrl?: string | null;
  regionArtifactUrl?: string | null;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  scalarOverlay: ScalarOverlayResponse | null;
  onRegionPick?: (regionId: string, additive?: boolean) => void;
  onSectionContourChange?: (payload: SectionContourPayload | null) => void;
}) {
  return (
    <Canvas shadows camera={{ position: [0, 0, 140], fov: 35 }} className="h-full w-full">
      <color attach="background" args={['#0a0a0b']} />
      <ambientLight intensity={1.2} />
      <directionalLight position={[60, 80, 40]} intensity={2.5} />
      <Suspense fallback={<Loading />}>
        <Bounds fit clip observe margin={1.1}>
          <MeshModel
            lowUrl={lowUrl}
            highUrl={highUrl}
            wireframe={wireframe}
            sectionEnabled={sectionEnabled}
            sectionConstant={sectionConstant}
            normalizedMeshUrl={normalizedMeshUrl}
            regionArtifactUrl={regionArtifactUrl}
            regionOverlayEnabled={regionOverlayEnabled}
            selectedRegionId={selectedRegionId}
            selectedRegionIds={selectedRegionIds}
            scalarOverlay={scalarOverlay}
            onRegionPick={onRegionPick}
            onSectionContourChange={onSectionContourChange}
          />
          <FitScene />
        </Bounds>
        <Environment preset="studio" />
      </Suspense>
      <OrbitControls makeDefault />
    </Canvas>
  );
}
