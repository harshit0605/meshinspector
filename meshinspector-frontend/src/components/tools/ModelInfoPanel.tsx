'use client';

/**
 * Model info panel showing analysis results
 */

import { MATERIALS } from '@/lib/constants';
import type { AnalysisResponse } from '@/lib/api/types';

interface ModelInfoPanelProps {
    analysis: AnalysisResponse | null;
    isLoading: boolean;
}

export default function ModelInfoPanel({ analysis, isLoading }: ModelInfoPanelProps) {
    if (isLoading) {
        return (
            <div className="p-4">
                <div className="animate-pulse space-y-4">
                    <div className="h-6 bg-zinc-700 rounded w-32" />
                    <div className="space-y-3">
                        <div className="h-4 bg-zinc-800 rounded" />
                        <div className="h-4 bg-zinc-800 rounded" />
                        <div className="h-4 bg-zinc-800 rounded" />
                    </div>
                </div>
            </div>
        );
    }

    if (!analysis) {
        return (
            <div className="p-4">
                <h3 className="text-lg font-semibold text-white mb-4">📊 Model Info</h3>
                <p className="text-sm text-zinc-500">Upload a model to see analysis</p>
            </div>
        );
    }

    return (
        <div className="p-4 space-y-4">
            {/* Header */}
            <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-blue-500/20 flex items-center justify-center">
                    <span>📊</span>
                </div>
                <div>
                    <h3 className="text-lg font-semibold text-white">Model Info</h3>
                    <p className="text-xs text-zinc-500">Analysis results</p>
                </div>
            </div>

            {/* Stats */}
            <div className="space-y-2">
                <StatRow
                    icon="⚖️"
                    label="Weight"
                    value={`${analysis.weight_grams.toFixed(2)} g`}
                    color="text-amber-400"
                />
                <StatRow
                    icon="📦"
                    label="Volume"
                    value={`${analysis.volume_mm3.toFixed(1)} mm³`}
                    color="text-blue-400"
                />
                <StatRow
                    icon="📐"
                    label="Dimensions"
                    value={`${analysis.bounding_box.x.toFixed(1)} × ${analysis.bounding_box.y.toFixed(1)} × ${analysis.bounding_box.z.toFixed(1)} mm`}
                    color="text-purple-400"
                />
                <StatRow
                    icon={analysis.is_watertight ? '✓' : '⚠️'}
                    label="Watertight"
                    value={analysis.is_watertight ? 'Yes' : 'No'}
                    color={analysis.is_watertight ? 'text-green-400' : 'text-yellow-400'}
                    highlight={!analysis.is_watertight}
                />
            </div>

            {/* Mesh stats */}
            <div className="pt-4 border-t border-zinc-800">
                <div className="flex justify-between text-xs text-zinc-500">
                    <span>{analysis.vertex_count?.toLocaleString() ?? 'N/A'} vertices</span>
                    <span>{analysis.face_count?.toLocaleString() ?? 'N/A'} faces</span>
                </div>
            </div>
        </div>
    );
}

function StatRow({
    icon,
    label,
    value,
    color = 'text-white',
    highlight = false,
}: {
    icon: string;
    label: string;
    value: string;
    color?: string;
    highlight?: boolean;
}) {
    return (
        <div className={`
      flex items-center justify-between p-3 rounded-lg
      ${highlight ? 'bg-yellow-500/10 border border-yellow-500/20' : 'bg-zinc-800/50'}
    `}>
            <div className="flex items-center gap-3">
                <span className="text-lg">{icon}</span>
                <span className="text-sm text-zinc-400">{label}</span>
            </div>
            <span className={`text-sm font-medium ${color}`}>{value}</span>
        </div>
    );
}
