import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { expect, test, vi, beforeEach } from 'vitest';
import AdminSetup from './AdminSetup';

// Mock the auth store
const mockCreateAdminAccount = vi.fn();
const mockClearError = vi.fn();

vi.mock('../store/auth', () => ({
  default: () => ({
    createAdminAccount: mockCreateAdminAccount,
    isLoading: false,
    error: null,
    clearError: mockClearError,
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

test('renders admin setup form', () => {
  render(<AdminSetup />);
  
  expect(screen.getByText('Welcome to Ferrocodex')).toBeInTheDocument();
  expect(screen.getByText('Create your administrator account to get started')).toBeInTheDocument();
  expect(screen.getByLabelText('Username')).toBeInTheDocument();
  expect(screen.getByLabelText('Password')).toBeInTheDocument();
  expect(screen.getByLabelText('Confirm Password')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Create Admin Account' })).toBeInTheDocument();
});

test('validates username requirements', async () => {
  render(<AdminSetup />);
  
  const usernameInput = screen.getByLabelText('Username');
  const submitButton = screen.getByRole('button', { name: 'Create Admin Account' });
  
  // Test empty username
  fireEvent.change(usernameInput, { target: { value: '' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(screen.getByText('Please enter a username')).toBeInTheDocument();
  });
  
  // Test short username
  fireEvent.change(usernameInput, { target: { value: 'ab' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(screen.getByText('Username must be at least 3 characters')).toBeInTheDocument();
  });
});

test('validates password requirements', async () => {
  render(<AdminSetup />);
  
  const passwordInput = screen.getByLabelText('Password');
  const submitButton = screen.getByRole('button', { name: 'Create Admin Account' });
  
  // Test empty password
  fireEvent.change(passwordInput, { target: { value: '' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(screen.getByText('Please enter a password')).toBeInTheDocument();
  });
  
  // Test short password
  fireEvent.change(passwordInput, { target: { value: '1234567' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(screen.getByText('Password must be at least 8 characters')).toBeInTheDocument();
  });
});

test('validates password confirmation', async () => {
  render(<AdminSetup />);
  
  const passwordInput = screen.getByLabelText('Password');
  const confirmPasswordInput = screen.getByLabelText('Confirm Password');
  const submitButton = screen.getByRole('button', { name: 'Create Admin Account' });
  
  fireEvent.change(passwordInput, { target: { value: 'password123' } });
  fireEvent.change(confirmPasswordInput, { target: { value: 'different_password' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
  });
});

test('shows password strength indicator', async () => {
  render(<AdminSetup />);
  
  const passwordInput = screen.getByLabelText('Password');
  
  // Test weak password
  fireEvent.change(passwordInput, { target: { value: 'password' } });
  
  await waitFor(() => {
    expect(screen.getByText(/Password Strength:/)).toBeInTheDocument();
  });
});

test('calls createAdminAccount on form submission', async () => {
  render(<AdminSetup />);
  
  const usernameInput = screen.getByLabelText('Username');
  const passwordInput = screen.getByLabelText('Password');
  const confirmPasswordInput = screen.getByLabelText('Confirm Password');
  const submitButton = screen.getByRole('button', { name: 'Create Admin Account' });
  
  fireEvent.change(usernameInput, { target: { value: 'admin' } });
  fireEvent.change(passwordInput, { target: { value: 'StrongPassword123!' } });
  fireEvent.change(confirmPasswordInput, { target: { value: 'StrongPassword123!' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(mockCreateAdminAccount).toHaveBeenCalledWith('admin', 'StrongPassword123!');
  });
});