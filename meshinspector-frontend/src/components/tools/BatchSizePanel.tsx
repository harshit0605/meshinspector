'use client';

/**
 * Batch Size Generation Panel - generate multiple ring sizes at once
 */

import { useState } from 'react';
import { RING_SIZE_CHART } from '@/lib/constants';

interface BatchSizeConfig {
    sizes: number[];
    material: string;
    wallThickness: number;
}

interface BatchSizePanelProps {
    onGenerate: (config: BatchSizeConfig) => void;
    isProcessing: boolean;
    currentModelId: string | null;
}

// Standard US ring size ranges
const SIZE_PRESETS = {
    women: [4, 4.5, 5, 5.5, 6, 6.5, 7, 7.5, 8],
    men: [8, 8.5, 9, 9.5, 10, 10.5, 11, 11.5, 12],
    all: [4, 5, 6, 7, 8, 9, 10, 11, 12],
};

export default function BatchSizePanel({
    onGenerate,
    isProcessing,
    currentModelId,
}: BatchSizePanelProps) {
    const [selectedSizes, setSelectedSizes] = useState<Set<number>>(new Set([6, 7, 8]));
    const [activePreset, setActivePreset] = useState<'women' | 'men' | 'all' | 'custom'>('custom');
    const [wallThickness, setWallThickness] = useState(0.8);
    const [material, setMaterial] = useState('gold_18k');

    const toggleSize = (size: number) => {
        const newSizes = new Set(selectedSizes);
        if (newSizes.has(size)) {
            newSizes.delete(size);
        } else {
            newSizes.add(size);
        }
        setSelectedSizes(newSizes);
        setActivePreset('custom');
    };

    const applyPreset = (preset: 'women' | 'men' | 'all') => {
        setSelectedSizes(new Set(SIZE_PRESETS[preset]));
        setActivePreset(preset);
    };

    const handleGenerate = () => {
        onGenerate({
            sizes: Array.from(selectedSizes).sort((a, b) => a - b),
            material,
            wallThickness,
        });
    };

    const allSizes = [3, 3.5, 4, 4.5, 5, 5.5, 6, 6.5, 7, 7.5, 8, 8.5, 9, 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13];

    return (
        <div className="p-4 space-y-5">
            {/* Header */}
            <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-purple-500/20 flex items-center justify-center">
                    <span>📦</span>
                </div>
                <div>
                    <h3 className="text-lg font-semibold text-white">Batch Sizes</h3>
                    <p className="text-xs text-zinc-500">Generate multiple ring sizes</p>
                </div>
            </div>

            {/* Quick Presets */}
            <div>
                <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                    Size Presets
                </label>
                <div className="flex gap-2">
                    <button
                        onClick={() => applyPreset('women')}
                        className={`
                            flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
                            ${activePreset === 'women'
                                ? 'bg-pink-500/20 text-pink-400 border border-pink-500/30'
                                : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
                        `}
                    >
                        👩 Women
                    </button>
                    <button
                        onClick={() => applyPreset('men')}
                        className={`
                            flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
                            ${activePreset === 'men'
                                ? 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                                : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
                        `}
                    >
                        👨 Men
                    </button>
                    <button
                        onClick={() => applyPreset('all')}
                        className={`
                            flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors
                            ${activePreset === 'all'
                                ? 'bg-purple-500/20 text-purple-400 border border-purple-500/30'
                                : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'}
                        `}
                    >
                        📊 All
                    </button>
                </div>
            </div>

            {/* Size Grid */}
            <div>
                <div className="flex justify-between items-center mb-2">
                    <label className="text-xs text-zinc-500 uppercase tracking-wide">
                        Select Sizes
                    </label>
                    <span className="text-xs text-amber-400">{selectedSizes.size} selected</span>
                </div>
                <div className="grid grid-cols-7 gap-1.5">
                    {allSizes.map((size) => {
                        const isSelected = selectedSizes.has(size);
                        const diameter = RING_SIZE_CHART[size as keyof typeof RING_SIZE_CHART] ||
                            (40 + size * 2.55) / Math.PI;
                        return (
                            <button
                                key={size}
                                onClick={() => toggleSize(size)}
                                title={`∅${diameter.toFixed(1)}mm`}
                                className={`
                                    aspect-square rounded-lg text-xs font-medium transition-all
                                    ${isSelected
                                        ? 'bg-amber-500 text-black scale-105'
                                        : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-white'}
                                `}
                            >
                                {size}
                            </button>
                        );
                    })}
                </div>
            </div>

            {/* Settings */}
            <div className="space-y-3">
                <div>
                    <label className="text-xs text-zinc-500 uppercase tracking-wide mb-1.5 block">
                        Wall Thickness
                    </label>
                    <div className="flex items-center gap-3">
                        <input
                            type="range"
                            min="0.3"
                            max="2.0"
                            step="0.1"
                            value={wallThickness}
                            onChange={(e) => setWallThickness(parseFloat(e.target.value))}
                            className="flex-1 accent-amber-500"
                        />
                        <span className="text-sm text-amber-400 w-14 text-right">{wallThickness}mm</span>
                    </div>
                </div>

                <div>
                    <label className="text-xs text-zinc-500 uppercase tracking-wide mb-1.5 block">
                        Material
                    </label>
                    <select
                        value={material}
                        onChange={(e) => setMaterial(e.target.value)}
                        className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-sm text-white"
                    >
                        <option value="gold_18k">18K Gold</option>
                        <option value="gold_14k">14K Gold</option>
                        <option value="silver_925">925 Silver</option>
                        <option value="platinum">Platinum</option>
                    </select>
                </div>
            </div>

            {/* Output Preview */}
            {selectedSizes.size > 0 && (
                <div className="p-3 bg-zinc-800/30 rounded-lg border border-zinc-700/50">
                    <p className="text-xs text-zinc-500 mb-1">Output Files:</p>
                    <div className="flex flex-wrap gap-1">
                        {Array.from(selectedSizes).sort((a, b) => a - b).slice(0, 5).map((size) => (
                            <span key={size} className="px-2 py-0.5 bg-zinc-700/50 rounded text-xs text-zinc-300">
                                ring_size_{size}.glb
                            </span>
                        ))}
                        {selectedSizes.size > 5 && (
                            <span className="px-2 py-0.5 bg-zinc-700/50 rounded text-xs text-zinc-400">
                                +{selectedSizes.size - 5} more
                            </span>
                        )}
                    </div>
                </div>
            )}

            {/* Generate Button */}
            <button
                onClick={handleGenerate}
                disabled={isProcessing || selectedSizes.size === 0 || !currentModelId}
                className={`
                    w-full py-3 px-4 rounded-xl text-sm font-semibold
                    transition-all duration-200 flex items-center justify-center gap-2
                    ${isProcessing || selectedSizes.size === 0 || !currentModelId
                        ? 'bg-zinc-700 text-zinc-400 cursor-not-allowed'
                        : 'bg-gradient-to-r from-purple-500 to-purple-600 text-white hover:from-purple-400 hover:to-purple-500 hover:scale-[1.02] active:scale-[0.98]'
                    }
                `}
            >
                {isProcessing ? (
                    <>
                        <div className="w-4 h-4 border-2 border-zinc-500 border-t-transparent rounded-full animate-spin" />
                        Generating...
                    </>
                ) : (
                    <>
                        📦 Generate {selectedSizes.size} Size{selectedSizes.size !== 1 ? 's' : ''}
                    </>
                )}
            </button>

            {/* Info */}
            <div className="p-3 bg-purple-500/10 rounded-lg border border-purple-500/20">
                <p className="text-xs text-purple-300">
                    <strong>Note:</strong> Each size will be scaled and hollowed with the specified settings.
                    Files will be downloaded as a ZIP archive.
                </p>
            </div>
        </div>
    );
}
