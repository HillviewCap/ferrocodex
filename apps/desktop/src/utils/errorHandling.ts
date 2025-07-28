/**
 * Error Handling Utilities - Backward Compatible Enhanced Error Support
 * 
 * This utility module provides enhanced error handling capabilities while
 * maintaining full backward compatibility with existing string-based error patterns.
 * All functions work with both enhanced and traditional errors.
 */

import { 
  EnhancedErrorWrapper,
  ErrorHandlingUtils
} from '../types/error-handling';
import type { 
  ErrorSeverity,
  ErrorDomain
} from '../types/error-handling';

/**
 * Traditional error handling functions - preserved for backward compatibility
 * These functions work exactly as before but now support optional enhanced features
 */

/**
 * Display user-friendly error message (backward compatible)
 * Works with both string errors and enhanced errors
 */
export const getErrorMessage = (error: any): string => {
  return ErrorHandlingUtils.getUserMessage(error);
};

/**
 * Check if error indicates a critical system issue (enhanced)
 * Fallback logic for string errors maintained for compatibility
 */
export const isCriticalError = (error: any): boolean => {
  return ErrorHandlingUtils.isCritical(error);
};

/**
 * Check if error might be recoverable by retry (enhanced)
 * Conservative fallback for string errors
 */
export const isRecoverableError = (error: any): boolean => {
  if (ErrorHandlingUtils.hasEnhancedInfo(error)) {
    const recoveryStrategy = error.getRecoveryStrategy();
    return recoveryStrategy === 'AutoRecoverable' || recoveryStrategy === 'UserRecoverable';
  }
  
  // Fallback logic for string errors
  const errorStr = String(error).toLowerCase();
  return errorStr.includes('timeout') || 
         errorStr.includes('network') || 
         errorStr.includes('connection') ||
         errorStr.includes('retry');
};

/**
 * Get error severity level (enhanced feature with fallback)
 */
export const getErrorSeverity = (error: any): ErrorSeverity => {
  const severity = ErrorHandlingUtils.hasEnhancedInfo(error) 
    ? error.getSeverity() 
    : null;
    
  if (severity) return severity;
  
  // Fallback classification for string errors
  const errorStr = String(error).toLowerCase();
  if (errorStr.includes('critical') || errorStr.includes('fatal') || errorStr.includes('system failure')) {
    return 'Critical';
  }
  if (errorStr.includes('authentication') || errorStr.includes('authorization') || errorStr.includes('permission') || 
      errorStr.includes('database') || errorStr.includes('connection')) {
    return 'High';
  }
  if (errorStr.includes('validation') || errorStr.includes('invalid')) {
    return 'Medium';
  }
  return 'Low';
};

/**
 * Get error domain classification (enhanced feature with fallback)
 */
export const getErrorDomain = (error: any): ErrorDomain => {
  const domain = ErrorHandlingUtils.hasEnhancedInfo(error) 
    ? error.getDomain() 
    : null;
    
  if (domain) return domain;
  
  // Fallback classification for string errors
  const errorStr = String(error).toLowerCase();
  if (errorStr.includes('auth') || errorStr.includes('login') || errorStr.includes('session')) {
    return 'Auth';
  }
  if (errorStr.includes('database') || errorStr.includes('data') || errorStr.includes('sql') || errorStr.includes('connection')) {
    return 'Data';
  }
  if (errorStr.includes('asset') || errorStr.includes('configuration') || errorStr.includes('firmware')) {
    return 'Assets';
  }
  if (errorStr.includes('system') || errorStr.includes('server')) {
    return 'System';
  }
  return 'UI';
};

/**
 * Format error for display in notifications (backward compatible)
 * Enhanced version provides better formatting when available
 */
export const formatErrorForDisplay = (error: any): string => {
  const message = getErrorMessage(error);
  const severity = getErrorSeverity(error);
  
  // For critical errors, always use generic message
  if (severity === 'Critical') {
    return 'A critical error has occurred. Please contact support.';
  }
  
  return message;
};

/**
 * Format error for logging (enhanced with fallback)
 * Provides detailed information when available, basic info otherwise
 */
export const formatErrorForLogging = (error: any): string => {
  if (ErrorHandlingUtils.hasEnhancedInfo(error)) {
    return error.getDebugMessage();
  }
  
  // Fallback formatting for string errors
  const timestamp = new Date().toISOString();
  const severity = getErrorSeverity(error);
  const domain = getErrorDomain(error);
  
  return `[${timestamp}] [${severity}:${domain}] ${String(error)}`;
};

/**
 * Determine appropriate user action for error (enhanced with fallback)
 */
export const getErrorAction = (error: any): string => {
  const severity = getErrorSeverity(error);
  const domain = getErrorDomain(error);
  const isRecoverable = isRecoverableError(error);
  
  if (severity === 'Critical') {
    return 'Contact system administrator immediately.';
  }
  
  if (domain === 'Auth') {
    return 'Please check your credentials and try logging in again.';
  }
  
  if (isRecoverable) {
    return 'Please wait a moment and try again.';
  }
  
  switch (domain) {
    case 'Data':
      return 'Please check your input and try again.';
    case 'Assets':
      return 'Please verify the asset information and retry.';
    case 'System':
      return 'System issue detected. Please try again later.';
    default:
      // Check for specific error patterns for UI domain
      const errorStr = String(error).toLowerCase();
      if (errorStr.includes('validation') || errorStr.includes('invalid')) {
        return 'Please check your input and try again.';
      }
      return 'Please try again or contact support if the problem persists.';
  }
};

/**
 * Context-aware error message generation based on error classification
 */
export interface MessageGenerationContext {
  /** User role for message customization */
  userRole?: 'Admin' | 'Engineer' | 'Operator';
  /** Current operation context */
  operation?: string;
  /** Component where error occurred */
  component?: string;
  /** Whether to include technical details */
  includeTechnicalDetails?: boolean;
}

/**
 * Enhanced error message templates by domain and severity
 */
const ERROR_MESSAGE_TEMPLATES = {
  Auth: {
    Critical: {
      user: 'Authentication system is unavailable. Please contact your administrator immediately.',
      admin: 'Critical authentication failure detected. Check system logs and authentication service status.',
    },
    High: {
      user: 'Login failed. Please check your credentials and try again.',
      admin: 'Authentication failure: Invalid credentials or session expired. User: {context}',
    },
    Medium: {
      user: 'Session expired. Please log in again.',
      admin: 'Session timeout occurred. User session: {context}',
    },
    Low: {
      user: 'Please verify your login information.',
      admin: 'Authentication warning: {context}',
    }
  },
  Data: {
    Critical: {
      user: 'Data system error occurred. Your work has been saved. Please contact support.',
      admin: 'Critical data integrity issue detected. Check database connectivity and corruption status.',
    },
    High: {
      user: 'Unable to save your changes. Please try again.',
      admin: 'Database operation failed: {context}',
    },
    Medium: {
      user: 'Some data could not be loaded. Please refresh and try again.',
      admin: 'Data retrieval warning: {context}',
    },
    Low: {
      user: 'Data validation issue. Please check your input.',
      admin: 'Validation error: {context}',
    }
  },
  Assets: {
    Critical: {
      user: 'Asset management system is unavailable. Critical operations should not be performed.',
      admin: 'Critical asset system failure. Check asset database and configuration integrity.',
    },
    High: {
      user: 'Unable to access asset information. Please try again.',
      admin: 'Asset operation failed: {context}',
    },
    Medium: {
      user: 'Asset configuration issue detected. Please verify settings.',
      admin: 'Asset configuration warning: {context}',
    },
    Low: {
      user: 'Please check asset information and try again.',
      admin: 'Asset validation issue: {context}',
    }
  },
  System: {
    Critical: {
      user: 'System emergency detected. Please contact administrator and avoid critical operations.',
      admin: 'CRITICAL SYSTEM ALERT: {context} - Immediate attention required.',
    },
    High: {
      user: 'System error occurred. Please wait and try again.',
      admin: 'System error detected: {context}',
    },
    Medium: {
      user: 'Temporary system issue. Please try again in a moment.',
      admin: 'System warning: {context}',
    },
    Low: {
      user: 'Minor system issue detected. Operation may proceed.',
      admin: 'System notice: {context}',
    }
  },
  UI: {
    Critical: {
      user: 'Interface error occurred. Please refresh the application.',
      admin: 'Critical UI failure: {context} - Check component state and rendering.',
    },
    High: {
      user: 'Page error occurred. Please refresh and try again.',
      admin: 'UI component error: {context}',
    },
    Medium: {
      user: 'Display issue detected. Some information may not be current.',
      admin: 'UI rendering warning: {context}',
    },
    Low: {
      user: 'Minor display issue. Please continue.',
      admin: 'UI notice: {context}',
    }
  }
} as const;

/**
 * Error processing context for enhanced context-aware processing
 */
export interface ErrorProcessingContext {
  /** Current user information */
  user?: {
    id?: number;
    role: 'Admin' | 'Engineer' | 'Operator';
    permissions?: string[];
  };
  /** Current session information */
  session?: {
    sessionId?: string;
    startTime?: Date;
    expiresAt?: Date;
  };
  /** Operation context */
  operation?: {
    name: string;
    component: string;
    startTime?: Date;
    parameters?: Record<string, any>;
  };
  /** Request context */
  request?: {
    correlationId?: string;
    path?: string;
    method?: string;
  };
  /** Client context */
  client?: {
    userAgent?: string;
    platform?: string;
    appVersion?: string;
  };
}

/**
 * Context-aware error processor
 */
export class ContextAwareErrorProcessor {
  private static instance: ContextAwareErrorProcessor;
  private currentContext: ErrorProcessingContext = {};

  static getInstance(): ContextAwareErrorProcessor {
    if (!ContextAwareErrorProcessor.instance) {
      ContextAwareErrorProcessor.instance = new ContextAwareErrorProcessor();
    }
    return ContextAwareErrorProcessor.instance;
  }

  /**
   * Set the current processing context
   */
  setContext(context: Partial<ErrorProcessingContext>): void {
    this.currentContext = { ...this.currentContext, ...context };
  }

  /**
   * Get the current processing context
   */
  getContext(): ErrorProcessingContext {
    return { ...this.currentContext };
  }

  /**
   * Clear specific context sections
   */
  clearContext(sections?: Array<keyof ErrorProcessingContext>): void {
    if (!sections) {
      this.currentContext = {};
      return;
    }
    
    sections.forEach(section => {
      delete this.currentContext[section];
    });
  }

  /**
   * Process error with current context
   */
  processError(error: any): ProcessedErrorInfo {
    const context = this.getContext();
    const severity = getErrorSeverity(error);
    const domain = getErrorDomain(error);
    const isRecoverable = isRecoverableError(error);

    // Generate context-aware message
    const messageContext: MessageGenerationContext = {
      userRole: context.user?.role || 'Operator',
      operation: context.operation?.name,
      component: context.operation?.component,
      includeTechnicalDetails: context.user?.role === 'Admin'
    };

    const userMessage = this.generateContextAwareMessage(error, messageContext);
    const debugMessage = this.generateDebugMessage(error, context);

    return {
      error,
      severity,
      domain,
      isRecoverable,
      userMessage,
      debugMessage,
      context: { ...context },
      timestamp: new Date().toISOString(),
      correlationId: context.request?.correlationId || this.generateCorrelationId()
    };
  }

  /**
   * Generate context-aware error message
   */
  private generateContextAwareMessage(
    error: any, 
    context: MessageGenerationContext
  ): string {
    const severity = getErrorSeverity(error);
    const domain = getErrorDomain(error);
    const { userRole = 'Operator', includeTechnicalDetails = false } = context;
    
    // Determine message audience (user vs admin)
    const isAdminMessage = userRole === 'Admin' || includeTechnicalDetails;
    const audience = isAdminMessage ? 'admin' : 'user';
    
    // Get base template
    const template = ERROR_MESSAGE_TEMPLATES[domain]?.[severity]?.[audience];
    if (!template) {
      // Fallback to generic message
      return isAdminMessage 
        ? `${severity} ${domain} error: ${getErrorMessage(error)}`
        : 'An error occurred. Please try again or contact support if the problem persists.';
    }
    
    // Replace context placeholders
    let message: string = template;
    if (message.includes('{context}') && context.operation) {
      message = message.replace('{context}', context.operation);
    } else if (message.includes('{context}')) {
      message = message.replace('{context}', getErrorMessage(error) || 'Unknown error');
    }
    
    return message;
  }

  /**
   * Generate debug message with full context
   */
  private generateDebugMessage(error: any, context: ErrorProcessingContext): string {
    const parts = [
      `[ERROR DEBUG]`,
      `Timestamp: ${new Date().toISOString()}`,
      `Severity: ${getErrorSeverity(error)}`,
      `Domain: ${getErrorDomain(error)}`,
      `Message: ${getErrorMessage(error)}`
    ];

    if (context.user) {
      parts.push(`User: ${context.user.id || 'unknown'} (${context.user.role})`);
    }

    if (context.operation) {
      parts.push(`Operation: ${context.operation.name} in ${context.operation.component}`);
    }

    if (context.request?.correlationId) {
      parts.push(`Correlation ID: ${context.request.correlationId}`);
    }

    if (context.session?.sessionId) {
      parts.push(`Session: ${context.session.sessionId}`);
    }

    if (context.client?.platform) {
      parts.push(`Platform: ${context.client.platform}`);
    }

    // Add enhanced error details if available
    if (ErrorHandlingUtils.hasEnhancedInfo(error)) {
      const enhanced = error.getEnhancedInfo();
      if (enhanced) {
        parts.push(`Enhanced ID: ${enhanced.id}`);
        if (enhanced.details) {
          parts.push(`Details: ${enhanced.details}`);
        }
      }
    }

    return parts.join('\n');
  }

  /**
   * Generate correlation ID for tracking
   */
  private generateCorrelationId(): string {
    return `err_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}

/**
 * Error message localization framework
 */
export interface LocalizedErrorTemplates {
  [locale: string]: typeof ERROR_MESSAGE_TEMPLATES;
}

/**
 * Error message localization service
 */
export class ErrorLocalizationService {
  private static instance: ErrorLocalizationService;
  private currentLocale: string = 'en';
  private templates: LocalizedErrorTemplates = {
    en: ERROR_MESSAGE_TEMPLATES
  };

  static getInstance(): ErrorLocalizationService {
    if (!ErrorLocalizationService.instance) {
      ErrorLocalizationService.instance = new ErrorLocalizationService();
    }
    return ErrorLocalizationService.instance;
  }

  /**
   * Set current locale
   */
  setLocale(locale: string): void {
    this.currentLocale = locale;
  }

  /**
   * Get current locale
   */
  getLocale(): string {
    return this.currentLocale;
  }

  /**
   * Add localized templates
   */
  addLocaleTemplates(locale: string, templates: typeof ERROR_MESSAGE_TEMPLATES): void {
    this.templates[locale] = templates;
  }

  /**
   * Get localized templates for current locale
   */
  getTemplates(): typeof ERROR_MESSAGE_TEMPLATES {
    return this.templates[this.currentLocale] || this.templates['en'];
  }

  /**
   * Get localized message template
   */
  getTemplate(domain: ErrorDomain, severity: ErrorSeverity, audience: 'user' | 'admin'): string | undefined {
    const templates = this.getTemplates();
    // @ts-ignore - Dynamic access needed for localization
    return templates[domain]?.[severity]?.[audience];
  }
}

/**
 * Error message validation utilities
 */
export class ErrorMessageValidator {
  /**
   * Validate error message content
   */
  static validateMessage(message: string): {
    isValid: boolean;
    issues: string[];
    suggestions: string[];
  } {
    const issues: string[] = [];
    const suggestions: string[] = [];

    // Check message length
    if (message.length < 10) {
      issues.push('Message too short');
      suggestions.push('Provide more descriptive error information');
    }

    if (message.length > 200) {
      issues.push('Message too long');
      suggestions.push('Consider using progressive disclosure for detailed information');
    }

    // Check for technical jargon that users might not understand
    const technicalTerms = [
      'null pointer', 'segfault', 'stack trace', 'exception', 'SQL',
      'HTTP 500', 'buffer overflow', 'memory leak', 'thread',
      'mutex', 'deadlock', 'race condition'
    ];

    const lowerMessage = message.toLowerCase();
    technicalTerms.forEach(term => {
      if (lowerMessage.includes(term.toLowerCase())) {
        issues.push(`Contains technical term: "${term}"`);
        suggestions.push(`Consider user-friendly alternative for "${term}"`);
      }
    });

    // Check for security information exposure
    const securityRisks = [
      'password', 'token', 'key', 'secret', 'credential',
      'internal server', 'database connection', 'file path',
      'ip address', 'port number'
    ];

    securityRisks.forEach(risk => {
      if (lowerMessage.includes(risk.toLowerCase())) {
        issues.push(`Potential security information exposure: "${risk}"`);
        suggestions.push(`Avoid exposing sensitive information like "${risk}"`);
      }
    });

    // Check for actionable guidance
    const actionWords = ['try', 'check', 'verify', 'contact', 'refresh', 'retry'];
    const hasActionableGuidance = actionWords.some(word => 
      lowerMessage.includes(word.toLowerCase())
    );

    if (!hasActionableGuidance) {
      issues.push('Message lacks actionable guidance');
      suggestions.push('Include specific next steps for the user');
    }

    return {
      isValid: issues.length === 0,
      issues,
      suggestions
    };
  }

  /**
   * Validate error template structure
   */
  static validateTemplate(templates: typeof ERROR_MESSAGE_TEMPLATES): {
    isValid: boolean;
    missingTemplates: string[];
    validationResults: Record<string, any>;
  } {
    const missingTemplates: string[] = [];
    const validationResults: Record<string, any> = {};

    const domains: ErrorDomain[] = ['Auth', 'Data', 'Assets', 'System', 'UI'];
    const severities: ErrorSeverity[] = ['Critical', 'High', 'Medium', 'Low'];
    const audiences = ['user', 'admin'] as const;

    domains.forEach(domain => {
      severities.forEach(severity => {
        audiences.forEach(audience => {
          const key = `${domain}.${severity}.${audience}`;
          // @ts-ignore - Dynamic access needed for validation
          const template = templates[domain]?.[severity]?.[audience];
          
          if (!template) {
            missingTemplates.push(key);
          } else {
            const validation = this.validateMessage(template);
            if (!validation.isValid) {
              validationResults[key] = validation;
            }
          }
        });
      });
    });

    return {
      isValid: missingTemplates.length === 0 && Object.keys(validationResults).length === 0,
      missingTemplates,
      validationResults
    };
  }

  /**
   * Test error message generation for all scenarios
   */
  static testMessageGeneration(): {
    passed: number;
    failed: number;
    results: Array<{
      scenario: string;
      passed: boolean;
      message?: string;
      error?: string;
    }>;
  } {
    const results: Array<{
      scenario: string;
      passed: boolean;
      message?: string;
      error?: string;
    }> = [];

    const testErrors = [
      { error: 'Authentication failed', expectedDomain: 'Auth' },
      { error: 'Database connection lost', expectedDomain: 'Data' },
      { error: 'Asset not found', expectedDomain: 'Assets' },
      { error: 'System overload detected', expectedDomain: 'System' },
      { error: 'Component rendering failed', expectedDomain: 'UI' }
    ];

    const userRoles: Array<'Admin' | 'Engineer' | 'Operator'> = ['Admin', 'Engineer', 'Operator'];

    testErrors.forEach(testCase => {
      userRoles.forEach(role => {
        const scenario = `${testCase.expectedDomain} error for ${role}`;
        
        try {
          const context: MessageGenerationContext = {
            userRole: role,
            operation: 'test_operation'
          };
          
          const message = generateContextAwareMessage(testCase.error, context);
          
          if (message && message.length > 0) {
            results.push({
              scenario,
              passed: true,
              message
            });
          } else {
            results.push({
              scenario,
              passed: false,
              error: 'Empty message generated'
            });
          }
        } catch (error) {
          results.push({
            scenario,
            passed: false,
            error: String(error)
          });
        }
      });
    });

    const passed = results.filter(r => r.passed).length;
    const failed = results.filter(r => !r.passed).length;

    return { passed, failed, results };
  }
}

/**
 * Recovery action definitions
 */
export interface RecoveryAction {
  id: string;
  label: string;
  description: string;
  type: 'automatic' | 'user_guided' | 'manual';
  priority: 'high' | 'medium' | 'low';
  handler?: () => Promise<boolean>;
  prerequisites?: string[];
  estimatedTime?: string;
  riskLevel?: 'low' | 'medium' | 'high';
}

/**
 * Recovery action suggestion engine
 */
export class RecoveryActionEngine {
  private static instance: RecoveryActionEngine;
  private actionRegistry: Map<string, RecoveryAction[]> = new Map();

  static getInstance(): RecoveryActionEngine {
    if (!RecoveryActionEngine.instance) {
      RecoveryActionEngine.instance = new RecoveryActionEngine();
      RecoveryActionEngine.instance.initializeDefaultActions();
    }
    return RecoveryActionEngine.instance;
  }

  /**
   * Initialize default recovery actions for each error domain
   */
  private initializeDefaultActions(): void {
    // Auth domain recovery actions
    this.actionRegistry.set('Auth:Critical', [
      {
        id: 'contact_admin_auth',
        label: 'Contact Administrator',
        description: 'Contact your system administrator immediately for authentication system issues.',
        type: 'manual',
        priority: 'high',
        estimatedTime: '15-30 minutes',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('Auth:High', [
      {
        id: 'retry_login',
        label: 'Retry Login',
        description: 'Clear login fields and try signing in again with correct credentials.',
        type: 'user_guided',
        priority: 'high',
        estimatedTime: '1-2 minutes',
        riskLevel: 'low'
      },
      {
        id: 'reset_session',
        label: 'Reset Session',
        description: 'Clear browser data and restart the application.',
        type: 'user_guided',
        priority: 'medium',
        estimatedTime: '2-3 minutes',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('Auth:Medium', [
      {
        id: 'refresh_login',
        label: 'Refresh and Retry',
        description: 'Refresh the page and log in again.',
        type: 'user_guided',
        priority: 'medium',
        estimatedTime: '30 seconds',
        riskLevel: 'low',
        handler: async () => {
          window.location.reload();
          return true;
        }
      }
    ]);

    // Data domain recovery actions
    this.actionRegistry.set('Data:Critical', [
      {
        id: 'contact_support_data',
        label: 'Contact Support',
        description: 'Contact technical support immediately for critical data issues.',
        type: 'manual',
        priority: 'high',
        estimatedTime: '30-60 minutes',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('Data:High', [
      {
        id: 'retry_operation',
        label: 'Retry Operation',
        description: 'Wait a moment and try the operation again.',
        type: 'user_guided',
        priority: 'high',
        estimatedTime: '1-2 minutes',
        riskLevel: 'low'
      },
      {
        id: 'check_connection',
        label: 'Check Connection',
        description: 'Verify your network connection and try again.',
        type: 'user_guided',
        priority: 'medium',
        estimatedTime: '2-3 minutes',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('Data:Medium', [
      {
        id: 'refresh_data',
        label: 'Refresh Data',
        description: 'Refresh the current view to reload data.',
        type: 'user_guided',
        priority: 'medium',
        estimatedTime: '30 seconds',
        riskLevel: 'low'
      }
    ]);

    // Assets domain recovery actions
    this.actionRegistry.set('Assets:Critical', [
      {
        id: 'halt_operations',
        label: 'Halt Critical Operations',
        description: 'Stop all critical asset operations and contact administrator.',
        type: 'manual',
        priority: 'high',
        estimatedTime: 'Immediate',
        riskLevel: 'medium'
      }
    ]);

    this.actionRegistry.set('Assets:High', [
      {
        id: 'verify_asset_config',
        label: 'Verify Asset Configuration',
        description: 'Check asset configuration and retry the operation.',
        type: 'user_guided',
        priority: 'high',
        estimatedTime: '5-10 minutes',
        riskLevel: 'medium'
      }
    ]);

    this.actionRegistry.set('Assets:Medium', [
      {
        id: 'reload_asset_data',
        label: 'Reload Asset Data',
        description: 'Refresh asset information and try again.',
        type: 'user_guided',
        priority: 'medium',
        estimatedTime: '1-2 minutes',
        riskLevel: 'low'
      }
    ]);

    // System domain recovery actions
    this.actionRegistry.set('System:Critical', [
      {
        id: 'emergency_contact',
        label: 'Emergency Contact',
        description: 'Contact system administrator immediately for critical system issues.',
        type: 'manual',
        priority: 'high',
        estimatedTime: 'Immediate',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('System:High', [
      {
        id: 'restart_application',
        label: 'Restart Application',
        description: 'Close and restart the application.',
        type: 'user_guided',
        priority: 'high',
        estimatedTime: '2-3 minutes',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('System:Medium', [
      {
        id: 'wait_and_retry',
        label: 'Wait and Retry',
        description: 'Wait a few minutes for the system to recover, then try again.',
        type: 'user_guided',
        priority: 'medium',
        estimatedTime: '3-5 minutes',
        riskLevel: 'low'
      }
    ]);

    // UI domain recovery actions
    this.actionRegistry.set('UI:Critical', [
      {
        id: 'refresh_application',
        label: 'Refresh Application',
        description: 'Refresh the application to restore functionality.',
        type: 'user_guided',
        priority: 'high',
        estimatedTime: '1 minute',
        riskLevel: 'low',
        handler: async () => {
          window.location.reload();
          return true;
        }
      }
    ]);

    this.actionRegistry.set('UI:High', [
      {
        id: 'reload_page',
        label: 'Reload Page',
        description: 'Reload the current page to fix display issues.',
        type: 'user_guided',
        priority: 'high',
        estimatedTime: '30 seconds',
        riskLevel: 'low'
      }
    ]);

    this.actionRegistry.set('UI:Medium', [
      {
        id: 'continue_anyway',
        label: 'Continue',
        description: 'The display issue is minor. You can continue working.',
        type: 'user_guided',
        priority: 'low',
        estimatedTime: 'None',
        riskLevel: 'low'
      }
    ]);
  }

  /**
   * Get recovery suggestions for an error
   */
  getSuggestedActions(error: any, context?: ErrorProcessingContext): RecoveryAction[] {
    const severity = getErrorSeverity(error);
    const domain = getErrorDomain(error);
    const key = `${domain}:${severity}`;
    
    const actions = this.actionRegistry.get(key) || [];
    
    // Filter actions based on user role if available
    if (context?.user?.role) {
      return this.filterActionsByUserRole(actions, context.user.role);
    }
    
    return actions;
  }

  /**
   * Filter actions based on user role
   */
  private filterActionsByUserRole(actions: RecoveryAction[], userRole: string): RecoveryAction[] {
    // Admin users get all actions
    if (userRole === 'Admin') {
      return actions;
    }
    
    // Engineers get most actions except high-risk ones
    if (userRole === 'Engineer') {
      return actions.filter(action => action.riskLevel !== 'high');
    }
    
    // Operators get only low-risk, user-guided actions
    return actions.filter(action => 
      action.type === 'user_guided' && 
      action.riskLevel === 'low'
    );
  }

  /**
   * Register custom recovery action
   */
  registerAction(domain: ErrorDomain, severity: ErrorSeverity, action: RecoveryAction): void {
    const key = `${domain}:${severity}`;
    const existing = this.actionRegistry.get(key) || [];
    existing.push(action);
    this.actionRegistry.set(key, existing);
  }

  /**
   * Execute a recovery action
   */
  async executeAction(actionId: string, actions: RecoveryAction[]): Promise<{
    success: boolean;
    message: string;
    error?: string;
  }> {
    const action = actions.find(a => a.id === actionId);
    if (!action) {
      return {
        success: false,
        message: 'Recovery action not found',
        error: `Action with ID ${actionId} not found`
      };
    }

    if (!action.handler) {
      return {
        success: false,
        message: 'This action requires manual execution',
        error: 'No automatic handler available'
      };
    }

    try {
      const result = await action.handler();
      return {
        success: result,
        message: result ? 'Recovery action completed successfully' : 'Recovery action failed'
      };
    } catch (error) {
      return {
        success: false,
        message: 'Recovery action execution failed',
        error: String(error)
      };
    }
  }
}

/**
 * Recovery workflow manager
 */
export class RecoveryWorkflowManager {
  private static instance: RecoveryWorkflowManager;
  private activeRecoveries: Map<string, {
    errorId: string;
    actions: RecoveryAction[];
    currentStep: number;
    progress: number;
    startTime: Date;
  }> = new Map();

  static getInstance(): RecoveryWorkflowManager {
    if (!RecoveryWorkflowManager.instance) {
      RecoveryWorkflowManager.instance = new RecoveryWorkflowManager();
    }
    return RecoveryWorkflowManager.instance;
  }

  /**
   * Start a recovery workflow
   */
  startRecovery(errorId: string, actions: RecoveryAction[]): string {
    const workflowId = `recovery_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    this.activeRecoveries.set(workflowId, {
      errorId,
      actions,
      currentStep: 0,
      progress: 0,
      startTime: new Date()
    });

    return workflowId;
  }

  /**
   * Get recovery workflow status
   */
  getRecoveryStatus(workflowId: string): {
    exists: boolean;
    isComplete: boolean;
    currentAction?: RecoveryAction;
    progress: number;
    remainingActions: number;
  } {
    const recovery = this.activeRecoveries.get(workflowId);
    
    if (!recovery) {
      return { exists: false, isComplete: false, progress: 0, remainingActions: 0 };
    }

    const isComplete = recovery.currentStep >= recovery.actions.length;
    const currentAction = !isComplete ? recovery.actions[recovery.currentStep] : undefined;
    const remainingActions = Math.max(0, recovery.actions.length - recovery.currentStep);

    return {
      exists: true,
      isComplete,
      currentAction,
      progress: recovery.progress,
      remainingActions
    };
  }

  /**
   * Advance to next step in recovery workflow
   */
  nextStep(workflowId: string, success: boolean = true): boolean {
    const recovery = this.activeRecoveries.get(workflowId);
    if (!recovery) return false;

    if (success) {
      recovery.currentStep++;
      recovery.progress = (recovery.currentStep / recovery.actions.length) * 100;
    }

    // Clean up completed recoveries
    if (recovery.currentStep >= recovery.actions.length) {
      this.activeRecoveries.delete(workflowId);
    }

    return true;
  }

  /**
   * Cancel recovery workflow
   */
  cancelRecovery(workflowId: string): boolean {
    return this.activeRecoveries.delete(workflowId);
  }
}

/**
 * Simplified API for recovery actions
 */
export const getRecoveryActions = (error: any, context?: ErrorProcessingContext): RecoveryAction[] => {
  return RecoveryActionEngine.getInstance().getSuggestedActions(error, context);
};

export const executeRecoveryAction = async (
  actionId: string, 
  actions: RecoveryAction[]
): Promise<{ success: boolean; message: string; error?: string }> => {
  return RecoveryActionEngine.getInstance().executeAction(actionId, actions);
};

export const startRecoveryWorkflow = (errorId: string, actions: RecoveryAction[]): string => {
  return RecoveryWorkflowManager.getInstance().startRecovery(errorId, actions);
};

export const getRecoveryWorkflowStatus = (workflowId: string) => {
  return RecoveryWorkflowManager.getInstance().getRecoveryStatus(workflowId);
};

/**
 * Processed error information with context
 */
export interface ProcessedErrorInfo {
  error: any;
  severity: ErrorSeverity;
  domain: ErrorDomain;
  isRecoverable: boolean;
  userMessage: string;
  debugMessage: string;
  context: ErrorProcessingContext;
  timestamp: string;
  correlationId: string;
}

/**
 * Context-aware error processing - simplified API
 */
export const processErrorWithContext = (error: any): ProcessedErrorInfo => {
  return ContextAwareErrorProcessor.getInstance().processError(error);
};

/**
 * Set error processing context
 */
export const setErrorContext = (context: Partial<ErrorProcessingContext>): void => {
  ContextAwareErrorProcessor.getInstance().setContext(context);
};

/**
 * Clear error processing context
 */
export const clearErrorContext = (sections?: Array<keyof ErrorProcessingContext>): void => {
  ContextAwareErrorProcessor.getInstance().clearContext(sections);
};

/**
 * Generate context-aware error message based on classification
 */
export const generateContextAwareMessage = (
  error: any, 
  context: MessageGenerationContext = {}
): string => {
  return ContextAwareErrorProcessor.getInstance()['generateContextAwareMessage'](error, context);
};

/**
 * Enhanced error handling for Tauri commands (optional enhancement)
 * Wraps existing invoke calls with enhanced error capabilities
 */
export const enhancedInvoke = async <T>(
  command: string, 
  args?: Record<string, any>
): Promise<T> => {
  const { invoke } = await import('@tauri-apps/api/core');
  
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    // Wrap string errors with enhanced error wrapper for optional features
    if (typeof error === 'string') {
      const wrapper = EnhancedErrorWrapper.fromString(error);
      throw wrapper;
    }
    throw error;
  }
};

/**
 * Error boundary helper for React components (backward compatible)
 * Works with both traditional and enhanced errors
 */
export interface ErrorInfo {
  error: any;
  severity: ErrorSeverity;
  domain: ErrorDomain;
  isRecoverable: boolean;
  userMessage: string;
  action: string;
}

export const analyzeError = (error: any): ErrorInfo => {
  return {
    error,
    severity: getErrorSeverity(error),
    domain: getErrorDomain(error),
    isRecoverable: isRecoverableError(error),
    userMessage: formatErrorForDisplay(error),
    action: getErrorAction(error)
  };
};

/**
 * Retry logic with enhanced error analysis (optional enhancement)
 */
export interface RetryOptions {
  maxRetries?: number;
  delay?: number;
  backoff?: boolean;
  retryIf?: (error: any) => boolean;
}

export const withRetry = async <T>(
  operation: () => Promise<T>,
  options: RetryOptions = {}
): Promise<T> => {
  const { maxRetries = 3, delay = 1000, backoff = true, retryIf } = options;
  
  let lastError: any;
  
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      
      // Check if we should retry based on error analysis
      const shouldRetry = retryIf ? retryIf(error) : isRecoverableError(error);
      
      if (attempt === maxRetries || !shouldRetry) {
        throw error;
      }
      
      // Calculate delay with optional backoff
      const currentDelay = backoff ? delay * Math.pow(2, attempt) : delay;
      await new Promise(resolve => setTimeout(resolve, currentDelay));
    }
  }
  
  throw lastError;
};

/**
 * Error notification helper (backward compatible with enhancements)
 * Integrates with existing notification system but uses enhanced error analysis
 */
export interface NotificationOptions {
  type?: 'error' | 'warning' | 'info';
  duration?: number;
  showAction?: boolean;
}

export const showErrorNotification = (
  error: any, 
  options: NotificationOptions = {}
): void => {
  const errorInfo = analyzeError(error);
  const { type = 'error', duration = 4500, showAction = true } = options;
  
  // This would integrate with your existing notification system
  // For now, we'll use console as a placeholder
  console.error('Error Notification:', {
    message: errorInfo.userMessage,
    severity: errorInfo.severity,
    domain: errorInfo.domain,
    action: showAction ? errorInfo.action : undefined,
    type,
    duration
  });
  
  // In a real implementation, this would call your notification system:
  // notificationService.show({
  //   message: errorInfo.userMessage,
  //   type: errorInfo.severity === 'Critical' ? 'error' : type,
  //   duration,
  //   actions: showAction ? [{ label: 'Dismiss', action: () => {} }] : undefined
  // });
};

/**
 * Development helper - check if enhanced error handling is available
 */
export const isEnhancedErrorHandlingAvailable = (): boolean => {
  return true; // Always available in this implementation
};

/**
 * Development helper - get debug information about error handling
 */
export const getErrorHandlingDebugInfo = (error: any) => {
  return {
    hasEnhancedInfo: ErrorHandlingUtils.hasEnhancedInfo(error),
    errorType: typeof error,
    severity: getErrorSeverity(error),
    domain: getErrorDomain(error),
    isRecoverable: isRecoverableError(error),
    rawError: error,
    debugMessage: formatErrorForLogging(error)
  };
};

// Export backward compatible utilities
export {
  ErrorHandlingUtils,
  EnhancedErrorWrapper
} from '../types/error-handling';
export type {
  ErrorSeverity,
  ErrorDomain
} from '../types/error-handling';

// Default export for convenience
export default {
  getErrorMessage,
  isCriticalError,
  isRecoverableError,
  getErrorSeverity,
  getErrorDomain,
  formatErrorForDisplay,
  formatErrorForLogging,
  getErrorAction,
  analyzeError,
  enhancedInvoke,
  withRetry,
  showErrorNotification,
  isEnhancedErrorHandlingAvailable,
  getErrorHandlingDebugInfo
};