import React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import FirmwareVersionList from '../FirmwareVersionList';
import { FirmwareVersionInfo } from '../../types/firmware';

// Mock the stores
vi.mock('../../store/auth', () => ({
  default: () => ({
    user: { role: 'Engineer', username: 'testuser' }
  })
}));

vi.mock('../../store/firmware', () => ({
  default: () => ({
    deleteFirmware: vi.fn(),
    updateFirmwareStatus: vi.fn(),
    getAvailableStatusTransitions: vi.fn().mockResolvedValue([]),
    promoteFirmwareToGolden: vi.fn(),
    updateFirmwareNotes: vi.fn()
  })
}));

// Mock child components
vi.mock('../firmware/FirmwareAnalysis', () => ({
  default: () => <div data-testid="firmware-analysis">Firmware Analysis</div>
}));

vi.mock('../LinkedConfigurationsList', () => ({
  default: () => <div data-testid="linked-configs">Linked Configurations</div>
}));

vi.mock('../firmware/FirmwareHistoryTimeline', () => ({
  default: () => <div data-testid="firmware-timeline">Timeline</div>
}));

vi.mock('../firmware/FirmwareStatusDialog', () => ({
  default: () => <div data-testid="status-dialog">Status Dialog</div>
}));

describe('FirmwareVersionList - Metadata Display', () => {
  const createMockFirmware = (overrides: Partial<FirmwareVersionInfo> = {}): FirmwareVersionInfo => ({
    id: 1,
    asset_id: 1,
    author_id: 1,
    author_username: 'testuser',
    vendor: 'TestVendor',
    model: 'TestModel',
    version: '1.0.0',
    notes: 'Test notes',
    status: 'Draft',
    file_path: '/test/path.bin',
    file_hash: 'abcdef1234567890',
    file_size: 1024,
    created_at: '2025-01-25T10:00:00Z',
    ...overrides
  });

  it('should handle firmware with null/undefined metadata gracefully', () => {
    const firmwareWithNullData = createMockFirmware({
      file_size: null as any,
      file_hash: null as any,
      created_at: null as any,
      author_username: null as any
    });

    render(<FirmwareVersionList versions={[firmwareWithNullData]} />);

    // Should show fallback values instead of "NaN undefined", "N/A", etc.
    expect(screen.getByText('0 Bytes')).toBeInTheDocument();
    expect(screen.getByText('N/A')).toBeInTheDocument();
    expect(screen.getByText('Unknown date')).toBeInTheDocument();
    expect(screen.getByText('Unknown user')).toBeInTheDocument();
  });

  it('should handle firmware with invalid metadata gracefully', () => {
    const firmwareWithInvalidData = createMockFirmware({
      file_size: -1,
      file_hash: '',
      created_at: 'invalid-date',
      author_username: '   '
    });

    render(<FirmwareVersionList versions={[firmwareWithInvalidData]} />);

    // Should show fallback values
    expect(screen.getByText('0 Bytes')).toBeInTheDocument();
    expect(screen.getByText('N/A')).toBeInTheDocument();
    expect(screen.getByText('Invalid date')).toBeInTheDocument();
    expect(screen.getByText('Unknown user')).toBeInTheDocument();
  });

  it('should handle firmware with "unknown" placeholder values from migration', () => {
    const firmwareWithPlaceholders = createMockFirmware({
      file_size: 0,
      file_hash: 'unknown',
      created_at: '2025-01-25T10:00:00Z',
      author_username: 'testuser'
    });

    render(<FirmwareVersionList versions={[firmwareWithPlaceholders]} />);

    // Should handle migration placeholder values
    expect(screen.getByText('0 Bytes')).toBeInTheDocument();
    expect(screen.getByText('N/A')).toBeInTheDocument(); // 'unknown' hash should show as N/A
    expect(screen.getByText(/ago/)).toBeInTheDocument(); // Valid date should show relative time
    expect(screen.getByText('testuser')).toBeInTheDocument();
  });

  it('should display valid metadata correctly', () => {
    const validFirmware = createMockFirmware({
      file_size: 2048,
      file_hash: 'abcdef1234567890abcdef1234567890',
      created_at: '2025-01-25T10:00:00Z',
      author_username: 'john.doe'
    });

    render(<FirmwareVersionList versions={[validFirmware]} />);

    // Should display formatted values correctly
    expect(screen.getByText('2 KB')).toBeInTheDocument();
    expect(screen.getByText('abcdef12...')).toBeInTheDocument(); // Truncated hash
    expect(screen.getByText(/ago/)).toBeInTheDocument(); // Relative time
    expect(screen.getByText('john.doe')).toBeInTheDocument();
  });

  it('should handle edge case file sizes', () => {
    const largeFirmware = createMockFirmware({
      file_size: 1073741824 // 1GB
    });

    render(<FirmwareVersionList versions={[largeFirmware]} />);
    expect(screen.getByText('1 GB')).toBeInTheDocument();
  });
});