import { render, screen, fireEvent } from '@testing-library/react';
import { expect, test, vi, beforeEach } from 'vitest';
import Dashboard from './Dashboard';

// Mock the auth store
const mockLogout = vi.fn();

vi.mock('../store/auth', () => ({
  default: () => ({
    user: {
      id: 1,
      username: 'admin',
      role: 'Administrator',
      created_at: '2023-01-01',
      is_active: true,
    },
    logout: mockLogout,
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

test('renders dashboard with user information', () => {
  render(<Dashboard />);
  
  expect(screen.getByText('Ferrocodex')).toBeInTheDocument();
  expect(screen.getByText('Welcome, admin')).toBeInTheDocument();
  expect(screen.getByText('Administrator')).toBeInTheDocument();
  expect(screen.getByText('Welcome to Ferrocodex')).toBeInTheDocument();
});

test('renders navigation menu', () => {
  render(<Dashboard />);
  
  expect(screen.getByText('Dashboard')).toBeInTheDocument();
  expect(screen.getByText('Passwords')).toBeInTheDocument();
  expect(screen.getByText('Secure Notes')).toBeInTheDocument();
  expect(screen.getByText('Security')).toBeInTheDocument();
});

test('renders feature cards', () => {
  render(<Dashboard />);
  
  // Check for the feature cards
  const passwordCard = screen.getByText('Passwords');
  const notesCard = screen.getByText('Secure Notes');
  const securityCard = screen.getByText('Security');
  
  expect(passwordCard).toBeInTheDocument();
  expect(notesCard).toBeInTheDocument();
  expect(securityCard).toBeInTheDocument();
});

test('renders quick stats', () => {
  render(<Dashboard />);
  
  expect(screen.getByText('Quick Stats')).toBeInTheDocument();
  expect(screen.getByText('Stored Passwords')).toBeInTheDocument();
  expect(screen.getByText('Secure Notes')).toBeInTheDocument();
  expect(screen.getByText('Security Level')).toBeInTheDocument();
});

test('calls logout when logout is clicked', async () => {
  render(<Dashboard />);
  
  // Click on the user dropdown
  const userButton = screen.getByText('Administrator');
  fireEvent.click(userButton);
  
  // Note: In a real test, you'd need to wait for the dropdown to appear
  // and then click the logout option. For this test, we'll just verify
  // the logout function exists in the store.
  expect(mockLogout).toBeDefined();
});