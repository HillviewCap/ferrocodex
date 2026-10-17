import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MetadataConfigurationStep } from './MetadataConfigurationStep';
import { useConsoleGuard } from '../../../test-utils/consoleGuard';

vi.mock('../../forms/DynamicMetadataForm', () => ({
  default: () => <div data-testid="dynamic-metadata-form" />,
}));

const stepProps = () => ({
  workflowId: 'wf-1',
  onDataChange: vi.fn(),
  onValidation: vi.fn(),
  onNext: vi.fn(),
  onPrevious: vi.fn(),
});

describe('MetadataConfigurationStep', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders device metadata with the configuration alert when a schema is selected', () => {
    const props = stepProps();
    render(
      <MetadataConfigurationStep
        {...props}
        data={{ asset_type: 'Device', asset_name: 'PLC-001', parent_path: 'Plant A / Line 1', metadata_schema_id: 3 }}
      />,
    );

    expect(screen.getByText('Device Metadata')).toBeInTheDocument();
    expect(screen.getByText('Asset: PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Type: Device • Location: Plant A / Line 1')).toBeInTheDocument();
    expect(screen.getByTestId('dynamic-metadata-form')).toBeInTheDocument();
    expect(screen.getByText('Metadata Configuration')).toBeInTheDocument();
    expect(screen.getByText('Metadata Guidelines')).toBeInTheDocument();

    // Schema chosen but the form has not reported valid yet
    expect(props.onValidation).toHaveBeenCalledWith(
      false,
      expect.arrayContaining([expect.objectContaining({ field: 'metadata_values' })]),
    );
  });

  it('renders folder metadata without the alert when no schema is selected', () => {
    const props = stepProps();
    render(<MetadataConfigurationStep {...props} data={{ asset_type: 'Folder', asset_name: 'Plant A' }} />);

    expect(screen.getByText('Folder Metadata')).toBeInTheDocument();
    expect(screen.getByText('Type: Folder • Location: Root Level')).toBeInTheDocument();
    expect(screen.queryByText('Metadata Configuration')).not.toBeInTheDocument();
    expect(props.onValidation).toHaveBeenCalledWith(
      false,
      expect.arrayContaining([expect.objectContaining({ field: 'metadata_schema_id' })]),
    );
  });
});
