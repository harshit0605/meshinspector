'use client';

/**
 * Section/Clipping tool panel with plane controls
 */

import { useState } from 'react';

export interface SectionConfig {
    enabled: boolean;
    planeType: 'YZ' | 'XZ' | 'XY' | 'custom';
    normal: [number, number, number];
    constant: number; // Distance from origin
    flip: boolean;
    showHelper: boolean;
}

interface SectionToolPanelProps {
    config: SectionConfig;
    onChange: (config: SectionConfig) => void;
}

const PLANE_PRESETS: Record<'YZ' | 'XZ' | 'XY', [number, number, number]> = {
    'YZ': [1, 0, 0],  // X-axis normal (cuts along YZ plane)
    'XZ': [0, 1, 0],  // Y-axis normal (cuts along XZ plane)
    'XY': [0, 0, 1],  // Z-axis normal (cuts along XY plane)
};

export default function SectionToolPanel({ config, onChange }: SectionToolPanelProps) {
    const updateConfig = (updates: Partial<SectionConfig>) => {
        onChange({ ...config, ...updates });
    };

    const setPlanePreset = (planeType: 'YZ' | 'XZ' | 'XY') => {
        const normal = PLANE_PRESETS[planeType];
        updateConfig({
            planeType,
            normal: config.flip ? [-normal[0], -normal[1], -normal[2]] : normal,
        });
    };

    const toggleFlip = () => {
        const newFlip = !config.flip;
        updateConfig({
            flip: newFlip,
            normal: [-config.normal[0], -config.normal[1], -config.normal[2]],
        });
    };

    return (
        <div className="p-4 space-y-6">
            {/* Header */}
            <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center">
                    <span>✂️</span>
                </div>
                <div>
                    <h3 className="text-lg font-semibold text-white">Section</h3>
                    <p className="text-xs text-zinc-500">Cross-section clipping</p>
                </div>
            </div>

            {/* Enable toggle */}
            <div className="flex items-center justify-between p-3 bg-zinc-800/50 rounded-lg">
                <span className="text-sm text-zinc-300">Clip Objects</span>
                <button
                    onClick={() => updateConfig({ enabled: !config.enabled })}
                    className={`
            w-12 h-6 rounded-full transition-colors relative
            ${config.enabled ? 'bg-amber-500' : 'bg-zinc-700'}
          `}
                >
                    <div className={`
            w-5 h-5 rounded-full bg-white absolute top-0.5 transition-transform
            ${config.enabled ? 'translate-x-6' : 'translate-x-0.5'}
          `} />
                </button>
            </div>

            {/* Plane presets */}
            <div>
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Preset Planes
                </label>
                <div className="flex gap-2">
                    {(['YZ', 'XZ', 'XY'] as const).map((plane) => (
                        <button
                            key={plane}
                            onClick={() => setPlanePreset(plane)}
                            className={`
                flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
                ${config.planeType === plane
                                    ? 'bg-amber-500 text-black'
                                    : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
              `}
                        >
                            Plane {plane}
                        </button>
                    ))}
                </div>
            </div>

            {/* Normal direction */}
            <div>
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Normal Direction
                </label>
                <div className="grid grid-cols-3 gap-2">
                    {(['X', 'Y', 'Z'] as const).map((axis, idx) => (
                        <div key={axis}>
                            <label className="text-xs text-zinc-400 mb-1 block">{axis}</label>
                            <input
                                type="number"
                                value={config.normal[idx]}
                                onChange={(e) => {
                                    const newNormal = [...config.normal] as [number, number, number];
                                    newNormal[idx] = parseFloat(e.target.value) || 0;
                                    updateConfig({ normal: newNormal, planeType: 'custom' });
                                }}
                                step="0.1"
                                className="w-full px-2 py-1.5 bg-zinc-800 border border-zinc-700 rounded text-sm text-white text-center"
                            />
                        </div>
                    ))}
                </div>
            </div>

            {/* Shift (plane position) */}
            <div>
                <div className="flex justify-between items-center mb-2">
                    <label className="text-xs text-zinc-500 uppercase tracking-wide">
                        Shift
                    </label>
                    <input
                        type="number"
                        value={config.constant}
                        onChange={(e) => updateConfig({ constant: parseFloat(e.target.value) || 0 })}
                        step="0.1"
                        className="w-20 px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-sm text-amber-400 text-right font-medium"
                    />
                </div>
                <input
                    type="range"
                    min="-100"
                    max="100"
                    step="0.01"
                    value={config.constant}
                    onChange={(e) => updateConfig({ constant: parseFloat(e.target.value) })}
                    className="w-full accent-amber-500 h-2 bg-zinc-700 rounded-lg appearance-none cursor-pointer"
                />
                <div className="flex justify-between text-xs text-zinc-600 mt-1">
                    <span>-100</span>
                    <span>0</span>
                    <span>+100</span>
                </div>
            </div>

            {/* Flip button */}
            <div className="flex gap-2">
                <button
                    onClick={toggleFlip}
                    className={`
            flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
            ${config.flip
                            ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30'
                            : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
          `}
                >
                    🔄 Flip
                </button>
                <button
                    onClick={() => updateConfig({ showHelper: !config.showHelper })}
                    className={`
            flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
            ${config.showHelper
                            ? 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                            : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
          `}
                >
                    👁 Show Plane
                </button>
            </div>

            {/* Quick actions */}
            <div className="pt-4 border-t border-zinc-800">
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Quick Actions
                </label>
                <div className="space-y-2">
                    <button
                        onClick={() => updateConfig({ constant: 0 })}
                        className="w-full py-2 px-3 bg-zinc-800 text-zinc-300 rounded-lg text-sm hover:bg-zinc-700 transition-colors"
                    >
                        Reset Position
                    </button>
                </div>
            </div>

            {/* Help text */}
            <div className="p-3 bg-zinc-800/30 rounded-lg border border-zinc-800">
                <p className="text-xs text-zinc-500">
                    <strong className="text-zinc-400">Tip:</strong> Use Shift slider to move the cutting plane through the model.
                    Enable "Show Plane" to see the clipping plane helper.
                </p>
            </div>
        </div>
    );
}
