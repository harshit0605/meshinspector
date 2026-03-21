'use client';

/**
 * Mesh Health Panel - displays mesh health status and repair options
 */

interface HealthData {
    is_closed: boolean;
    self_intersections: number;
    holes_count: number;
    health_score: number;
}

interface HealthPanelProps {
    data: HealthData | null;
    isLoading: boolean;
    onRepair: () => void;
    isRepairing?: boolean;
}

export default function HealthPanel({
    data,
    isLoading,
    onRepair,
    isRepairing = false
}: HealthPanelProps) {
    if (isLoading) {
        return (
            <div className="p-4">
                <div className="flex items-center gap-3 mb-4">
                    <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center">
                        <span>🔍</span>
                    </div>
                    <div>
                        <h3 className="text-lg font-semibold text-white">Health Check</h3>
                        <p className="text-xs text-zinc-500">Analyzing mesh...</p>
                    </div>
                </div>
                <div className="flex items-center justify-center py-8">
                    <div className="w-8 h-8 border-2 border-amber-500 border-t-transparent rounded-full animate-spin" />
                </div>
            </div>
        );
    }

    if (!data) {
        return (
            <div className="p-4">
                <div className="flex items-center gap-3 mb-4">
                    <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center">
                        <span>🔍</span>
                    </div>
                    <div>
                        <h3 className="text-lg font-semibold text-white">Health Check</h3>
                        <p className="text-xs text-zinc-500">Mesh health analysis</p>
                    </div>
                </div>
                <p className="text-sm text-zinc-500 text-center py-4">
                    No health data available
                </p>
            </div>
        );
    }

    const getScoreColor = (score: number) => {
        if (score >= 80) return 'text-green-400';
        if (score >= 50) return 'text-amber-400';
        return 'text-red-400';
    };

    const getScoreBg = (score: number) => {
        if (score >= 80) return 'bg-green-500/10 border-green-500/30';
        if (score >= 50) return 'bg-amber-500/10 border-amber-500/30';
        return 'bg-red-500/10 border-red-500/30';
    };

    return (
        <div className="p-4 space-y-4">
            {/* Header */}
            <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center">
                    <span>🔍</span>
                </div>
                <div>
                    <h3 className="text-lg font-semibold text-white">Health Check</h3>
                    <p className="text-xs text-zinc-500">Mesh health analysis</p>
                </div>
            </div>

            {/* Health Score */}
            <div className={`flex items-center justify-between p-4 rounded-xl border ${getScoreBg(data.health_score)}`}>
                <div>
                    <span className="text-xs text-zinc-400 uppercase tracking-wide">Health Score</span>
                    <div className={`text-3xl font-bold ${getScoreColor(data.health_score)}`}>
                        {data.health_score}
                        <span className="text-lg text-zinc-500">/100</span>
                    </div>
                </div>
                <div className="w-16 h-16 relative">
                    <svg className="w-full h-full transform -rotate-90">
                        <circle
                            cx="32"
                            cy="32"
                            r="28"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="4"
                            className="text-zinc-700"
                        />
                        <circle
                            cx="32"
                            cy="32"
                            r="28"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="4"
                            strokeLinecap="round"
                            strokeDasharray={`${(data.health_score / 100) * 176} 176`}
                            className={getScoreColor(data.health_score)}
                        />
                    </svg>
                </div>
            </div>

            {/* Check Items */}
            <div className="space-y-2">
                <HealthCheckItem
                    label="Watertight"
                    passed={data.is_closed}
                    detail={data.is_closed ? 'Mesh is closed' : 'Has open boundaries'}
                    icon={data.is_closed ? '💧' : '🕳️'}
                />
                <HealthCheckItem
                    label="Self-Intersections"
                    passed={data.self_intersections === 0}
                    detail={data.self_intersections === 0
                        ? 'None detected'
                        : `${data.self_intersections} face pairs`}
                    icon={data.self_intersections === 0 ? '✂️' : '⚠️'}
                />
                <HealthCheckItem
                    label="Holes"
                    passed={data.holes_count === 0}
                    detail={data.holes_count === 0
                        ? 'No holes found'
                        : `${data.holes_count} hole${data.holes_count > 1 ? 's' : ''}`}
                    icon={data.holes_count === 0 ? '🔘' : '⭕'}
                />
            </div>

            {/* Auto Repair Button */}
            {data.health_score < 100 && (
                <button
                    onClick={onRepair}
                    disabled={isRepairing}
                    className={`
                        w-full py-3 px-4 rounded-xl text-sm font-semibold
                        transition-all duration-200 flex items-center justify-center gap-2
                        ${isRepairing
                            ? 'bg-zinc-700 text-zinc-400 cursor-wait'
                            : 'bg-gradient-to-r from-amber-500 to-amber-600 text-black hover:from-amber-400 hover:to-amber-500 hover:scale-[1.02] active:scale-[0.98]'
                        }
                    `}
                >
                    {isRepairing ? (
                        <>
                            <div className="w-4 h-4 border-2 border-zinc-500 border-t-transparent rounded-full animate-spin" />
                            Repairing...
                        </>
                    ) : (
                        <>
                            🔧 Auto Repair
                        </>
                    )}
                </button>
            )}

            {/* Success state */}
            {data.health_score === 100 && (
                <div className="p-3 bg-green-500/10 border border-green-500/30 rounded-xl">
                    <p className="text-sm text-green-400 text-center flex items-center justify-center gap-2">
                        <span>✓</span>
                        Mesh is healthy and ready for manufacturing
                    </p>
                </div>
            )}
        </div>
    );
}

function HealthCheckItem({ label, passed, detail, icon }: {
    label: string;
    passed: boolean;
    detail: string;
    icon: string;
}) {
    return (
        <div className={`
            flex items-center justify-between p-3 rounded-lg transition-colors
            ${passed ? 'bg-zinc-800/30' : 'bg-red-500/5 border border-red-500/20'}
        `}>
            <div className="flex items-center gap-3">
                <span className="text-lg">{icon}</span>
                <div>
                    <span className="text-sm text-zinc-300">{label}</span>
                    <p className="text-xs text-zinc-500">{detail}</p>
                </div>
            </div>
            <span className={`text-lg ${passed ? 'text-green-400' : 'text-red-400'}`}>
                {passed ? '✓' : '✗'}
            </span>
        </div>
    );
}
