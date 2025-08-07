import { BulkOperation, BulkOperationType } from '../types/bulkOperations';

export interface HistoryOperation {
  id: string;
  type: BulkOperationType | 'rename';
  timestamp: string;
  description: string;
  assetIds: number[];
  originalData?: Record<string, any>;
  newData?: Record<string, any>;
  canUndo: boolean;
  canRedo: boolean;
}

export interface HistoryState {
  operations: HistoryOperation[];
  currentIndex: number;
  maxHistorySize: number;
}

export class HistoryManager {
  private static instance: HistoryManager;
  private state: HistoryState;
  private listeners: Set<() => void> = new Set();

  private constructor() {
    this.state = {
      operations: [],
      currentIndex: -1,
      maxHistorySize: 50
    };
  }

  static getInstance(): HistoryManager {
    if (!HistoryManager.instance) {
      HistoryManager.instance = new HistoryManager();
    }
    return HistoryManager.instance;
  }

  // Subscribe to history changes
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    this.listeners.forEach(listener => listener());
  }

  // Add a new operation to history
  addOperation(operation: Omit<HistoryOperation, 'id' | 'timestamp' | 'canUndo' | 'canRedo'>): void {
    const historyOperation: HistoryOperation = {
      ...operation,
      id: `op_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      timestamp: new Date().toISOString(),
      canUndo: true,
      canRedo: false
    };

    // Remove any operations after current index (when adding new operation after undo)
    const newOperations = this.state.operations.slice(0, this.state.currentIndex + 1);
    
    // Add new operation
    newOperations.push(historyOperation);

    // Maintain max history size
    if (newOperations.length > this.state.maxHistorySize) {
      newOperations.shift();
    } else {
      this.state.currentIndex++;
    }

    this.state.operations = newOperations;
    this.notify();
  }

  // Group multiple operations into a single undoable action
  addGroupedOperation(
    operations: Omit<HistoryOperation, 'id' | 'timestamp' | 'canUndo' | 'canRedo'>[],
    groupDescription: string
  ): void {
    if (operations.length === 0) return;

    if (operations.length === 1) {
      this.addOperation(operations[0]);
      return;
    }

    // Combine multiple operations into one
    const allAssetIds = [...new Set(operations.flatMap(op => op.assetIds))];
    const combinedOperation: Omit<HistoryOperation, 'id' | 'timestamp' | 'canUndo' | 'canRedo'> = {
      type: operations[0].type, // Use first operation's type
      description: groupDescription,
      assetIds: allAssetIds,
      originalData: operations.reduce((acc, op) => ({ ...acc, ...op.originalData }), {}),
      newData: operations.reduce((acc, op) => ({ ...acc, ...op.newData }), {})
    };

    this.addOperation(combinedOperation);
  }

  // Check if undo is possible
  canUndo(): boolean {
    return this.state.currentIndex >= 0;
  }

  // Check if redo is possible
  canRedo(): boolean {
    return this.state.currentIndex < this.state.operations.length - 1;
  }

  // Get the operation that would be undone
  getUndoOperation(): HistoryOperation | null {
    if (!this.canUndo()) return null;
    return this.state.operations[this.state.currentIndex];
  }

  // Get the operation that would be redone
  getRedoOperation(): HistoryOperation | null {
    if (!this.canRedo()) return null;
    return this.state.operations[this.state.currentIndex + 1];
  }

  // Perform undo operation
  undo(): HistoryOperation | null {
    if (!this.canUndo()) return null;

    const operation = this.state.operations[this.state.currentIndex];
    this.state.currentIndex--;
    this.notify();
    
    return operation;
  }

  // Perform redo operation
  redo(): HistoryOperation | null {
    if (!this.canRedo()) return null;

    this.state.currentIndex++;
    const operation = this.state.operations[this.state.currentIndex];
    this.notify();
    
    return operation;
  }

  // Get recent operations for display
  getRecentOperations(limit: number = 10): HistoryOperation[] {
    return this.state.operations
      .slice(Math.max(0, this.state.operations.length - limit))
      .reverse();
  }

  // Get all operations
  getAllOperations(): HistoryOperation[] {
    return [...this.state.operations];
  }

  // Clear history
  clear(): void {
    this.state.operations = [];
    this.state.currentIndex = -1;
    this.notify();
  }

  // Get current state for debugging
  getState(): HistoryState {
    return { ...this.state };
  }

  // Create operation description based on type and data
  static createOperationDescription(
    type: BulkOperationType | 'rename',
    assetCount: number,
    additionalInfo?: string
  ): string {
    const assetText = assetCount === 1 ? 'asset' : 'assets';
    
    switch (type) {
      case 'move':
        return `Moved ${assetCount} ${assetText}${additionalInfo ? ` to ${additionalInfo}` : ''}`;
      case 'delete':
        return `Deleted ${assetCount} ${assetText}`;
      case 'export':
        return `Exported ${assetCount} ${assetText}${additionalInfo ? ` as ${additionalInfo}` : ''}`;
      case 'classify':
        return `Classified ${assetCount} ${assetText}${additionalInfo ? ` as ${additionalInfo}` : ''}`;
      case 'rename':
        return `Renamed ${assetCount} ${assetText}`;
      default:
        return `Performed operation on ${assetCount} ${assetText}`;
    }
  }

  // Convert BulkOperation to HistoryOperation
  static fromBulkOperation(bulkOp: BulkOperation, originalData?: any, newData?: any): HistoryOperation {
    return {
      id: bulkOp.id,
      type: bulkOp.operation_type,
      timestamp: bulkOp.started_at,
      description: this.createOperationDescription(bulkOp.operation_type, bulkOp.asset_ids.length),
      assetIds: bulkOp.asset_ids,
      originalData,
      newData,
      canUndo: bulkOp.status === 'completed',
      canRedo: false
    };
  }
}

export default HistoryManager;