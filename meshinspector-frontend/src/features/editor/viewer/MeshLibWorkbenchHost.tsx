'use client';

import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type { MeshLibRuntimeManifest, MeshLibWorkbenchManifest } from '@/lib/api/types';

type MeshLibWorkbenchHostProps = {
  manifest: MeshLibWorkbenchManifest | null;
  children: ReactNode;
};

type WorkbenchMessage =
  | { type: 'meshlib-workbench:request-init' }
  | { type: 'meshlib-workbench:ready' };

export default function MeshLibWorkbenchHost({ manifest, children }: MeshLibWorkbenchHostProps) {
  const iframeRef = useRef<HTMLIFrameElement | null>(null);
  const [runtimeManifest, setRuntimeManifest] = useState<MeshLibRuntimeManifest | null>(null);
  const [runtimeLoadError, setRuntimeLoadError] = useState<string | null>(null);
  const [workbenchReady, setWorkbenchReady] = useState(false);
  const apiBase = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

  useEffect(() => {
    let active = true;
    setRuntimeLoadError(null);
    void fetch('/meshlib-workbench/runtime/manifest.json', { cache: 'no-store' })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error(`Runtime manifest unavailable (${response.status})`);
        }
        return response.json() as Promise<MeshLibRuntimeManifest>;
      })
      .then((payload) => {
        if (!active) {
          return;
        }
        setRuntimeManifest(payload);
      })
      .catch((error) => {
        if (!active) {
          return;
        }
        setRuntimeManifest({
          status: 'missing',
          message: error instanceof Error ? error.message : 'Runtime manifest unavailable',
        });
        setRuntimeLoadError(error instanceof Error ? error.message : 'Runtime manifest unavailable');
      });

    return () => {
      active = false;
    };
  }, []);

  const normalizedManifest = useMemo<MeshLibWorkbenchManifest | null>(() => {
    if (!manifest) {
      return null;
    }
    const absolutize = (value: string | null | undefined) => {
      if (!value) {
        return value ?? null;
      }
      if (value.startsWith('http://') || value.startsWith('https://')) {
        return value;
      }
      return `${apiBase}${value}`;
    };

    return {
      ...manifest,
      normalized_mesh_url: absolutize(manifest.normalized_mesh_url),
      preview_low_url: absolutize(manifest.preview_low_url),
      preview_high_url: absolutize(manifest.preview_high_url),
      commit_endpoint_url: absolutize(manifest.commit_endpoint_url) || manifest.commit_endpoint_url,
    };
  }, [apiBase, manifest]);

  useEffect(() => {
    const handleMessage = (event: MessageEvent<WorkbenchMessage>) => {
      if (event.origin !== window.location.origin) {
        return;
      }
      if (event.source !== iframeRef.current?.contentWindow) {
        return;
      }
      if (event.data?.type === 'meshlib-workbench:request-init') {
        iframeRef.current?.contentWindow?.postMessage(
          {
            type: 'meshlib-workbench:init',
            payload: {
              manifest: normalizedManifest,
              runtimeManifest,
            },
          },
          window.location.origin,
        );
      }
      if (event.data?.type === 'meshlib-workbench:ready') {
        setWorkbenchReady(true);
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [normalizedManifest, runtimeManifest]);

  const runtimeReady = runtimeManifest?.status === 'ready' && !!manifest;
  const iframeSrc = useMemo(() => {
    if (!manifest) {
      return null;
    }
    return manifest.entry_html_url;
  }, [manifest]);

  if (runtimeReady && iframeSrc) {
    return (
      <div className="relative h-full w-full">
        <iframe
          ref={iframeRef}
          title="MeshLib Workbench"
          src={iframeSrc}
          className="h-full w-full border-0 bg-zinc-950"
          allow="clipboard-read; clipboard-write"
        />
        {!workbenchReady && (
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-zinc-950/40 backdrop-blur-[1px]">
            <div className="rounded-2xl border border-zinc-800 bg-zinc-950/90 px-4 py-3 text-sm text-zinc-300">
              Booting MeshLib workbench runtime...
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="relative h-full w-full">
      {children}
      <div className="pointer-events-none absolute bottom-4 right-4 z-20 max-w-sm rounded-2xl border border-amber-500/20 bg-zinc-950/92 px-4 py-3 text-xs text-zinc-300 shadow-[0_18px_48px_rgba(0,0,0,0.35)] backdrop-blur">
        <p className="text-[10px] uppercase tracking-[0.24em] text-amber-300">MeshLib Workbench</p>
        <p className="mt-2 leading-5 text-zinc-300">
          The MeshLib Viewer host seam is active, but a compiled runtime bundle is not installed in
          <span className="mx-1 rounded bg-zinc-900 px-1.5 py-0.5 text-zinc-100">/public/meshlib-workbench/runtime</span>.
          The classic viewer remains active until the WASM workbench is built.
        </p>
        {(runtimeManifest?.message || runtimeLoadError) && (
          <p className="mt-2 text-zinc-500">{runtimeManifest?.message || runtimeLoadError}</p>
        )}
      </div>
    </div>
  );
}
