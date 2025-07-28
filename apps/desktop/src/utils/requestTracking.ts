/**
 * Request ID Tracking and Correlation Utilities
 * 
 * This module provides utilities for generating, managing, and tracking request IDs
 * throughout the system layers. It enables end-to-end correlation of operations
 * and errors across Frontend → Tauri → Backend → Database.
 */

// Use native crypto API for UUID generation

// Request ID format validation regex
const REQUEST_ID_REGEX = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

/**
 * Request context information for operations
 */
export interface RequestContext {
  /** Unique request identifier */
  requestId: string;
  /** Operation being performed */
  operation: string;
  /** Component initiating the operation */
  component: string;
  /** Timestamp when request was initiated */
  timestamp: Date;
  /** User ID if available */
  userId?: number;
  /** Session ID if available */
  sessionId?: string;
  /** Additional metadata */
  metadata?: Record<string, string>;
}

/**
 * Request ID generator with performance tracking
 */
export class RequestIdGenerator {
  private static instance: RequestIdGenerator | null = null;
  private generationTimes: number[] = [];
  private readonly maxSamples = 1000;

  private constructor() {}

  /**
   * Get singleton instance
   */
  public static getInstance(): RequestIdGenerator {
    if (!RequestIdGenerator.instance) {
      RequestIdGenerator.instance = new RequestIdGenerator();
    }
    return RequestIdGenerator.instance;
  }

  /**
   * Generate a new UUID v4 request ID with performance tracking
   * Requirement: <1ms generation time
   */
  public generateRequestId(): string {
    const start = performance.now();
    const requestId = crypto.randomUUID();
    const end = performance.now();
    
    const generationTime = end - start;
    this.generationTimes.push(generationTime);
    
    // Keep only last N samples for performance monitoring
    if (this.generationTimes.length > this.maxSamples) {
      this.generationTimes.shift();
    }
    
    return requestId;
  }

  /**
   * Get average generation time in milliseconds
   */
  public getAverageGenerationTime(): number {
    if (this.generationTimes.length === 0) return 0;
    
    const sum = this.generationTimes.reduce((acc, time) => acc + time, 0);
    return sum / this.generationTimes.length;
  }

  /**
   * Check if performance requirement (<1ms) is being met
   */
  public meetsPerformanceRequirement(): boolean {
    const avgTime = this.getAverageGenerationTime();
    return avgTime < 1.0; // 1ms requirement
  }

  /**
   * Get performance statistics
   */
  public getPerformanceStats() {
    if (this.generationTimes.length === 0) {
      return {
        sampleCount: 0,
        averageTimeMs: 0,
        minTimeMs: 0,
        maxTimeMs: 0,
        meetsRequirement: true
      };
    }

    const minTime = Math.min(...this.generationTimes);
    const maxTime = Math.max(...this.generationTimes);
    const avgTime = this.getAverageGenerationTime();

    return {
      sampleCount: this.generationTimes.length,
      averageTimeMs: avgTime,
      minTimeMs: minTime,
      maxTimeMs: maxTime,
      meetsRequirement: avgTime < 1.0
    };
  }
}

/**
 * Request ID validation utilities
 */
export class RequestIdValidator {
  /**
   * Validate request ID format (UUID v4)
   */
  public static isValidRequestId(requestId: string): boolean {
    if (!requestId || typeof requestId !== 'string') {
      return false;
    }
    
    return REQUEST_ID_REGEX.test(requestId);
  }

  /**
   * Normalize request ID format
   */
  public static normalizeRequestId(requestId: string): string {
    return requestId.toLowerCase();
  }

  /**
   * Extract request ID from various input formats
   */
  public static extractRequestId(input: any): string | null {
    if (typeof input === 'string') {
      return this.isValidRequestId(input) ? this.normalizeRequestId(input) : null;
    }
    
    if (typeof input === 'object' && input !== null) {
      // Check common property names
      const candidates = ['requestId', 'request_id', 'id', 'correlationId', 'correlation_id'];
      
      for (const prop of candidates) {
        if (input[prop] && this.isValidRequestId(input[prop])) {
          return this.normalizeRequestId(input[prop]);
        }
      }
    }
    
    return null;
  }
}

/**
 * Request context manager for operation tracking
 */
export class RequestContextManager {
  private static instance: RequestContextManager | null = null;
  private activeRequests: Map<string, RequestContext> = new Map();

  private constructor() {}

  /**
   * Get singleton instance
   */
  public static getInstance(): RequestContextManager {
    if (!RequestContextManager.instance) {
      RequestContextManager.instance = new RequestContextManager();
    }
    return RequestContextManager.instance;
  }

  /**
   * Create a new request context
   */
  public createRequestContext(
    operation: string,
    component: string,
    options: {
      userId?: number;
      sessionId?: string;
      metadata?: Record<string, string>;
    } = {}
  ): RequestContext {
    const requestId = RequestIdGenerator.getInstance().generateRequestId();
    
    const context: RequestContext = {
      requestId,
      operation,
      component,
      timestamp: new Date(),
      userId: options.userId,
      sessionId: options.sessionId,
      metadata: options.metadata
    };

    this.activeRequests.set(requestId, context);
    return context;
  }

  /**
   * Get request context by ID
   */
  public getRequestContext(requestId: string): RequestContext | null {
    return this.activeRequests.get(requestId) || null;
  }

  /**
   * Update request context
   */
  public updateRequestContext(
    requestId: string, 
    updates: Partial<RequestContext>
  ): boolean {
    const context = this.activeRequests.get(requestId);
    if (!context) return false;

    // Don't allow changing core identifiers
    const { requestId: _, timestamp: __, ...allowedUpdates } = updates;
    
    Object.assign(context, allowedUpdates);
    return true;
  }

  /**
   * Complete a request (remove from active tracking)
   */
  public completeRequest(requestId: string): boolean {
    return this.activeRequests.delete(requestId);
  }

  /**
   * Get all active requests
   */
  public getActiveRequests(): RequestContext[] {
    return Array.from(this.activeRequests.values());
  }

  /**
   * Get active requests count
   */
  public getActiveRequestCount(): number {
    return this.activeRequests.size;
  }

  /**
   * Clean up old requests (older than specified minutes)
   */
  public cleanupOldRequests(maxAgeMinutes: number = 30): number {
    const cutoffTime = new Date(Date.now() - maxAgeMinutes * 60 * 1000);
    let removedCount = 0;

    for (const [requestId, context] of this.activeRequests.entries()) {
      if (context.timestamp < cutoffTime) {
        this.activeRequests.delete(requestId);
        removedCount++;
      }
    }

    return removedCount;
  }
}

/**
 * Enhanced invoke function with request ID tracking
 */
export async function invokeWithRequestId<T>(
  command: string,
  args: Record<string, any> = {},
  context?: RequestContext
): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  
  // Create or use provided context
  const requestContext = context || RequestContextManager.getInstance().createRequestContext(
    command,
    'Frontend'
  );

  try {
    // Add request ID to arguments
    const argsWithRequestId = {
      ...args,
      request_id: requestContext.requestId
    };

    const result = await invoke<T>(command, argsWithRequestId);
    
    // Mark request as completed
    RequestContextManager.getInstance().completeRequest(requestContext.requestId);
    
    return result;
  } catch (error) {
    // Don't remove request context on error - it may be needed for correlation
    throw error;
  }
}

/**
 * Request ID utilities for operation tracking
 */
export const RequestTracker = {
  /**
   * Generate a new request ID
   */
  generateId(): string {
    return RequestIdGenerator.getInstance().generateRequestId();
  },

  /**
   * Validate request ID format
   */
  isValidId(requestId: string): boolean {
    return RequestIdValidator.isValidRequestId(requestId);
  },

  /**
   * Create request context for operation
   */
  createContext(operation: string, component: string, options: {
    userId?: number;
    sessionId?: string;
    metadata?: Record<string, string>;
  } = {}): RequestContext {
    return RequestContextManager.getInstance().createRequestContext(operation, component, options);
  },

  /**
   * Get request context by ID
   */
  getContext(requestId: string): RequestContext | null {
    return RequestContextManager.getInstance().getRequestContext(requestId);
  },

  /**
   * Complete request tracking
   */
  completeRequest(requestId: string): boolean {
    return RequestContextManager.getInstance().completeRequest(requestId);
  },

  /**
   * Get performance statistics
   */
  getPerformanceStats() {
    return RequestIdGenerator.getInstance().getPerformanceStats();
  },

  /**
   * Check if performance requirements are met
   */
  meetsPerformanceRequirement(): boolean {
    return RequestIdGenerator.getInstance().meetsPerformanceRequirement();
  }
};

// Export for convenience
export default RequestTracker;