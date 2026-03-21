'use client';

import { useState } from 'react';
import type {
  InspectionSnapshotResponse,
  RegionManifestEntry,
  ScalarOverlayResponse,
  SectionContourPayload,
} from '@/lib/api/types';

type SectionPreset = {
  id: string;
  label: string;
  description: string;
};

export default function OverlayLegendPanel({
  overlay,
  sectionEnabled,
  sectionConstant,
  selectedRegion,
  sectionContour,
  sectionPresets,
  savedSnapshots,
  onSectionConstantChange,
  onSnapToRegion,
  onSnapToCenter,
  onApplySectionPreset,
  onExportSection,
  onSaveSnapshot,
  onLoadSnapshot,
}: {
  overlay: ScalarOverlayResponse | null;
  sectionEnabled: boolean;
  sectionConstant: number;
  selectedRegion: RegionManifestEntry | null;
  sectionContour: SectionContourPayload | null;
  sectionPresets: SectionPreset[];
  savedSnapshots: InspectionSnapshotResponse[];
  onSectionConstantChange: (value: number) => void;
  onSnapToRegion: () => void;
  onSnapToCenter: () => void;
  onApplySectionPreset: (presetId: string) => void;
  onExportSection: () => void;
  onSaveSnapshot: (name: string) => void;
  onLoadSnapshot: (snapshot: InspectionSnapshotResponse) => void;
}) {
  const [snapshotName, setSnapshotName] = useState('');
  const planeDelta =
    selectedRegion?.centroid_mm && sectionEnabled
      ? Math.abs(sectionConstant - selectedRegion.centroid_mm[1])
      : null;

  return (
    <div className="rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4">
      <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Inspect</p>

      <div className="mt-4">
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm text-zinc-200">Section Plane</p>
          <span className="text-xs text-zinc-500">{sectionEnabled ? `${sectionConstant.toFixed(1)} mm` : 'Off'}</span>
        </div>
        <input
          type="range"
          min={-40}
          max={40}
          step={0.5}
          value={sectionConstant}
          onChange={(event) => onSectionConstantChange(Number(event.target.value))}
          className="mt-3 w-full accent-amber-400"
        />
      </div>

      <div className="mt-3 flex flex-wrap gap-2">
        <button
          onClick={onSnapToCenter}
          className="rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs text-zinc-300 hover:bg-zinc-900"
        >
          Snap Center
        </button>
        <button
          onClick={onSnapToRegion}
          disabled={!selectedRegion?.centroid_mm}
          className="rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs text-zinc-300 hover:bg-zinc-900 disabled:opacity-40"
        >
          Snap Region
        </button>
        <button
          onClick={onExportSection}
          disabled={!sectionEnabled || !sectionContour?.segments.length}
          className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-100 hover:bg-amber-500/15 disabled:opacity-40"
        >
          Export SVG
        </button>
      </div>

      {sectionPresets.length ? (
        <div className="mt-5">
          <p className="text-sm text-zinc-200">Section Presets</p>
          <div className="mt-3 space-y-2">
            {sectionPresets.map((preset) => (
              <button
                key={preset.id}
                onClick={() => onApplySectionPreset(preset.id)}
                className="w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-left hover:bg-zinc-900"
              >
                <p className="text-sm text-zinc-100">{preset.label}</p>
                <p className="mt-1 text-xs text-zinc-500">{preset.description}</p>
              </button>
            ))}
          </div>
        </div>
      ) : null}

      {sectionEnabled && sectionContour ? (
        <div className="mt-5 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
          <div className="flex items-center justify-between gap-3">
            <p className="text-sm text-zinc-100">Section Contour</p>
            <span className="text-xs text-zinc-500">{sectionContour.contour_count} contours</span>
          </div>
          <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Perimeter" value={sectionContour.perimeter_mm != null ? `${sectionContour.perimeter_mm.toFixed(2)} mm` : 'n/a'} />
            <Readout label="Slice width" value={sectionContour.width_mm != null ? `${sectionContour.width_mm.toFixed(2)} mm` : 'n/a'} />
            <Readout label="Slice depth" value={sectionContour.depth_mm != null ? `${sectionContour.depth_mm.toFixed(2)} mm` : 'n/a'} />
            <Readout label="Segments" value={String(sectionContour.segment_count)} />
            <Readout label="Region segs" value={String(sectionContour.selected_region_segment_count)} />
          </div>
          {sectionContour.width_mm != null && sectionContour.depth_mm != null ? (
            <p className="mt-3 text-xs text-zinc-500">
              Section envelope: {sectionContour.width_mm.toFixed(2)} mm × {sectionContour.depth_mm.toFixed(2)} mm
            </p>
          ) : null}
        </div>
      ) : null}

      {selectedRegion ? (
        <div className="mt-5 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
          <div className="flex items-center justify-between gap-3">
            <p className="text-sm text-zinc-100">{selectedRegion.label}</p>
            <span className="text-xs text-zinc-500">{selectedRegion.coverage_pct}%</span>
          </div>
          <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-zinc-400">
            <Readout label="Min thickness" value={selectedRegion.min_thickness_mm != null ? `${selectedRegion.min_thickness_mm.toFixed(2)} mm` : 'n/a'} />
            <Readout label="Avg thickness" value={selectedRegion.avg_thickness_mm != null ? `${selectedRegion.avg_thickness_mm.toFixed(2)} mm` : 'n/a'} />
            <Readout label="Violations" value={String(selectedRegion.violation_count)} />
            <Readout label="Allowed ops" value={selectedRegion.allowed_operations.length ? selectedRegion.allowed_operations.join(', ') : 'none'} />
          </div>
          {selectedRegion.centroid_mm ? (
            <div className="mt-3 text-xs text-zinc-500">
              Centroid: {selectedRegion.centroid_mm.map((value) => value.toFixed(1)).join(', ')} mm
            </div>
          ) : null}
          {planeDelta != null ? (
            <div className="mt-1 text-xs text-zinc-500">Plane to centroid delta: {planeDelta.toFixed(2)} mm</div>
          ) : null}
        </div>
      ) : (
        <p className="mt-5 text-sm text-zinc-500">Select a region from the panel or by clicking the mesh.</p>
      )}

      <div className="mt-5 rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-3">
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm text-zinc-100">Inspection Snapshots</p>
          <span className="text-xs text-zinc-500">{savedSnapshots.length}</span>
        </div>
        <div className="mt-3 flex gap-2">
          <input
            value={snapshotName}
            onChange={(event) => setSnapshotName(event.target.value)}
            placeholder="Name current view"
            className="min-w-0 flex-1 rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-500"
          />
          <button
            onClick={() => {
              const trimmed = snapshotName.trim();
              if (!trimmed) return;
              onSaveSnapshot(trimmed);
              setSnapshotName('');
            }}
            className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-xs text-zinc-200 hover:bg-zinc-800"
          >
            Save
          </button>
        </div>
        <div className="mt-3 space-y-2">
          {savedSnapshots.length ? (
            savedSnapshots.slice(0, 6).map((snapshot) => (
              <button
                key={snapshot.id}
                onClick={() => onLoadSnapshot(snapshot)}
                className="w-full rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-left hover:bg-zinc-800"
              >
                <p className="text-sm text-zinc-100">{snapshot.name}</p>
                <p className="mt-1 text-xs text-zinc-500">
                  section {snapshot.section_enabled ? `${snapshot.section_constant.toFixed(1)} mm` : 'off'} | {new Date(snapshot.created_at).toLocaleString()}
                </p>
              </button>
            ))
          ) : (
            <p className="text-sm text-zinc-500">No saved inspection snapshots yet.</p>
          )}
        </div>
      </div>

      {overlay ? (
        <div className="mt-5">
          <div className="flex items-center justify-between gap-3">
            <p className="text-sm text-zinc-200">
              {overlay.overlay_type === 'compare' ? 'Compare Overlay' : 'Thickness Heatmap'}
            </p>
            <span className="text-xs text-zinc-500">
              {overlay.overlay_type === 'compare' ? 'Signed distance' : 'Wall thickness'}
            </span>
          </div>
          <div className="mt-3 h-3 rounded-full bg-gradient-to-r from-red-500 via-amber-400 to-green-500" />
          <div className="mt-2 flex items-center justify-between text-xs text-zinc-500">
            <span>{overlay.min_value.toFixed(2)}</span>
            <span>{overlay.center_value.toFixed(2)}</span>
            <span>{overlay.max_value.toFixed(2)}</span>
          </div>
        </div>
      ) : (
        <p className="mt-5 text-sm text-zinc-500">Enable thickness or compare overlay to inspect scalar ranges.</p>
      )}
    </div>
  );
}

function Readout({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-zinc-800 px-2 py-2">
      <p className="text-[10px] uppercase tracking-[0.18em] text-zinc-500">{label}</p>
      <p className="mt-1 text-sm text-zinc-200">{value}</p>
    </div>
  );
}
