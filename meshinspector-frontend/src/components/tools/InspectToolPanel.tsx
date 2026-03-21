'use client';

/**
 * Inspect Tool Panel - combines mesh health and thickness analysis
 */

import { useState } from 'react';
import HealthPanel from './HealthPanel';

interface ThicknessData {
    min_thickness: number;
    max_thickness: number;
    avg_thickness: number;
    violation_count: number;
}

interface HealthData {
    is_closed: boolean;
    self_intersections: number;
    holes_count: number;
    health_score: number;
}

interface InspectToolPanelProps {
    healthData: HealthData | null;
    thicknessData: ThicknessData | null;
    isLoadingHealth: boolean;
    isLoadingThickness: boolean;
    onRepair: () => void;
    isRepairing: boolean;
    heatmapEnabled: boolean;
    onHeatmapToggle: () => void;
    onRefresh: () => void;
}

export default function InspectToolPanel({
    healthData,
    thicknessData,
    isLoadingHealth,
    isLoadingThickness,
    onRepair,
    isRepairing,
    heatmapEnabled,
    onHeatmapToggle,
    onRefresh,
}: InspectToolPanelProps) {
    const [activeTab, setActiveTab] = useState<'health' | 'thickness'>('health');

    return (
        <div className="flex flex-col h-full">
            {/* Header */}
            <div className="p-4 border-b border-zinc-800">
                <div className="flex items-center gap-3 mb-3">
                    <div className="w-8 h-8 rounded-lg bg-blue-500/20 flex items-center justify-center">
                        <span>🔍</span>
                    </div>
                    <div>
                        <h3 className="text-lg font-semibold text-white">Inspect</h3>
                        <p className="text-xs text-zinc-500">Mesh health & thickness analysis</p>
                    </div>
                </div>

                {/* Tab Switcher */}
                <div className="flex gap-1 p-1 bg-zinc-800/50 rounded-lg">
                    <button
                        onClick={() => setActiveTab('health')}
                        className={`
                            flex-1 py-2 px-3 rounded-md text-sm font-medium transition-all
                            ${activeTab === 'health'
                                ? 'bg-zinc-700 text-white shadow'
                                : 'text-zinc-400 hover:text-white'}
                        `}
                    >
                        🏥 Health
                    </button>
                    <button
                        onClick={() => setActiveTab('thickness')}
                        className={`
                            flex-1 py-2 px-3 rounded-md text-sm font-medium transition-all
                            ${activeTab === 'thickness'
                                ? 'bg-zinc-700 text-white shadow'
                                : 'text-zinc-400 hover:text-white'}
                        `}
                    >
                        📊 Thickness
                    </button>
                </div>
            </div>

            {/* Tab Content */}
            <div className="flex-1 overflow-y-auto">
                {activeTab === 'health' && (
                    <HealthPanel
                        data={healthData}
                        isLoading={isLoadingHealth}
                        onRepair={onRepair}
                        isRepairing={isRepairing}
                    />
                )}

                {activeTab === 'thickness' && (
                    <div className="p-4 space-y-4">
                        {/* Heatmap Toggle */}
                        <div className="flex items-center justify-between p-3 bg-zinc-800/50 rounded-lg">
                            <div className="flex items-center gap-2">
                                <span className="text-lg">🌡️</span>
                                <div>
                                    <span className="text-sm text-zinc-300">Thickness Heatmap</span>
                                    <p className="text-xs text-zinc-500">Color model by wall thickness</p>
                                </div>
                            </div>
                            <button
                                onClick={onHeatmapToggle}
                                className={`
                                    relative w-12 h-6 rounded-full transition-colors
                                    ${heatmapEnabled ? 'bg-blue-500' : 'bg-zinc-600'}
                                `}
                            >
                                <span className={`
                                    absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform
                                    ${heatmapEnabled ? 'left-6' : 'left-0.5'}
                                `} />
                            </button>
                        </div>

                        {/* Color Legend */}
                        {heatmapEnabled && (
                            <div className="p-3 bg-zinc-800/30 rounded-lg">
                                <p className="text-xs text-zinc-500 uppercase tracking-wide mb-2">Color Legend</p>
                                <div className="flex items-center gap-2">
                                    <div className="flex-1 h-3 rounded-full bg-gradient-to-r from-red-500 via-yellow-500 via-green-500 to-blue-500" />
                                </div>
                                <div className="flex justify-between text-xs text-zinc-500 mt-1">
                                    <span>Thin</span>
                                    <span>OK</span>
                                    <span>Thick</span>
                                </div>
                            </div>
                        )}

                        {/* Thickness Stats */}
                        {isLoadingThickness ? (
                            <div className="flex items-center justify-center py-8">
                                <div className="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
                            </div>
                        ) : thicknessData ? (
                            <div className="space-y-2">
                                <div className="grid grid-cols-3 gap-2">
                                    <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-center">
                                        <p className="text-lg font-bold text-red-400">{thicknessData.min_thickness.toFixed(2)}</p>
                                        <p className="text-[10px] text-zinc-500 uppercase">Min (mm)</p>
                                    </div>
                                    <div className="p-3 bg-green-500/10 border border-green-500/30 rounded-lg text-center">
                                        <p className="text-lg font-bold text-green-400">{thicknessData.avg_thickness.toFixed(2)}</p>
                                        <p className="text-[10px] text-zinc-500 uppercase">Avg (mm)</p>
                                    </div>
                                    <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg text-center">
                                        <p className="text-lg font-bold text-blue-400">{thicknessData.max_thickness.toFixed(2)}</p>
                                        <p className="text-[10px] text-zinc-500 uppercase">Max (mm)</p>
                                    </div>
                                </div>

                                {/* Violations Warning */}
                                {thicknessData.violation_count > 0 && (
                                    <div className="p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
                                        <div className="flex items-center gap-2">
                                            <span className="text-red-400">⚠️</span>
                                            <div>
                                                <p className="text-sm text-red-400 font-medium">
                                                    {thicknessData.violation_count} thin areas detected
                                                </p>
                                                <p className="text-xs text-zinc-500">
                                                    Below minimum thickness for 3D printing
                                                </p>
                                            </div>
                                        </div>
                                    </div>
                                )}
                            </div>
                        ) : (
                            <div className="text-center py-8">
                                <p className="text-sm text-zinc-500">No thickness data available</p>
                                <button
                                    onClick={onRefresh}
                                    className="mt-2 px-4 py-2 bg-zinc-800 text-zinc-300 rounded-lg text-sm hover:bg-zinc-700"
                                >
                                    Analyze Thickness
                                </button>
                            </div>
                        )}

                        {/* Threshold Settings */}
                        <div className="pt-4 border-t border-zinc-800">
                            <label className="text-xs text-zinc-500 uppercase tracking-wide mb-2 block">
                                Minimum Threshold
                            </label>
                            <select className="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-sm text-white">
                                <option value="0.4">0.4mm - SLA Printing</option>
                                <option value="0.6" selected>0.6mm - Jewelry Casting</option>
                                <option value="0.8">0.8mm - Standard</option>
                                <option value="1.0">1.0mm - FDM Printing</option>
                            </select>
                        </div>
                    </div>
                )}
            </div>

            {/* Refresh Button */}
            <div className="p-4 border-t border-zinc-800">
                <button
                    onClick={onRefresh}
                    className="w-full py-2 px-4 bg-zinc-800 text-zinc-300 rounded-lg text-sm hover:bg-zinc-700 transition-colors flex items-center justify-center gap-2"
                >
                    🔄 Refresh Analysis
                </button>
            </div>
        </div>
    );
}
