import { render, screen } from '@testing-library/react';
import { expect, test, vi, beforeEach, describe } from 'vitest';
import RetryPreferences from '../RetryPreferences';

// Mock external dependencies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));

vi.mock('../store/auth', () => ({
  default: vi.fn(() => ({
    token: null, // Start with no token to test auth state
  })),
}));

describe('RetryPreferences', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('should render component', () => {
    render(<RetryPreferences />);
    
    // Component should render without crashing
    expect(document.body).toBeInTheDocument();
  });

  test('should show loading spinner initially', () => {
    render(<RetryPreferences />);
    
    // Look for Ant Design Spin component
    const spinElement = document.querySelector('.ant-spin');
    expect(spinElement).toBeInTheDocument();
  });
});