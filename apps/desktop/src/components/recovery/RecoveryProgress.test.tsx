import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { RecoveryProgress } from './RecoveryProgress';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

describe('RecoveryProgress', () => {
  useConsoleGuard();

  it('renders the in-progress state with a cancel action', () => {
    const onCancel = vi.fn();
    render(
      <RecoveryProgress
        progress={{ step: 'exporting_firmware', progress: 45, message: 'Writing firmware image...' }}
        onCancel={onCancel}
      />,
    );

    expect(screen.getByText('Exporting Firmware')).toBeInTheDocument();
    expect(screen.getByText('Writing firmware image...')).toBeInTheDocument();
    expect(screen.getByText('45%')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Cancel/ }));
    expect(onCancel).toHaveBeenCalled();
  });

  it('renders timing statistics and the completed banner', () => {
    render(
      <RecoveryProgress
        progress={{
          step: 'completed',
          progress: 100,
          message: 'Done',
          timing: { config_export_ms: 250, firmware_export_ms: 1500, total_ms: 1900 },
        }}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByText('Export Complete')).toBeInTheDocument();
    expect(screen.getByText('Config Export')).toBeInTheDocument();
    expect(screen.getByText('250ms')).toBeInTheDocument();
    expect(screen.getByText('Firmware Export')).toBeInTheDocument();
    expect(screen.getByText('1.50s')).toBeInTheDocument();
    expect(screen.getByText('Total Time')).toBeInTheDocument();
    expect(screen.getByText('1.90s')).toBeInTheDocument();
    expect(screen.getByText('Recovery package exported successfully!')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Cancel/ })).not.toBeInTheDocument();
  });

  it('renders the error state', () => {
    render(<RecoveryProgress progress={{ step: 'error', progress: 60, message: 'Disk full' }} />);

    expect(screen.getByText('Export Failed')).toBeInTheDocument();
    expect(screen.getByText('Disk full')).toBeInTheDocument();
    expect(screen.getByText('Export failed. Please try again.')).toBeInTheDocument();
  });
});
