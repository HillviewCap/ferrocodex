import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import FirmwareVersionList from './FirmwareVersionList';
import { FirmwareVersionInfo } from '../types/firmware';
import * as roleUtils from '../utils/roleUtils';

// Mock dependencies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

vi.mock('../store/auth', () => ({
  default: vi.fn(() => ({
    user: null
  }))
}));

vi.mock('../store/firmware', () => ({
  default: vi.fn(() => ({
    deleteFirmware: vi.fn(),
    updateFirmwareStatus: vi.fn(),
    getAvailableStatusTransitions: vi.fn(),
    promoteFirmwareToGolden: vi.fn(),
    updateFirmwareNotes: vi.fn()
  }))
}));

// Mock role utils
vi.mock('../utils/roleUtils', () => ({
  canChangeFirmwareStatus: vi.fn(),
  canPromoteFirmwareToGolden: vi.fn(),
  canUpdateFirmwareNotes: vi.fn()
}));

// Mock child components
vi.mock('./firmware/FirmwareAnalysis', () => ({
  default: () => <div>Firmware Analysis</div>
}));

vi.mock('./firmware/FirmwareHistoryTimeline', () => ({
  default: () => <div>Firmware History Timeline</div>
}));

vi.mock('./firmware/FirmwareStatusDialog', () => ({
  default: () => <div>Firmware Status Dialog</div>
}));

vi.mock('./LinkedConfigurationsList', () => ({
  default: () => <div>Linked Configurations</div>
}));

const mockFirmware: FirmwareVersionInfo = {
  id: 1,
  asset_id: 1,
  author_id: 1,
  author_username: 'test_user',
  vendor: 'TestVendor',
  model: 'TestModel',
  version: '1.0.0',
  notes: 'Test notes',
  status: 'Draft',
  file_path: '/test/path',
  file_hash: 'abc123',
  file_size: 1024,
  created_at: new Date().toISOString()
};

describe('FirmwareVersionList - Permission-based UI', () => {
  const useAuthStore = require('../store/auth').default;
  const useFirmwareStore = require('../store/firmware').default;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Engineer permissions', () => {
    beforeEach(() => {
      useAuthStore.mockReturnValue({
        user: { id: 1, username: 'engineer', role: 'Engineer', is_active: true }
      });
    });

    it('shows Change Status option when user can change firmware status', async () => {
      vi.mocked(roleUtils.canChangeFirmwareStatus).mockReturnValue(true);
      vi.mocked(roleUtils.canUpdateFirmwareNotes).mockReturnValue(true);
      
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Click the dropdown menu
      const moreButton = screen.getByRole('button', { name: /more/i });
      await userEvent.click(moreButton);
      
      // Should show Change Status option
      expect(screen.getByText('Change Status')).toBeInTheDocument();
    });

    it('hides Change Status option when user cannot change firmware status', async () => {
      vi.mocked(roleUtils.canChangeFirmwareStatus).mockReturnValue(false);
      
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Click the dropdown menu
      const moreButton = screen.getByRole('button', { name: /more/i });
      await userEvent.click(moreButton);
      
      // Should not show Change Status option
      expect(screen.queryByText('Change Status')).not.toBeInTheDocument();
    });

    it('shows Edit button for notes when user can update firmware notes', () => {
      vi.mocked(roleUtils.canUpdateFirmwareNotes).mockReturnValue(true);
      
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Should show Edit button next to notes
      expect(screen.getByRole('button', { name: /edit/i })).toBeInTheDocument();
    });

    it('hides Edit button for notes when user cannot update firmware notes', () => {
      vi.mocked(roleUtils.canUpdateFirmwareNotes).mockReturnValue(false);
      
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Should not show Edit button
      expect(screen.queryByRole('button', { name: /edit/i })).not.toBeInTheDocument();
    });

    it('shows Delete option for Engineers', async () => {
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Click the dropdown menu
      const moreButton = screen.getByRole('button', { name: /more/i });
      await userEvent.click(moreButton);
      
      // Should show Delete option
      expect(screen.getByText('Delete')).toBeInTheDocument();
    });
  });

  describe('Administrator permissions', () => {
    beforeEach(() => {
      useAuthStore.mockReturnValue({
        user: { id: 1, username: 'admin', role: 'Administrator', is_active: true }
      });
    });

    it('can promote firmware to Golden status', async () => {
      vi.mocked(roleUtils.canChangeFirmwareStatus).mockReturnValue(true);
      vi.mocked(roleUtils.canPromoteFirmwareToGolden).mockReturnValue(true);
      
      const getAvailableStatusTransitions = vi.fn().mockResolvedValue(['Approved', 'Golden', 'Archived']);
      useFirmwareStore.mockReturnValue({
        deleteFirmware: vi.fn(),
        updateFirmwareStatus: vi.fn(),
        getAvailableStatusTransitions,
        promoteFirmwareToGolden: vi.fn(),
        updateFirmwareNotes: vi.fn()
      });
      
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Click the dropdown menu
      const moreButton = screen.getByRole('button', { name: /more/i });
      await userEvent.click(moreButton);
      
      // Click Change Status
      await userEvent.click(screen.getByText('Change Status'));
      
      // Should call getAvailableStatusTransitions
      await waitFor(() => {
        expect(getAvailableStatusTransitions).toHaveBeenCalledWith(mockFirmware.id);
      });
    });
  });

  describe('Read-only user', () => {
    beforeEach(() => {
      useAuthStore.mockReturnValue({
        user: { id: 1, username: 'readonly', role: 'Engineer', is_active: true }
      });
      vi.mocked(roleUtils.canChangeFirmwareStatus).mockReturnValue(false);
      vi.mocked(roleUtils.canUpdateFirmwareNotes).mockReturnValue(false);
    });

    it('only shows View Analysis and View History options', async () => {
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Click the dropdown menu
      const moreButton = screen.getByRole('button', { name: /more/i });
      await userEvent.click(moreButton);
      
      // Should only show these options
      expect(screen.getByText('View Analysis')).toBeInTheDocument();
      expect(screen.getByText('View History')).toBeInTheDocument();
      
      // Should not show these options
      expect(screen.queryByText('Change Status')).not.toBeInTheDocument();
      expect(screen.queryByText('Delete')).not.toBeInTheDocument();
    });
  });

  describe('Status display', () => {
    it('shows crown icon for Golden status', () => {
      const goldenFirmware = { ...mockFirmware, status: 'Golden' as const };
      render(<FirmwareVersionList versions={[goldenFirmware]} />);
      
      const goldenTag = screen.getByText('Golden').closest('.ant-tag');
      expect(goldenTag).toHaveStyle({ backgroundColor: expect.stringContaining('gold') });
    });

    it('shows correct color for Approved status', () => {
      const approvedFirmware = { ...mockFirmware, status: 'Approved' as const };
      render(<FirmwareVersionList versions={[approvedFirmware]} />);
      
      const approvedTag = screen.getByText('Approved').closest('.ant-tag');
      expect(approvedTag).toHaveStyle({ backgroundColor: expect.stringContaining('green') });
    });
  });

  describe('Notes editing', () => {
    beforeEach(() => {
      useAuthStore.mockReturnValue({
        user: { id: 1, username: 'engineer', role: 'Engineer', is_active: true }
      });
      vi.mocked(roleUtils.canUpdateFirmwareNotes).mockReturnValue(true);
    });

    it('allows editing notes when permitted', async () => {
      const updateFirmwareNotes = vi.fn().mockResolvedValue(undefined);
      useFirmwareStore.mockReturnValue({
        deleteFirmware: vi.fn(),
        updateFirmwareStatus: vi.fn(),
        getAvailableStatusTransitions: vi.fn(),
        promoteFirmwareToGolden: vi.fn(),
        updateFirmwareNotes
      });
      
      render(<FirmwareVersionList versions={[mockFirmware]} />);
      
      // Click Edit button
      const editButton = screen.getByRole('button', { name: /edit/i });
      await userEvent.click(editButton);
      
      // Should show textarea
      const textarea = screen.getByRole('textbox');
      expect(textarea).toHaveValue('Test notes');
      
      // Edit notes
      await userEvent.clear(textarea);
      await userEvent.type(textarea, 'Updated notes');
      
      // Save
      const saveButton = screen.getByRole('button', { name: /save/i });
      await userEvent.click(saveButton);
      
      await waitFor(() => {
        expect(updateFirmwareNotes).toHaveBeenCalledWith(mockFirmware.id, 'Updated notes');
      });
    });
  });
});