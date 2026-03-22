'use client';

export type ToolbarGroup = 'file' | 'prepare' | 'modify' | 'inspect' | 'review';

export type RightDockTab = 'tool' | 'model' | 'review' | 'activity';

export type ReviewPane = 'compare' | 'history';

export type ContextToolId =
  | 'repair'
  | 'fit-size'
  | 'reduce-weight'
  | 'prepare-casting'
  | 'make-manufacturable'
  | 'resize'
  | 'protected-hollow'
  | 'hollow-drains'
  | 'thicken-violations'
  | 'thicken-region'
  | 'batch-thicken'
  | 'scoop'
  | 'smooth'
  | 'batch-smooth'
  | 'section'
  | 'heatmap'
  | 'regions'
  | 'wireframe'
  | 'snapshots';

export type WorkspaceCommandId =
  | 'upload-new'
  | 'download-stl'
  | 'export-section'
  | 'compare-versions'
  | 'version-history'
  | 'restore-branch'
  | 'job-activity'
  | ContextToolId;

export type ToolDrafts = {
  targetRingSize: number;
  targetWeight: number;
  wallThickness: number;
  minThickness: number;
  resizeTargetSize: number;
  thickenTarget: number;
  scoopDepth: number;
  scoopFalloff: number;
  smoothIterations: number;
  smoothStrength: number;
  snapshotName: string;
};

export const DEFAULT_TOOL_DRAFTS: ToolDrafts = {
  targetRingSize: 7,
  targetWeight: 5,
  wallThickness: 0.8,
  minThickness: 0.6,
  resizeTargetSize: 8,
  thickenTarget: 0.8,
  scoopDepth: 0.35,
  scoopFalloff: 1.5,
  smoothIterations: 6,
  smoothStrength: 0.35,
  snapshotName: '',
};
