'use client';

/**
 * Drag-and-drop file upload component with glassmorphism
 */

import { useCallback } from 'react';
import { useDropzone } from 'react-dropzone';
import { ALLOWED_EXTENSIONS, MAX_FILE_SIZE_MB } from '@/lib/constants';

interface UploadDropzoneProps {
    onFileSelect: (file: File) => void;
    isUploading?: boolean;
    error?: string | null;
}

export default function UploadDropzone({
    onFileSelect,
    isUploading = false,
    error = null
}: UploadDropzoneProps) {
    const onDrop = useCallback((acceptedFiles: File[]) => {
        if (acceptedFiles.length > 0) {
            onFileSelect(acceptedFiles[0]);
        }
    }, [onFileSelect]);

    const { getRootProps, getInputProps, isDragActive, isDragReject } = useDropzone({
        onDrop,
        accept: {
            'model/gltf-binary': ['.glb'],
            'model/gltf+json': ['.gltf'],
            'model/stl': ['.stl'],
            'model/obj': ['.obj'],
            'application/octet-stream': ['.ply'],
        },
        maxSize: MAX_FILE_SIZE_MB * 1024 * 1024,
        multiple: false,
        disabled: isUploading,
    });

    return (
        <div
            {...getRootProps()}
            className={`
        relative flex flex-col items-center justify-center
        w-full p-16 rounded-3xl cursor-pointer
        transition-all duration-300 ease-out
        ${isDragActive && !isDragReject
                    ? 'glass-card border-amber-500/50 scale-[1.02] glow-amber'
                    : isDragReject
                        ? 'glass-card border-red-500/50'
                        : 'glass-card hover:border-white/20 hover:scale-[1.01]'
                }
        ${isUploading ? 'opacity-70 cursor-wait pointer-events-none' : ''}
      `}
        >
            <input {...getInputProps()} />

            {/* Animated icon */}
            <div className={`
        mb-8 p-6 rounded-2xl
        ${isDragActive ? 'bg-amber-500/20' : 'bg-white/5'}
        transition-all duration-300
      `}>
                <svg
                    className={`w-16 h-16 transition-colors duration-300 ${isDragActive ? 'text-amber-400' : 'text-zinc-500'
                        }`}
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={1.5}
                >
                    <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M12 16.5V9.75m0 0l3 3m-3-3l-3 3M6.75 19.5a4.5 4.5 0 01-1.41-8.775 5.25 5.25 0 0110.338-2.329 4.5 4.5 0 113.356 8.045H6.75z"
                    />
                </svg>
            </div>

            {/* Text */}
            <h3 className="text-2xl font-semibold text-white mb-3">
                {isDragActive ? 'Drop your model here' : 'Upload 3D Model'}
            </h3>
            <p className="text-zinc-400 text-center mb-4">
                Drag and drop or <span className="text-amber-400 font-medium">browse</span> to upload
            </p>
            <div className="flex items-center gap-3 text-xs text-zinc-500">
                <span className="px-3 py-1.5 rounded-full bg-white/5 border border-white/10">.glb</span>
                <span className="px-3 py-1.5 rounded-full bg-white/5 border border-white/10">.gltf</span>
                <span className="px-3 py-1.5 rounded-full bg-white/5 border border-white/10">.stl</span>
                <span className="px-3 py-1.5 rounded-full bg-white/5 border border-white/10">.obj</span>
                <span className="px-3 py-1.5 rounded-full bg-white/5 border border-white/10">.ply</span>
            </div>
            <p className="text-xs text-zinc-600 mt-3">
                Maximum file size: {MAX_FILE_SIZE_MB}MB
            </p>

            {/* Error message */}
            {error && (
                <div className="absolute bottom-6 left-6 right-6 px-4 py-3 bg-red-500/20 border border-red-500/30 rounded-xl">
                    <p className="text-sm text-red-400 text-center">{error}</p>
                </div>
            )}

            {/* Loading overlay */}
            {isUploading && (
                <div className="absolute inset-0 flex items-center justify-center bg-black/60 backdrop-blur-sm rounded-3xl">
                    <div className="flex flex-col items-center gap-4">
                        <div className="w-12 h-12 border-3 border-amber-500 border-t-transparent rounded-full animate-spin" />
                        <p className="text-white font-medium">Uploading your model...</p>
                    </div>
                </div>
            )}

            {/* Corner decorations */}
            <div className="absolute top-4 left-4 w-8 h-8 border-l-2 border-t-2 border-white/10 rounded-tl-lg" />
            <div className="absolute top-4 right-4 w-8 h-8 border-r-2 border-t-2 border-white/10 rounded-tr-lg" />
            <div className="absolute bottom-4 left-4 w-8 h-8 border-l-2 border-b-2 border-white/10 rounded-bl-lg" />
            <div className="absolute bottom-4 right-4 w-8 h-8 border-r-2 border-b-2 border-white/10 rounded-br-lg" />
        </div>
    );
}
