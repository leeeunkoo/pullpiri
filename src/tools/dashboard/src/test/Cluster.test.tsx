import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { Cluster } from '../components/Cluster';
import { vi } from 'vitest';

// Mock lucide-react icons used in Cluster
vi.mock('lucide-react', () => {
  return {
    Search: (props: any) => <svg data-testid="icon-search" {...props} />,
    MoreHorizontal: (props: any) => <svg data-testid="icon-more" {...props} />,
    Plus: (props: any) => <svg data-testid="icon-plus" {...props} />,
    Server: (props: any) => <svg data-testid="icon-server" {...props} />,
    Cpu: (props: any) => <svg data-testid="icon-cpu" {...props} />,
    MemoryStick: (props: any) => <svg data-testid="icon-mem" {...props} />,
    HardDrive: (props: any) => <svg data-testid="icon-drive" {...props} />,
  };
});

test('renders Cluster and displays nodes', () => {
  render(<Cluster />);

  // There should be a title
  expect(screen.getByText(/PULLPIRI Nodes/i)).toBeInTheDocument();

  // Nodes count badge should show 4
  expect(screen.getByText(/4 Nodes/)).toBeInTheDocument();

  // Table rows for nodes (names) should be present
  expect(screen.getByText(/master-node-1/)).toBeInTheDocument();
  expect(screen.getByText(/worker-node-1/)).toBeInTheDocument();
});

test('search filters nodes by name', () => {
  render(<Cluster />);

  const input = screen.getByPlaceholderText(/Search nodes/i) as HTMLInputElement;
  expect(input).toBeInTheDocument();

  // Type a search that matches only worker-node-2
  fireEvent.change(input, { target: { value: 'worker-node-2' } });

  // worker-node-2 should be visible, others not
  expect(screen.getByText(/worker-node-2/)).toBeInTheDocument();
  expect(screen.queryByText(/master-node-1/)).toBeNull();
});
