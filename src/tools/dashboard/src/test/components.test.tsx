import React from 'react';
import { render } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

import { Dashboard } from '../components/Dashboard';
import { Header } from '../components/Header';
import { Workloads } from '../components/Workloads';
import { PodDetail } from '../components/PodDetail';
import { Scenarios } from '../components/Scenarios';
import { Sidebar } from '../components/Sidebar';
import { ThemeProvider } from '../components/ThemeProvider';
import { YamlEditor } from '../components/YamlEditor';

describe('Component smoke renders', () => {
  it('renders Dashboard without crashing', () => {
    const { container } = render(
      <ThemeProvider>
        <Dashboard />
      </ThemeProvider> as any,
    );
    expect(container).toBeTruthy();
  });

  it('renders Header with props', () => {
    const { container: headerContainer } = render(
      <ThemeProvider>
        <Header compact={false} mobile={false} podCount={0 as any} pods={[]} />
      </ThemeProvider> as any,
    );
    expect(headerContainer).toBeTruthy();
  });

  it('renders Workloads with minimal props', () => {
    const { container: workloadsContainer } = render(<Workloads onPodClick={() => {}} pods={[]} setPods={() => {}} recentEvents={[]} setRecentEvents={() => {}} /> as any);
    expect(workloadsContainer).toBeTruthy();
  });

  it('renders PodDetail with minimal props', () => {
    const { container: podDetailContainer } = render(<PodDetail podName={"p1"} podData={{ labels: { app: 'x' } } as any} onBack={() => {}} /> as any);
    expect(podDetailContainer).toBeTruthy();
  });

  it('renders Scenarios and Sidebar', () => {
    render(<Scenarios namespace={"default"} /> as any);
    render(<Sidebar compact={false} /> as any);
  });

  it('ThemeProvider and YamlEditor mount', () => {
    const { container: yamlContainer } = render(
      <ThemeProvider>
        <YamlEditor open={false} onOpenChange={() => {}} podName={undefined as any} />
      </ThemeProvider>,
    );
    expect(yamlContainer).toBeTruthy();
  });
});
