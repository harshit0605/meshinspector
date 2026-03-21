'use client';

import { create } from 'zustand';
import type { MaterialType } from '@/lib/api/types';

type EditorStore = {
  wireframe: boolean;
  sectionEnabled: boolean;
  sectionConstant: number;
  heatmapEnabled: boolean;
  regionOverlayEnabled: boolean;
  selectedRegionId: string | null;
  selectedRegionIds: string[];
  compareOverlayEnabled: boolean;
  compareTargetVersionId: string | null;
  selectedMaterial: MaterialType;
  setWireframe: (value: boolean) => void;
  setSectionEnabled: (value: boolean) => void;
  setSectionConstant: (value: number) => void;
  setHeatmapEnabled: (value: boolean) => void;
  setRegionOverlayEnabled: (value: boolean) => void;
  setSelectedRegionId: (value: string | null) => void;
  setSelectedRegionIds: (value: string[]) => void;
  toggleSelectedRegionId: (value: string) => void;
  setCompareOverlayEnabled: (value: boolean) => void;
  setCompareTargetVersionId: (value: string | null) => void;
  setSelectedMaterial: (value: MaterialType) => void;
};

export const useEditorStore = create<EditorStore>((set) => ({
  wireframe: false,
  sectionEnabled: false,
  sectionConstant: 0,
  heatmapEnabled: false,
  regionOverlayEnabled: true,
  selectedRegionId: null,
  selectedRegionIds: [],
  compareOverlayEnabled: false,
  compareTargetVersionId: null,
  selectedMaterial: 'gold_18k',
  setWireframe: (wireframe) => set({ wireframe }),
  setSectionEnabled: (sectionEnabled) => set({ sectionEnabled }),
  setSectionConstant: (sectionConstant) => set({ sectionConstant }),
  setHeatmapEnabled: (heatmapEnabled) => set({ heatmapEnabled }),
  setRegionOverlayEnabled: (regionOverlayEnabled) => set({ regionOverlayEnabled }),
  setSelectedRegionId: (selectedRegionId) =>
    set((state) => ({
      selectedRegionId,
      selectedRegionIds:
        selectedRegionId == null
          ? []
          : [selectedRegionId, ...state.selectedRegionIds.filter((regionId) => regionId !== selectedRegionId)],
    })),
  setSelectedRegionIds: (selectedRegionIds) =>
    set((state) => {
      const deduped = Array.from(new Set(selectedRegionIds.filter(Boolean)));
      return {
        selectedRegionIds: deduped,
        selectedRegionId: deduped.includes(state.selectedRegionId ?? '') ? state.selectedRegionId : (deduped[0] ?? null),
      };
    }),
  toggleSelectedRegionId: (regionId) =>
    set((state) => {
      const active = state.selectedRegionIds.includes(regionId);
      const nextIds = active
        ? state.selectedRegionIds.filter((id) => id !== regionId)
        : [...state.selectedRegionIds, regionId];
      const nextPrimary = active
        ? state.selectedRegionId === regionId
          ? (nextIds[0] ?? null)
          : state.selectedRegionId
        : regionId;
      return {
        selectedRegionIds: nextIds,
        selectedRegionId: nextPrimary,
      };
    }),
  setCompareOverlayEnabled: (compareOverlayEnabled) => set({ compareOverlayEnabled }),
  setCompareTargetVersionId: (compareTargetVersionId) => set({ compareTargetVersionId }),
  setSelectedMaterial: (selectedMaterial) => set({ selectedMaterial }),
}));
