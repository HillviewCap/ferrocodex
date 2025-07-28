/**
 * Request Tracking Tests
 * 
 * Tests for Story EH-2.1: Request ID Tracking and Correlation System
 * Validates request ID generation, propagation, and performance requirements.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  RequestIdGenerator,
  RequestIdValidator,
  RequestContextManager,
  RequestTracker,
  invokeWithRequestId,
  type RequestContext
} from './requestTracking';

// Mock crypto.randomUUID for testing
const mockRandomUUID = vi.fn(() => '12345678-1234-4234-8234-123456789012');
Object.defineProperty(global, 'crypto', {
  value: {
    randomUUID: mockRandomUUID
  },
  writable: true
});

// Mock performance API
Object.defineProperty(global, 'performance', {
  value: {
    now: vi.fn(() => Date.now())
  }
});

describe('RequestIdGenerator', () => {
  let generator: RequestIdGenerator;

  beforeEach(() => {
    // Reset singleton instance for testing
    (RequestIdGenerator as any).instance = null;
    generator = RequestIdGenerator.getInstance();
    vi.clearAllMocks();
    mockRandomUUID.mockReturnValue('12345678-1234-4234-8234-123456789012');
  });

  describe('TC-EH2.1.1: Request ID Generation Performance', () => {
    it('should generate unique request IDs', () => {
      const mockUUIDs = [
        '12345678-1234-4234-8234-123456789012',
        '87654321-4321-4321-8421-210987654321'
      ];
      let callCount = 0;
      
      (crypto.randomUUID as any).mockImplementation(() => mockUUIDs[callCount++]);

      const id1 = generator.generateRequestId();
      const id2 = generator.generateRequestId();

      expect(id1).toBe(mockUUIDs[0]);
      expect(id2).toBe(mockUUIDs[1]);
      expect(id1).not.toBe(id2);
    });

    it('should meet performance requirement (<1ms)', () => {
      const startTime = 100;
      const endTime = 100.5; // 0.5ms
      
      (performance.now as any)
        .mockReturnValueOnce(startTime)
        .mockReturnValueOnce(endTime);

      generator.generateRequestId();
      
      const stats = generator.getPerformanceStats();
      expect(stats.averageTimeMs).toBe(0.5);
      expect(stats.meetsRequirement).toBe(true);
    });

    it('should track performance statistics correctly', () => {
      const performanceTimes = [
        [100, 100.3], // 0.3ms
        [200, 200.7], // 0.7ms
        [300, 300.5], // 0.5ms
      ];

      performanceTimes.forEach(([start, end]) => {
        (performance.now as any)
          .mockReturnValueOnce(start)
          .mockReturnValueOnce(end);
        generator.generateRequestId();
      });

      const stats = generator.getPerformanceStats();
      expect(stats.sampleCount).toBe(3);
      expect(stats.averageTimeMs).toBeCloseTo(0.5, 5); // (0.3 + 0.7 + 0.5) / 3, allow floating point precision
      expect(stats.minTimeMs).toBeCloseTo(0.3, 5);
      expect(stats.maxTimeMs).toBeCloseTo(0.7, 5);
      expect(stats.meetsRequirement).toBe(true);
    });

    it('should identify performance violations', () => {
      const startTime = 100;
      const endTime = 102; // 2ms - exceeds requirement
      
      (performance.now as any)
        .mockReturnValueOnce(startTime)
        .mockReturnValueOnce(endTime);

      generator.generateRequestId();
      
      const stats = generator.getPerformanceStats();
      expect(stats.averageTimeMs).toBe(2);
      expect(stats.meetsRequirement).toBe(false);
    });

    it('should limit performance sample storage', () => {
      // Generate more than maxSamples (1000) to test circular buffer
      for (let i = 0; i < 1005; i++) {
        (performance.now as any)
          .mockReturnValueOnce(i)
          .mockReturnValueOnce(i + 0.5);
        generator.generateRequestId();
      }

      const stats = generator.getPerformanceStats();
      expect(stats.sampleCount).toBeLessThanOrEqual(1000);
    });
  });

  describe('Singleton Pattern', () => {
    it('should return the same instance', () => {
      const instance1 = RequestIdGenerator.getInstance();
      const instance2 = RequestIdGenerator.getInstance();
      
      expect(instance1).toBe(instance2);
    });
  });
});

describe('RequestIdValidator', () => {
  describe('Request ID Validation', () => {
    it('should validate correct UUID v4 format', () => {
      const validIds = [
        '12345678-1234-4234-8234-123456789012',
        'abcdef12-1234-4234-8234-123456789abc',
        '00000000-0000-4000-8000-000000000000'
      ];

      validIds.forEach(id => {
        expect(RequestIdValidator.isValidRequestId(id)).toBe(true);
      });
    });

    it('should reject invalid request ID formats', () => {
      const invalidIds = [
        '',
        'not-a-uuid',
        '12345678-1234-1234-1234-123456789012', // Wrong version (should be 4)
        '12345678-1234-4234-1234-123456789012', // Wrong variant (should be 8-b)
        '12345678123442348234123456789012', // Missing hyphens
        '12345678-1234-4234-8234-12345678901', // Too short
        '12345678-1234-4234-8234-1234567890123', // Too long
        null,
        undefined,
        123
      ];

      invalidIds.forEach(id => {
        expect(RequestIdValidator.isValidRequestId(id as any)).toBe(false);
      });
    });

    it('should normalize request ID format', () => {
      const mixedCaseId = 'ABCDEF12-1234-4234-8234-123456789ABC';
      const normalized = RequestIdValidator.normalizeRequestId(mixedCaseId);
      
      expect(normalized).toBe('abcdef12-1234-4234-8234-123456789abc');
    });

    it('should extract request ID from objects', () => {
      const testCases = [
        { requestId: '12345678-1234-4234-8234-123456789012' },
        { request_id: '12345678-1234-4234-8234-123456789012' },
        { id: '12345678-1234-4234-8234-123456789012' },
        { correlationId: '12345678-1234-4234-8234-123456789012' },
        { correlation_id: '12345678-1234-4234-8234-123456789012' }
      ];

      testCases.forEach(testCase => {
        const extracted = RequestIdValidator.extractRequestId(testCase);
        expect(extracted).toBe('12345678-1234-4234-8234-123456789012');
      });
    });

    it('should return null for objects without valid request ID', () => {
      const testCases = [
        { someOtherField: 'value' },
        { requestId: 'invalid-id' },
        null,
        undefined,
        'just-a-string'
      ];

      testCases.forEach(testCase => {
        const extracted = RequestIdValidator.extractRequestId(testCase);
        expect(extracted).toBeNull();
      });
    });
  });
});

describe('RequestContextManager', () => {
  let manager: RequestContextManager;

  beforeEach(() => {
    // Reset singleton instance for testing
    (RequestContextManager as any).instance = null;
    (RequestIdGenerator as any).instance = null;
    manager = RequestContextManager.getInstance();
    vi.clearAllMocks();
    mockRandomUUID.mockReturnValue('12345678-1234-4234-8234-123456789012');
  });

  describe('TC-EH2.1.2: Request Context Management', () => {
    it('should create request context with basic information', () => {
      const context = manager.createRequestContext('test_operation', 'TestComponent');

      expect(context.requestId).toBe('12345678-1234-4234-8234-123456789012');
      expect(context.operation).toBe('test_operation');
      expect(context.component).toBe('TestComponent');
      expect(context.timestamp).toBeInstanceOf(Date);
      expect(context.userId).toBeUndefined();
      expect(context.sessionId).toBeUndefined();
      expect(context.metadata).toBeUndefined();
    });

    it('should create request context with optional parameters', () => {
      const options = {
        userId: 123,
        sessionId: 'session-456',
        metadata: { key1: 'value1', key2: 'value2' }
      };

      const context = manager.createRequestContext('test_op', 'TestComp', options);

      expect(context.userId).toBe(123);
      expect(context.sessionId).toBe('session-456');
      expect(context.metadata).toEqual(options.metadata);
    });

    it('should retrieve request context by ID', () => {
      const context = manager.createRequestContext('test_operation', 'TestComponent');
      const retrieved = manager.getRequestContext(context.requestId);

      expect(retrieved).toEqual(context);
    });

    it('should return null for non-existent request ID', () => {
      const nonExistentId = '87654321-4321-4321-8421-210987654321';
      const retrieved = manager.getRequestContext(nonExistentId);

      expect(retrieved).toBeNull();
    });

    it('should update request context', () => {
      const context = manager.createRequestContext('test_operation', 'TestComponent');
      
      const success = manager.updateRequestContext(context.requestId, {
        operation: 'updated_operation',
        userId: 456
      });

      expect(success).toBe(true);
      
      const updated = manager.getRequestContext(context.requestId);
      expect(updated?.operation).toBe('updated_operation');
      expect(updated?.userId).toBe(456);
      expect(updated?.component).toBe('TestComponent'); // Unchanged
      expect(updated?.requestId).toBe(context.requestId); // Unchanged
    });

    it('should not allow updating core identifiers', () => {
      const context = manager.createRequestContext('test_operation', 'TestComponent');
      const originalRequestId = context.requestId;
      const originalTimestamp = context.timestamp;
      
      manager.updateRequestContext(context.requestId, {
        requestId: 'new-id',
        timestamp: new Date('2025-01-01')
      } as any);

      const updated = manager.getRequestContext(context.requestId);
      expect(updated?.requestId).toBe(originalRequestId);
      expect(updated?.timestamp).toEqual(originalTimestamp);
    });

    it('should complete request tracking', () => {
      const context = manager.createRequestContext('test_operation', 'TestComponent');
      
      const completed = manager.completeRequest(context.requestId);
      expect(completed).toBe(true);

      const retrieved = manager.getRequestContext(context.requestId);
      expect(retrieved).toBeNull();
    });

    it('should return false when completing non-existent request', () => {
      const nonExistentId = '87654321-4321-4321-8421-210987654321';
      const completed = manager.completeRequest(nonExistentId);
      
      expect(completed).toBe(false);
    });

    it('should track active requests', () => {
      // Use different UUIDs for each request
      mockRandomUUID
        .mockReturnValueOnce('12345678-1234-4234-8234-123456789012')
        .mockReturnValueOnce('87654321-4321-4321-8421-210987654321');

      const context1 = manager.createRequestContext('op1', 'Comp1');
      const context2 = manager.createRequestContext('op2', 'Comp2');

      const activeRequests = manager.getActiveRequests();
      expect(activeRequests).toHaveLength(2);
      expect(activeRequests.map(r => r.requestId)).toContain(context1.requestId);
      expect(activeRequests.map(r => r.requestId)).toContain(context2.requestId);
    });

    it('should get active request count', () => {
      expect(manager.getActiveRequestCount()).toBe(0);

      // Use different UUIDs for each request
      mockRandomUUID
        .mockReturnValueOnce('12345678-1234-4234-8234-123456789012')
        .mockReturnValueOnce('87654321-4321-4321-8421-210987654321');

      manager.createRequestContext('op1', 'Comp1');
      expect(manager.getActiveRequestCount()).toBe(1);

      manager.createRequestContext('op2', 'Comp2');
      expect(manager.getActiveRequestCount()).toBe(2);
    });

    it('should clean up old requests', () => {
      // Create requests with mocked timestamps
      const oldDate = new Date(Date.now() - 35 * 60 * 1000); // 35 minutes ago
      const recentDate = new Date(Date.now() - 10 * 60 * 1000); // 10 minutes ago

      // Use different UUIDs for each request
      mockRandomUUID
        .mockReturnValueOnce('12345678-1234-4234-8234-123456789012')
        .mockReturnValueOnce('87654321-4321-4321-8421-210987654321');

      const context1 = manager.createRequestContext('old_op', 'OldComp');
      const context2 = manager.createRequestContext('recent_op', 'RecentComp');

      // Manually set timestamps to simulate age using object assignment
      Object.assign(context1, { timestamp: oldDate });
      Object.assign(context2, { timestamp: recentDate });

      const removedCount = manager.cleanupOldRequests(30); // Remove requests older than 30 minutes

      expect(removedCount).toBe(1);
      expect(manager.getRequestContext(context1.requestId)).toBeNull();
      expect(manager.getRequestContext(context2.requestId)).not.toBeNull();
    });
  });

  describe('Singleton Pattern', () => {
    it('should return the same instance', () => {
      const instance1 = RequestContextManager.getInstance();
      const instance2 = RequestContextManager.getInstance();
      
      expect(instance1).toBe(instance2);
    });
  });
});

describe('RequestTracker Convenience API', () => {
  beforeEach(() => {
    // Reset singleton instances
    (RequestIdGenerator as any).instance = null;
    (RequestContextManager as any).instance = null;
    vi.clearAllMocks();
    mockRandomUUID.mockReturnValue('12345678-1234-4234-8234-123456789012');
  });

  it('should provide convenient access to request ID generation', () => {
    const requestId = RequestTracker.generateId();
    expect(requestId).toBe('12345678-1234-4234-8234-123456789012');
  });

  it('should provide convenient access to request ID validation', () => {
    expect(RequestTracker.isValidId('12345678-1234-4234-8234-123456789012')).toBe(true);
    expect(RequestTracker.isValidId('invalid-id')).toBe(false);
  });

  it('should provide convenient access to context creation', () => {
    const context = RequestTracker.createContext('test_op', 'TestComp', {
      userId: 123,
      metadata: { test: 'value' }
    });

    expect(context.operation).toBe('test_op');
    expect(context.component).toBe('TestComp');
    expect(context.userId).toBe(123);
    expect(context.metadata).toEqual({ test: 'value' });
  });

  it('should provide convenient access to context retrieval', () => {
    const context = RequestTracker.createContext('test_op', 'TestComp');
    const retrieved = RequestTracker.getContext(context.requestId);

    expect(retrieved).toEqual(context);
  });

  it('should provide convenient access to request completion', () => {
    const context = RequestTracker.createContext('test_op', 'TestComp');
    const completed = RequestTracker.completeRequest(context.requestId);

    expect(completed).toBe(true);
    expect(RequestTracker.getContext(context.requestId)).toBeNull();
  });

  it('should provide access to performance statistics', () => {
    RequestTracker.generateId(); // Generate one ID to create stats
    
    const stats = RequestTracker.getPerformanceStats();
    expect(stats.sampleCount).toBeGreaterThan(0);
    expect(typeof stats.averageTimeMs).toBe('number');
    expect(typeof stats.meetsRequirement).toBe('boolean');
  });

  it('should check performance requirements', () => {
    (performance.now as any)
      .mockReturnValueOnce(100)
      .mockReturnValueOnce(100.5); // 0.5ms - meets requirement

    RequestTracker.generateId();
    
    expect(RequestTracker.meetsPerformanceRequirement()).toBe(true);
  });
});

describe('invokeWithRequestId Integration', () => {
  beforeEach(() => {
    // Reset singleton instances
    (RequestContextManager as any).instance = null;
    (RequestIdGenerator as any).instance = null;
    vi.clearAllMocks();
    mockRandomUUID.mockReturnValue('12345678-1234-4234-8234-123456789012');
  });

  it('should add request ID to Tauri invoke calls', async () => {
    const mockInvoke = vi.fn().mockResolvedValue('test-result');
    
    // Mock the dynamic import
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: mockInvoke
    }));

    const result = await invokeWithRequestId('test_command', { param1: 'value1' });

    expect(mockInvoke).toHaveBeenCalledWith('test_command', {
      param1: 'value1',
      request_id: '12345678-1234-4234-8234-123456789012'
    });
    expect(result).toBe('test-result');
  });

  it('should use provided context if given', async () => {
    const mockInvoke = vi.fn().mockResolvedValue('test-result');
    
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: mockInvoke
    }));

    const customContext: RequestContext = {
      requestId: 'custom-request-id',
      operation: 'custom_operation',
      component: 'CustomComponent',
      timestamp: new Date()
    };

    await invokeWithRequestId('test_command', { param1: 'value1' }, customContext);

    expect(mockInvoke).toHaveBeenCalledWith('test_command', {
      param1: 'value1',
      request_id: 'custom-request-id'
    });
  });

  it('should handle errors and maintain request context', async () => {
    const mockInvoke = vi.fn().mockRejectedValue(new Error('Test error'));
    
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: mockInvoke
    }));

    const manager = RequestContextManager.getInstance();
    
    await expect(invokeWithRequestId('test_command', {})).rejects.toThrow('Test error');
    
    // Request should still be tracked for error correlation
    expect(manager.getActiveRequestCount()).toBe(1);
  });
});