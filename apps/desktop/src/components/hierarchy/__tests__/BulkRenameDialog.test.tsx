import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import BulkRenameDialog from '../BulkRenameDialog';
import { AssetHierarchy } from '../../../types/assets';

// Mock the bulk operations store
const mockStore = {
  validateBulkRename: vi.fn().mockResolvedValue({
    is_valid: true,
    errors: [],
    warnings: [],
    conflicts: []
  })
};

vi.mock('../../../store/bulkOperations', () => ({
  default: vi.fn(() => mockStore)
}));

// Mock Ant Design Modal
vi.mock('antd', async () => {
  const actual = await vi.importActual<typeof import('antd')>('antd');
  const Modal = (props: any) => <actual.Modal {...props} />;
  Object.assign(Modal, actual.Modal);
  (Modal as any).error = vi.fn();
  return {
    ...actual,
    Modal,
  };
});

const mockAssets: AssetHierarchy[] = [
  {
    id: 1,
    name: 'Asset1.txt',
    asset_type: 'Equipment',
    parent_id: null,
    path: '/asset1.txt',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: []
  },
  {
    id: 2,
    name: 'Asset2.doc',
    asset_type: 'Equipment',
    parent_id: null,
    path: '/asset2.doc',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: []
  },
  {
    id: 3,
    name: 'Folder1',
    asset_type: 'Folder',
    parent_id: null,
    path: '/folder1',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: []
  }
];

describe('BulkRenameDialog', () => {
  const mockOnCancel = vi.fn();
  const mockOnConfirm = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render when visible', () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    expect(screen.getByText('Bulk Rename Assets')).toBeInTheDocument();
    expect(screen.getByText('2 assets')).toBeInTheDocument();
  });

  it('should not render when not visible', () => {
    render(
      <BulkRenameDialog
        visible={false}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    expect(screen.queryByText('Bulk Rename Assets')).not.toBeInTheDocument();
  });

  it('should handle cancel', () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(mockOnCancel).toHaveBeenCalled();
  });

  it('should show simple mode by default', () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    expect(screen.getByText('Prefix')).toBeInTheDocument();
    expect(screen.getByText('Suffix')).toBeInTheDocument();
    expect(screen.queryByText('Rename Pattern')).not.toBeInTheDocument();
  });

  it('should switch to pattern mode', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    const patternSwitch = screen.getAllByRole('switch')[0];
    fireEvent.click(patternSwitch);

    await waitFor(() => {
      expect(screen.getByText('Rename Pattern')).toBeInTheDocument();
      expect(screen.getByText('Starting Number')).toBeInTheDocument();
    });
  });

  it('should show pattern examples in pattern mode', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    const patternSwitch = screen.getAllByRole('switch')[0];
    fireEvent.click(patternSwitch);

    await waitFor(() => {
      expect(screen.getByText('Pattern Examples')).toBeInTheDocument();
      expect(screen.getByText('{name}_{number}')).toBeInTheDocument();
      expect(screen.getByText('Add sequential numbers')).toBeInTheDocument();
    });
  });

  it('should show preview when toggled', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    const previewButton = screen.getByText('Show Preview');
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText('Preview')).toBeInTheDocument();
      expect(screen.getByText('Asset1.txt')).toBeInTheDocument();
      expect(screen.getByText('Asset2.doc')).toBeInTheDocument();
    });
  });

  it('should update preview when options change', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Show preview
    const previewButton = screen.getByText('Show Preview');
    fireEvent.click(previewButton);

    // Add prefix
    const prefixInput = screen.getByPlaceholderText('Text to add before each name');
    fireEvent.change(prefixInput, { target: { value: 'new_' } });

    await waitFor(() => {
      expect(screen.getByText('new_Asset1.txt')).toBeInTheDocument();
      expect(screen.getByText('new_Asset2.doc')).toBeInTheDocument();
    });
  });

  it('should handle pattern-based renaming in preview', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Switch to pattern mode
    const patternSwitch = screen.getAllByRole('switch')[0];
    fireEvent.click(patternSwitch);

    // Set pattern
    await waitFor(() => {
      const patternInput = screen.getByPlaceholderText('e.g., {name}_{number:3}');
      fireEvent.change(patternInput, { target: { value: '{name}_{number}' } });
    });

    // Show preview
    const previewButton = screen.getByText('Show Preview');
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText('Asset1_1.txt')).toBeInTheDocument();
      expect(screen.getByText('Asset2_2.doc')).toBeInTheDocument();
    });
  });

  it('should detect name conflicts in preview', async () => {
    const assetsWithConflict = [
      ...mockAssets,
      {
        id: 4,
        name: 'new_Asset1.txt', // This will conflict when prefix is added
        asset_type: 'Equipment' as const,
        parent_id: null,
        path: '/new_asset1.txt',
        created_at: '2023-01-01T00:00:00Z',
        updated_at: '2023-01-01T00:00:00Z',
        children: []
      }
    ];

    render(
      <BulkRenameDialog
        visible={true}
        assets={assetsWithConflict}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Add prefix
    const prefixInput = screen.getByPlaceholderText('Text to add before each name');
    fireEvent.change(prefixInput, { target: { value: 'new_' } });

    // Show preview
    const previewButton = screen.getByText('Show Preview');
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText('Name conflicts with existing asset')).toBeInTheDocument();
    });
  });

  it('should validate names and show errors', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Switch to pattern mode and set invalid pattern
    const patternSwitch = screen.getAllByRole('switch')[0];
    fireEvent.click(patternSwitch);

    await waitFor(() => {
      const patternInput = screen.getByPlaceholderText('e.g., {name}_{number:3}');
      fireEvent.change(patternInput, { target: { value: '<invalid>' } }); // Contains invalid characters
    });

    // Show preview
    const previewButton = screen.getByText('Show Preview');
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText('Name contains invalid characters')).toBeInTheDocument();
    });
  });

  it('should handle extension preservation', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Disable extension preservation
    const extensionSwitch = screen.getByText('Preserve Extensions').closest('.ant-switch');
    if (extensionSwitch) {
      fireEvent.click(extensionSwitch);
    }

    // Add suffix
    const suffixInput = screen.getByPlaceholderText('Text to add after each name');
    fireEvent.change(suffixInput, { target: { value: '_backup' } });

    // Show preview
    const previewButton = screen.getByText('Show Preview');
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText('Asset1.txt_backup')).toBeInTheDocument();
      expect(screen.getByText('Asset2.doc_backup')).toBeInTheDocument();
    });
  });

  it('should call onConfirm with correct options', async () => {
    mockStore.validateBulkRename.mockResolvedValue({
      is_valid: true,
      errors: [],
      warnings: [],
      conflicts: []
    });

    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Add prefix
    const prefixInput = screen.getByPlaceholderText('Text to add before each name');
    fireEvent.change(prefixInput, { target: { value: 'new_' } });

    // Click confirm button
    const confirmButton = screen.getByText(/Rename \d+ Asset/);
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(mockStore.validateBulkRename).toHaveBeenCalledWith(
        [1, 2],
        expect.objectContaining({
          prefix: 'new_',
          suffix: '',
          use_pattern: false,
          preserve_extension: true
        })
      );
      expect(mockOnConfirm).toHaveBeenCalledWith(
        expect.objectContaining({
          prefix: 'new_',
          suffix: '',
          use_pattern: false,
          preserve_extension: true
        })
      );
    });
  });

  it('should prevent confirmation with validation errors', async () => {
    mockStore.validateBulkRename.mockResolvedValue({
      is_valid: false,
      errors: [{ asset_id: 1, asset_name: 'Asset1', error_type: 'invalid_name', message: 'Invalid name', blocking: true }],
      warnings: [],
      conflicts: []
    });

    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Switch to pattern mode and set invalid pattern
    const patternSwitch = screen.getAllByRole('switch')[0];
    fireEvent.click(patternSwitch);

    await waitFor(() => {
      const patternInput = screen.getByPlaceholderText('e.g., {name}_{number:3}');
      fireEvent.change(patternInput, { target: { value: '<invalid>' } });
    });

    // Try to confirm
    const confirmButton = screen.getByText(/Rename \d+ Asset/);
    expect(confirmButton).toBeDisabled();
  });

  it('should click pattern examples to set pattern', async () => {
    render(
      <BulkRenameDialog
        visible={true}
        assets={mockAssets}
        selectedAssetIds={[1, 2]}
        onCancel={mockOnCancel}
        onConfirm={mockOnConfirm}
      />
    );

    // Switch to pattern mode
    const patternSwitch = screen.getAllByRole('switch')[0];
    fireEvent.click(patternSwitch);

    await waitFor(() => {
      // Click on a pattern example
      const examplePattern = screen.getByText('{name}_{number}');
      fireEvent.click(examplePattern.closest('li')!);
    });

    // Pattern input should be updated
    await waitFor(() => {
      const patternInput = screen.getByPlaceholderText('e.g., {name}_{number:3}') as HTMLInputElement;
      expect(patternInput.value).toBe('{name}_{number}');
    });
  });
});