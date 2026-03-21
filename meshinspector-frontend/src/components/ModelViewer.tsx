'use client';

/**
 * 3D Model Viewer using React Three Fiber
 * Preserves original materials and textures from GLB/GLTF files
 * Supports clipping planes for cross-section visualization
 */

import { Suspense, useEffect, useState, useRef, useCallback, useMemo } from 'react';
import { Canvas, useThree, useFrame } from '@react-three/fiber';
import {
    OrbitControls,
    Environment,
    useGLTF,
    Bounds,
    useBounds,
    Html,
    GizmoHelper,
    GizmoViewport
} from '@react-three/drei';
import * as THREE from 'three';
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib';

export interface ClippingConfig {
    enabled: boolean;
    normal: [number, number, number];
    constant: number;
    showHelper: boolean;
}

interface ModelViewerProps {
    modelUrl: string;
    wireframe?: boolean;
    clipping?: ClippingConfig;
    onResetCamera?: () => void;
    showModelInfo?: boolean;
    onModelInfoToggle?: () => void;
    analysis?: {
        volume_mm3?: number;
        weight_grams?: number;
        is_watertight?: boolean;
    };
}

function LoadingSpinner() {
    return (
        <Html center>
            <div className="flex flex-col items-center gap-4">
                <div className="w-12 h-12 border-3 border-amber-500 border-t-transparent rounded-full animate-spin" />
                <p className="text-zinc-400">Loading model...</p>
            </div>
        </Html>
    );
}

// Dimension axes with labels
function DimensionAxes({ size }: { size: THREE.Vector3 }) {
    const maxDim = Math.max(size.x, size.y, size.z);
    const axisLength = maxDim * 0.5;

    return (
        <group position={[-maxDim * 0.5, -maxDim * 0.4, -maxDim * 0.5]}>
            {/* X Axis - Red */}
            <arrowHelper args={[
                new THREE.Vector3(1, 0, 0),
                new THREE.Vector3(0, 0, 0),
                axisLength,
                0xff4444,
                axisLength * 0.12,
                axisLength * 0.06
            ]} />
            <Html position={[axisLength * 1.15, 0, 0]} center>
                <div className="px-2 py-1 rounded-lg bg-red-500/90 text-white text-xs font-bold whitespace-nowrap shadow-lg">
                    X: {size.x.toFixed(1)}mm
                </div>
            </Html>

            {/* Y Axis - Green */}
            <arrowHelper args={[
                new THREE.Vector3(0, 1, 0),
                new THREE.Vector3(0, 0, 0),
                axisLength,
                0x44ff44,
                axisLength * 0.12,
                axisLength * 0.06
            ]} />
            <Html position={[0, axisLength * 1.15, 0]} center>
                <div className="px-2 py-1 rounded-lg bg-green-500/90 text-white text-xs font-bold whitespace-nowrap shadow-lg">
                    Y: {size.y.toFixed(1)}mm
                </div>
            </Html>

            {/* Z Axis - Blue */}
            <arrowHelper args={[
                new THREE.Vector3(0, 0, 1),
                new THREE.Vector3(0, 0, 0),
                axisLength,
                0x4444ff,
                axisLength * 0.12,
                axisLength * 0.06
            ]} />
            <Html position={[0, 0, axisLength * 1.15]} center>
                <div className="px-2 py-1 rounded-lg bg-blue-500/90 text-white text-xs font-bold whitespace-nowrap shadow-lg">
                    Z: {size.z.toFixed(1)}mm
                </div>
            </Html>
        </group>
    );
}

// Component that fits camera to bounds on load
function FitToView() {
    const bounds = useBounds();
    useEffect(() => {
        bounds.refresh().clip().fit();
    }, [bounds]);
    return null;
}

// Clipping plane helper visualization - Rhino3D/Blender style
function ClippingPlaneHelper({ clipping }: { clipping: ClippingConfig }) {
    const groupRef = useRef<THREE.Group>(null);

    // Calculate position and rotation
    useEffect(() => {
        if (!groupRef.current || !clipping.enabled || !clipping.showHelper) return;

        const normal = new THREE.Vector3(
            clipping.normal[0],
            clipping.normal[1],
            clipping.normal[2]
        ).normalize();

        // Position: move along the normal by -constant (plane equation is n·p + d = 0)
        groupRef.current.position.copy(normal.clone().multiplyScalar(-clipping.constant));

        // Rotation: align plane's +Z axis with the normal
        const quaternion = new THREE.Quaternion();
        quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), normal);
        groupRef.current.quaternion.copy(quaternion);
    }, [clipping.enabled, clipping.showHelper, clipping.normal[0], clipping.normal[1], clipping.normal[2], clipping.constant]);

    if (!clipping.showHelper || !clipping.enabled) return null;

    const planeSize = 150;
    const gridDivisions = 30;

    return (
        <group ref={groupRef}>
            {/* Grid lines - rotated to align with plane */}
            <gridHelper
                args={[planeSize, gridDivisions, '#3b82f6', '#1e40af']}
                rotation={[Math.PI / 2, 0, 0]}
            />

            {/* Subtle transparent fill */}
            <mesh>
                <planeGeometry args={[planeSize, planeSize]} />
                <meshBasicMaterial
                    color="#3b82f6"
                    transparent
                    opacity={0.06}
                    side={THREE.DoubleSide}
                    depthWrite={false}
                />
            </mesh>

            {/* Edge glow ring */}
            <mesh>
                <ringGeometry args={[planeSize * 0.48, planeSize * 0.5, 64]} />
                <meshBasicMaterial
                    color="#60a5fa"
                    transparent
                    opacity={0.5}
                    side={THREE.DoubleSide}
                    depthWrite={false}
                />
            </mesh>

            {/* Center cross indicator */}
            <mesh>
                <ringGeometry args={[0.3, 0.5, 32]} />
                <meshBasicMaterial
                    color="#93c5fd"
                    transparent
                    opacity={0.8}
                    side={THREE.DoubleSide}
                />
            </mesh>
        </group>
    );
}

function Model({ url, wireframe = false, onDimensionsChange, clipping }: {
    url: string;
    wireframe?: boolean;
    onDimensionsChange?: (size: THREE.Vector3) => void;
    clipping?: ClippingConfig;
}) {
    const { scene, materials } = useGLTF(url);
    const groupRef = useRef<THREE.Group>(null);
    const { gl } = useThree();

    // Create a persistent clipping plane ref
    const clippingPlaneRef = useRef<THREE.Plane>(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0));

    // Track if clipping planes have been applied to materials
    const clippingAppliedRef = useRef(false);

    // Enable clipping on renderer
    useEffect(() => {
        gl.localClippingEnabled = clipping?.enabled ?? false;
    }, [gl, clipping?.enabled]);

    // Apply clipping planes to materials when enabled changes
    useEffect(() => {
        const clippingPlanes = clipping?.enabled ? [clippingPlaneRef.current] : [];
        clippingAppliedRef.current = false;

        if (materials) {
            Object.values(materials).forEach((material) => {
                if (material instanceof THREE.MeshStandardMaterial ||
                    material instanceof THREE.MeshPhysicalMaterial ||
                    material instanceof THREE.MeshBasicMaterial) {
                    material.wireframe = wireframe;
                    material.clippingPlanes = clippingPlanes;
                    material.clipShadows = true;
                    material.needsUpdate = true;
                }
            });
        }

        scene.traverse((child) => {
            if (child instanceof THREE.Mesh && child.material) {
                const applyToMaterial = (m: THREE.Material) => {
                    if (m instanceof THREE.MeshStandardMaterial ||
                        m instanceof THREE.MeshPhysicalMaterial ||
                        m instanceof THREE.MeshBasicMaterial) {
                        m.wireframe = wireframe;
                        m.clippingPlanes = clippingPlanes;
                        m.clipShadows = true;
                        m.needsUpdate = true;
                    }
                };

                if (Array.isArray(child.material)) {
                    child.material.forEach(applyToMaterial);
                } else {
                    applyToMaterial(child.material);
                }
                child.castShadow = true;
                child.receiveShadow = true;
            }
        });

        clippingAppliedRef.current = clipping?.enabled ?? false;
    }, [scene, materials, wireframe, clipping?.enabled]);

    // Update clipping plane values reactively on each frame
    useFrame(() => {
        if (clipping?.enabled) {
            const normal = new THREE.Vector3(
                clipping.normal[0],
                clipping.normal[1],
                clipping.normal[2]
            ).normalize();
            clippingPlaneRef.current.normal.copy(normal);
            clippingPlaneRef.current.constant = clipping.constant;
        }
    });

    // Calculate dimensions and report
    useEffect(() => {
        const box = new THREE.Box3().setFromObject(scene);
        const originalSize = box.getSize(new THREE.Vector3());
        const maxDim = Math.max(originalSize.x, originalSize.y, originalSize.z);

        let scale = 1;
        let unitMultiplier = 1;

        if (maxDim < 0.1) {
            scale = 1000;
            unitMultiplier = 1000;
        } else if (maxDim < 10) {
            scale = 10;
            unitMultiplier = 10;
        } else if (maxDim > 1000) {
            scale = 0.001;
            unitMultiplier = 0.001;
        }

        if (groupRef.current) {
            groupRef.current.scale.setScalar(scale);

            const dimensionsInMM = new THREE.Vector3(
                originalSize.x * unitMultiplier,
                originalSize.y * unitMultiplier,
                originalSize.z * unitMultiplier
            );

            if (onDimensionsChange) {
                onDimensionsChange(dimensionsInMM);
            }
        }
    }, [scene, onDimensionsChange]);

    return (
        <group ref={groupRef}>
            <primitive object={scene} />
        </group>
    );
}

function CameraController({ controlsRef }: { controlsRef: React.RefObject<OrbitControlsImpl | null> }) {
    return (
        <OrbitControls
            ref={controlsRef}
            makeDefault
            enablePan={true}
            enableZoom={true}
            enableRotate={true}
            minDistance={0.001}
            maxDistance={100000}
        />
    );
}

function ZoomControls({ onZoomIn, onZoomOut, onReset, onInfoToggle, showInfo }: {
    onZoomIn: () => void;
    onZoomOut: () => void;
    onReset: () => void;
    onInfoToggle?: () => void;
    showInfo?: boolean;
}) {
    return (
        <div className="absolute bottom-6 left-6 z-20 flex flex-col gap-2">
            <div className="bg-zinc-900/90 backdrop-blur-sm border border-zinc-700 rounded-lg p-1.5 flex flex-col gap-1">
                <button
                    onClick={onZoomIn}
                    className="w-9 h-9 rounded-md bg-zinc-800 hover:bg-amber-500/30 
                               flex items-center justify-center text-white text-xl 
                               transition-all hover:scale-105 active:scale-95"
                    title="Zoom In"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="11" cy="11" r="8" />
                        <line x1="21" y1="21" x2="16.65" y2="16.65" />
                        <line x1="11" y1="8" x2="11" y2="14" />
                        <line x1="8" y1="11" x2="14" y2="11" />
                    </svg>
                </button>
                <button
                    onClick={onZoomOut}
                    className="w-9 h-9 rounded-md bg-zinc-800 hover:bg-amber-500/30 
                               flex items-center justify-center text-white text-xl 
                               transition-all hover:scale-105 active:scale-95"
                    title="Zoom Out"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="11" cy="11" r="8" />
                        <line x1="21" y1="21" x2="16.65" y2="16.65" />
                        <line x1="8" y1="11" x2="14" y2="11" />
                    </svg>
                </button>
                <div className="w-full h-px bg-zinc-700" />
                <button
                    onClick={onReset}
                    className="w-9 h-9 rounded-md bg-zinc-800 hover:bg-amber-500/30 
                               flex items-center justify-center text-white text-sm font-medium
                               transition-all hover:scale-105 active:scale-95"
                    title="Reset View"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                        <path d="M3 3v5h5" />
                    </svg>
                </button>
                {onInfoToggle && (
                    <>
                        <div className="w-full h-px bg-zinc-700" />
                        <button
                            onClick={onInfoToggle}
                            className={`w-9 h-9 rounded-md 
                                       flex items-center justify-center text-lg
                                       transition-all hover:scale-105 active:scale-95
                                       ${showInfo ? 'bg-amber-500/30 text-amber-400' : 'bg-zinc-800 hover:bg-amber-500/30 text-white'}`}
                            title="Model Info"
                        >
                            ℹ️
                        </button>
                    </>
                )}
            </div>
        </div>
    );
}

function Scene({ modelUrl, wireframe, onDimensionsChange, controlsRef, clipping }: {
    modelUrl: string;
    wireframe: boolean;
    onDimensionsChange: (size: THREE.Vector3) => void;
    controlsRef: React.RefObject<OrbitControlsImpl | null>;
    clipping?: ClippingConfig;
}) {
    return (
        <>
            {/* Lighting setup for PBR materials */}
            <ambientLight intensity={0.5} />
            <directionalLight position={[10, 20, 15]} intensity={1.2} castShadow />
            <directionalLight position={[-10, 10, -10]} intensity={0.6} />
            <hemisphereLight intensity={0.4} groundColor="#1a1a1a" />

            {/* Environment for realistic reflections on PBR materials */}
            <Environment preset="apartment" background={false} />

            {/* Model with auto-fitting bounds */}
            <Suspense fallback={<LoadingSpinner />}>
                <Bounds fit clip observe margin={1.5}>
                    <Model
                        url={modelUrl}
                        wireframe={wireframe}
                        onDimensionsChange={onDimensionsChange}
                        clipping={clipping}
                    />
                    <FitToView />
                </Bounds>
            </Suspense>

            {/* Clipping plane visualization */}
            {clipping && <ClippingPlaneHelper clipping={clipping} />}

            {/* Navigation gizmo */}
            <GizmoHelper alignment="bottom-right" margin={[80, 80]}>
                <GizmoViewport
                    axisColors={['#ff4444', '#44ff44', '#4444ff']}
                    labelColor="white"
                />
            </GizmoHelper>

            {/* Camera controls */}
            <CameraController controlsRef={controlsRef} />

            {/* Floor grid */}
            <gridHelper
                args={[100, 40, '#333333', '#222222']}
                position={[0, -0.5, 0]}
            />
        </>
    );
}

export default function ModelViewer({
    modelUrl,
    wireframe = false,
    clipping,
    onResetCamera,
    showModelInfo = false,
    onModelInfoToggle,
    analysis
}: ModelViewerProps) {
    const [dimensions, setDimensions] = useState<THREE.Vector3 | null>(null);
    const controlsRef = useRef<OrbitControlsImpl | null>(null);

    const handleZoomIn = useCallback(() => {
        if (controlsRef.current) {
            const controls = controlsRef.current;
            const camera = controls.object as THREE.PerspectiveCamera;
            const direction = new THREE.Vector3();
            camera.getWorldDirection(direction);
            camera.position.addScaledVector(direction, camera.position.length() * 0.2);
            controls.update();
        }
    }, []);

    const handleZoomOut = useCallback(() => {
        if (controlsRef.current) {
            const controls = controlsRef.current;
            const camera = controls.object as THREE.PerspectiveCamera;
            const direction = new THREE.Vector3();
            camera.getWorldDirection(direction);
            camera.position.addScaledVector(direction, -camera.position.length() * 0.2);
            controls.update();
        }
    }, []);

    const handleReset = useCallback(() => {
        if (controlsRef.current) {
            controlsRef.current.reset();
        }
        onResetCamera?.();
    }, [onResetCamera]);

    return (
        <div className="w-full h-full overflow-hidden relative">
            <Canvas
                camera={{ position: [5, 5, 10], fov: 45, near: 0.0001, far: 100000 }}
                gl={{
                    preserveDrawingBuffer: true,
                    antialias: true,
                    toneMapping: THREE.ACESFilmicToneMapping,
                    toneMappingExposure: 1.0,
                    localClippingEnabled: true,
                }}
                shadows
                style={{ background: 'transparent' }}
            >
                <Scene
                    modelUrl={modelUrl}
                    wireframe={wireframe}
                    onDimensionsChange={setDimensions}
                    controlsRef={controlsRef}
                    clipping={clipping}
                />
                {dimensions && <DimensionAxes size={dimensions} />}
            </Canvas>

            {/* Zoom Controls UI with Info button */}
            <ZoomControls
                onZoomIn={handleZoomIn}
                onZoomOut={handleZoomOut}
                onReset={handleReset}
                onInfoToggle={onModelInfoToggle}
                showInfo={showModelInfo}
            />

            {/* Model Info Popup */}
            {showModelInfo && analysis && (
                <div className="absolute bottom-6 left-20 z-20 bg-zinc-900/95 backdrop-blur-sm border border-zinc-700 rounded-lg p-4 min-w-48">
                    <div className="flex items-center justify-between mb-3">
                        <h4 className="text-sm font-semibold text-white">Model Info</h4>
                        <button
                            onClick={onModelInfoToggle}
                            className="text-zinc-500 hover:text-white text-lg leading-none"
                        >
                            ×
                        </button>
                    </div>
                    <div className="space-y-2">
                        {analysis.weight_grams !== undefined && (
                            <div className="flex justify-between text-sm">
                                <span className="text-zinc-400">Weight</span>
                                <span className="text-amber-400 font-medium">{analysis.weight_grams.toFixed(2)}g</span>
                            </div>
                        )}
                        {analysis.volume_mm3 !== undefined && (
                            <div className="flex justify-between text-sm">
                                <span className="text-zinc-400">Volume</span>
                                <span className="text-white">{analysis.volume_mm3.toFixed(1)} mm³</span>
                            </div>
                        )}
                        {analysis.is_watertight !== undefined && (
                            <div className="flex justify-between text-sm">
                                <span className="text-zinc-400">Watertight</span>
                                <span className={analysis.is_watertight ? 'text-green-400' : 'text-red-400'}>
                                    {analysis.is_watertight ? '✓ Yes' : '✗ No'}
                                </span>
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}

