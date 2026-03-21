'use client';

/**
 * Stats panel component with glassmorphism
 */

import { MATERIALS } from '@/lib/constants';
import type { AnalysisResponse, MaterialType } from '@/lib/api/types';

interface ViewerStatsProps {
    analysis: AnalysisResponse | null;
    material: MaterialType;
    isLoading?: boolean;
}

export default function ViewerStats({ analysis, material, isLoading }: ViewerStatsProps) {
    const materialInfo = MATERIALS[material] || MATERIALS.gold_18k;

    if (isLoading) {
        return (
            <div className="glass-card p-6 w-80">
                <div className="animate-pulse space-y-4">
                    <div className="h-6 bg-white/10 rounded w-32" />
                    <div className="space-y-3">
                        <div className="h-4 bg-white/5 rounded" />
                        <div className="h-4 bg-white/5 rounded" />
                        <div className="h-4 bg-white/5 rounded" />
                    </div>
                </div>
            </div>
        );
    }

    if (!analysis) {
        return (
            <div className="glass-card p-6 w-80">
                <p className="text-zinc-500 text-sm">Upload a model to see analysis</p>
            </div>
        );
    }

    return (
        <div className="glass-card p-6 w-80">
            {/* Header */}
            <div className="flex items-center gap-3 mb-6 pb-4 border-b border-white/10">
                <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-400/20 to-amber-600/20 flex items-center justify-center">
                    <span className="text-lg">📊</span>
                </div>
                <div>
                    <h3 className="text-lg font-semibold text-white">Model Analysis</h3>
                    <p className="text-xs text-zinc-500">{materialInfo.label}</p>
                </div>
            </div>

            {/* Stats Grid */}
            <div className="space-y-4">
                <StatRow
                    label="Weight"
                    value={`${(analysis.weight_grams ?? 0).toFixed(2)} g`}
                    icon="⚖️"
                    color="text-amber-400"
                />
                <StatRow
                    label="Volume"
                    value={`${(analysis.volume_mm3 ?? 0).toFixed(1)} mm³`}
                    icon="📦"
                    color="text-blue-400"
                />
                <StatRow
                    label="Dimensions"
                    value={`${(analysis.bounding_box?.x ?? 0).toFixed(1)} × ${(analysis.bounding_box?.y ?? 0).toFixed(1)} × ${(analysis.bounding_box?.z ?? 0).toFixed(1)} mm`}
                    icon="📐"
                    color="text-purple-400"
                />
                <StatRow
                    label="Watertight"
                    value={analysis.is_watertight ? 'Yes' : 'No'}
                    icon={analysis.is_watertight ? '✓' : '⚠️'}
                    color={analysis.is_watertight ? 'text-green-400' : 'text-yellow-400'}
                    highlight={!analysis.is_watertight}
                />
            </div>

            {/* Footer */}
            <div className="mt-6 pt-4 border-t border-white/10 flex items-center justify-between text-xs text-zinc-500">
                <span>{analysis.vertex_count?.toLocaleString() ?? 'N/A'} vertices</span>
                <span>{analysis.face_count?.toLocaleString() ?? 'N/A'} faces</span>
            </div>
        </div>
    );
}

function StatRow({
    label,
    value,
    icon,
    color = 'text-white',
    highlight = false
}: {
    label: string;
    value: string;
    icon: string;
    color?: string;
    highlight?: boolean;
}) {
    return (
        <div className={`
            flex items-center justify-between p-3 rounded-xl
            ${highlight ? 'bg-yellow-500/10 border border-yellow-500/20' : 'bg-white/5'}
            transition-colors
        `}>
            <div className="flex items-center gap-3">
                <span className="text-lg">{icon}</span>
                <span className="text-zinc-400 text-sm">{label}</span>
            </div>
            <span className={`font-medium ${color}`}>{value}</span>
        </div>
    );
}
