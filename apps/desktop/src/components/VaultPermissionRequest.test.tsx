import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import VaultPermissionRequest from './VaultPermissionRequest';
import { PermissionRequest } from '../types/vault';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const engineer = { id: 2, username: 'eng', role: 'Engineer' as const, created_at: '2024-01-01', is_active: true };

const requests: PermissionRequest[] = [
  { request_id: 1, user_id: 2, vault_id: 5, requested_permission: 'Read', requested_by: 2, status: 'Pending', created_at: '2024-03-01T10:00:00Z', updated_at: '2024-03-01T10:00:00Z' },
  { request_id: 2, user_id: 2, vault_id: 5, requested_permission: 'Write', requested_by: 2, status: 'Approved', approved_by: 1, created_at: '2024-02-01T10:00:00Z', updated_at: '2024-02-02T10:00:00Z', approval_notes: 'Needed for maintenance' },
  { request_id: 3, user_id: 3, vault_id: 5, requested_permission: 'Read', requested_by: 3, status: 'Pending', created_at: '2024-02-01T10:00:00Z', updated_at: '2024-02-01T10:00:00Z' },
  { request_id: 4, user_id: 2, vault_id: 8, requested_permission: 'Read', requested_by: 2, status: 'Denied', created_at: '2024-02-01T10:00:00Z', updated_at: '2024-02-03T10:00:00Z' },
];

describe('VaultPermissionRequest', () => {
  useConsoleGuard({
    // List migration was explicitly deferred by d052492; Listy is not a drop-in.
    ignore: [/\[antd: List\]/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token', user: engineer });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_user_vault_permissions') return requests;
      if (cmd === 'create_permission_request') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it("renders the user's own requests for the vault, split into pending and history", async () => {
    render(<VaultPermissionRequest vaultId={5} vaultName="PLC Vault" />);

    expect(screen.getByText('Permission Requests')).toBeInTheDocument();
    expect(screen.getByText('PLC Vault')).toBeInTheDocument();
    expect(await screen.findByText(/Pending Requests/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_user_vault_permissions', { token: 'test-token', userId: 2 });

    expect(screen.getByText('Awaiting administrator approval')).toBeInTheDocument();
    expect(screen.getByText(/Request History/)).toBeInTheDocument();
    expect(screen.getByText('Approved')).toBeInTheDocument();
    expect(screen.getByText(/Approved on .* by User #1/)).toBeInTheDocument();
    expect(screen.getByText('Note: Needed for maintenance')).toBeInTheDocument();
    expect(screen.getAllByText('Vault #5')).toHaveLength(2);
    // Other users' requests and other vaults are filtered out
    expect(screen.queryByText('Denied')).not.toBeInTheDocument();
    expect(screen.queryByText('Vault #8')).not.toBeInTheDocument();
  });

  it('opens the request modal and submits a request for the vault', async () => {
    const onRequestSubmitted = vi.fn();
    render(<VaultPermissionRequest vaultId={5} vaultName="PLC Vault" onRequestSubmitted={onRequestSubmitted} />);
    await screen.findByText(/Pending Requests/);

    fireEvent.click(screen.getByRole('button', { name: /Request Access/ }));
    expect(await screen.findByText('Request Vault Access')).toBeInTheDocument();
    expect(screen.getByText('Permission Type')).toBeInTheDocument();
    expect(screen.getByText('Reason for Request (Optional)')).toBeInTheDocument();
    expect(screen.getByText('Access Request Process')).toBeInTheDocument();
    expect(screen.queryByText('Select Vault')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Submit Request/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('create_permission_request', {
        token: 'test-token',
        request: { vault_id: 5, requested_permission: 'Read', requested_by: 2 },
      });
    });
    expect(await screen.findByText('Request Submitted')).toBeInTheDocument();
    expect(onRequestSubmitted).toHaveBeenCalled();
  });

  it('shows the empty state with a first-request call to action', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    render(<VaultPermissionRequest vaultId={5} />);

    expect(await screen.findByText('No permission requests')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Request Your First Access/ })).toBeInTheDocument();
  });
});
