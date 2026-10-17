import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import AssociationReview from './AssociationReview';
import { FileAssociationWizardState } from '../../types/associations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const makeFile = (name: string, size = 2048) => {
  const file = new File(['x'], name);
  Object.defineProperty(file, 'size', { value: size });
  return file;
};

const wizardState: FileAssociationWizardState = {
  currentStep: 2,
  selectedAsset: {
    id: 2,
    name: 'PLC-001',
    description: 'Primary PLC',
    assetType: 'Device',
    sortOrder: 0,
    createdBy: 1,
    createdByUsername: 'admin',
    createdAt: '2024-01-01T00:00:00Z',
    versionCount: 0,
  },
  selectedFiles: [],
  uploadedFiles: [
    { file: makeFile('plc.cfg'), fileId: 10, progress: 100, status: 'completed' },
    { file: makeFile('firmware_test.bin'), fileId: 11, progress: 100, status: 'completed' },
    { file: makeFile('pending.cfg'), progress: 20, status: 'uploading' },
  ],
  associations: [],
  validationResults: [],
};

describe('AssociationReview', () => {
  useConsoleGuard();

  it('builds review rows from completed uploads and reports them to the parent', async () => {
    const onAssociationsUpdate = vi.fn();
    render(<AssociationReview wizardState={wizardState} onAssociationsUpdate={onAssociationsUpdate} />);

    expect(screen.getByText('Review & Confirm Associations')).toBeInTheDocument();
    expect(screen.getByText('Target Asset:')).toBeInTheDocument();
    expect(screen.getByText('PLC-001')).toBeInTheDocument();
    expect(screen.getByText('2 of 2 selected')).toBeInTheDocument();

    expect(screen.getByText('plc.cfg')).toBeInTheDocument();
    expect(screen.getByText('firmware_test.bin')).toBeInTheDocument();
    expect(screen.queryByText('pending.cfg')).not.toBeInTheDocument();

    // File type detection + validation: "test" in the name yields a warning
    expect(screen.getByText('1 warnings')).toBeInTheDocument();
    expect(screen.getByText('Validation Warnings')).toBeInTheDocument();
    expect(screen.getByText('Passed')).toBeInTheDocument();
    expect(screen.getByText('Warning')).toBeInTheDocument();
    expect(screen.getByText('Review Instructions')).toBeInTheDocument();

    await waitFor(() => expect(onAssociationsUpdate).toHaveBeenCalled());
    const last = onAssociationsUpdate.mock.calls.at(-1)?.[0];
    expect(last).toEqual([
      expect.objectContaining({ assetId: 2, fileId: 10, fileType: 'Configuration' }),
      expect.objectContaining({ assetId: 2, fileId: 11, fileType: 'Firmware' }),
    ]);
  });

  it('shows the no-selection alert when everything is unchecked', async () => {
    const onAssociationsUpdate = vi.fn();
    render(<AssociationReview wizardState={wizardState} onAssociationsUpdate={onAssociationsUpdate} />);

    const selectAll = screen.getByRole('checkbox', { name: /Select/ });
    fireEvent.click(selectAll);

    expect(await screen.findByText('No Associations Selected')).toBeInTheDocument();
    expect(screen.getByText('0 of 2 selected')).toBeInTheDocument();
    expect(onAssociationsUpdate).toHaveBeenLastCalledWith([]);
  });

  it('renders the empty table when no files were uploaded', () => {
    render(
      <AssociationReview
        wizardState={{ ...wizardState, uploadedFiles: [] }}
        onAssociationsUpdate={vi.fn()}
      />,
    );
    expect(screen.getByText('No file associations to review')).toBeInTheDocument();
    expect(screen.getByText('No Associations Selected')).toBeInTheDocument();
  });
});
