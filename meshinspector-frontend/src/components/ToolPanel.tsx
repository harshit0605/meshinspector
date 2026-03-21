'use client';

/**
 * Stackable tool panel - shows active overlays collapsed, main panel expanded
 */

import { type ToolType } from './Navbar';
import SectionToolPanel, { type SectionConfig } from './tools/SectionToolPanel';
import HollowToolPanel, { type HollowConfig } from './tools/HollowToolPanel';
import ModelInfoPanel from './tools/ModelInfoPanel';
import InspectToolPanel from './tools/InspectToolPanel';
import type { AnalysisResponse } from '@/lib/api/types';

interface ToolPanelProps {
    activeTool: ToolType;
    analysis: AnalysisResponse | null;
    isLoading: boolean;
    // Section tool props
    sectionConfig: SectionConfig;
    onSectionConfigChange: (config: SectionConfig) => void;
    // Hollow tool props
    hollowConfig: HollowConfig;
    onHollowConfigChange: (config: HollowConfig) => void;
    onProcess: () => void;
    isProcessing: boolean;
    // Inspect tool props (optional for now)
    heatmapEnabled?: boolean;
    onHeatmapToggle?: () => void;
    onRepair?: () => void;
    isRepairing?: boolean;
}

// Collapsed overlay header component
function CollapsedOverlay({
    icon,
    label,
    summary,
    onExpand,
    onClose
}: {
    icon: string;
    label: string;
    summary: string;
    onExpand: () => void;
    onClose: () => void;
}) {
    return (
        <div className="border-b border-zinc-700/50 bg-zinc-800/30">
            <div className="flex items-center justify-between px-4 py-2.5">
                <button
                    onClick={onExpand}
                    className="flex items-center gap-2 text-left hover:text-amber-400 transition-colors"
                >
                    <span className="text-base">{icon}</span>
                    <span className="text-sm font-medium text-zinc-200">{label}</span>
                    <span className="text-xs text-zinc-500 ml-2">{summary}</span>
                </button>
                <button
                    onClick={onClose}
                    className="text-zinc-500 hover:text-red-400 text-lg leading-none px-1.5 transition-colors"
                    title={`Disable ${label}`}
                >
                    ×
                </button>
            </div>
        </div>
    );
}

export default function ToolPanel({
    activeTool,
    analysis,
    isLoading,
    sectionConfig,
    onSectionConfigChange,
    hollowConfig,
    onHollowConfigChange,
    onProcess,
    isProcessing,
    heatmapEnabled = false,
    onHeatmapToggle,
    onRepair,
    isRepairing = false,
}: ToolPanelProps) {
    // Build section summary line
    const sectionSummary = sectionConfig.enabled
        ? `${sectionConfig.planeType}, ${sectionConfig.constant >= 0 ? '+' : ''}${sectionConfig.constant.toFixed(1)}mm`
        : 'Off';

    // Show panel if any tool is active OR if section overlay is enabled
    const showPanel = activeTool !== 'none' || sectionConfig.enabled;

    if (!showPanel) {
        return null;
    }

    return (
        <div className="w-80 shrink-0 h-full bg-zinc-900/95 border-l border-zinc-800 overflow-y-auto flex flex-col">
            {/* Collapsed Section overlay when not the active panel but enabled */}
            {sectionConfig.enabled && activeTool !== 'section' && (
                <CollapsedOverlay
                    icon="✂️"
                    label="Section"
                    summary={sectionSummary}
                    onExpand={() => {/* Could switch to section panel */ }}
                    onClose={() => onSectionConfigChange({ ...sectionConfig, enabled: false })}
                />
            )}

            {/* Active Panel */}
            <div className="flex-1">
                {activeTool === 'section' && (
                    <SectionToolPanel
                        config={sectionConfig}
                        onChange={onSectionConfigChange}
                    />
                )}

                {activeTool === 'hollow' && (
                    <HollowToolPanel
                        config={hollowConfig}
                        onChange={onHollowConfigChange}
                        onProcess={onProcess}
                        isProcessing={isProcessing}
                        currentWeight={analysis?.weight_grams ?? 0}
                        sectionEnabled={sectionConfig.enabled}
                        onSectionToggle={() => onSectionConfigChange({ ...sectionConfig, enabled: !sectionConfig.enabled })}
                    />
                )}

                {activeTool === 'info' && (
                    <ModelInfoPanel
                        analysis={analysis}
                        isLoading={isLoading}
                    />
                )}

                {activeTool === 'measure' && (
                    <div className="p-4">
                        <h3 className="text-lg font-semibold text-white mb-4">📏 Measure Tool</h3>
                        <p className="text-sm text-zinc-400">Coming soon...</p>
                    </div>
                )}

                {activeTool === 'inspect' && (
                    <InspectToolPanel
                        healthData={null}
                        thicknessData={null}
                        isLoadingHealth={false}
                        isLoadingThickness={false}
                        onRepair={onRepair || (() => { })}
                        isRepairing={isRepairing}
                        heatmapEnabled={heatmapEnabled}
                        onHeatmapToggle={onHeatmapToggle || (() => { })}
                        onRefresh={() => { }}
                    />
                )}

                {activeTool === 'repair' && (
                    <div className="p-4 space-y-4">
                        <div className="flex items-center gap-3">
                            <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center">
                                <span>🔧</span>
                            </div>
                            <div>
                                <h3 className="text-lg font-semibold text-white">Auto Repair</h3>
                                <p className="text-xs text-zinc-500">Fix mesh issues automatically</p>
                            </div>
                        </div>

                        <div className="space-y-2">
                            <div className="p-3 bg-zinc-800/50 rounded-lg flex items-center gap-3">
                                <span className="text-green-400">✓</span>
                                <span className="text-sm text-zinc-300">Fill holes</span>
                            </div>
                            <div className="p-3 bg-zinc-800/50 rounded-lg flex items-center gap-3">
                                <span className="text-green-400">✓</span>
                                <span className="text-sm text-zinc-300">Fix non-manifold edges</span>
                            </div>
                            <div className="p-3 bg-zinc-800/50 rounded-lg flex items-center gap-3">
                                <span className="text-green-400">✓</span>
                                <span className="text-sm text-zinc-300">Remove degenerate faces</span>
                            </div>
                        </div>

                        <button
                            onClick={onRepair}
                            disabled={isRepairing}
                            className={`
                                w-full py-3 px-4 rounded-xl text-sm font-semibold
                                transition-all duration-200 flex items-center justify-center gap-2
                                ${isRepairing
                                    ? 'bg-zinc-700 text-zinc-400 cursor-wait'
                                    : 'bg-gradient-to-r from-green-500 to-green-600 text-white hover:from-green-400 hover:to-green-500'
                                }
                            `}
                        >
                            {isRepairing ? (
                                <>
                                    <div className="w-4 h-4 border-2 border-zinc-500 border-t-transparent rounded-full animate-spin" />
                                    Repairing...
                                </>
                            ) : (
                                <>🔧 Run Auto Repair</>
                            )}
                        </button>

                        <div className="p-3 bg-green-500/10 rounded-lg border border-green-500/20">
                            <p className="text-xs text-green-300">
                                Auto repair uses MeshLib to automatically fix common mesh issues
                                that could cause problems during 3D printing or rendering.
                            </p>
                        </div>
                    </div>
                )}

                {/* When only section is active (no other panel) */}
                {activeTool === 'none' && sectionConfig.enabled && (
                    <SectionToolPanel
                        config={sectionConfig}
                        onChange={onSectionConfigChange}
                    />
                )}
            </div>
        </div>
    );
}
