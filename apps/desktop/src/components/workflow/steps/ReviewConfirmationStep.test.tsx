import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ReviewConfirmationStep } from './ReviewConfirmationStep';
import { useConsoleGuard } from '../../../test-utils/consoleGuard';

const stepProps = () => ({
  workflowId: 'wf-1',
  onDataChange: vi.fn(),
  onValidation: vi.fn(),
  onNext: vi.fn(),
  onPrevious: vi.fn(),
  onComplete: vi.fn(),
});

describe('ReviewConfirmationStep', () => {
  useConsoleGuard();

  it('renders the summary Descriptions from items with every label and value', () => {
    const props = stepProps();
    render(
      <ReviewConfirmationStep
        {...props}
        data={{
          asset_type: 'Device',
          asset_name: 'PLC-001',
          asset_description: 'Primary line controller',
          parent_id: 1,
          parent_path: 'Plant A / Line 1',
          security_classification: 'CONFIDENTIAL',
          metadata_schema_id: 3,
          metadata_values: { ip_address: '10.0.0.5', port: 502 },
        }}
      />,
    );

    expect(screen.getByText('Review & Confirm')).toBeInTheDocument();
    for (const label of ['Name', 'Description', 'Type', 'Location', 'Security Classification']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getAllByText('PLC-001').length).toBeGreaterThan(0);
    expect(screen.getByText('Primary line controller')).toBeInTheDocument();
    expect(screen.getAllByText('Device').length).toBeGreaterThan(0);
    expect(screen.getByText('Plant A / Line 1')).toBeInTheDocument();
    expect(screen.getByText('CONFIDENTIAL')).toBeInTheDocument();

    expect(screen.getByText('Schema ID:')).toBeInTheDocument();
    expect(screen.getByText('ip_address:')).toBeInTheDocument();
    expect(screen.getByText('10.0.0.5')).toBeInTheDocument();
    expect(screen.getByText('502')).toBeInTheDocument();
    expect(screen.getByText('What happens next?')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Create Asset' }));
    expect(props.onComplete).toHaveBeenCalled();
  });

  it('renders fallbacks for a bare root-level folder', () => {
    render(<ReviewConfirmationStep {...stepProps()} data={{ asset_type: 'Folder', asset_name: 'Plant A', parent_id: null }} />);

    expect(screen.getByText('No description provided')).toBeInTheDocument();
    expect(screen.getByText('Root Level')).toBeInTheDocument();
    expect(screen.getByText('Not set')).toBeInTheDocument();
    expect(screen.getByText('No metadata schema selected')).toBeInTheDocument();
    expect(screen.getByText(/You can also create additional assets within this folder/)).toBeInTheDocument();
  });
});
