import { render, screen, fireEvent, within, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import FileAssociationWizard from './FileAssociationWizard';
import useAssociationStore from '../../store/associations';
import { AssetInfo } from '../../types/associations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('./AssetFileSelector', () => ({
  default: ({ onAssetSelect }: { onAssetSelect: (a: AssetInfo) => void }) => (
    <div data-testid="asset-file-selector">
      <button onClick={() => onAssetSelect(preselectedAsset)}>pick asset</button>
    </div>
  ),
}));
vi.mock('./FileUploadStep', () => ({ default: () => <div data-testid="file-upload-step" /> }));
vi.mock('./AssociationReview', () => ({ default: () => <div data-testid="association-review" /> }));

const preselectedAsset: AssetInfo = {
  id: 2,
  name: 'PLC-001',
  description: 'Primary PLC',
  assetType: 'Device',
  sortOrder: 0,
  createdBy: 1,
  createdByUsername: 'admin',
  createdAt: '2024-01-01T00:00:00Z',
  versionCount: 0,
};

describe('FileAssociationWizard', () => {
  useConsoleGuard();

  beforeEach(() => {
    useAssociationStore.getState().reset();
  });

  it('renders the step list from items and starts on the asset step', () => {
    render(<FileAssociationWizard visible onClose={vi.fn()} />);

    expect(screen.getByText('File Association Wizard')).toBeInTheDocument();
    const steps = within(document.querySelector('.ant-steps') as HTMLElement);
    for (const title of ['Select Asset', 'Upload Files', 'Review & Confirm']) {
      expect(steps.getByText(title)).toBeInTheDocument();
    }
    for (const content of ['Choose target asset', 'Upload configuration/firmware files', 'Review associations and complete']) {
      expect(steps.getByText(content)).toBeInTheDocument();
    }

    expect(screen.getByTestId('asset-file-selector')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
  });

  it('advances to the upload step after an asset is selected', () => {
    render(<FileAssociationWizard visible onClose={vi.fn()} />);

    fireEvent.click(screen.getByText('pick asset'));
    const next = screen.getByRole('button', { name: 'Next' });
    expect(next).toBeEnabled();
    fireEvent.click(next);

    expect(screen.getByTestId('file-upload-step')).toBeInTheDocument();
    expect(useAssociationStore.getState().wizardState?.currentStep).toBe(1);

    fireEvent.click(screen.getByRole('button', { name: 'Previous' }));
    expect(screen.getByTestId('asset-file-selector')).toBeInTheDocument();
  });

  it('shows the review step with a complete button when the store is on step 2', () => {
    render(<FileAssociationWizard visible onClose={vi.fn()} preselectedAsset={preselectedAsset} />);
    act(() => {
      useAssociationStore.getState().updateWizardState({ currentStep: 2, associations: [] });
    });

    expect(screen.getByTestId('association-review')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Complete Import' })).toBeDisabled();
  });

  it('renders a closable store error and cancels the wizard on close', () => {
    const onClose = vi.fn();
    render(<FileAssociationWizard visible onClose={onClose} />);
    act(() => {
      useAssociationStore.setState({ error: 'association failed' });
    });

    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('association failed')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'close' }));
    expect(useAssociationStore.getState().error).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onClose).toHaveBeenCalled();
    // cancelWizard clears the state; while the modal stays visible the effect re-initialises
    // a fresh wizard, so the observable outcome is a reset to step 0.
    expect(useAssociationStore.getState().wizardState?.currentStep).toBe(0);
  });
});
