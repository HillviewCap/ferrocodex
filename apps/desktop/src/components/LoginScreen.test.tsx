import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { expect, test, vi, beforeEach } from 'vitest';
import LoginScreen from './LoginScreen';

// Mock the auth store
const mockLogin = vi.fn();
const mockClearError = vi.fn();

vi.mock('../store/auth', () => ({
  default: () => ({
    login: mockLogin,
    isLoading: false,
    error: null,
    clearError: mockClearError,
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

test('renders login form', () => {
  render(<LoginScreen />);
  
  expect(screen.getByAltText('Ferrocodex Logo')).toBeInTheDocument();
  expect(screen.getByText('Sign in to your account')).toBeInTheDocument();
  expect(screen.getByLabelText('Username')).toBeInTheDocument();
  expect(screen.getByLabelText('Password')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Sign In' })).toBeInTheDocument();
});

test('validates required fields', async () => {
  render(<LoginScreen />);
  
  const submitButton = screen.getByRole('button', { name: 'Sign In' });
  
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(screen.getByText('Please enter your username')).toBeInTheDocument();
    expect(screen.getByText('Please enter your password')).toBeInTheDocument();
  });
});

test('calls login on form submission', async () => {
  render(<LoginScreen />);
  
  const usernameInput = screen.getByLabelText('Username');
  const passwordInput = screen.getByLabelText('Password');
  const submitButton = screen.getByRole('button', { name: 'Sign In' });
  
  fireEvent.change(usernameInput, { target: { value: 'admin' } });
  fireEvent.change(passwordInput, { target: { value: 'password123' } });
  fireEvent.click(submitButton);
  
  await waitFor(() => {
    expect(mockLogin).toHaveBeenCalledWith('admin', 'password123');
  });
});

test('shows remember me checkbox', () => {
  render(<LoginScreen />);
  
  expect(screen.getByLabelText('Remember me')).toBeInTheDocument();
});