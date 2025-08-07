import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import KeyboardShortcuts from '../KeyboardShortcuts';
import { AssetHierarchy } from '../../../types/assets';

// Mock the bulk operations store
const mockStore = {
  selectAll: vi.fn(),
  selectNone: vi.fn(),
  getSelectedAssets: vi.fn(() => []),
  getSelectedCount: vi.fn(() => 0),
  isSelected: vi.fn(() => false),
  setKeyboardNavigationMode: vi.fn(),
  setLastFocusedAsset: vi.fn(),
  setSelectionAnchor: vi.fn(),
  keyboardState: {
    last_focused_asset: null,
    selection_anchor: null,
    keyboard_navigation_mode: false
  }
};

vi.mock('../../../store/bulkOperations', () => ({
  default: vi.fn(() => mockStore)
}));

// Mock Ant Design message
vi.mock('antd', async () => {
  const actual = await vi.importActual('antd');
  return {
    ...actual,
    message: {
      success: vi.fn(),
      error: vi.fn(),
      info: vi.fn()
    }
  };
});

const mockAssets: AssetHierarchy[] = [
  {
    id: 1,
    name: 'Folder 1',
    asset_type: 'Folder',
    parent_id: null,
    path: '/folder1',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: [
      {
        id: 2,
        name: 'Asset 1',
        asset_type: 'Equipment',
        parent_id: 1,
        path: '/folder1/asset1',
        created_at: '2023-01-01T00:00:00Z',
        updated_at: '2023-01-01T00:00:00Z',
        children: []
      }
    ]
  },
  {
    id: 3,
    name: 'Asset 2',
    asset_type: 'Equipment',
    parent_id: null,
    path: '/asset2',
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
    children: []
  }
];

describe('KeyboardShortcuts', () => {
  const mockOnAssetSelect = vi.fn();
  const mockOnRenameStart = vi.fn();
  const mockOnDeleteStart = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render children correctly', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    expect(screen.getByTestId('child-content')).toBeInTheDocument();
  });

  it('should handle Ctrl+A (Select All)', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'a', ctrlKey: true });

    expect(mockStore.selectAll).toHaveBeenCalledWith([1, 2, 3], 'manual');
  });

  it('should handle Escape (Clear Selection)', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(mockStore.selectNone).toHaveBeenCalled();
    expect(mockOnAssetSelect).toHaveBeenCalledWith(null);
    expect(mockStore.setLastFocusedAsset).toHaveBeenCalledWith(null);
    expect(mockStore.setSelectionAnchor).toHaveBeenCalledWith(null);
  });

  it('should handle Delete key', () => {
    mockStore.getSelectedCount.mockReturnValue(2);

    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'Delete' });

    expect(mockOnDeleteStart).toHaveBeenCalled();
  });

  it('should handle F2 (Rename)', () => {
    const selectedAsset = mockAssets[0];

    render(
      <KeyboardShortcuts
        assets={mockAssets}
        selectedAsset={selectedAsset}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'F2' });

    expect(mockOnRenameStart).toHaveBeenCalledWith(selectedAsset);
  });

  it('should handle Arrow Down navigation', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'ArrowDown' });

    // Should select first asset when none is selected
    expect(mockOnAssetSelect).toHaveBeenCalledWith(mockAssets[0]);
    expect(mockStore.setLastFocusedAsset).toHaveBeenCalledWith(mockAssets[0].id);
  });

  it('should handle Arrow Up navigation', () => {
    const selectedAsset = mockAssets[1]; // Asset 2

    render(
      <KeyboardShortcuts
        assets={mockAssets}
        selectedAsset={selectedAsset}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'ArrowUp' });

    // Should navigate to previous asset
    expect(mockOnAssetSelect).toHaveBeenCalled();
    expect(mockStore.setLastFocusedAsset).toHaveBeenCalled();
  });

  it('should handle Enter key for selection', () => {
    const selectedAsset = mockAssets[0];

    render(
      <KeyboardShortcuts
        assets={mockAssets}
        selectedAsset={selectedAsset}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'Enter' });

    expect(mockOnAssetSelect).toHaveBeenCalledWith(selectedAsset);
  });

  it('should handle Space for toggle selection', () => {
    mockStore.keyboardState.last_focused_asset = 1;
    mockStore.isSelected.mockReturnValue(false);

    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: ' ' });

    // Should show message about selection toggle
    expect(vi.mocked(require('antd').message.info)).toHaveBeenCalledWith('Asset selected');
  });

  it('should handle Shift+Arrow for range selection', () => {
    const selectedAsset = mockAssets[0];
    mockStore.keyboardState.selection_anchor = 1;

    render(
      <KeyboardShortcuts
        assets={mockAssets}
        selectedAsset={selectedAsset}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'ArrowDown', shiftKey: true });

    expect(mockStore.selectAll).toHaveBeenCalled();
    expect(mockOnAssetSelect).toHaveBeenCalled();
    expect(mockStore.setLastFocusedAsset).toHaveBeenCalled();
  });

  it('should ignore shortcuts when typing in input fields', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <input data-testid="test-input" />
      </KeyboardShortcuts>
    );

    const input = screen.getByTestId('test-input');
    input.focus();

    fireEvent.keyDown(input, { key: 'a', ctrlKey: true });

    // Should not trigger select all when typing in input
    expect(mockStore.selectAll).not.toHaveBeenCalled();
  });

  it('should be disabled when enabled prop is false', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        enabled={false}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'a', ctrlKey: true });

    // Should not trigger shortcuts when disabled
    expect(mockStore.selectAll).not.toHaveBeenCalled();
  });

  it('should set keyboard navigation mode on key events', () => {
    render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    fireEvent.keyDown(document, { key: 'ArrowDown' });

    expect(mockStore.setKeyboardNavigationMode).toHaveBeenCalledWith(true);
  });

  it('should handle focus and blur events', async () => {
    const { container } = render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    const wrapper = container.firstChild as HTMLElement;

    fireEvent.focus(wrapper);
    expect(mockStore.setKeyboardNavigationMode).toHaveBeenCalledWith(true);

    fireEvent.blur(wrapper);
    expect(mockStore.setKeyboardNavigationMode).toHaveBeenCalledWith(false);
  });

  it('should handle mouse events to disable keyboard navigation', () => {
    mockStore.keyboardState.keyboard_navigation_mode = true;

    const { container } = render(
      <KeyboardShortcuts
        assets={mockAssets}
        onAssetSelect={mockOnAssetSelect}
        onRenameStart={mockOnRenameStart}
        onDeleteStart={mockOnDeleteStart}
      >
        <div data-testid="child-content">Test Content</div>
      </KeyboardShortcuts>
    );

    const wrapper = container.firstChild as HTMLElement;

    fireEvent.mouseDown(wrapper);
    expect(mockStore.setKeyboardNavigationMode).toHaveBeenCalledWith(false);

    fireEvent.click(wrapper);
    expect(mockStore.setKeyboardNavigationMode).toHaveBeenCalledWith(false);
  });
});