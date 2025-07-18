import { describe, it, expect } from 'vitest';
import { 
  isAdministrator, 
  isEngineer, 
  canAccessUserManagement, 
  canCreateUsers, 
  canManageUsers,
  hasRole,
  getUserDisplayName,
  getRoleDisplayName,
  getRoleColor,
  getStatusColor,
  getStatusText
} from './roleUtils';
import type { UserInfo } from '../store/auth';

describe('Role Utils', () => {
  const adminUser: UserInfo = {
    id: 1,
    username: 'admin',
    role: 'Administrator',
    created_at: '2023-01-01',
    is_active: true,
  };

  const engineerUser: UserInfo = {
    id: 2,
    username: 'engineer',
    role: 'Engineer',
    created_at: '2023-01-02',
    is_active: true,
  };

  const inactiveAdminUser: UserInfo = {
    id: 3,
    username: 'inactive_admin',
    role: 'Administrator',
    created_at: '2023-01-03',
    is_active: false,
  };

  const inactiveEngineerUser: UserInfo = {
    id: 4,
    username: 'inactive_engineer',
    role: 'Engineer',
    created_at: '2023-01-04',
    is_active: false,
  };

  describe('isAdministrator', () => {
    it('returns true for active administrator', () => {
      expect(isAdministrator(adminUser)).toBe(true);
    });

    it('returns false for engineer', () => {
      expect(isAdministrator(engineerUser)).toBe(false);
    });

    it('returns false for inactive administrator', () => {
      expect(isAdministrator(inactiveAdminUser)).toBe(false);
    });

    it('returns false for null user', () => {
      expect(isAdministrator(null)).toBe(false);
    });
  });

  describe('isEngineer', () => {
    it('returns true for active engineer', () => {
      expect(isEngineer(engineerUser)).toBe(true);
    });

    it('returns false for administrator', () => {
      expect(isEngineer(adminUser)).toBe(false);
    });

    it('returns false for inactive engineer', () => {
      expect(isEngineer(inactiveEngineerUser)).toBe(false);
    });

    it('returns false for null user', () => {
      expect(isEngineer(null)).toBe(false);
    });
  });

  describe('canAccessUserManagement', () => {
    it('returns true for active administrator', () => {
      expect(canAccessUserManagement(adminUser)).toBe(true);
    });

    it('returns false for engineer', () => {
      expect(canAccessUserManagement(engineerUser)).toBe(false);
    });

    it('returns false for inactive administrator', () => {
      expect(canAccessUserManagement(inactiveAdminUser)).toBe(false);
    });

    it('returns false for null user', () => {
      expect(canAccessUserManagement(null)).toBe(false);
    });
  });

  describe('canCreateUsers', () => {
    it('returns true for active administrator', () => {
      expect(canCreateUsers(adminUser)).toBe(true);
    });

    it('returns false for engineer', () => {
      expect(canCreateUsers(engineerUser)).toBe(false);
    });

    it('returns false for inactive administrator', () => {
      expect(canCreateUsers(inactiveAdminUser)).toBe(false);
    });

    it('returns false for null user', () => {
      expect(canCreateUsers(null)).toBe(false);
    });
  });

  describe('canManageUsers', () => {
    it('returns true for active administrator', () => {
      expect(canManageUsers(adminUser)).toBe(true);
    });

    it('returns false for engineer', () => {
      expect(canManageUsers(engineerUser)).toBe(false);
    });

    it('returns false for inactive administrator', () => {
      expect(canManageUsers(inactiveAdminUser)).toBe(false);
    });

    it('returns false for null user', () => {
      expect(canManageUsers(null)).toBe(false);
    });
  });

  describe('hasRole', () => {
    it('returns true for active administrator with Administrator role', () => {
      expect(hasRole(adminUser, 'Administrator')).toBe(true);
    });

    it('returns true for active engineer with Engineer role', () => {
      expect(hasRole(engineerUser, 'Engineer')).toBe(true);
    });

    it('returns false for administrator checked against Engineer role', () => {
      expect(hasRole(adminUser, 'Engineer')).toBe(false);
    });

    it('returns false for engineer checked against Administrator role', () => {
      expect(hasRole(engineerUser, 'Administrator')).toBe(false);
    });

    it('returns false for inactive user with matching role', () => {
      expect(hasRole(inactiveAdminUser, 'Administrator')).toBe(false);
    });

    it('returns false for null user', () => {
      expect(hasRole(null, 'Administrator')).toBe(false);
    });
  });

  describe('getUserDisplayName', () => {
    it('returns username for valid user', () => {
      expect(getUserDisplayName(adminUser)).toBe('admin');
    });

    it('returns "Unknown User" for null user', () => {
      expect(getUserDisplayName(null)).toBe('Unknown User');
    });
  });

  describe('getRoleDisplayName', () => {
    it('returns role name for Administrator', () => {
      expect(getRoleDisplayName('Administrator')).toBe('Administrator');
    });

    it('returns role name for Engineer', () => {
      expect(getRoleDisplayName('Engineer')).toBe('Engineer');
    });
  });

  describe('getRoleColor', () => {
    it('returns red for Administrator', () => {
      expect(getRoleColor('Administrator')).toBe('red');
    });

    it('returns blue for Engineer', () => {
      expect(getRoleColor('Engineer')).toBe('blue');
    });
  });

  describe('getStatusColor', () => {
    it('returns green for active status', () => {
      expect(getStatusColor(true)).toBe('green');
    });

    it('returns gray for inactive status', () => {
      expect(getStatusColor(false)).toBe('gray');
    });
  });

  describe('getStatusText', () => {
    it('returns "Active" for active status', () => {
      expect(getStatusText(true)).toBe('Active');
    });

    it('returns "Inactive" for inactive status', () => {
      expect(getStatusText(false)).toBe('Inactive');
    });
  });
});