import type { UserInfo } from '../store/auth';

export const isAdministrator = (user: UserInfo | null): boolean => {
  return user?.role === 'Administrator' && user?.is_active === true;
};

export const isEngineer = (user: UserInfo | null): boolean => {
  return user?.role === 'Engineer' && user?.is_active === true;
};

export const canAccessUserManagement = (user: UserInfo | null): boolean => {
  return isAdministrator(user);
};

export const canCreateUsers = (user: UserInfo | null): boolean => {
  return isAdministrator(user);
};

export const canManageUsers = (user: UserInfo | null): boolean => {
  return isAdministrator(user);
};

export const hasRole = (user: UserInfo | null, role: 'Administrator' | 'Engineer'): boolean => {
  return user?.role === role && user?.is_active === true;
};

export const getUserDisplayName = (user: UserInfo | null): string => {
  if (!user) return 'Unknown User';
  return user.username;
};

export const getRoleDisplayName = (role: 'Administrator' | 'Engineer'): string => {
  return role;
};

export const getRoleColor = (role: 'Administrator' | 'Engineer'): string => {
  return role === 'Administrator' ? 'red' : 'blue';
};

export const getStatusColor = (isActive: boolean): string => {
  return isActive ? 'green' : 'gray';
};

export const getStatusText = (isActive: boolean): string => {
  return isActive ? 'Active' : 'Inactive';
};