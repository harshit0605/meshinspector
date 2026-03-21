'use client';

/**
 * Home page - Upload landing with Apple-grade design
 */

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import UploadDropzone from '@/components/UploadDropzone';
import { useUploadModel } from '@/hooks/useModelProcessing';

export default function HomePage() {
  const router = useRouter();
  const uploadMutation = useUploadModel();
  const [error, setError] = useState<string | null>(null);

  const handleFileSelect = async (file: File) => {
    setError(null);

    try {
      const result = await uploadMutation.mutateAsync(file);
      router.push(`/viewer?model=${result.model.id}&version=${result.version.id}&job=${result.job?.id ?? ''}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    }
  };

  return (
    <main className="min-h-screen bg-gradient-mesh relative overflow-hidden">
      {/* Ambient glow effects */}
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-amber-500/20 rounded-full blur-[120px] pointer-events-none" />
      <div className="absolute bottom-1/4 right-1/4 w-80 h-80 bg-purple-500/15 rounded-full blur-[100px] pointer-events-none" />

      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 px-8 py-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-400 to-amber-600 flex items-center justify-center">
              <span className="text-black font-bold text-sm">M</span>
            </div>
            <span className="text-xl font-semibold text-white">MeshInspector</span>
          </div>
          <div className="glass-strong px-4 py-2 rounded-full">
            <span className="text-sm text-zinc-400">v1.0</span>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="flex flex-col items-center justify-center min-h-screen px-8 pt-20">
        {/* Headline */}
        <div className="text-center mb-12 max-w-3xl">
          <h1 className="text-5xl md:text-7xl font-bold mb-6 tracking-tight">
            <span className="bg-gradient-to-r from-white via-white to-zinc-400 bg-clip-text text-transparent">
              3D Jewelry
            </span>
            <br />
            <span className="bg-gradient-to-r from-amber-400 to-amber-600 bg-clip-text text-transparent text-glow">
              Made Perfect
            </span>
          </h1>
          <p className="text-lg md:text-xl text-zinc-400 max-w-xl mx-auto leading-relaxed">
            Analyze, resize, and optimize your 3D jewelry models for manufacturing.
            Professional-grade tools in your browser.
          </p>
        </div>

        {/* Upload Zone */}
        <div className="w-full max-w-2xl">
          <UploadDropzone
            onFileSelect={handleFileSelect}
            isUploading={uploadMutation.isPending}
            error={error}
          />
        </div>

        {/* Features Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mt-20 max-w-5xl w-full">
          <FeatureCard
            icon="📐"
            title="Ring Sizing"
            description="Precisely resize rings to any standard size while maintaining perfect proportions"
            gradient="from-blue-500/20 to-cyan-500/20"
          />
          <FeatureCard
            icon="⚖️"
            title="Weight Control"
            description="Hollow models with exact wall thickness to achieve your target weight"
            gradient="from-amber-500/20 to-orange-500/20"
          />
          <FeatureCard
            icon="✨"
            title="Export Ready"
            description="Download optimized GLB for preview or STL for manufacturing"
            gradient="from-purple-500/20 to-pink-500/20"
          />
        </div>

        {/* Trust badges */}
        <div className="flex items-center gap-8 mt-16 text-zinc-500 text-sm">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 bg-green-500 rounded-full" />
            <span>Real-time Analysis</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 bg-blue-500 rounded-full" />
            <span>STL / GLB / OBJ Support</span>
          </div>
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 bg-purple-500 rounded-full" />
            <span>Browser-based</span>
          </div>
        </div>
      </section>
    </main>
  );
}

function FeatureCard({
  icon,
  title,
  description,
  gradient
}: {
  icon: string;
  title: string;
  description: string;
  gradient: string;
}) {
  return (
    <div className={`
      glass-card p-8 relative overflow-hidden group
      hover:scale-[1.02] transition-all duration-300
    `}>
      {/* Gradient background */}
      <div className={`
        absolute inset-0 bg-gradient-to-br ${gradient} opacity-0 
        group-hover:opacity-100 transition-opacity duration-500
      `} />

      {/* Content */}
      <div className="relative z-10">
        <div className="text-4xl mb-4">{icon}</div>
        <h3 className="text-xl font-semibold text-white mb-3">{title}</h3>
        <p className="text-zinc-400 text-sm leading-relaxed">{description}</p>
      </div>

      {/* Corner glow */}
      <div className="absolute top-0 right-0 w-20 h-20 bg-gradient-to-br from-white/10 to-transparent rounded-bl-full opacity-0 group-hover:opacity-100 transition-opacity" />
    </div>
  );
}
