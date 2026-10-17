import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import PerformanceMonitor from './PerformanceMonitor';
import { TreePerformanceMonitor } from '../../utils/treePerformance';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

describe('PerformanceMonitor', () => {
  useConsoleGuard();

  let monitor: TreePerformanceMonitor;

  beforeEach(() => {
    monitor = new TreePerformanceMonitor();
  });

  afterEach(() => {
    monitor.destroy();
  });

  it('renders the full view with core metric statistics and collapse panels', () => {
    render(
      <PerformanceMonitor
        monitor={monitor}
        treeSize={1500}
        maxDepth={6}
        visible
        enableRealtimeUpdates={false}
      />,
    );

    expect(screen.getByText('Performance Monitor')).toBeInTheDocument();
    expect(screen.getByText('Core Metrics')).toBeInTheDocument();
    expect(screen.getByText('Render Time')).toBeInTheDocument();
    expect(screen.getByText('Frame Rate')).toBeInTheDocument();
    expect(screen.getByText('Memory Usage')).toBeInTheDocument();
    expect(screen.getByText('Last Update')).toBeInTheDocument();

    // Collapse headers
    expect(screen.getByText('Tree Statistics')).toBeInTheDocument();
    expect(screen.getByText('Performance Tips')).toBeInTheDocument();
  });

  it('expands the collapse panels on click', () => {
    render(
      <PerformanceMonitor
        monitor={monitor}
        treeSize={1500}
        maxDepth={6}
        visible
        enableRealtimeUpdates={false}
      />,
    );

    fireEvent.click(screen.getByText('Tree Statistics'));
    expect(screen.getByText('Tree Size')).toBeInTheDocument();
    expect(screen.getByText('Max Depth')).toBeInTheDocument();
    expect(screen.getByText('Cache Hit Rate')).toBeInTheDocument();
    expect(screen.getByText('1,500')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Performance Tips'));
    expect(screen.getByText(/Virtualization improves performance/)).toBeInTheDocument();
  });

  it('shows recommendations from the monitor when real-time updates are enabled', () => {
    vi.spyOn(monitor, 'getRecommendations').mockReturnValue([
      {
        type: 'virtualization',
        priority: 'high',
        description: 'Enable virtualization for large trees',
        impact: 'Reduces render time by 80%',
        action: () => {},
      } as never,
    ]);

    render(
      <PerformanceMonitor
        monitor={monitor}
        treeSize={1500}
        maxDepth={6}
        visible
        enableRealtimeUpdates
      />,
    );

    expect(screen.getByText('Performance Recommendations')).toBeInTheDocument();
    expect(screen.getByText('Enable virtualization for large trees')).toBeInTheDocument();
    expect(screen.getByText(/Impact: Reduces render time by 80%/)).toBeInTheDocument();
    expect(screen.getByText('high')).toBeInTheDocument();
  });

  it('renders the compact bar and the floating toggle when hidden', () => {
    const onToggle = vi.fn();
    const { rerender } = render(
      <PerformanceMonitor monitor={monitor} treeSize={10} maxDepth={2} visible={false} compact onToggleVisibility={onToggle} />,
    );
    expect(screen.getByRole('switch')).toBeInTheDocument();
    expect(screen.getByText(/ms$/)).toBeInTheDocument();

    rerender(
      <PerformanceMonitor monitor={monitor} treeSize={10} maxDepth={2} visible={false} onToggleVisibility={onToggle} />,
    );
    fireEvent.click(screen.getByRole('button'));
    expect(onToggle).toHaveBeenCalledWith(true);
  });
});
