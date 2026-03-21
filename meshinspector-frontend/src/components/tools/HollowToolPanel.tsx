'use client';

/**
 * Hollow tool panel with wall thickness and target weight controls
 */

import { MATERIALS, RING_SIZE_CHART } from '@/lib/constants';
import type { MaterialType } from '@/lib/api/types';

export interface HollowConfig {
    mode: 'fixed' | 'target';
    wallThickness: number;
    targetWeight: number;
    material: MaterialType;
    ringSize: number;
}

interface HollowToolPanelProps {
    config: HollowConfig;
    onChange: (config: HollowConfig) => void;
    onProcess: () => void;
    isProcessing: boolean;
    currentWeight: number;
    // Section overlay quick toggle
    sectionEnabled?: boolean;
    onSectionToggle?: () => void;
}

export default function HollowToolPanel({
    config,
    onChange,
    onProcess,
    isProcessing,
    currentWeight,
    sectionEnabled = false,
    onSectionToggle,
}: HollowToolPanelProps) {
    const updateConfig = (updates: Partial<HollowConfig>) => {
        onChange({ ...config, ...updates });
    };

    const diameter = RING_SIZE_CHART[config.ringSize as keyof typeof RING_SIZE_CHART] ||
        (40 + config.ringSize * 2.55) / Math.PI;

    return (
        <div className="p-4 space-y-6">
            {/* Header */}
            <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-amber-500/20 flex items-center justify-center">
                    <span>🔘</span>
                </div>
                <div>
                    <h3 className="text-lg font-semibold text-white">Hollow</h3>
                    <p className="text-xs text-zinc-500">Create shell for weight reduction</p>
                </div>
            </div>

            {/* Quick Overlays */}
            {onSectionToggle && (
                <div className="flex items-center justify-between p-2 bg-zinc-800/30 rounded-lg border border-zinc-700/50">
                    <div className="flex items-center gap-2">
                        <span className="text-sm">✂️</span>
                        <span className="text-sm text-zinc-300">Show Section</span>
                    </div>
                    <button
                        onClick={onSectionToggle}
                        className={`
                            relative w-10 h-5 rounded-full transition-colors
                            ${sectionEnabled
                                ? 'bg-amber-500'
                                : 'bg-zinc-600'}
                        `}
                    >
                        <span className={`
                            absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform
                            ${sectionEnabled ? 'left-5' : 'left-0.5'}
                        `} />
                    </button>
                </div>
            )}

            {/* Current weight display */}
            {currentWeight > 0 && (
                <div className="p-3 bg-zinc-800/50 rounded-lg flex justify-between items-center">
                    <span className="text-sm text-zinc-400">Current Weight</span>
                    <span className="text-lg font-semibold text-amber-400">{currentWeight.toFixed(2)}g</span>
                </div>
            )}

            {/* Quick Presets */}
            <div>
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Quick Presets
                </label>
                <div className="grid grid-cols-3 gap-2">
                    <button
                        onClick={() => updateConfig({
                            mode: 'fixed',
                            wallThickness: 0.5,
                            material: 'silver_925'
                        })}
                        className={`flex flex-col items-center p-2 rounded-lg transition-colors group ${config.wallThickness === 0.5 && config.mode === 'fixed'
                                ? 'bg-amber-500/20 border border-amber-500/40 ring-1 ring-amber-500/30'
                                : 'bg-zinc-800 hover:bg-zinc-700'
                            }`}
                    >
                        <span className="text-lg mb-1">🪶</span>
                        <span className={`text-xs ${config.wallThickness === 0.5 && config.mode === 'fixed' ? 'text-amber-400' : 'text-zinc-300 group-hover:text-white'}`}>Light</span>
                        <span className="text-[10px] text-zinc-500">0.5mm</span>
                    </button>
                    <button
                        onClick={() => updateConfig({
                            mode: 'fixed',
                            wallThickness: 0.8,
                            material: 'gold_18k'
                        })}
                        className={`flex flex-col items-center p-2 rounded-lg transition-colors group ${config.wallThickness === 0.8 && config.mode === 'fixed'
                                ? 'bg-amber-500/20 border border-amber-500/40 ring-1 ring-amber-500/30'
                                : 'bg-zinc-800 hover:bg-zinc-700'
                            }`}
                    >
                        <span className="text-lg mb-1">💎</span>
                        <span className={`text-xs ${config.wallThickness === 0.8 && config.mode === 'fixed' ? 'text-amber-400' : 'text-zinc-300 group-hover:text-white'}`}>Standard</span>
                        <span className="text-[10px] text-zinc-500">0.8mm</span>
                    </button>
                    <button
                        onClick={() => updateConfig({
                            mode: 'fixed',
                            wallThickness: 1.2,
                            material: 'platinum'
                        })}
                        className={`flex flex-col items-center p-2 rounded-lg transition-colors group ${config.wallThickness === 1.2 && config.mode === 'fixed'
                                ? 'bg-amber-500/20 border border-amber-500/40 ring-1 ring-amber-500/30'
                                : 'bg-zinc-800 hover:bg-zinc-700'
                            }`}
                    >
                        <span className="text-lg mb-1">🏆</span>
                        <span className={`text-xs ${config.wallThickness === 1.2 && config.mode === 'fixed' ? 'text-amber-400' : 'text-zinc-300 group-hover:text-white'}`}>Premium</span>
                        <span className="text-[10px] text-zinc-500">1.2mm</span>
                    </button>
                </div>
                <p className="text-[10px] text-zinc-600 mt-1.5 text-center">
                    Light = Casting • Standard = Daily Wear • Premium = Luxury
                </p>
            </div>

            {/* Ring Size */}
            <div>
                <div className="flex justify-between items-center mb-2">
                    <label className="text-xs text-zinc-500 uppercase tracking-wide">Ring Size</label>
                    <span className="text-xs text-zinc-400">∅ {diameter.toFixed(2)}mm</span>
                </div>
                <input
                    type="range"
                    min="3"
                    max="13"
                    step="0.5"
                    value={config.ringSize}
                    onChange={(e) => updateConfig({ ringSize: parseFloat(e.target.value) })}
                    className="w-full accent-amber-500"
                />
                <div className="flex justify-between text-xs text-zinc-600 mt-1">
                    <span>3</span>
                    <span className="text-amber-400 font-medium">{config.ringSize}</span>
                    <span>13</span>
                </div>
            </div>

            {/* Material */}
            <div>
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Material
                </label>
                <select
                    value={config.material}
                    onChange={(e) => updateConfig({ material: e.target.value as MaterialType })}
                    className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-sm text-white"
                >
                    {Object.entries(MATERIALS).map(([key, { label }]) => (
                        <option key={key} value={key}>{label}</option>
                    ))}
                </select>
            </div>

            {/* Hollowing Mode */}
            <div>
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Hollowing Mode
                </label>
                <div className="flex gap-2">
                    <button
                        onClick={() => updateConfig({ mode: 'fixed' })}
                        className={`
              flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
              ${config.mode === 'fixed'
                                ? 'bg-amber-500 text-black'
                                : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
            `}
                    >
                        Fixed Thickness
                    </button>
                    <button
                        onClick={() => updateConfig({ mode: 'target' })}
                        className={`
              flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
              ${config.mode === 'target'
                                ? 'bg-amber-500 text-black'
                                : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
            `}
                    >
                        Target Weight
                    </button>
                </div>
            </div>

            {/* Wall Thickness (Fixed mode) */}
            {config.mode === 'fixed' && (
                <div>
                    <div className="flex justify-between items-center mb-2">
                        <label className="text-xs text-zinc-500 uppercase tracking-wide">
                            Wall Thickness
                        </label>
                        <span className="text-xs text-zinc-400">{config.wallThickness.toFixed(1)} mm</span>
                    </div>
                    <input
                        type="range"
                        min="0.3"
                        max="3.0"
                        step="0.1"
                        value={config.wallThickness}
                        onChange={(e) => updateConfig({ wallThickness: parseFloat(e.target.value) })}
                        className="w-full accent-amber-500"
                    />
                    <div className="flex justify-between text-xs text-zinc-600 mt-1">
                        <span>0.3mm</span>
                        <span>1.5mm</span>
                        <span>3.0mm</span>
                    </div>
                </div>
            )}

            {/* Target Weight (Target mode) */}
            {config.mode === 'target' && (
                <div>
                    <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                        Target Weight (grams)
                    </label>
                    <input
                        type="number"
                        min="0.5"
                        max="100"
                        step="0.1"
                        value={config.targetWeight}
                        onChange={(e) => updateConfig({ targetWeight: parseFloat(e.target.value) || 0.5 })}
                        className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-sm text-white"
                    />
                    {currentWeight > 0 && (
                        <p className="text-xs text-zinc-500 mt-1">
                            {config.targetWeight < currentWeight
                                ? `↓ ${((1 - config.targetWeight / currentWeight) * 100).toFixed(0)}% weight reduction`
                                : 'Target should be less than current weight'}
                        </p>
                    )}
                </div>
            )}

            {/* Texture Warning */}
            <div className="p-3 bg-zinc-800/50 rounded-lg border border-zinc-700/50">
                <div className="flex items-start gap-2">
                    <span className="text-amber-400 text-sm mt-0.5">⚠️</span>
                    <div>
                        <p className="text-xs text-zinc-300 font-medium">Material Preview</p>
                        <p className="text-xs text-zinc-500 mt-0.5">
                            Processing will replace textures with <span className="text-amber-400">{config.material.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase())}</span> metallic material.
                        </p>
                    </div>
                </div>
            </div>

            {/* Process Button */}
            <button
                onClick={onProcess}
                disabled={isProcessing}
                className={`
          w-full py-3 px-4 rounded-lg font-semibold text-sm transition-all
          ${isProcessing
                        ? 'bg-zinc-700 text-zinc-400 cursor-wait'
                        : 'bg-gradient-to-r from-amber-500 to-amber-600 text-black hover:from-amber-400 hover:to-amber-500'}
        `}
            >
                {isProcessing ? (
                    <span className="flex items-center justify-center gap-2">
                        <div className="w-4 h-4 border-2 border-zinc-400 border-t-transparent rounded-full animate-spin" />
                        {config.mode === 'target' ? 'Optimizing...' : 'Processing...'}
                    </span>
                ) : (
                    'Process Model'
                )}
            </button>

            {/* Help text */}
            {config.mode === 'target' && (
                <div className="p-3 bg-amber-500/10 rounded-lg border border-amber-500/20">
                    <p className="text-xs text-amber-300">
                        <strong>Note:</strong> Adaptive hollowing uses binary search to find optimal wall thickness.
                        This may take 30-60 seconds.
                    </p>
                </div>
            )}
        </div>
    );
}
