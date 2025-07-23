import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import FirmwareAnalysis from './FirmwareAnalysis';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { FirmwareAnalysisResult } from '../../types/firmware';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

// Mock useAuthStore
vi.mock('../../store/auth', () => ({
  default: () => ({
    token: 'test-token',
    isAuthenticated: true,
  }),
}));

describe('FirmwareAnalysis', () => {
  const mockAnalysis: FirmwareAnalysisResult = {
    id: 1,
    firmwareVersionId: 123,
    analysisStatus: 'completed',
    fileType: 'ELF',
    detectedVersions: ['1.2.3', '4.5.6'],
    entropyScore: 7.2,
    securityFindings: [
      {
        severity: 'high',
        findingType: 'Hardcoded Credentials',
        description: 'Found hardcoded admin password',
        offset: 0x1234,
      },
      {
        severity: 'medium',
        findingType: 'Telnet Service',
        description: 'Telnet service detected',
      },
    ],
    createdAt: '2024-01-01T12:00:00Z',
    startedAt: '2024-01-01T12:00:00Z',
    completedAt: '2024-01-01T12:05:00Z',
  };

  beforeEach(() => {
    vi.clearAllMocks();
    // Mock the event listener
    (listen as any).mockResolvedValue(() => {});
  });

  it('renders loading state initially', () => {
    (invoke as any).mockImplementation(() => new Promise(() => {})); // Never resolves
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    expect(screen.getByText('Loading analysis results...')).toBeInTheDocument();
  });

  it('renders no analysis state when analysis is null', async () => {
    (invoke as any).mockResolvedValue(null);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('No Analysis Available')).toBeInTheDocument();
      expect(screen.getByText('This firmware has not been analyzed yet.')).toBeInTheDocument();
      expect(screen.getByText('Start Analysis')).toBeInTheDocument();
    });
  });

  it('renders completed analysis with results', async () => {
    (invoke as any).mockResolvedValue(mockAnalysis);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('Firmware Analysis')).toBeInTheDocument();
      expect(screen.getByText('COMPLETED')).toBeInTheDocument();
      expect(screen.getByText('ELF')).toBeInTheDocument();
      expect(screen.getByText('7.20')).toBeInTheDocument(); // Entropy score
    });
  });

  it('renders security findings', async () => {
    (invoke as any).mockResolvedValue(mockAnalysis);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('Security Findings')).toBeInTheDocument();
    });

    // Click to expand security findings
    const securityPanel = screen.getByText('Security Findings');
    fireEvent.click(securityPanel);

    await waitFor(() => {
      expect(screen.getByText('Hardcoded Credentials')).toBeInTheDocument();
      expect(screen.getByText('Found hardcoded admin password')).toBeInTheDocument();
      expect(screen.getByText('Telnet Service')).toBeInTheDocument();
      expect(screen.getByText('HIGH')).toBeInTheDocument();
      expect(screen.getByText('MEDIUM')).toBeInTheDocument();
    });
  });

  it('renders detected versions', async () => {
    (invoke as any).mockResolvedValue(mockAnalysis);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('Detected Versions')).toBeInTheDocument();
    });

    // Click to expand versions
    const versionsPanel = screen.getByText('Detected Versions');
    fireEvent.click(versionsPanel);

    await waitFor(() => {
      expect(screen.getByText('1.2.3')).toBeInTheDocument();
      expect(screen.getByText('4.5.6')).toBeInTheDocument();
    });
  });

  it('renders failed analysis with error message', async () => {
    const failedAnalysis: FirmwareAnalysisResult = {
      ...mockAnalysis,
      analysisStatus: 'failed',
      errorMessage: 'Analysis timeout',
      securityFindings: undefined,
      detectedVersions: undefined,
    };
    
    (invoke as any).mockResolvedValue(failedAnalysis);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('FAILED')).toBeInTheDocument();
      expect(screen.getByText('Analysis Failed')).toBeInTheDocument();
      expect(screen.getByText('Analysis timeout')).toBeInTheDocument();
      expect(screen.getByText('Retry Analysis')).toBeInTheDocument();
    });
  });

  it('handles retry analysis', async () => {
    const failedAnalysis: FirmwareAnalysisResult = {
      ...mockAnalysis,
      analysisStatus: 'failed',
    };
    
    (invoke as any).mockResolvedValue(failedAnalysis);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('Retry Analysis')).toBeInTheDocument();
    });

    // Mock retry
    (invoke as any).mockImplementation((cmd: string) => {
      if (cmd === 'retry_firmware_analysis') {
        return Promise.resolve();
      }
      return Promise.resolve(failedAnalysis);
    });

    fireEvent.click(screen.getByText('Retry Analysis'));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('retry_firmware_analysis', {
        token: 'test-token',
        firmwareId: 123,
      });
    });
  });

  it('listens for analysis progress events', async () => {
    const mockUnlisten = vi.fn();
    (listen as any).mockResolvedValue(mockUnlisten);
    (invoke as any).mockResolvedValue(null);
    
    const { unmount } = render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(listen).toHaveBeenCalledWith('firmware-analysis-progress', expect.any(Function));
    });

    unmount();
    expect(mockUnlisten).toHaveBeenCalled();
  });

  it('renders analysis in progress', async () => {
    const inProgressAnalysis: FirmwareAnalysisResult = {
      ...mockAnalysis,
      analysisStatus: 'in_progress',
      securityFindings: undefined,
      detectedVersions: undefined,
      completedAt: undefined,
    };
    
    (invoke as any).mockResolvedValue(inProgressAnalysis);
    
    // Mock progress event
    let progressCallback: any;
    (listen as any).mockImplementation((event: string, cb: any) => {
      progressCallback = cb;
      return Promise.resolve(() => {});
    });
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      expect(screen.getByText('IN_PROGRESS')).toBeInTheDocument();
    });

    // Simulate progress event
    progressCallback({
      payload: {
        firmwareId: 123,
        status: 'in_progress',
        progress: 50,
        message: 'Analyzing firmware structure...',
      },
    });

    await waitFor(() => {
      expect(screen.getByText('Analysis in Progress')).toBeInTheDocument();
      expect(screen.getByText('Analyzing firmware structure...')).toBeInTheDocument();
    });
  });

  it('renders no security findings message when none found', async () => {
    const safeAnalysis: FirmwareAnalysisResult = {
      ...mockAnalysis,
      securityFindings: [],
    };
    
    (invoke as any).mockResolvedValue(safeAnalysis);
    
    render(<FirmwareAnalysis firmwareId={123} />);
    
    await waitFor(() => {
      const securityPanel = screen.getByText('Security Findings');
      fireEvent.click(securityPanel);
    });

    await waitFor(() => {
      expect(screen.getByText('No security issues found')).toBeInTheDocument();
      expect(screen.getByText('The firmware analysis did not detect any security concerns.')).toBeInTheDocument();
    });
  });
});