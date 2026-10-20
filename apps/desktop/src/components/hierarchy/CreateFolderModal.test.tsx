import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { CreateFolderModal } from './CreateFolderModal';
import { AssetHierarchy } from '../../types/assets';
import { SecurityClassificationLevel } from '../../types/security';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const restricted: AssetHierarchy = {
  ...base, id: 3, name: 'Safety Systems', asset_type: 'Folder', parent_id: 1, children: [],
  security_classification: SecurityClassificationLevel.RESTRICTED,
};
const plant: AssetHierarchy = {
  ...base, id: 1, name: 'Plant A', asset_type: 'Folder', parent_id: null, children: [restricted],
  security_classification: SecurityClassificationLevel.CONFIDENTIAL,
};
const hierarchy = [plant];
// Only a device at the root: no folder to pick
const devicesOnly: AssetHierarchy[] = [
  { ...base, id: 5, name: 'PLC-5', asset_type: 'Device', parent_id: null, children: [], security_classification: SecurityClassificationLevel.INTERNAL },
];

const setRole = (role: 'Administrator' | 'Engineer') =>
  useAuthStore.setState({
    token: 'test-token',
    user: { id: 1, username: 'user', role, created_at: '2024-01-01', is_active: true },
  });

const optionFor = (label: string) =>
  Array.from(document.querySelectorAll('.ant-select-item-option')).find(option => option.textContent?.includes(label));

describe('CreateFolderModal', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    setRole('Administrator');
    vi.mocked(invoke).mockResolvedValue({ id: 20, name: 'New Folder' });
  });

  it('is a single form titled Create Folder with no wizard', () => {
    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    expect(screen.getAllByText('Create Folder').length).toBeGreaterThan(0);
    expect(document.querySelector('.ant-steps')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Folder Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Description')).toBeInTheDocument();
    expect(screen.getByText('Security Classification')).toBeInTheDocument();
    expect(screen.queryByText(/Asset Type/)).not.toBeInTheDocument();
  });

  it('hides the parent field when the hierarchy has no folders', () => {
    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={devicesOnly} />);
    expect(screen.queryByText('Parent Folder')).not.toBeInTheDocument();

    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={[]} />);
    expect(screen.queryByText('Parent Folder')).not.toBeInTheDocument();
  });

  it('shows the parent field when folders exist, at root by default', () => {
    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    expect(screen.getByText('Parent Folder')).toBeInTheDocument();
    expect(screen.getByText('Asset will be created at the root level')).toBeInTheDocument();
    // No inherited level at the root
    expect(screen.queryByText(/Inherited Classification/)).not.toBeInTheDocument();
  });

  it('defaults to the invoking folder and pre-fills its classification, disabling lower levels', async () => {
    const user = userEvent.setup();
    render(
      <CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} initialParentId={1} />
    );

    // Folder is selected in the picker (also shown in the tree title) and its level is inherited
    expect(screen.getByText('Asset will be created inside the selected folder')).toBeInTheDocument();
    expect(screen.getByText('Plant A')).toBeInTheDocument();
    expect(screen.getByText('Inherited Classification: Confidential')).toBeInTheDocument();

    const [, classificationSelect] = screen.getAllByRole('combobox');
    await user.click(classificationSelect);
    await waitFor(() => expect(optionFor('Public')).toBeDefined());
    expect(optionFor('Public')).toHaveClass('ant-select-item-option-disabled');
    expect(optionFor('Internal')).toHaveClass('ant-select-item-option-disabled');
    expect(optionFor('Confidential')).not.toHaveClass('ant-select-item-option-disabled');
    expect(optionFor('Secret')).not.toHaveClass('ant-select-item-option-disabled');
  });

  it('keeps an inherited RESTRICTED level selectable for an Engineer', async () => {
    setRole('Engineer');
    const user = userEvent.setup();
    render(
      <CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} initialParentId={3} />
    );

    expect(screen.getByText('Inherited Classification: Restricted')).toBeInTheDocument();
    // Inheriting the level isn't an elevation request
    expect(screen.queryByText('Administrator Approval Required')).not.toBeInTheDocument();

    const [, classificationSelect] = screen.getAllByRole('combobox');
    await user.click(classificationSelect);
    await waitFor(() => expect(optionFor('Restricted')).toBeDefined());
    expect(optionFor('Confidential')).toHaveClass('ant-select-item-option-disabled');
    expect(optionFor('Restricted')).not.toHaveClass('ant-select-item-option-disabled');
  });

  it('submits the folder with its parent and classification', async () => {
    const onSuccess = vi.fn();
    render(
      <CreateFolderModal open onCancel={vi.fn()} onSuccess={onSuccess} hierarchyData={hierarchy} initialParentId={3} />
    );

    fireEvent.change(screen.getByLabelText('Folder Name'), { target: { value: '  Interlocks  ' } });
    fireEvent.change(screen.getByLabelText('Description'), { target: { value: 'SIS logic' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_asset', {
        token: 'test-token',
        name: 'Interlocks',
        description: 'SIS logic',
        assetType: 'folder',
        parentId: 3,
        securityClassification: SecurityClassificationLevel.RESTRICTED,
      });
    });
    await waitFor(() => expect(onSuccess).toHaveBeenCalledWith({ id: 20, name: 'New Folder' }));
  });

  it('submits a root folder with the default classification', async () => {
    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={devicesOnly} initialParentId={5} />);

    fireEvent.change(screen.getByLabelText('Folder Name'), { target: { value: 'Building 1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_asset', expect.objectContaining({
        name: 'Building 1',
        assetType: 'folder',
        parentId: null,
        securityClassification: SecurityClassificationLevel.INTERNAL,
      }));
    });
  });

  it('requires a name', async () => {
    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));
    expect(await screen.findByText('Please enter a folder name')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('shows the backend error when creation is rejected', async () => {
    vi.mocked(invoke).mockRejectedValue('Failed to create asset: Asset with this name already exists');
    render(<CreateFolderModal open onCancel={vi.fn()} onSuccess={vi.fn()} hierarchyData={hierarchy} />);

    fireEvent.change(screen.getByLabelText('Folder Name'), { target: { value: 'Plant A' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Folder' }));
    expect(await screen.findByText('Failed to create asset: Asset with this name already exists')).toBeInTheDocument();
  });
});
