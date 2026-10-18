import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import UserManagement from './UserManagement';
import useAuthStore from '../store/auth';
import useUserManagementStore from '../store/userManagement';
import useKeyRotationStore from '../store/keyRotation';

// Mock the stores
vi.mock('../store/auth');
vi.mock('../store/userManagement');

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Ant Design form validation is async and can exceed waitFor's 1s default in jsdom
const VALIDATION_TIMEOUT = { timeout: 5000 };

describe('UserManagement', () => {
  const mockAuthStore = {
    token: 'test-token',
    user: { id: 1, username: 'admin', role: 'Administrator' as const, created_at: '2023-01-01', is_active: true }
  };

  const mockUserManagementStore = {
    users: [
      { id: 1, username: 'admin', role: 'Administrator' as const, created_at: '2023-01-01', is_active: true },
      { id: 2, username: 'engineer1', role: 'Engineer' as const, created_at: '2023-01-02', is_active: true },
      { id: 3, username: 'engineer2', role: 'Engineer' as const, created_at: '2023-01-03', is_active: false },
    ],
    isLoading: false,
    error: null,
    isCreatingUser: false,
    createUserError: null,
    fetchUsers: vi.fn().mockResolvedValue(undefined),
    createEngineerUser: vi.fn().mockResolvedValue(undefined),
    deactivateUser: vi.fn().mockResolvedValue(undefined),
    reactivateUser: vi.fn().mockResolvedValue(undefined),
    resetUserPassword: vi.fn().mockResolvedValue(undefined),
    clearError: vi.fn(),
    clearCreateUserError: vi.fn(),
    setLoading: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (useAuthStore as any).mockReturnValue(mockAuthStore);
    (useUserManagementStore as any).mockReturnValue(mockUserManagementStore);
  });

  it('renders user management interface', () => {
    render(<UserManagement />);
    
    expect(screen.getByText('User Management')).toBeInTheDocument();
    expect(screen.getByText('Create Engineer Account')).toBeInTheDocument();
    expect(screen.getByText('Refresh')).toBeInTheDocument();
  });

  it('displays user statistics correctly', () => {
    render(<UserManagement />);
    
    expect(screen.getByText('3')).toBeInTheDocument(); // Total users
    const twoElements = screen.getAllByText('2');
    expect(twoElements.length).toBe(2); // Active users and Engineers
    const oneElements = screen.getAllByText('1');
    expect(oneElements.length).toBeGreaterThan(0); // Administrators
  });

  it('displays user list with correct information', () => {
    render(<UserManagement />);
    
    expect(screen.getByText('admin')).toBeInTheDocument();
    expect(screen.getByText('engineer1')).toBeInTheDocument();
    expect(screen.getByText('engineer2')).toBeInTheDocument();
    
    // Check role tags
    expect(screen.getByText('Administrator')).toBeInTheDocument();
    expect(screen.getAllByText('Engineer')).toHaveLength(2);
    
    // Check status tags
    expect(screen.getAllByText('Active')).toHaveLength(2);
    expect(screen.getByText('Inactive')).toBeInTheDocument();
  });

  it('opens create user modal when button is clicked', async () => {
    render(<UserManagement />);
    
    const createButton = screen.getAllByText('Create Engineer Account')[0];
    fireEvent.click(createButton);
    
    await waitFor(() => {
      const modalTitles = screen.getAllByText('Create Engineer Account');
      expect(modalTitles.length).toBeGreaterThan(1); // Button + modal title
      expect(screen.getByPlaceholderText('Enter username')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Enter initial password')).toBeInTheDocument();
    });
  });

  it('validates form inputs in create user modal', async () => {
    render(<UserManagement />);
    
    const createButton = screen.getByText('Create Engineer Account');
    fireEvent.click(createButton);
    
    fireEvent.click(await screen.findByText('Create Account'));
    
    await waitFor(() => {
      expect(screen.getByText('Please input username')).toBeInTheDocument();
      expect(screen.getByText('Please input password')).toBeInTheDocument();
    }, VALIDATION_TIMEOUT);
  });

  it('calls createEngineerUser when form is submitted with valid data', async () => {
    render(<UserManagement />);
    
    const createButton = screen.getByText('Create Engineer Account');
    fireEvent.click(createButton);
    
    await waitFor(() => {
      const usernameInput = screen.getByPlaceholderText('Enter username');
      const passwordInput = screen.getByPlaceholderText('Enter initial password');
      const confirmPasswordInput = screen.getByPlaceholderText('Confirm password');
      
      fireEvent.change(usernameInput, { target: { value: 'newengineer' } });
      fireEvent.change(passwordInput, { target: { value: 'password123' } });
      fireEvent.change(confirmPasswordInput, { target: { value: 'password123' } });
      
      const submitButton = screen.getByText('Create Account');
      fireEvent.click(submitButton);
    });
    
    await waitFor(() => {
      expect(mockUserManagementStore.createEngineerUser).toHaveBeenCalledWith(
        'test-token',
        'newengineer',
        'password123'
      );
    });
  });

  it('shows deactivate button for active engineer users', () => {
    render(<UserManagement />);
    
    const deactivateButtons = screen.getAllByText('Deactivate');
    expect(deactivateButtons.length).toBeGreaterThanOrEqual(1); // At least one for engineer users
  });

  it('shows reactivate button for inactive users', () => {
    render(<UserManagement />);
    
    const reactivateButtons = screen.getAllByText('Reactivate');
    expect(reactivateButtons).toHaveLength(1); // Only for engineer2
  });

  it('calls deactivateUser when deactivate is confirmed', async () => {
    render(<UserManagement />);
    
    // The admin row's Deactivate button is disabled, so target engineer1 (id 2)
    const engineerRow = screen.getByText('engineer1').closest('tr')!;
    fireEvent.click(within(engineerRow).getByText('Deactivate'));
    
    fireEvent.click(await screen.findByText('Yes'));
    
    await waitFor(() => {
      expect(mockUserManagementStore.deactivateUser).toHaveBeenCalledWith('test-token', 2);
    });
  });

  it('asks for a temporary password and calls reactivateUser', async () => {
    render(<UserManagement />);

    fireEvent.click(screen.getByText('Reactivate'));

    const dialog = await screen.findByRole('dialog');
    fireEvent.change(within(dialog).getByPlaceholderText('Enter temporary password'), { target: { value: 'Temp-Pass-456' } });
    fireEvent.change(within(dialog).getByPlaceholderText('Confirm temporary password'), { target: { value: 'Temp-Pass-456' } });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Reactivate' }));

    await waitFor(() => {
      expect(mockUserManagementStore.reactivateUser).toHaveBeenCalledWith('test-token', 3, 'Temp-Pass-456');
    }, VALIDATION_TIMEOUT);
  });

  it('does not reactivate when the passwords differ', async () => {
    render(<UserManagement />);

    fireEvent.click(screen.getByText('Reactivate'));

    const dialog = await screen.findByRole('dialog');
    fireEvent.change(within(dialog).getByPlaceholderText('Enter temporary password'), { target: { value: 'Temp-Pass-456' } });
    fireEvent.change(within(dialog).getByPlaceholderText('Confirm temporary password'), { target: { value: 'Different-789' } });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Reactivate' }));

    expect(await within(dialog).findByText('Passwords do not match', undefined, VALIDATION_TIMEOUT)).toBeInTheDocument();
    expect(mockUserManagementStore.reactivateUser).not.toHaveBeenCalled();
  });

  it('offers Reset password only for other active users and calls resetUserPassword', async () => {
    render(<UserManagement />);

    // Active users are admin (self, hidden) and engineer1; engineer2 is inactive
    expect(screen.getAllByText('Reset password')).toHaveLength(1);

    const engineerRow = screen.getByText('engineer1').closest('tr')!;
    fireEvent.click(within(engineerRow).getByText('Reset password'));

    const dialog = await screen.findByRole('dialog');
    fireEvent.change(within(dialog).getByPlaceholderText('Enter temporary password'), { target: { value: 'Temp-Pass-456' } });
    fireEvent.change(within(dialog).getByPlaceholderText('Confirm temporary password'), { target: { value: 'Temp-Pass-456' } });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Set Password' }));

    await waitFor(() => {
      expect(mockUserManagementStore.resetUserPassword).toHaveBeenCalledWith('test-token', 2, 'Temp-Pass-456');
    }, VALIDATION_TIMEOUT);
  });

  it('warns that deactivation revokes encrypted-data access', async () => {
    render(<UserManagement />);

    const engineerRow = screen.getByText('engineer1').closest('tr')!;
    fireEvent.click(within(engineerRow).getByText('Deactivate'));

    expect(await screen.findByText(/access to encrypted data is revoked/)).toBeInTheDocument();
  });

  it('calls fetchUsers when refresh button is clicked', async () => {
    render(<UserManagement />);
    
    const refreshButton = screen.getByText('Refresh');
    fireEvent.click(refreshButton);
    
    expect(mockUserManagementStore.fetchUsers).toHaveBeenCalledWith('test-token');
  });

  it('displays error message when there is an error', () => {
    const storeWithError = {
      ...mockUserManagementStore,
      error: 'Failed to load users',
    };
    (useUserManagementStore as any).mockReturnValue(storeWithError);
    
    render(<UserManagement />);
    
    expect(screen.getByText('Failed to load users')).toBeInTheDocument();
  });

  it('displays create user error in modal', async () => {
    const storeWithCreateError = {
      ...mockUserManagementStore,
      createUserError: 'Username already exists',
    };
    (useUserManagementStore as any).mockReturnValue(storeWithCreateError);
    
    render(<UserManagement />);
    
    const createButton = screen.getByText('Create Engineer Account');
    fireEvent.click(createButton);
    
    await waitFor(() => {
      expect(screen.getByText('Username already exists')).toBeInTheDocument();
    });
  });

  it('prevents password mismatch in create user form', async () => {
    render(<UserManagement />);
    
    const createButton = screen.getByText('Create Engineer Account');
    fireEvent.click(createButton);
    
    await waitFor(() => {
      const usernameInput = screen.getByPlaceholderText('Enter username');
      const passwordInput = screen.getByPlaceholderText('Enter initial password');
      const confirmPasswordInput = screen.getByPlaceholderText('Confirm password');
      
      fireEvent.change(usernameInput, { target: { value: 'newengineer' } });
      fireEvent.change(passwordInput, { target: { value: 'password123' } });
      fireEvent.change(confirmPasswordInput, { target: { value: 'different' } });
      
      const submitButton = screen.getByText('Create Account');
      fireEvent.click(submitButton);
    });
    
    await waitFor(() => {
      expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
    });
  });

  describe('encryption access', () => {
    const statusWith = (details: Record<string, unknown> | null) => ({
      in_progress: false,
      active_version: 1,
      retiring_version: null,
      details,
    });

    beforeEach(() => {
      useKeyRotationStore.getState().reset();
    });

    it('marks users without a key and hints at Reset password', () => {
      useKeyRotationStore.setState({
        status: statusWith({
          remaining: { configuration_versions: 0, vault_secrets: 0, standalone_credentials: 0, firmware_versions: 0, total: 0 },
          failures: [],
          users_without_keypair: [{ id: 2, username: 'engineer1', role: 'Engineer' }],
          users_without_access: [],
          active_analysis_jobs: 0,
        }),
      });
      render(<UserManagement />);

      expect(screen.getByText('Encryption access')).toBeInTheDocument();
      expect(screen.getAllByText('No access - reset password')).toHaveLength(1);
      // The active admin has access; the inactive engineer shows neither tag
      expect(screen.getAllByText('Has access')).toHaveLength(1);
    });

    it('also marks users listed as without access after a rotation', () => {
      useKeyRotationStore.setState({
        status: statusWith({
          remaining: { configuration_versions: 0, vault_secrets: 0, standalone_credentials: 0, firmware_versions: 0, total: 0 },
          failures: [],
          users_without_keypair: [],
          users_without_access: [{ id: 1, username: 'admin', role: 'Administrator' }],
          active_analysis_jobs: 0,
        }),
      });
      render(<UserManagement />);
      expect(screen.getAllByText('No access - reset password')).toHaveLength(1);
    });

    it('opens the rotation modal from the Rotate Encryption Key button', async () => {
      render(<UserManagement />);
      fireEvent.click(screen.getByRole('button', { name: /Rotate Encryption Key/ }));
      expect(await screen.findByText('Begin rotation', undefined, VALIDATION_TIMEOUT)).toBeInTheDocument();
    });
  });
});
