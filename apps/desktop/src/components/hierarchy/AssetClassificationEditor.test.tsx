import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { AssetClassificationEditor } from './AssetClassificationEditor';
import { AssetHierarchy } from '../../types/assets';
import { SecurityClassificationLevel } from '../../types/security';
import useAuthStore from '../../store/auth';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const base = { description: '', sort_order: 0, created_by: 1, created_at: '2024-01-01T00:00:00Z', updated_at: '2024-01-01T00:00:00Z' };
const plc: AssetHierarchy = { ...base, id: 3, name: 'PLC-1', asset_type: 'Device', parent_id: 2, children: [], security_classification: SecurityClassificationLevel.INTERNAL };
const hmi: AssetHierarchy = { ...base, id: 4, name: 'HMI-1', asset_type: 'Device', parent_id: 2, children: [], security_classification: SecurityClassificationLevel.SECRET };
const cell: AssetHierarchy = { ...base, id: 5, name: 'Cell A', asset_type: 'Folder', parent_id: 2, children: [], security_classification: SecurityClassificationLevel.CONFIDENTIAL };
const line: AssetHierarchy = {
  ...base, id: 2, name: 'Line 1', asset_type: 'Folder', parent_id: 1, children: [plc, hmi, cell],
  security_classification: SecurityClassificationLevel.INTERNAL,
};
const plant: AssetHierarchy = {
  ...base, id: 1, name: 'Plant', asset_type: 'Folder', parent_id: null, children: [line],
  security_classification: SecurityClassificationLevel.INTERNAL,
};
const hierarchy = [plant];

const optionFor = (label: string) =>
  Array.from(document.querySelectorAll('.ant-select-item-option')).find(option => option.textContent?.includes(label));

const chooseLevel = async (user: ReturnType<typeof userEvent.setup>, label: string) => {
  await user.click(screen.getByRole('combobox'));
  await waitFor(() => expect(optionFor(label)).toBeDefined());
  await user.click(optionFor(label) as HTMLElement);
};

describe('AssetClassificationEditor', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({
      token: 'test-token',
      user: { id: 1, username: 'admin', role: 'Administrator', created_at: '2024-01-01', is_active: true },
    });
    vi.mocked(invoke).mockResolvedValue({ id: line.id });
  });

  it('shows the current classification as a tag', () => {
    render(<AssetClassificationEditor asset={hmi} hierarchyData={hierarchy} />);
    expect(screen.getByText('Secret')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Change/ })).toBeInTheDocument();
  });

  it('states how many contained items will be raised before saving', async () => {
    const user = userEvent.setup();
    const onUpdated = vi.fn();
    render(<AssetClassificationEditor asset={line} hierarchyData={hierarchy} onUpdated={onUpdated} />);

    fireEvent.click(screen.getByRole('button', { name: /Change/ }));
    expect(await screen.findByText('Security Classification - Line 1')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();

    await chooseLevel(user, 'Restricted');
    // PLC-1 (Internal) and Cell A (Confidential) are below Restricted; HMI-1 (Secret) is not
    expect(await screen.findByText('2 items inside "Line 1" will be raised to Restricted')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Raise 2 items and Save' }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('update_asset_classification', {
        token: 'test-token',
        assetId: line.id,
        securityClassification: SecurityClassificationLevel.RESTRICTED,
      });
    });
    await waitFor(() => expect(onUpdated).toHaveBeenCalledWith({ id: line.id }));
  });

  it('lowers a folder without raising anything and disables levels below the parent', async () => {
    const user = userEvent.setup();
    const parentIsConfidential = [{ ...plant, security_classification: SecurityClassificationLevel.CONFIDENTIAL }];
    const secretCell = { ...cell, security_classification: SecurityClassificationLevel.SECRET, parent_id: 1 };
    render(<AssetClassificationEditor asset={secretCell} hierarchyData={parentIsConfidential} />);

    fireEvent.click(screen.getByRole('button', { name: /Change/ }));
    await user.click(await screen.findByRole('combobox'));
    await waitFor(() => expect(optionFor('Internal')).toBeDefined());
    expect(optionFor('Internal')).toHaveClass('ant-select-item-option-disabled');
    await user.click(optionFor('Confidential') as HTMLElement);

    expect(await screen.findByText('Items inside this folder keep their current classification.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('update_asset_classification', expect.objectContaining({
        securityClassification: SecurityClassificationLevel.CONFIDENTIAL,
      }));
    });
  });

  it('shows the backend rejection', async () => {
    const user = userEvent.setup();
    vi.mocked(invoke).mockRejectedValue("Cannot set 'PLC-1' to PUBLIC: its folder 'Line 1' is classified INTERNAL.");
    render(<AssetClassificationEditor asset={plc} hierarchyData={hierarchy} />);

    fireEvent.click(screen.getByRole('button', { name: /Change/ }));
    await chooseLevel(user, 'Secret');
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    expect(await screen.findByText(/its folder 'Line 1' is classified INTERNAL/)).toBeInTheDocument();
  });
});
