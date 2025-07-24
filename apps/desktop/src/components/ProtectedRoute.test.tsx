import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BrowserRouter, MemoryRouter, Route, Routes } from 'react-router-dom';
import ProtectedRoute from './ProtectedRoute';

// Mock the auth store
const mockAuthStore = {
  isAuthenticated: true
};

vi.mock('../store/auth', () => ({
  default: vi.fn(() => mockAuthStore)
}));

// Mock Navigate to track navigation calls
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    Navigate: ({ to, replace }: any) => {
      mockNavigate(to, replace);
      return null;
    }
  };
});

describe('ProtectedRoute', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAuthStore.isAuthenticated = true;
  });

  it('renders children when user is authenticated', () => {
    render(
      <BrowserRouter>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Protected Content')).toBeInTheDocument();
  });

  it('redirects to login when user is not authenticated', () => {
    mockAuthStore.isAuthenticated = false;

    render(
      <BrowserRouter>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(mockNavigate).toHaveBeenCalledWith('/login', true);
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument();
  });

  it('works with nested components', () => {
    const NestedComponent = () => <div>Nested Protected Content</div>;

    render(
      <BrowserRouter>
        <ProtectedRoute>
          <NestedComponent />
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Nested Protected Content')).toBeInTheDocument();
  });

  it('protects multiple children', () => {
    render(
      <BrowserRouter>
        <ProtectedRoute>
          <div>Child 1</div>
          <div>Child 2</div>
          <div>Child 3</div>
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Child 1')).toBeInTheDocument();
    expect(screen.getByText('Child 2')).toBeInTheDocument();
    expect(screen.getByText('Child 3')).toBeInTheDocument();
  });

  it('works within a route configuration', () => {
    const TestApp = () => (
      <MemoryRouter initialEntries={['/protected']}>
        <Routes>
          <Route path="/login" element={<div>Login Page</div>} />
          <Route
            path="/protected"
            element={
              <ProtectedRoute>
                <div>Protected Page</div>
              </ProtectedRoute>
            }
          />
        </Routes>
      </MemoryRouter>
    );

    render(<TestApp />);

    expect(screen.getByText('Protected Page')).toBeInTheDocument();
  });

  it('redirects to login within route configuration when not authenticated', () => {
    mockAuthStore.isAuthenticated = false;

    const TestApp = () => (
      <MemoryRouter initialEntries={['/protected']}>
        <Routes>
          <Route path="/login" element={<div>Login Page</div>} />
          <Route
            path="/protected"
            element={
              <ProtectedRoute>
                <div>Protected Page</div>
              </ProtectedRoute>
            }
          />
        </Routes>
      </MemoryRouter>
    );

    render(<TestApp />);

    expect(mockNavigate).toHaveBeenCalledWith('/login', true);
    expect(screen.queryByText('Protected Page')).not.toBeInTheDocument();
  });

  it('updates when authentication status changes', () => {
    const { rerender } = render(
      <BrowserRouter>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Protected Content')).toBeInTheDocument();

    // Change authentication status
    mockAuthStore.isAuthenticated = false;

    rerender(
      <BrowserRouter>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(mockNavigate).toHaveBeenCalledWith('/login', true);
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument();
  });

  it('preserves component props when rendering children', () => {
    const ChildComponent = ({ message }: { message: string }) => (
      <div>{message}</div>
    );

    render(
      <BrowserRouter>
        <ProtectedRoute>
          <ChildComponent message="Hello Protected World" />
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Hello Protected World')).toBeInTheDocument();
  });

  it('handles React fragments as children', () => {
    render(
      <BrowserRouter>
        <ProtectedRoute>
          <>
            <div>Fragment Child 1</div>
            <div>Fragment Child 2</div>
          </>
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Fragment Child 1')).toBeInTheDocument();
    expect(screen.getByText('Fragment Child 2')).toBeInTheDocument();
  });

  it('handles null children gracefully', () => {
    render(
      <BrowserRouter>
        <ProtectedRoute>
          {null}
        </ProtectedRoute>
      </BrowserRouter>
    );

    // Should not throw error
    expect(document.body).toBeInTheDocument();
  });

  it('handles conditional children', () => {
    const showContent = true;

    render(
      <BrowserRouter>
        <ProtectedRoute>
          {showContent && <div>Conditional Content</div>}
        </ProtectedRoute>
      </BrowserRouter>
    );

    expect(screen.getByText('Conditional Content')).toBeInTheDocument();
  });

  it('uses replace navigation to prevent back button issues', () => {
    mockAuthStore.isAuthenticated = false;

    render(
      <BrowserRouter>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </BrowserRouter>
    );

    // Verify Navigate is called with replace=true
    expect(mockNavigate).toHaveBeenCalledWith('/login', true);
  });
});