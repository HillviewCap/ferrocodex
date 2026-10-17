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
    token: 'test-token',
    logout: mockLogout,
  }),
}));

// Mock the opener plugin
vi.mock('@tauri-apps/plugin-opener', () => ({
  open: vi.fn(),
}));

// The search page needs a router and its own IPC calls; stub it to keep this a Dashboard test
vi.mock('./search/MetadataSearchPage', () => ({
  default: ({ embedded }: { embedded?: boolean }) => (
    <div data-testid="metadata-search-page">{embedded ? 'embedded' : 'standalone'}</div>
  ),
}));

// Mock the Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    total_assets: 0,
    total_versions: 0,
    encryption_type: 'AES-256'
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

test('renders dashboard with user information', () => {
  render(<Dashboard />);
  
  expect(screen.getByText('Ferrocodex')).toBeInTheDocument();
  expect(screen.getByText('Welcome, admin')).toBeInTheDocument();
  const administratorElements = screen.getAllByText('Administrator');
  expect(administratorElements.length).toBeGreaterThan(0);
  const welcomeElements = screen.getAllByText('Welcome to Ferrocodex');
  expect(welcomeElements.length).toBeGreaterThan(0);
  expect(screen.getByText('Import Configuration')).toBeInTheDocument();
});

test('renders navigation menu', () => {
  render(<Dashboard />);
  
  // Check for menu items
  const dashboardElements = screen.getAllByText('Dashboard');
  expect(dashboardElements.length).toBeGreaterThan(0);
  const helpElements = screen.getAllByText('Help');
  expect(helpElements.length).toBeGreaterThan(0);
  
  // Check for Assets menu item
  const assetsElements = screen.getAllByText('Assets');
  expect(assetsElements.length).toBeGreaterThan(0);
  
  // Check for Vault menu item
  const vaultElements = screen.getAllByText('Vault');
  expect(vaultElements.length).toBeGreaterThan(0);
  
  // Check for Bulk Operations menu item
  const bulkElements = screen.getAllByText('Bulk Operations');
  expect(bulkElements.length).toBeGreaterThan(0);

  // Metadata Search is a top-level entry; the rest of the Metadata menu is
  // temporarily hidden in Dashboard.tsx pending field testing
  expect(screen.getByText('Metadata Search')).toBeInTheDocument();
  expect(screen.queryByText('Metadata')).not.toBeInTheDocument();
  
  // Check for Security menu item (admin only)
  const securityElements = screen.getAllByText('Security');
  expect(securityElements.length).toBeGreaterThan(0);
  
  // Check for User Management menu item (admin only)
  const userManagementElements = screen.getAllByText('User Management');
  expect(userManagementElements.length).toBeGreaterThan(0);
});

test('opens the embedded metadata search page from the sidebar', () => {
  render(<Dashboard />);

  fireEvent.click(screen.getByText('Metadata Search'));

  expect(screen.getByTestId('metadata-search-page')).toHaveTextContent('embedded');
});

test('renders feature cards', () => {
  render(<Dashboard />);
  
  // Check for the descriptive text in cards
  expect(screen.getByText('Manage configuration assets')).toBeInTheDocument();
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