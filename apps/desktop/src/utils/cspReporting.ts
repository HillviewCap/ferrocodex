/**
 * CSP Violation Reporting Utility
 * Handles Content Security Policy violations and logs them for security monitoring
 */

interface CSPViolationEvent {
  blockedURI: string;
  documentURI: string;
  effectiveDirective: string;
  originalPolicy: string;
  referrer: string;
  statusCode: number;
  violatedDirective: string;
  lineNumber: number;
  columnNumber: number;
  sourceFile: string;
}

/**
 * Initialize CSP violation reporting
 * Should be called early in the application lifecycle
 */
export function initializeCSPReporting(): void {
  // Listen for CSP violations
  document.addEventListener('securitypolicyviolation', (event: SecurityPolicyViolationEvent) => {
    const violation: CSPViolationEvent = {
      blockedURI: event.blockedURI,
      documentURI: event.documentURI,
      effectiveDirective: event.effectiveDirective,
      originalPolicy: event.originalPolicy,
      referrer: event.referrer,
      statusCode: event.statusCode,
      violatedDirective: event.violatedDirective,
      lineNumber: event.lineNumber || 0,
      columnNumber: event.columnNumber || 0,
      sourceFile: event.sourceFile || ''
    };

    handleCSPViolation(violation);
  });

  console.log('CSP violation reporting initialized');
}

/**
 * Handle a CSP violation by logging it and optionally reporting to backend
 */
function handleCSPViolation(violation: CSPViolationEvent): void {
  // Log the violation for debugging
  console.warn('CSP Violation detected:', violation);

  // In a production environment, you might want to send this to an audit log
  // For now, we'll just log it to the console
  const violationSummary = {
    type: 'CSP_VIOLATION',
    blockedURI: violation.blockedURI,
    violatedDirective: violation.violatedDirective,
    sourceFile: violation.sourceFile,
    lineNumber: violation.lineNumber,
    timestamp: new Date().toISOString()
  };

  // Log in a structured format for potential audit trail integration
  console.error('Security Policy Violation:', violationSummary);

  // TODO: In future, integrate with the audit system to log security violations
  // await invoke('log_security_violation', { violation: violationSummary });
}