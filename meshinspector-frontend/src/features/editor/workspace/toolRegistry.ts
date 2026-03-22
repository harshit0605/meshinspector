'use client';

import type { ContextToolId, ReviewPane, RightDockTab, ToolbarGroup, WorkspaceCommandId } from './types';

export type CommandSurface = 'direct' | 'tool' | 'review' | 'activity';

export type WorkspaceCommandDefinition = {
  id: WorkspaceCommandId;
  group: ToolbarGroup;
  label: string;
  description: string;
  icon: string;
  surface: CommandSurface;
  dockTab?: RightDockTab;
  reviewPane?: ReviewPane;
  contextualToolId?: ContextToolId;
};

export const TOOLBAR_GROUP_ORDER: ToolbarGroup[] = ['file', 'prepare', 'modify', 'inspect', 'review'];

export const TOOLBAR_GROUP_LABELS: Record<ToolbarGroup, string> = {
  file: 'File',
  prepare: 'Prepare',
  modify: 'Modify',
  inspect: 'Inspect',
  review: 'Review',
};

export const WORKSPACE_COMMANDS: WorkspaceCommandDefinition[] = [
  {
    id: 'upload-new',
    group: 'file',
    label: 'Upload New',
    description: 'Start a new workspace from another model.',
    icon: 'UP',
    surface: 'direct',
  },
  {
    id: 'download-stl',
    group: 'file',
    label: 'Download STL',
    description: 'Export the current version manufacturing STL.',
    icon: 'ST',
    surface: 'direct',
  },
  {
    id: 'export-section',
    group: 'file',
    label: 'Export Section SVG',
    description: 'Export the current section contour as SVG.',
    icon: 'SV',
    surface: 'direct',
  },
  {
    id: 'repair',
    group: 'prepare',
    label: 'Auto Repair',
    description: 'Heal holes, degeneracies, and blockers before editing.',
    icon: 'AR',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'repair',
  },
  {
    id: 'fit-size',
    group: 'prepare',
    label: 'Fit To Size',
    description: 'Resize to a production ring size with axis-aware scaling.',
    icon: 'FS',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'fit-size',
  },
  {
    id: 'reduce-weight',
    group: 'prepare',
    label: 'Reduce Weight',
    description: 'Target a weight class while protecting detail-heavy regions.',
    icon: 'RW',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'reduce-weight',
  },
  {
    id: 'prepare-casting',
    group: 'prepare',
    label: 'Prepare For Casting',
    description: 'Build a castable hollow shell with conservative drain holes.',
    icon: 'PC',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'prepare-casting',
  },
  {
    id: 'make-manufacturable',
    group: 'prepare',
    label: 'Make Manufacturable',
    description: 'Run the guided repair, size, optimize, and validate flow.',
    icon: 'MM',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'make-manufacturable',
  },
  {
    id: 'resize',
    group: 'modify',
    label: 'Resize',
    description: 'Create a resized version with preserved ornament-heavy regions.',
    icon: 'RS',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'resize',
  },
  {
    id: 'protected-hollow',
    group: 'modify',
    label: 'Protected Hollow',
    description: 'Build a weighted shell while protecting decorative regions.',
    icon: 'PH',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'protected-hollow',
  },
  {
    id: 'hollow-drains',
    group: 'modify',
    label: 'Hollow + Drains',
    description: 'Build the protected shell and add drain holes.',
    icon: 'HD',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'hollow-drains',
  },
  {
    id: 'thicken-violations',
    group: 'modify',
    label: 'Thicken Violations',
    description: 'Only thicken unsafe areas below the minimum target.',
    icon: 'TV',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'thicken-violations',
  },
  {
    id: 'thicken-region',
    group: 'modify',
    label: 'Thicken Region',
    description: 'Apply a localized outward thickening pass to the primary region.',
    icon: 'TR',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'thicken-region',
  },
  {
    id: 'batch-thicken',
    group: 'modify',
    label: 'Batch Thicken',
    description: 'Thicken all batch-selected regions in one pass.',
    icon: 'BT',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'batch-thicken',
  },
  {
    id: 'scoop',
    group: 'modify',
    label: 'Scoop',
    description: 'Carve a controlled recess into a scoop-safe region.',
    icon: 'SC',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'scoop',
  },
  {
    id: 'smooth',
    group: 'modify',
    label: 'Smooth',
    description: 'Run a conservative localized or global smoothing pass.',
    icon: 'SM',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'smooth',
  },
  {
    id: 'batch-smooth',
    group: 'modify',
    label: 'Batch Smooth',
    description: 'Smooth all batch-selected regions with one command.',
    icon: 'BS',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'batch-smooth',
  },
  {
    id: 'section',
    group: 'inspect',
    label: 'Section',
    description: 'Inspect slice position, presets, contour stats, and export.',
    icon: 'SE',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'section',
  },
  {
    id: 'heatmap',
    group: 'inspect',
    label: 'Heatmap',
    description: 'Inspect wall thickness as a scalar overlay.',
    icon: 'HM',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'heatmap',
  },
  {
    id: 'regions',
    group: 'inspect',
    label: 'Regions',
    description: 'Inspect region coverage, selection, and allowed operations.',
    icon: 'RG',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'regions',
  },
  {
    id: 'wireframe',
    group: 'inspect',
    label: 'Wireframe',
    description: 'Toggle mesh wireframe for topology inspection.',
    icon: 'WF',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'wireframe',
  },
  {
    id: 'snapshots',
    group: 'inspect',
    label: 'Inspection Snapshots',
    description: 'Save and restore inspection states for repeated checks.',
    icon: 'IS',
    surface: 'tool',
    dockTab: 'tool',
    contextualToolId: 'snapshots',
  },
  {
    id: 'compare-versions',
    group: 'review',
    label: 'Compare Versions',
    description: 'Review cached compare overlays and switch compare targets.',
    icon: 'CV',
    surface: 'review',
    dockTab: 'review',
    reviewPane: 'compare',
  },
  {
    id: 'version-history',
    group: 'review',
    label: 'Version History',
    description: 'Open, compare, and branch historical versions.',
    icon: 'VH',
    surface: 'review',
    dockTab: 'review',
    reviewPane: 'history',
  },
  {
    id: 'restore-branch',
    group: 'review',
    label: 'Restore As Branch',
    description: 'Branch from an existing version in review mode.',
    icon: 'RB',
    surface: 'review',
    dockTab: 'review',
    reviewPane: 'history',
  },
  {
    id: 'job-activity',
    group: 'review',
    label: 'Job Activity',
    description: 'Monitor job progress and event history.',
    icon: 'JA',
    surface: 'activity',
    dockTab: 'activity',
  },
];

export const COMMANDS_BY_GROUP = TOOLBAR_GROUP_ORDER.reduce<Record<ToolbarGroup, WorkspaceCommandDefinition[]>>(
  (accumulator, group) => {
    accumulator[group] = WORKSPACE_COMMANDS.filter((command) => command.group === group);
    return accumulator;
  },
  {
    file: [],
    prepare: [],
    modify: [],
    inspect: [],
    review: [],
  },
);

export function isContextToolId(value: WorkspaceCommandId | null): value is ContextToolId {
  return !!value && WORKSPACE_COMMANDS.some((command) => command.contextualToolId === value);
}
