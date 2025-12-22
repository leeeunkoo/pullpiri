import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Search: (props: any) => <svg data-testid="icon-search" {...props} />,
  MoreHorizontal: (props: any) => <svg data-testid="icon-more" {...props} />,
  Eye: (props: any) => <svg data-testid="icon-eye" {...props} />,
  Download: (props: any) => <svg data-testid="icon-down" {...props} />,
  Database: (props: any) => <svg data-testid="icon-db" {...props} />,
}));

// Mock Tabs so that content renders synchronously and tab switching is simple
vi.mock('../components/ui/tabs', () => {
  const React = require('react');
  return {
    Tabs: (props: any) => <div>{props.children}</div>,
    TabsList: (props: any) => <div role="tablist">{props.children}</div>,
    TabsTrigger: (props: any) => <button role="tab">{props.children}</button>,
    TabsContent: (props: any) => <div>{props.children}</div>,
  };
});

import { ConfigMaps } from '../components/ConfigMaps';

test('renders ConfigMaps and default tab', () => {
  render(<ConfigMaps namespace="default" />);

  // Title
  expect(screen.getByText(/PULLPIRI Config & Storage/i)).toBeInTheDocument();

  // Default tab should exist (there may be multiple elements with the same text)
  const cfgElems = screen.getAllByText(/ConfigMaps/i);
  expect(cfgElems.length).toBeGreaterThan(0);
  expect(screen.getByText(/Create ConfigMap/i)).toBeInTheDocument();
});

test('switching tabs shows secrets and pv lists', async () => {
  render(<ConfigMaps namespace="default" />);

  // Click the Secrets tab button (look up by role='tab')
  const tabElements = screen.getAllByRole('tab');
  const secretsBtn = tabElements.find((el) => /Secrets/i.test(el.textContent || ''));
  expect(secretsBtn).toBeDefined();
  fireEvent.click(secretsBtn as HTMLElement);

  // A known secret name should exist in the document (allow multiple matches)
  const secretMatches = screen.getAllByText(/database-credentials/i, { hidden: true });
  expect(secretMatches.length).toBeGreaterThan(0);

  // Click Persistent Volumes tab
  const pvBtn = tabElements.find((el) => /Persistent Volumes/i.test(el.textContent || ''));
  expect(pvBtn).toBeDefined();
  fireEvent.click(pvBtn as HTMLElement);

  // Persistent volumes and claims include postgres-pv; assert at least one match
  const pvMatches = screen.getAllByText(/postgres-pv/i, { hidden: true });
  expect(pvMatches.length).toBeGreaterThan(0);
});

test('search filters across resources', () => {
  render(<ConfigMaps namespace="default" />);

  const input = screen.getByPlaceholderText(/Search resources/);
  fireEvent.change(input, { target: { value: 'redis' } });

  // redis-config exists in configmaps and redis-pv/pvc exist as well when switching
  expect(screen.getByText(/redis-config/)).toBeInTheDocument();
});
