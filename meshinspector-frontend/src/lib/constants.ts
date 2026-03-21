/**
 * Application constants
 */

export const RING_SIZE_CHART = {
  3: 14.05,
  3.5: 14.45,
  4: 14.86,
  4.5: 15.27,
  5: 15.67,
  5.5: 16.08,
  6: 16.48,
  6.5: 16.89,
  7: 17.30,
  7.5: 17.70,
  8: 18.11,
  8.5: 18.51,
  9: 18.92,
  9.5: 19.33,
  10: 19.73,
  10.5: 20.14,
  11: 20.54,
  11.5: 20.95,
  12: 21.35,
  12.5: 21.76,
  13: 22.16,
} as const;

export const MATERIALS = {
  gold_24k: { label: '24K Gold', density: 19.32 },
  gold_22k: { label: '22K Gold', density: 17.54 },
  gold_18k: { label: '18K Gold', density: 15.58 },
  gold_14k: { label: '14K Gold', density: 13.57 },
  gold_10k: { label: '10K Gold', density: 11.57 },
  silver_925: { label: 'Sterling Silver', density: 10.36 },
  platinum: { label: 'Platinum', density: 21.45 },
} as const;

export const ALLOWED_EXTENSIONS = ['.glb', '.gltf', '.obj', '.stl', '.ply'];

export const MAX_FILE_SIZE_MB = 100;

export const DEFAULT_WALL_THICKNESS_MM = 0.8;
export const MIN_WALL_THICKNESS_MM = 0.3;
export const MAX_WALL_THICKNESS_MM = 5.0;

export const DEFAULT_RING_SIZE = 7;
export const MIN_RING_SIZE = 3;
export const MAX_RING_SIZE = 13;
