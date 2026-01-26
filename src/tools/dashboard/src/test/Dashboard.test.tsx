import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import { Dashboard } from '../components/Dashboard';

// Mock child components to keep test focused on Dashboard behavior
vi.mock('../components/Sidebar', () => ({ Sidebar: (props: any) => <div data-testid="mock-sidebar" /> }));
vi.mock('../components/Header', () => ({ Header: (props: any) => <div data-testid="mock-header" /> }));
vi.mock('../components/Overview', () => ({ Overview: () => <div data-testid="mock-overview" /> }));
vi.mock('../components/Workloads', () => ({ Workloads: (props: any) => <div data-testid="mock-workloads">Workloads</div> }));
vi.mock('../components/Services', () => ({ Services: () => <div data-testid="mock-services" /> }));
vi.mock('../components/Storage', () => ({ Storage: () => <div data-testid="mock-storage" /> }));
vi.mock('../components/Cluster', () => ({ Cluster: () => <div data-testid="mock-cluster" /> }));
vi.mock('../components/Scenarios', () => ({ Scenarios: (props: any) => <div data-testid="mock-scenarios" /> }));
vi.mock('../components/PodDetail', () => ({ PodDetail: (props: any) => <div data-testid="mock-poddetail" /> }));

// Provide a basic env and fetch mock
const originalEnv = { ...process.env };

beforeAll(() => {
  // Provide fallback for import.meta.env for the test if consumed at runtime
  try {
    (globalThis as any).importMeta = (globalThis as any).importMeta || {};
    (globalThis as any).importMeta.env = { VITE_SETTING_SERVICE_API_URL: 'http://localhost:12345', VITE_SETTING_SERVICE_TIMEOUT: 100 };
  } catch (e) {
    // ignore
  }

  // Mock fetch to return an empty metrics array by default
  global.fetch = vi.fn(() =>
    Promise.resolve({ json: () => Promise.resolve([]) } as any)
  ) as any;
});

afterAll(() => {
  process.env = originalEnv;
  vi.resetAllMocks();
  // cleanup any global fetch
  try {
    // @ts-ignore
    delete (global as any).fetch;
  } catch (e) {}
});

test('renders Dashboard and child placeholders', async () => {
  render(<Dashboard />);

  expect(screen.getAllByTestId('mock-sidebar').length).toBeGreaterThan(0);
  expect(screen.getAllByTestId('mock-header').length).toBeGreaterThan(0);

  // Workloads is the default view (one per layout breakpoint)
  await waitFor(() => expect(screen.getAllByTestId('mock-workloads').length).toBeGreaterThan(0));
});

test('handles fetch failure gracefully', async () => {
  // Override fetch to throw
  (global.fetch as any).mockImplementationOnce(() => Promise.reject(new Error('network')));

  render(<Dashboard />);

  // Should still render header and sidebar (at least one of each layout)
  expect(screen.getAllByTestId('mock-header').length).toBeGreaterThan(0);
  expect(screen.getAllByTestId('mock-sidebar').length).toBeGreaterThan(0);
});
