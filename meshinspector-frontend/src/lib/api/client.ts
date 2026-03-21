/**
 * API client with fetch wrapper
 */

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public detail?: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE}${endpoint}`;
  
  const response = await fetch(url, {
    ...options,
    headers: {
      ...options.headers,
    },
  });
  
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    throw new ApiError(
      errorData.detail || `Request failed: ${response.statusText}`,
      response.status,
      errorData.detail
    );
  }
  
  // Handle file downloads
  if (response.headers.get('content-type')?.includes('model/')) {
    return response.blob() as unknown as T;
  }
  
  return response.json();
}

export function getPreviewUrl(modelId: string): string {
  return `${API_BASE}/api/preview/${modelId}`;
}

export function getDownloadUrl(modelId: string, format: 'glb' | 'stl'): string {
  return `${API_BASE}/api/download/${modelId}/${format}`;
}

export function getArtifactUrl(artifactId: string): string {
  return `${API_BASE}/api/artifacts/${artifactId}`;
}
