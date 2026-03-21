'use client';

/**
 * Loading overlay with glassmorphism
 */

interface LoadingOverlayProps {
    message?: string;
    progress?: number; // 0-100
}

export default function LoadingOverlay({
    message = 'Loading...',
    progress
}: LoadingOverlayProps) {
    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xl">
            <div className="glass-card p-10 flex flex-col items-center gap-6 min-w-[280px]">
                {/* Spinner */}
                <div className="relative">
                    <div className="w-16 h-16 border-4 border-white/10 rounded-full" />
                    <div className="absolute inset-0 w-16 h-16 border-4 border-t-amber-500 border-r-transparent border-b-transparent border-l-transparent rounded-full animate-spin" />
                    <div className="absolute inset-2 w-12 h-12 border-4 border-t-transparent border-r-amber-400 border-b-transparent border-l-transparent rounded-full animate-spin" style={{ animationDirection: 'reverse', animationDuration: '0.8s' }} />
                </div>

                {/* Message */}
                <p className="text-white font-medium text-lg">{message}</p>

                {/* Progress bar */}
                {typeof progress === 'number' && (
                    <div className="w-full">
                        <div className="h-2 bg-white/10 rounded-full overflow-hidden">
                            <div
                                className="h-full bg-gradient-to-r from-amber-400 to-amber-600 rounded-full transition-all duration-300"
                                style={{ width: `${progress}%` }}
                            />
                        </div>
                        <p className="text-center text-xs text-zinc-500 mt-2">{progress}%</p>
                    </div>
                )}
            </div>
        </div>
    );
}
