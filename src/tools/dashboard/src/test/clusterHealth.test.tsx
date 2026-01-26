import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useClusterHealth } from '../components/ui/use-cluster-health';

describe('useClusterHealth', () => {
  it('returns Healthy for all running pods', () => {
    const pods = [
      { name: 'a', status: 'Running' },
      { name: 'b', status: 'Running' },
    ] as any;
    const { result } = renderHook(() => useClusterHealth(pods as any));
    expect(result.current.status).toBe('Healthy');
    expect(result.current.runningPods).toBe(2);
  });

  it('returns Warning when pending pods exceed threshold', () => {
    const pods = [
      { name: 'a', status: 'Running' },
      { name: 'b', status: 'Pending' },
      { name: 'c', status: 'Pending' },
      { name: 'd', status: 'Running' },
    ] as any;
    const { result } = renderHook(() => useClusterHealth(pods as any));
    expect(result.current.status).toBe('Warning');
  });

  it('returns Critical when failed pods exceed 20%', () => {
    const pods = [
      { name: 'a', status: 'Running' },
      { name: 'b', status: 'Failed' },
      { name: 'c', status: 'Running' },
      { name: 'd', status: 'Running' },
    ] as any;
    const { result } = renderHook(() => useClusterHealth(pods as any));
    expect(result.current.status).toBe('Critical');
  });
});
