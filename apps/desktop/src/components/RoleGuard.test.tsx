import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import RoleGuard from './RoleGuard';
import useAuthStore from '../store/auth';

// Mock the auth store
vi.mock('../store/auth');

describe('RoleGuard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders children for user with correct role', () => {
    (useAuthStore as any).mockReturnValue({
      user: { id: 1, username: 'admin', role: 'Administrator', is_active: true },
    });

    render(
      <RoleGuard allowedRoles={['Administrator']}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Protected Content')).toBeInTheDocument();
  });

  it('renders access denied for user with wrong role', () => {
    (useAuthStore as any).mockReturnValue({
      user: { id: 2, username: 'engineer', role: 'Engineer', is_active: true },
    });

    render(
      <RoleGuard allowedRoles={['Administrator']}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Access Denied')).toBeInTheDocument();
    expect(screen.getByText(/Only Administrator/)).toBeInTheDocument();
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument();
  });

  it('renders access denied for inactive user', () => {
    (useAuthStore as any).mockReturnValue({
      user: { id: 1, username: 'admin', role: 'Administrator', is_active: false },
    });

    render(
      <RoleGuard allowedRoles={['Administrator']}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Access Denied')).toBeInTheDocument();
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument();
  });

  it('renders access denied for null user', () => {
    (useAuthStore as any).mockReturnValue({
      user: null,
    });

    render(
      <RoleGuard allowedRoles={['Administrator']}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Access Denied')).toBeInTheDocument();
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument();
  });

  it('allows multiple roles', () => {
    (useAuthStore as any).mockReturnValue({
      user: { id: 2, username: 'engineer', role: 'Engineer', is_active: true },
    });

    render(
      <RoleGuard allowedRoles={['Administrator', 'Engineer']}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Protected Content')).toBeInTheDocument();
  });

  it('renders custom fallback when provided', () => {
    (useAuthStore as any).mockReturnValue({
      user: { id: 2, username: 'engineer', role: 'Engineer', is_active: true },
    });

    render(
      <RoleGuard 
        allowedRoles={['Administrator']}
        fallback={<div>Custom Fallback</div>}
      >
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Custom Fallback')).toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
    expect(screen.queryByText('Protected Content')).not.toBeInTheDocument();
  });

  it('allows inactive users when requireActive is false', () => {
    (useAuthStore as any).mockReturnValue({
      user: { id: 1, username: 'admin', role: 'Administrator', is_active: false },
    });

    render(
      <RoleGuard allowedRoles={['Administrator']} requireActive={false}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText('Protected Content')).toBeInTheDocument();
  });

  it('displays correct message for multiple allowed roles', () => {
    (useAuthStore as any).mockReturnValue({
      user: null,
    });

    render(
      <RoleGuard allowedRoles={['Administrator', 'Engineer']}>
        <div>Protected Content</div>
      </RoleGuard>
    );

    expect(screen.getByText(/Only Administrator and Engineer users/)).toBeInTheDocument();
  });
});