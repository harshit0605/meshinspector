'use client';

/**
 * Control panel for ring sizing, hollowing, and weight control using Leva
 */

import { useControls, button, folder, buttonGroup } from 'leva';
import { useEffect, useState } from 'react';
import { MATERIALS, RING_SIZE_CHART } from '@/lib/constants';
import type { MaterialType } from '@/lib/api/types';

export type HollowingMode = 'fixed' | 'target';

export interface ControlValues {
    ringSize: number;
    material: MaterialType;
    wallThickness: number;
    wireframe: boolean;
    hollowingMode: HollowingMode;
    targetWeight: number;
}

interface ViewerControlsProps {
    onProcess: (values: ControlValues) => void;
    onDownload: (format: 'glb' | 'stl') => void;
    onWireframeChange?: (wireframe: boolean) => void;
    isProcessing?: boolean;
    disabled?: boolean;
    currentWeight?: number; // Current model weight for reference
}

export default function ViewerControls({
    onProcess,
    onDownload,
    onWireframeChange,
    isProcessing = false,
    disabled = false,
    currentWeight = 0,
}: ViewerControlsProps) {
    const [hollowingMode, setHollowingMode] = useState<HollowingMode>('fixed');

    // Material options for select
    const materialOptions = Object.entries(MATERIALS).reduce(
        (acc, [key, { label }]) => ({ ...acc, [label]: key }),
        {} as Record<string, string>
    );

    const [values, set] = useControls(() => ({
        'Ring Properties': folder({
            ringSize: {
                value: 7,
                min: 3,
                max: 13,
                step: 0.5,
                label: 'Ring Size',
            },
            diameter: {
                value: `${RING_SIZE_CHART[7]}mm`,
                editable: false,
                label: 'Inner Diameter',
            },
        }),

        Material: folder({
            material: {
                value: 'gold_18k',
                options: materialOptions,
                label: 'Material',
            },
        }),

        Hollowing: folder({
            hollowingModeSelect: buttonGroup({
                label: 'Mode',
                opts: {
                    'Fixed': () => setHollowingMode('fixed'),
                    'Target Wt': () => setHollowingMode('target'),
                },
            }),
            wallThickness: {
                value: 0.8,
                min: 0.3,
                max: 3.0,
                step: 0.1,
                label: 'Wall Thickness (mm)',
                hint: 'Fixed wall thickness for hollowing',
            },
            targetWeight: {
                value: currentWeight > 0 ? Math.round(currentWeight * 0.5 * 10) / 10 : 5.0,
                min: 0.5,
                max: 100,
                step: 0.1,
                label: 'Target Weight (g)',
                hint: 'Desired final weight after hollowing',
            },
            currentWeightDisplay: {
                value: currentWeight > 0 ? `${currentWeight.toFixed(2)}g` : 'N/A',
                editable: false,
                label: 'Current Weight',
            },
        }),

        Display: folder({
            wireframe: {
                value: false,
                label: 'Wireframe Mode',
            },
        }),

        'Process Model': button(
            () => {
                onProcess({
                    ringSize: values.ringSize,
                    material: values.material as MaterialType,
                    wallThickness: values.wallThickness,
                    wireframe: values.wireframe,
                    hollowingMode: hollowingMode,
                    targetWeight: values.targetWeight,
                });
            },
            { disabled: isProcessing || disabled }
        ),

        'Export': folder({
            'Download GLB': button(() => onDownload('glb'), { disabled }),
            'Download STL': button(() => onDownload('stl'), { disabled }),
        }),
    }), [isProcessing, disabled, hollowingMode, currentWeight]);

    // Update diameter display when ring size changes
    useEffect(() => {
        const size = values.ringSize;
        const diameter = RING_SIZE_CHART[size as keyof typeof RING_SIZE_CHART] ||
            (40 + size * 2.55) / Math.PI;
        set({ diameter: `${diameter.toFixed(2)}mm` });
    }, [values.ringSize, set]);

    // Update current weight display when prop changes
    useEffect(() => {
        if (currentWeight > 0) {
            set({ currentWeightDisplay: `${currentWeight.toFixed(2)}g` });
        }
    }, [currentWeight, set]);

    // Notify parent about wireframe changes in real-time
    useEffect(() => {
        if (onWireframeChange) {
            onWireframeChange(values.wireframe);
        }
    }, [values.wireframe, onWireframeChange]);

    // Visual indicator for current mode
    return (
        <div className="pointer-events-none fixed top-4 left-1/2 -translate-x-1/2 z-50">
            {isProcessing && (
                <div className="glass-card px-4 py-2 flex items-center gap-3 pointer-events-auto">
                    <div className="w-4 h-4 border-2 border-amber-500 border-t-transparent rounded-full animate-spin" />
                    <span className="text-white text-sm font-medium">
                        {hollowingMode === 'target'
                            ? 'Adaptive hollowing... (may take up to 60s)'
                            : 'Processing model...'}
                    </span>
                </div>
            )}
        </div>
    );
}
