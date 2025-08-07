import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import { HistoryManagerComponent } from '../HistoryManager';
import { HistoryManager } from '../../../utils/historyManager';

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

// Mock the bulk operations store
vi.mock('../../../store/bulkOperations', () => ({
  default: vi.fn(() => ({
    undoOperation: vi.fn().mockResolvedValue({ success: true })
  }))
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

describe('HistoryManager', () => {
  let historyManager: HistoryManager;

  beforeEach(() => {
    historyManager = HistoryManager.getInstance();
    historyManager.clear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('HistoryManager utility', () => {
    it('should be a singleton', () => {
      const instance1 = HistoryManager.getInstance();
      const instance2 = HistoryManager.getInstance();
      expect(instance1).toBe(instance2);
    });

    it('should add operations to history', () => {
      historyManager.addOperation({
        type: 'move',
        description: 'Moved 2 assets',
        assetIds: [1, 2]
      });

      const operations = historyManager.getAllOperations();
      expect(operations).toHaveLength(1);
      expect(operations[0].type).toBe('move');
      expect(operations[0].assetIds).toEqual([1, 2]);
      expect(operations[0].canUndo).toBe(true);
    });

    it('should handle undo/redo correctly', () => {
      // Add operation
      historyManager.addOperation({
        type: 'delete',
        description: 'Deleted 1 asset',
        assetIds: [1]
      });

      expect(historyManager.canUndo()).toBe(true);
      expect(historyManager.canRedo()).toBe(false);

      // Undo
      const undoOp = historyManager.undo();
      expect(undoOp?.type).toBe('delete');
      expect(historyManager.canUndo()).toBe(false);
      expect(historyManager.canRedo()).toBe(true);

      // Redo
      const redoOp = historyManager.redo();
      expect(redoOp?.type).toBe('delete');
      expect(historyManager.canUndo()).toBe(true);
      expect(historyManager.canRedo()).toBe(false);
    });

    it('should group operations correctly', () => {
      const operations = [
        { type: 'move' as const, description: 'Move 1', assetIds: [1] },
        { type: 'move' as const, description: 'Move 2', assetIds: [2] }
      ];

      historyManager.addGroupedOperation(operations, 'Bulk move operation');

      const allOps = historyManager.getAllOperations();
      expect(allOps).toHaveLength(1);
      expect(allOps[0].description).toBe('Bulk move operation');
      expect(allOps[0].assetIds).toEqual([1, 2]);
    });

    it('should maintain max history size', () => {
      // Add more operations than max size
      for (let i = 0; i < 55; i++) {
        historyManager.addOperation({
          type: 'move',
          description: `Operation ${i}`,
          assetIds: [i]
        });
      }

      const operations = historyManager.getAllOperations();
      expect(operations.length).toBeLessThanOrEqual(50);
    });

    it('should create proper operation descriptions', () => {
      const description1 = HistoryManager.createOperationDescription('move', 1);
      expect(description1).toBe('Moved 1 asset');

      const description2 = HistoryManager.createOperationDescription('delete', 5);
      expect(description2).toBe('Deleted 5 assets');

      const description3 = HistoryManager.createOperationDescription('export', 2, 'CSV');
      expect(description3).toBe('Exported 2 assets as CSV');
    });
  });

  describe('HistoryManagerComponent', () => {
    it('should render undo/redo buttons', () => {
      render(<HistoryManagerComponent />);
      
      expect(screen.getByText('Undo')).toBeInTheDocument();
      expect(screen.getByText('Redo')).toBeInTheDocument();
      expect(screen.getByText('History')).toBeInTheDocument();
    });

    it('should disable buttons when no operations available', () => {
      render(<HistoryManagerComponent />);
      
      const undoButton = screen.getByText('Undo').closest('button');
      const redoButton = screen.getByText('Redo').closest('button');
      
      expect(undoButton).toBeDisabled();
      expect(redoButton).toBeDisabled();
    });

    it('should enable undo button when operations are available', async () => {
      // Add an operation
      historyManager.addOperation({
        type: 'move',
        description: 'Test operation',
        assetIds: [1]
      });

      render(<HistoryManagerComponent />);
      
      await waitFor(() => {
        const undoButton = screen.getByText('Undo').closest('button');
        expect(undoButton).not.toBeDisabled();
      });
    });

    it('should open history drawer when history button is clicked', async () => {
      render(<HistoryManagerComponent />);
      
      const historyButton = screen.getByText('History');
      fireEvent.click(historyButton);
      
      await waitFor(() => {
        expect(screen.getByText('Operation History')).toBeInTheDocument();
      });
    });

    it('should handle keyboard shortcuts', () => {
      const mockUndo = vi.fn();
      const mockRedo = vi.fn();
      
      render(<HistoryManagerComponent />);
      
      // Test Ctrl+Z
      fireEvent.keyDown(document, { key: 'z', ctrlKey: true });
      
      // Test Ctrl+Y
      fireEvent.keyDown(document, { key: 'y', ctrlKey: true });
      
      // Test Ctrl+Shift+Z
      fireEvent.keyDown(document, { key: 'Z', ctrlKey: true, shiftKey: true });
    });

    it('should display recent operations in drawer', async () => {
      // Add some operations
      historyManager.addOperation({
        type: 'move',
        description: 'Moved 2 assets',
        assetIds: [1, 2]
      });
      
      historyManager.addOperation({
        type: 'delete',
        description: 'Deleted 1 asset',
        assetIds: [3]
      });

      render(<HistoryManagerComponent />);
      
      // Open drawer
      const historyButton = screen.getByText('History');
      fireEvent.click(historyButton);
      
      await waitFor(() => {
        expect(screen.getByText('Moved 2 assets')).toBeInTheDocument();
        expect(screen.getByText('Deleted 1 asset')).toBeInTheDocument();
      });
    });

    it('should show empty state when no operations', async () => {
      render(<HistoryManagerComponent />);
      
      // Open drawer
      const historyButton = screen.getByText('History');
      fireEvent.click(historyButton);
      
      await waitFor(() => {
        expect(screen.getByText('No operations yet')).toBeInTheDocument();
      });
    });
  });

  describe('History integration with notifications', () => {
    it('should subscribe to history changes', () => {
      let callbackCalled = false;
      
      const unsubscribe = historyManager.subscribe(() => {
        callbackCalled = true;
      });
      
      historyManager.addOperation({
        type: 'move',
        description: 'Test',
        assetIds: [1]
      });
      
      expect(callbackCalled).toBe(true);
      unsubscribe();
    });

    it('should unsubscribe properly', () => {
      let callbackCalled = false;
      
      const unsubscribe = historyManager.subscribe(() => {
        callbackCalled = true;
      });
      
      unsubscribe();
      
      historyManager.addOperation({
        type: 'move',
        description: 'Test',
        assetIds: [1]
      });
      
      expect(callbackCalled).toBe(false);
    });
  });
});