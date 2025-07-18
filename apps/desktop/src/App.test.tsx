import { render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from './App';

vi.mock('@tauri-apps/api/core');

// Mock the stores
vi.mock('./store/auth', () => ({
  default: () => ({
    isAuthenticated: false,
    checkSession: vi.fn(),
    user: null,
    token: null,
    isLoading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    createAdminAccount: vi.fn(),
    clearError: vi.fn(),
  }),
}));

vi.mock('./store/app', () => ({
  default: () => ({
    isFirstLaunch: true,
    isDatabaseInitialized: false,
    isLoading: false,
    error: null,
    checkFirstLaunch: vi.fn(),
    initializeDatabase: vi.fn(),
    setLoading: vi.fn(),
    clearError: vi.fn(),
  }),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

test('renders loading state initially', () => {
  render(<App />);
  expect(screen.getByText('Initializing Ferrocodex...')).toBeInTheDocument();
});

test('shows admin setup for first launch', async () => {
  const mockInvoke = vi.mocked(invoke);
  mockInvoke.mockResolvedValue(true);

  render(<App />);
  
  // Wait for the admin setup to appear
  await waitFor(() => {
    expect(screen.getByText('Welcome to Ferrocodex')).toBeInTheDocument();
    expect(screen.getByText('Create your administrator account to get started')).toBeInTheDocument();
  });
});