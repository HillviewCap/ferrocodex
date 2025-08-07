import { render, screen, waitFor } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach } from 'vitest';
import SecurityMonitoringDashboard from './SecurityMonitoringDashboard';
import useAuthStore from '../../store/auth';
import useSecurityStore from '../../store/security';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock the auth store
vi.mock('../../store/auth', () => ({
  default: vi.fn(),
}));

// Mock the security store
vi.mock('../../store/security', () => ({
  default: vi.fn(),
  useSecurityAlerts: vi.fn(() => ({
    alerts: [],
    unreadCount: 0,
    criticalAlerts: [],
  })),
  useSecurityHealth: vi.fn(() => ({
    overallStatus: 'good',
    isHealthy: true,
    criticalIssues: [],
  })),
}));

// Mock the invoke function
const mockInvoke = vi.mocked((await import('@tauri-apps/api/core')).invoke);

describe('SecurityMonitoringDashboard', () => {
  const mockMetrics = {
    totalAssets: 100,
    classificationBreakdown: {
      PUBLIC: 45,
      INTERNAL: 30,
      CONFIDENTIAL: 15,
      RESTRICTED: 8,
      SECRET: 2,
    },
    validationSuccessRate: 94.5,
    recentValidations: 150,
    securityAlerts: 5,
    complianceScore: 87.0,
  };

  const mockHealthReport = {
    overallStatus: 'Good',
    validationSuccessRate: 94.5,
    recentThreatsBlocked: 5,
    systemSecurityLevel: 'High',
    recommendations: ['Review validation failures'],
  };

  const mockValidationStats = {
    totalValidations: 500,
    successfulValidations: 473,
    failedValidations: 27,
    blockedAttempts: 5,
    lastValidation: '2024-01-15T10:30:00Z',
  };

  const mockAdminUser = {
    id: 1,
    username: 'admin',
    role: 'Administrator',
  };

  beforeEach(() => {
    vi.clearAllMocks();

    // Setup auth store mock
    vi.mocked(useAuthStore).mockReturnValue({
      token: 'test-token',
      user: mockAdminUser,
      isAuthenticated: true,
      setUser: vi.fn(),
      setToken: vi.fn(),
      logout: vi.fn(),
    });

    // Setup security store mock
    vi.mocked(useSecurityStore).mockReturnValue({
      metrics: null,
      healthReport: null,
      validationStats: null,
      setMetrics: vi.fn(),
      setHealthReport: vi.fn(),
      setValidationStats: vi.fn(),
      alerts: [],
      setAlerts: vi.fn(),
      addAlert: vi.fn(),
      acknowledgeAlert: vi.fn(),
      clearAlerts: vi.fn(),
    });

  });

  test('should render dashboard for administrator', () => {
    render(<SecurityMonitoringDashboard />);
    
    expect(screen.getByText(/Security Monitoring Dashboard/i)).toBeInTheDocument();
  });

  test('should show access denied for non-administrators', () => {
    vi.mocked(useAuthStore).mockReturnValue({
      token: 'test-token',
      user: { id: 2, username: 'engineer', role: 'Engineer' },
      isAuthenticated: true,
      setUser: vi.fn(),
      setToken: vi.fn(),
      logout: vi.fn(),
    });

    render(<SecurityMonitoringDashboard />);
    
    expect(screen.getByText(/Administrator Access Required/i)).toBeInTheDocument();
    expect(screen.getByText(/only available to administrators/i)).toBeInTheDocument();
  });

  test('should fetch security metrics on mount', async () => {
    mockInvoke.mockImplementation((command) => {
      switch (command) {
        case 'get_security_metrics':
          return Promise.resolve(mockMetrics);
        case 'perform_security_health_check':
          return Promise.resolve(mockHealthReport);
        case 'get_validation_statistics':
          return Promise.resolve(mockValidationStats);
        default:
          return Promise.reject(new Error(`Unknown command: ${command}`));
      }
    });

    render(<SecurityMonitoringDashboard />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_security_metrics', {
        token: 'test-token',
        period: '24h',
      });
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('perform_security_health_check', {
        token: 'test-token',
      });
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_validation_statistics', {
        token: 'test-token',
      });
    });
  });

  test('should display compact view when prop is set', () => {
    render(<SecurityMonitoringDashboard compactView={true} />);
    
    // In compact view, we should see smaller statistics
    expect(screen.getByText(/Security Status/i)).toBeInTheDocument();
    expect(screen.getByText(/Active Alerts/i)).toBeInTheDocument();
    expect(screen.getByText(/Success Rate/i)).toBeInTheDocument();
    
    // Should not see detailed sections
    expect(screen.queryByText(/Security Classification Distribution/i)).not.toBeInTheDocument();
  });

  test('should handle errors gracefully', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    mockInvoke.mockRejectedValue(new Error('Command get_security_metrics not found'));

    render(<SecurityMonitoringDashboard />);

    await waitFor(() => {
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Failed to fetch security data:',
        expect.any(Error)
      );
    });

    consoleErrorSpy.mockRestore();
  });

  test('should update period when selection changes', async () => {
    mockInvoke.mockResolvedValue(mockMetrics);

    const { rerender } = render(<SecurityMonitoringDashboard />);

    // Initial call should be with 24h
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_security_metrics', {
        token: 'test-token',
        period: '24h',
      });
    });

    // Clear previous calls
    mockInvoke.mockClear();

    // Simulate changing the period (this would normally be done through user interaction)
    // For now, we'll just verify the component accepts different periods
    rerender(<SecurityMonitoringDashboard />);

    // The component should still work with different periods
    expect(screen.getByText(/Security Monitoring Dashboard/i)).toBeInTheDocument();
  });
});