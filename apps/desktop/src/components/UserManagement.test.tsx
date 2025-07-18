import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import UserManagement from './UserManagement';
import useAuthStore from '../store/auth';
import useUserManagementStore from '../store/userManagement';

// Mock the stores
vi.mock('../store/auth');
vi.mock('../store/userManagement');

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

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
    expect(screen.getByText('2')).toBeInTheDocument(); // Active users
    expect(screen.getByText('2')).toBeInTheDocument(); // Engineers
    expect(screen.getByText('1')).toBeInTheDocument(); // Administrators
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
    
    const createButton = screen.getByText('Create Engineer Account');
    fireEvent.click(createButton);
    
    await waitFor(() => {
      expect(screen.getByText('Create Engineer Account')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Enter username')).toBeInTheDocument();
      expect(screen.getByPlaceholderText('Enter initial password')).toBeInTheDocument();
    });
  });

  it('validates form inputs in create user modal', async () => {
    render(<UserManagement />);
    
    const createButton = screen.getByText('Create Engineer Account');
    fireEvent.click(createButton);
    
    await waitFor(() => {
      const submitButton = screen.getByText('Create Account');
      fireEvent.click(submitButton);
    });
    
    await waitFor(() => {
      expect(screen.getByText('Please input username')).toBeInTheDocument();
      expect(screen.getByText('Please input password')).toBeInTheDocument();
    });
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
    expect(deactivateButtons).toHaveLength(1); // Only for engineer1, not admin
  });

  it('shows reactivate button for inactive users', () => {
    render(<UserManagement />);
    
    const reactivateButtons = screen.getAllByText('Reactivate');
    expect(reactivateButtons).toHaveLength(1); // Only for engineer2
  });

  it('calls deactivateUser when deactivate is confirmed', async () => {
    render(<UserManagement />);
    
    const deactivateButton = screen.getByText('Deactivate');
    fireEvent.click(deactivateButton);
    
    await waitFor(() => {
      const confirmButton = screen.getByText('Yes');
      fireEvent.click(confirmButton);
    });
    
    await waitFor(() => {
      expect(mockUserManagementStore.deactivateUser).toHaveBeenCalledWith('test-token', 2);
    });
  });

  it('calls reactivateUser when reactivate is clicked', async () => {
    render(<UserManagement />);
    
    const reactivateButton = screen.getByText('Reactivate');
    fireEvent.click(reactivateButton);
    
    await waitFor(() => {
      expect(mockUserManagementStore.reactivateUser).toHaveBeenCalledWith('test-token', 3);
    });
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
});