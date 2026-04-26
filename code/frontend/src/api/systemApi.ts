import client from './client';

export interface HealthResponse {
  status: string;
}

export async function fetchHealth(): Promise<HealthResponse> {
  const { data } = await client.get<HealthResponse>('/health');
  return data;
}

export async function isServerHealthy(): Promise<boolean> {
  try {
    const health = await fetchHealth();
    return health.status === 'ok';
  } catch {
    return false;
  }
}
