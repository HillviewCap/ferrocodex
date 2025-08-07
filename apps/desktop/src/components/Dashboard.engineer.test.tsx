import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import Dashboard from './Dashboard';

// Mock the auth store with Engineer role
const mockLogout = vi.fn();

vi.mock('../store/auth', () => ({
  default: () => ({
    user: {
      id: 2,
      username: 'engineer',
      role: 'Engineer',
      created_at: '2023-01-01',
      is_active: true,
    },
    token: 'test-token',
    logout: mockLogout,
  }),
}));

// Mock the opener plugin
vi.mock('@tauri-apps/plugin-opener', () => ({
  open: vi.fn(),
}));

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    total_assets: 0,
    total_versions: 0,
    encryption_type: 'AES-256'
  }),
}));

test('engineer role does not see admin-only menus', () => {
  render(<Dashboard />);
  
  // Check that engineer has access to basic menus
  expect(screen.getAllByText('Dashboard').length).toBeGreaterThan(0);
  expect(screen.getAllByText('Assets').length).toBeGreaterThan(0);
  expect(screen.getAllByText('Vault').length).toBeGreaterThan(0);
  
  // Check that engineer does NOT have access to admin-only menus
  expect(screen.queryByText('Metadata')).not.toBeInTheDocument();
  expect(screen.queryByText('Security')).not.toBeInTheDocument();
  expect(screen.queryByText('User Management')).not.toBeInTheDocument();
});

test('engineer sees correct role in header', () => {
  render(<Dashboard />);
  
  expect(screen.getByText('Welcome, engineer')).toBeInTheDocument();
  const engineerElements = screen.getAllByText('Engineer');
  expect(engineerElements.length).toBeGreaterThan(0);
});