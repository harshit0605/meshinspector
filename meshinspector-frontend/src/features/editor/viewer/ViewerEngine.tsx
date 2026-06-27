'use client';

import { Suspense, useEffect, useMemo, useRef, useState } from 'react';
import { Canvas, ThreeEvent, useFrame, useLoader, useThree } from '@react-three/fiber';
import { Bounds, Environment, GizmoHelper, GizmoViewport, Html, OrbitControls, useBounds, useGLTF } from '@react-three/drei';
import * as THREE from 'three';
import { PLYLoader } from 'three/examples/jsm/loaders/PLYLoader.js';
import { acceleratedRaycast, computeBoundsTree, disposeBoundsTree } from 'three-mesh-bvh';
import type { ScalarOverlayResponse, SectionContourPayload, TextureArtifactManifest } from '@/lib/api/types';

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
  gem_seat: '#a78bfa',
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

function createPlaneFrame(sectionAxis?: [number, number, number] | null) {
  const normal = new THREE.Vector3(...(sectionAxis ?? [0, 1, 0]));
  if (normal.lengthSq() < 1e-8) {
    normal.set(0, 1, 0);
  }
  normal.normalize();
  const reference = Math.abs(normal.y) < 0.95 ? new THREE.Vector3(0, 1, 0) : new THREE.Vector3(1, 0, 0);
  const uAxis = new THREE.Vector3().crossVectors(reference, normal).normalize();
  const vAxis = new THREE.Vector3().crossVectors(normal, uAxis).normalize();
  return { normal, uAxis, vAxis };
}

function createClippingPlane(sectionAxis?: [number, number, number] | null) {
  const { normal } = createPlaneFrame(sectionAxis);
  return new THREE.Plane(normal, 0);
}

function setClippingPlaneOffset(plane: THREE.Plane, offset: number, sectionAxis?: [number, number, number] | null) {
  const { normal } = createPlaneFrame(sectionAxis);
  plane.normal.copy(normal);
  plane.constant = -offset;
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

function cloneTexturedMaterial(source: THREE.Material, texture: THREE.Texture) {
  const material =
    source instanceof THREE.MeshStandardMaterial ||
    source instanceof THREE.MeshPhysicalMaterial ||
    source instanceof THREE.MeshBasicMaterial
      ? source.clone()
      : new THREE.MeshStandardMaterial();
  if (
    material instanceof THREE.MeshStandardMaterial ||
    material instanceof THREE.MeshPhysicalMaterial ||
    material instanceof THREE.MeshBasicMaterial
  ) {
    material.map = texture;
    material.color.set(0xffffff);
    material.needsUpdate = true;
  }
  return material;
}

function textureImageSize(texture: THREE.Texture) {
  const image = texture.image as TexImageSource | undefined;
  const width = Number(
    image && 'width' in image ? image.width : image && 'videoWidth' in image ? image.videoWidth : 0,
  );
  const height = Number(
    image && 'height' in image ? image.height : image && 'videoHeight' in image ? image.videoHeight : 0,
  );
  return Number.isFinite(width) && Number.isFinite(height) && width > 0 && height > 0
    ? { width, height }
    : null;
}

function createMeshLibTextureArray(textures: THREE.Texture[]) {
  const firstSize = textureImageSize(textures[0]);
  if (!firstSize || textures.length === 0 || typeof document === 'undefined') {
    return null;
  }

  const canvas = document.createElement('canvas');
  canvas.width = firstSize.width;
  canvas.height = firstSize.height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) {
    return null;
  }

  const layerSize = firstSize.width * firstSize.height * 4;
  const data = new Uint8Array(layerSize * textures.length);
  textures.forEach((texture, layerIndex) => {
    const image = texture.image as CanvasImageSource | undefined;
    if (!image) {
      return;
    }
    context.clearRect(0, 0, firstSize.width, firstSize.height);
    context.drawImage(image, 0, 0, firstSize.width, firstSize.height);
    data.set(context.getImageData(0, 0, firstSize.width, firstSize.height).data, layerIndex * layerSize);
  });

  const textureArray = new THREE.DataArrayTexture(data, firstSize.width, firstSize.height, textures.length);
  textureArray.format = THREE.RGBAFormat;
  textureArray.type = THREE.UnsignedByteType;
  textureArray.minFilter = THREE.LinearFilter;
  textureArray.magFilter = THREE.LinearFilter;
  textureArray.wrapS = THREE.ClampToEdgeWrapping;
  textureArray.wrapT = THREE.ClampToEdgeWrapping;
  textureArray.needsUpdate = true;
  return textureArray;
}

function texturePerFaceResolution(faceCount: number) {
  const width = Math.max(1, Math.ceil(Math.sqrt(faceCount)));
  return { width, height: Math.max(1, Math.ceil(faceCount / width)) };
}

function createMeshLibTexturePerFaceTexture({
  textureEntries,
  texturePerFace,
  faceCount,
}: {
  textureEntries: TextureArtifactManifest[];
  texturePerFace: number[];
  faceCount: number;
}) {
  const { width, height } = texturePerFaceResolution(faceCount);
  const data = new Uint8Array(width * height);
  const layerByTextureId = new Map(textureEntries.map((entry, layerIndex) => [entry.texture_index, layerIndex]));
  for (let faceIndex = 0; faceIndex < faceCount; faceIndex += 1) {
    const textureId = texturePerFace[faceIndex] ?? 0;
    const layerIndex = layerByTextureId.get(textureId) ?? 0;
    data[faceIndex] = Math.max(0, Math.min(255, layerIndex));
  }

  const texture = new THREE.DataTexture(data, width, height, THREE.RedIntegerFormat, THREE.UnsignedByteType);
  texture.minFilter = THREE.NearestFilter;
  texture.magFilter = THREE.NearestFilter;
  texture.wrapS = THREE.ClampToEdgeWrapping;
  texture.wrapT = THREE.ClampToEdgeWrapping;
  texture.needsUpdate = true;
  return { texture, width, height };
}

function ensureMeshLibFaceIndexAttribute(mesh: THREE.Mesh) {
  if (mesh.geometry.index) {
    mesh.geometry = mesh.geometry.toNonIndexed();
  }
  const geometry = mesh.geometry;
  if (!geometry.attributes.uv) {
    return null;
  }
  const vertexCount = geometry.attributes.position.count;
  const faceCount = Math.floor(vertexCount / 3);
  if (faceCount === 0) {
    return null;
  }

  const faceIndices = new Float32Array(vertexCount);
  for (let faceIndex = 0; faceIndex < faceCount; faceIndex += 1) {
    faceIndices[faceIndex * 3] = faceIndex;
    faceIndices[faceIndex * 3 + 1] = faceIndex;
    faceIndices[faceIndex * 3 + 2] = faceIndex;
  }
  geometry.setAttribute('meshlibFaceIndex', new THREE.BufferAttribute(faceIndices, 1));
  geometry.computeVertexNormals();
  return { geometry, faceCount };
}

function createMeshLibTextureArrayMaterial({
  textureArray,
  texturePerFaceTexture,
  texturePerFaceSize,
  wireframe,
}: {
  textureArray: THREE.DataArrayTexture;
  texturePerFaceTexture: THREE.DataTexture;
  texturePerFaceSize: [number, number];
  wireframe: boolean;
}) {
  return new THREE.ShaderMaterial({
    glslVersion: THREE.GLSL3,
    uniforms: {
      tex: { value: textureArray },
      texturePerFace: { value: texturePerFaceTexture },
      texturePerFaceSize: { value: new THREE.Vector2(texturePerFaceSize[0], texturePerFaceSize[1]) },
      mainColor: { value: new THREE.Color(0xffffff) },
      lightDirection: { value: new THREE.Vector3(0.25, 0.45, 1).normalize() },
    },
    vertexShader: `
      in vec3 position;
      in vec3 normal;
      in vec2 uv;
      in float meshlibFaceIndex;

      uniform mat4 modelViewMatrix;
      uniform mat4 projectionMatrix;
      uniform mat3 normalMatrix;

      out vec2 vUv;
      out vec3 vNormal;
      flat out uint vPrimitiveId;

      void main() {
        vUv = uv;
        vNormal = normalize(normalMatrix * normal);
        vPrimitiveId = uint(meshlibFaceIndex);
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `,
    fragmentShader: `
      precision highp float;
      precision highp int;
      precision highp usampler2D;

      uniform highp sampler2DArray tex;
      uniform highp usampler2D texturePerFace;
      uniform vec2 texturePerFaceSize;
      uniform vec3 mainColor;
      uniform vec3 lightDirection;

      in vec2 vUv;
      in vec3 vNormal;
      flat in uint vPrimitiveId;

      out vec4 outColor;

      void main() {
        uint textureWidth = uint(texturePerFaceSize.x);
        uint textId = texelFetch(texturePerFace, ivec2(vPrimitiveId % textureWidth, vPrimitiveId / textureWidth), 0).r;
        vec4 textColor = texture(tex, vec3(vUv, float(textId)));
        float light = max(dot(normalize(vNormal), normalize(lightDirection)), 0.18);
        vec4 colorCpy = vec4(mainColor * light, 1.0);
        float destA = colorCpy.a;
        colorCpy.a = textColor.a + destA * (1.0 - textColor.a);
        if (colorCpy.a == 0.0) {
          colorCpy.rgb = vec3(0.0);
        } else {
          colorCpy.rgb = mix(colorCpy.rgb * destA, textColor.rgb, textColor.a) / colorCpy.a;
        }
        outColor = colorCpy;
      }
    `,
    wireframe,
  });
}

function applyMeshLibTextureArrayShader({
  mesh,
  textureArray,
  textureEntries,
  texturePerFace,
  wireframe,
}: {
  mesh: THREE.Mesh;
  textureArray: THREE.DataArrayTexture | null;
  textureEntries: TextureArtifactManifest[];
  texturePerFace: number[];
  wireframe: boolean;
}) {
  if (!textureArray || textureEntries.length <= 1 || texturePerFace.length === 0) {
    return false;
  }
  const geometryDetails = ensureMeshLibFaceIndexAttribute(mesh);
  if (!geometryDetails) {
    return false;
  }
  const { texture, width, height } = createMeshLibTexturePerFaceTexture({
    textureEntries,
    texturePerFace,
    faceCount: geometryDetails.faceCount,
  });
  mesh.material = createMeshLibTextureArrayMaterial({
    textureArray,
    texturePerFaceTexture: texture,
    texturePerFaceSize: [width, height],
    wireframe,
  });
  return true;
}

function applyMeshLibTexturePerFaceGroups({
  mesh,
  textures,
  textureEntries,
  texturePerFace,
}: {
  mesh: THREE.Mesh;
  textures: THREE.Texture[];
  textureEntries: TextureArtifactManifest[];
  texturePerFace: number[];
}) {
  const sourceMaterials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
  const sourceMaterial = sourceMaterials.find((material) => material instanceof THREE.Material) ?? new THREE.MeshStandardMaterial();
  const materials = textureEntries.map((entry, index) =>
    cloneTexturedMaterial(sourceMaterial, textures[index] ?? textures[0]),
  );
  const geometry = mesh.geometry;
  const faceCount = Math.floor((geometry.index?.count ?? geometry.attributes.position.count) / 3);

  if (materials.length <= 1 || texturePerFace.length === 0 || faceCount === 0) {
    mesh.material = materials[0] ?? sourceMaterial;
    return;
  }

  const materialIndexByTextureId = new Map(textureEntries.map((entry, materialIndex) => [entry.texture_index, materialIndex]));
  geometry.clearGroups();
  for (let faceIndex = 0; faceIndex < faceCount; faceIndex += 1) {
    const textureId = texturePerFace[faceIndex] ?? 0;
    const materialIndex = materialIndexByTextureId.get(textureId) ?? 0;
    geometry.addGroup(faceIndex * 3, 3, materialIndex);
  }
  mesh.material = materials;
}

function MeshTextureSync({
  scene,
  textureArtifactUrl,
  textureMetadata,
  textureArtifacts = [],
  texturePerFace = [],
  wireframe,
}: {
  scene: THREE.Object3D;
  textureArtifactUrl?: string | null;
  textureMetadata?: Record<string, unknown>;
  textureArtifacts?: TextureArtifactManifest[];
  texturePerFace?: number[];
  wireframe: boolean;
}) {
  const { gl } = useThree();
  const textureEntries = useMemo<TextureArtifactManifest[]>(() => {
    const artifacts = textureArtifacts
      .filter((texture) => texture.artifact_url)
      .slice()
      .sort((left, right) => left.texture_index - right.texture_index);
    if (artifacts.length > 0) {
      return artifacts;
    }
    return textureArtifactUrl
      ? [
          {
            texture_index: 0,
            artifact_url: textureArtifactUrl,
            metadata: textureMetadata ?? {},
          },
        ]
      : [];
  }, [textureArtifactUrl, textureArtifacts, textureMetadata]);
  const textureUrls = textureEntries.map((texture) => texture.artifact_url);
  const textures = useLoader(THREE.TextureLoader, textureUrls);
  const textureArray = useMemo(() => {
    if (!((gl.capabilities as { isWebGL2?: boolean }).isWebGL2)) {
      return null;
    }
    return createMeshLibTextureArray(textures);
  }, [gl.capabilities, textures]);

  useEffect(() => {
    textures.forEach((texture) => {
      texture.minFilter = THREE.LinearFilter;
      texture.magFilter = THREE.LinearFilter;
      texture.wrapS = THREE.ClampToEdgeWrapping;
      texture.wrapT = THREE.ClampToEdgeWrapping;
      texture.needsUpdate = true;
    });

    scene.traverse((child) => {
      if (!(child instanceof THREE.Mesh)) return;
      const usingTextureArrayShader = applyMeshLibTextureArrayShader({
        mesh: child,
        textureArray,
        textureEntries,
        texturePerFace,
        wireframe,
      });
      if (usingTextureArrayShader) {
        return;
      }
      applyMeshLibTexturePerFaceGroups({
        mesh: child,
        textures,
        textureEntries,
        texturePerFace,
      });
    });
  }, [scene, textureArray, textureEntries, textureMetadata, texturePerFace, textures, wireframe]);

  useEffect(() => () => {
    textureArray?.dispose();
    textures.forEach((texture) => texture.dispose());
  }, [textureArray, textures]);

  return null;
}

function ScalarOverlay({
  geometry,
  overlay,
  sectionEnabled,
  sectionConstant,
  sectionAxis,
}: {
  geometry: THREE.BufferGeometry;
  overlay: ScalarOverlayResponse | null;
  sectionEnabled: boolean;
  sectionConstant: number;
  sectionAxis?: [number, number, number] | null;
}) {
  const { gl } = useThree();
  const clippingPlane = useMemo(() => createClippingPlane(sectionAxis), [sectionAxis]);
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
    setClippingPlaneOffset(clippingPlane, sectionConstant, sectionAxis);
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
    if (!sectionEnabled || !contour?.projected_bounds_min || !contour.projected_bounds_max) {
      return null;
    }
    const origin = new THREE.Vector3(...contour.plane_origin);
    const uAxis = new THREE.Vector3(...contour.plane_u_axis);
    const vAxis = new THREE.Vector3(...contour.plane_v_axis);
    const [minU, minV] = contour.projected_bounds_min;
    const [maxU, maxV] = contour.projected_bounds_max;
    const p00 = origin.clone().addScaledVector(uAxis, minU).addScaledVector(vAxis, minV);
    const p10 = origin.clone().addScaledVector(uAxis, maxU).addScaledVector(vAxis, minV);
    const p11 = origin.clone().addScaledVector(uAxis, maxU).addScaledVector(vAxis, maxV);
    const points = [
      p00.x, p00.y, p00.z,
      p10.x, p10.y, p10.z,
      p10.x, p10.y, p10.z,
      p11.x, p11.y, p11.z,
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
  sectionAxis,
}: {
  geometry: THREE.BufferGeometry;
  regionPayload: RegionPayload | null;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  enabled: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  sectionAxis?: [number, number, number] | null;
}) {
  const { gl } = useThree();
  const clippingPlane = useMemo(() => createClippingPlane(sectionAxis), [sectionAxis]);
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
    setClippingPlaneOffset(clippingPlane, sectionConstant, sectionAxis);
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
  sectionAxis,
  sectionContour,
  onRegionPick,
}: {
  normalizedMeshUrl?: string | null;
  regionArtifactUrl?: string | null;
  scalarOverlay: ScalarOverlayResponse | null;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  sectionEnabled: boolean;
  sectionConstant: number;
  sectionAxis?: [number, number, number] | null;
  sectionContour: SectionContourPayload | null;
  onRegionPick?: (regionId: string, additive?: boolean) => void;
}) {
  const geometry = useLoader(PLYLoader, normalizedMeshUrl || '');
  const regionPayload = useJsonPayload<RegionPayload>(regionArtifactUrl);

  useEffect(() => {
    geometry.computeVertexNormals();
    (geometry as THREE.BufferGeometry & { computeBoundsTree?: () => void }).computeBoundsTree?.();
  }, [geometry]);

  return (
    <>
      <RegionPickMesh geometry={geometry} regionPayload={regionPayload} onRegionPick={onRegionPick} />
      <ScalarOverlay
        geometry={geometry}
        overlay={scalarOverlay}
        sectionEnabled={sectionEnabled}
        sectionConstant={sectionConstant}
        sectionAxis={sectionAxis}
      />
      <SectionContourOverlay contour={sectionContour} sectionEnabled={sectionEnabled} />
      <RegionOverlay
        geometry={geometry}
        regionPayload={regionPayload}
        selectedRegionId={selectedRegionId}
        selectedRegionIds={selectedRegionIds}
        enabled={regionOverlayEnabled}
        sectionEnabled={sectionEnabled}
        sectionConstant={sectionConstant}
        sectionAxis={sectionAxis}
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
  sectionAxis,
  sectionContour,
  normalizedMeshUrl,
  regionArtifactUrl,
  regionOverlayEnabled,
  selectedRegionId,
  selectedRegionIds,
  scalarOverlay,
  textureArtifactUrl,
  textureMetadata,
  textureArtifacts,
  texturePerFace,
  onRegionPick,
}: {
  lowUrl: string;
  highUrl?: string | null;
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  sectionAxis?: [number, number, number] | null;
  sectionContour: SectionContourPayload | null;
  normalizedMeshUrl?: string | null;
  regionArtifactUrl?: string | null;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  scalarOverlay: ScalarOverlayResponse | null;
  textureArtifactUrl?: string | null;
  textureMetadata?: Record<string, unknown>;
  textureArtifacts?: TextureArtifactManifest[];
  texturePerFace?: number[];
  onRegionPick?: (regionId: string, additive?: boolean) => void;
}) {
  const [useHigh, setUseHigh] = useState(false);
  const model = useGLTF(useHigh && highUrl ? highUrl : lowUrl);
  const groupRef = useRef<THREE.Group>(null);
  const { gl } = useThree();
  const clippingPlane = useMemo(() => createClippingPlane(sectionAxis), [sectionAxis]);

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
    setClippingPlaneOffset(clippingPlane, sectionConstant, sectionAxis);
  });

  return (
    <group ref={groupRef}>
      {textureArtifactUrl || (textureArtifacts?.length ?? 0) > 0 ? (
        <MeshTextureSync
          scene={model.scene}
          textureArtifactUrl={textureArtifactUrl}
          textureMetadata={textureMetadata}
          textureArtifacts={textureArtifacts}
          texturePerFace={texturePerFace}
          wireframe={wireframe}
        />
      ) : null}
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
          sectionAxis={sectionAxis}
          sectionContour={sectionContour}
          onRegionPick={onRegionPick}
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
  sectionAxis,
  sectionContour,
  normalizedMeshUrl,
  regionArtifactUrl,
  regionOverlayEnabled,
  selectedRegionId,
  selectedRegionIds,
  scalarOverlay,
  textureArtifactUrl,
  textureMetadata,
  textureArtifacts,
  texturePerFace,
  onRegionPick,
}: {
  lowUrl: string;
  highUrl?: string | null;
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  sectionAxis?: [number, number, number] | null;
  sectionContour: SectionContourPayload | null;
  normalizedMeshUrl?: string | null;
  regionArtifactUrl?: string | null;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  scalarOverlay: ScalarOverlayResponse | null;
  textureArtifactUrl?: string | null;
  textureMetadata?: Record<string, unknown>;
  textureArtifacts?: TextureArtifactManifest[];
  texturePerFace?: number[];
  onRegionPick?: (regionId: string, additive?: boolean) => void;
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
            sectionAxis={sectionAxis}
            sectionContour={sectionContour}
            normalizedMeshUrl={normalizedMeshUrl}
            regionArtifactUrl={regionArtifactUrl}
            regionOverlayEnabled={regionOverlayEnabled}
            selectedRegionId={selectedRegionId}
            selectedRegionIds={selectedRegionIds}
            scalarOverlay={scalarOverlay}
            textureArtifactUrl={textureArtifactUrl}
            textureMetadata={textureMetadata}
            textureArtifacts={textureArtifacts}
            texturePerFace={texturePerFace}
            onRegionPick={onRegionPick}
          />
          <FitScene />
        </Bounds>
        <Environment preset="studio" />
        <GizmoHelper alignment="bottom-left" margin={[72, 72]}>
          <GizmoViewport axisColors={['#ef4444', '#22c55e', '#3b82f6']} labelColor="#f8fafc" />
        </GizmoHelper>
      </Suspense>
      <OrbitControls makeDefault />
    </Canvas>
  );
}
