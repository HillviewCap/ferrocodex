/**
 * Enhanced Error Handling Types for Frontend
 * 
 * This file provides optional enhanced error interfaces that work alongside
 * existing string-based error handling patterns. These interfaces allow
 * progressive enhancement of error handling without breaking existing code.
 */

// Basic error severity levels matching Rust backend
export type ErrorSeverity = 'Critical' | 'High' | 'Medium' | 'Low';

// Error domain classification matching Rust backend
export type ErrorDomain = 'Auth' | 'Data' | 'Assets' | 'System' | 'UI';

// Recovery strategy classification matching Rust backend
export type RecoveryStrategy = 'AutoRecoverable' | 'UserRecoverable' | 'AdminRecoverable' | 'ManualRecoverable';

/**
 * Enhanced Error Interface - Optional interface for accessing detailed error information
 * This interface is available alongside existing string error handling
 */
export interface EnhancedError {
  /** Unique identifier for this error instance */
  id: string;
  /** Error severity level */
  severity: ErrorSeverity;
  /** Domain classification */
  domain: ErrorDomain;
  /** Recovery strategy */
  recovery_strategy: RecoveryStrategy;
  /** Original error message */
  message: string;
  /** Detailed error description for debugging */
  details?: string;
  /** Error context identifier */
  context_id?: string;
  /** Timestamp when error occurred */
  timestamp: string; // ISO 8601 format
  /** Optional correlation ID for tracking across layers */
  correlation_id?: string;
  /** Component where error originated */
  component?: string;
  /** Operation that was being performed when error occurred */
  operation?: string;
}

/**
 * Error Context Information - Additional context about where errors occurred
 */
export interface ErrorContext {
  /** Unique context identifier */
  context_id: string;
  /** Operation being performed */
  operation: string;
  /** Component where operation occurred */
  component: string;
  /** User ID if available */
  user_id?: number;
  /** Session information if available */
  session_info?: SessionInfo;
  /** Request path if applicable */
  request_path?: string;
  /** Client information */
  client_info?: ClientInfo;
  /** Additional metadata */
  metadata?: Record<string, string>;
  /** Context creation timestamp */
  created_at: string; // ISO 8601 format
  /** Optional correlation ID */
  correlation_id?: string;
}

/**
 * Session information for error context
 */
export interface SessionInfo {
  /** Session ID */
  session_id: string;
  /** User ID associated with session */
  user_id: number;
  /** Session creation time */
  created_at: string;
  /** Session expiry time */
  expires_at: string;
}

/**
 * Client information for error context
 */
export interface ClientInfo {
  /** User agent string */
  user_agent?: string;
  /** Client IP address (if available) */
  ip_address?: string;
  /** Platform information */
  platform?: string;
  /** Application version */
  app_version?: string;
}

/**
 * Performance statistics for error handling
 */
export interface PerformanceStats {
  /** Average processing time in microseconds */
  average_time_us: number;
  /** Total number of operations */
  total_operations: number;
  /** Whether performance requirements are met */
  meets_requirements: boolean;
}

/**
 * Enhanced Error Access Interface - Optional interface for accessing enhanced error details
 * This interface can be implemented by error handling utilities to provide enhanced access
 * without breaking existing string-based error handling
 */
export interface EnhancedErrorAccess {
  /** Check if enhanced error information is available */
  hasEnhancedInfo(): boolean;
  
  /** Get enhanced error information if available */
  getEnhancedInfo(): EnhancedError | null;
  
  /** Get error context if available */
  getErrorContext(): ErrorContext | null;
  
  /** Get user-friendly error message (same as string error for compatibility) */
  getUserMessage(): string;
  
  /** Get debug error message with full context */
  getDebugMessage(): string;
  
  /** Get error severity */
  getSeverity(): ErrorSeverity | null;
  
  /** Get error domain */
  getDomain(): ErrorDomain | null;
  
  /** Get recovery strategy */
  getRecoveryStrategy(): RecoveryStrategy | null;
  
  /** Check if error is critical */
  isCritical(): boolean;
  
  /** Check if error is auto-recoverable */
  isAutoRecoverable(): boolean;
}

/**
 * Error Handling Feature Flags - Controls which enhanced features are enabled
 */
export interface ErrorHandlingFeatureFlags {
  /** Enable enhanced error classification */
  enable_classification: boolean;
  /** Enable error context tracking */
  enable_context_tracking: boolean;
  /** Enable cross-layer correlation */
  enable_correlation: boolean;
  /** Enable database storage of error data */
  enable_database_storage: boolean;
  /** Enable performance monitoring */
  enable_performance_monitoring: boolean;
}

/**
 * Enhanced Error Wrapper - Wraps string errors with optional enhanced functionality
 * This allows progressive enhancement while maintaining backward compatibility
 */
export class EnhancedErrorWrapper implements EnhancedErrorAccess {
  private errorString: string;
  private enhancedInfo: EnhancedError | null = null;
  private errorContext: ErrorContext | null = null;

  constructor(errorString: string, enhancedInfo?: EnhancedError, errorContext?: ErrorContext) {
    this.errorString = errorString;
    this.enhancedInfo = enhancedInfo || null;
    this.errorContext = errorContext || null;
  }

  // Implement string conversion for backward compatibility
  toString(): string {
    return this.errorString;
  }

  // Implement valueOf for primitive conversion
  valueOf(): string {
    return this.errorString;
  }

  // Enhanced error access methods
  hasEnhancedInfo(): boolean {
    return this.enhancedInfo !== null;
  }

  getEnhancedInfo(): EnhancedError | null {
    return this.enhancedInfo;
  }

  getErrorContext(): ErrorContext | null {
    return this.errorContext;
  }

  getUserMessage(): string {
    return this.errorString;
  }

  getDebugMessage(): string {
    if (this.enhancedInfo) {
      const info = this.enhancedInfo;
      const debugParts = [
        `ID: ${info.id}`,
        `Severity: ${info.severity}`,
        `Domain: ${info.domain}`,
        `Recovery: ${info.recovery_strategy}`,
        `Message: ${info.message}`,
        `Timestamp: ${info.timestamp}`
      ];

      if (info.details) debugParts.push(`Details: ${info.details}`);
      if (info.context_id) debugParts.push(`Context ID: ${info.context_id}`);
      if (info.correlation_id) debugParts.push(`Correlation ID: ${info.correlation_id}`);
      if (info.component) debugParts.push(`Component: ${info.component}`);
      if (info.operation) debugParts.push(`Operation: ${info.operation}`);

      return `[Enhanced Error Debug]\n${debugParts.join('\n')}`;
    }
    return this.errorString;
  }

  getSeverity(): ErrorSeverity | null {
    return this.enhancedInfo?.severity || null;
  }

  getDomain(): ErrorDomain | null {
    return this.enhancedInfo?.domain || null;
  }

  getRecoveryStrategy(): RecoveryStrategy | null {
    return this.enhancedInfo?.recovery_strategy || null;
  }

  isCritical(): boolean {
    return this.enhancedInfo?.severity === 'Critical' || false;
  }

  isAutoRecoverable(): boolean {
    return this.enhancedInfo?.recovery_strategy === 'AutoRecoverable' || false;
  }

  // Static factory methods for creating enhanced error wrappers
  static fromString(errorString: string): EnhancedErrorWrapper {
    return new EnhancedErrorWrapper(errorString);
  }

  static fromEnhanced(errorString: string, enhancedInfo: EnhancedError, errorContext?: ErrorContext): EnhancedErrorWrapper {
    return new EnhancedErrorWrapper(errorString, enhancedInfo, errorContext);
  }
}

/**
 * Utility functions for working with enhanced errors
 */
export class ErrorHandlingUtils {
  /** Check if an error object has enhanced information */
  static hasEnhancedInfo(error: any): error is EnhancedErrorAccess {
    return error && typeof error.hasEnhancedInfo === 'function' && error.hasEnhancedInfo();
  }

  /** Extract enhanced error information if available, otherwise return null */
  static getEnhancedInfo(error: any): EnhancedError | null {
    if (this.hasEnhancedInfo(error)) {
      return error.getEnhancedInfo();
    }
    return null;
  }

  /** Get user-friendly message from any error (enhanced or string) */
  static getUserMessage(error: any): string {
    if (this.hasEnhancedInfo(error)) {
      return error.getUserMessage();
    }
    return String(error);
  }

  /** Get debug message from any error (enhanced or string) */
  static getDebugMessage(error: any): string {
    if (this.hasEnhancedInfo(error)) {
      return error.getDebugMessage();
    }
    return String(error);
  }

  /** Check if error is critical */
  static isCritical(error: any): boolean {
    if (this.hasEnhancedInfo(error)) {
      return error.isCritical();
    }
    // For string errors, check if they contain critical indicators
    const errorStr = String(error).toLowerCase();
    return errorStr.includes('critical') || errorStr.includes('fatal') || errorStr.includes('system failure');
  }

  /** Check if error is auto-recoverable */
  static isAutoRecoverable(error: any): boolean {
    if (this.hasEnhancedInfo(error)) {
      return error.isAutoRecoverable();
    }
    // For string errors, assume they are not auto-recoverable by default
    return false;
  }

  /** Create enhanced error wrapper from Tauri command result */
  static fromTauriResult<T>(result: Promise<T>): Promise<T> {
    return result.catch((error) => {
      // If the error is already enhanced, return it as-is
      if (this.hasEnhancedInfo(error)) {
        return Promise.reject(error);
      }
      
      // Wrap string errors in EnhancedErrorWrapper for optional enhancement
      const wrapper = EnhancedErrorWrapper.fromString(String(error));
      return Promise.reject(wrapper);
    });
  }
}

/**
 * Backward compatibility type aliases
 * These ensure existing code continues to work unchanged
 */
export type ErrorResult<T> = Promise<T>; // Standard Promise-based error handling
export type StringError = string; // Traditional string error type

/**
 * Optional enhanced error result type for new code
 * Can be used alongside existing error handling patterns
 */
export type EnhancedErrorResult<T> = Promise<T>; // Same signature for compatibility, enhanced through wrappers