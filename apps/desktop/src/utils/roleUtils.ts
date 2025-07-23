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

export const canChangeConfigurationStatus = (user: UserInfo | null): boolean => {
  return (isAdministrator(user) || isEngineer(user));
};

export const canChangeStatusFromApprovedToDraft = (user: UserInfo | null): boolean => {
  return isAdministrator(user);
};

export const canApproveConfiguration = (user: UserInfo | null): boolean => {
  return (isAdministrator(user) || isEngineer(user));
};

export const canSetGoldenStatus = (user: UserInfo | null): boolean => {
  return isAdministrator(user);
};

export const canArchiveConfiguration = (user: UserInfo | null): boolean => {
  return isAdministrator(user);
};

export const canPromoteToGolden = (user: UserInfo | null): boolean => {
  return (isAdministrator(user) || isEngineer(user));
};

export const canExportConfiguration = (user: UserInfo | null): boolean => {
  // All authenticated users (Engineers and Administrators) can export configurations
  return (isAdministrator(user) || isEngineer(user));
};

export const canArchiveVersion = (user: UserInfo | null): boolean => {
  // Both Engineers and Administrators can archive versions
  return (isAdministrator(user) || isEngineer(user));
};

export const canRestoreVersion = (user: UserInfo | null): boolean => {
  // Both Engineers and Administrators can restore archived versions
  return (isAdministrator(user) || isEngineer(user));
};

export const canLinkFirmware = (user: UserInfo | null): boolean => {
  // Both Engineers and Administrators can link firmware to configurations
  return (isAdministrator(user) || isEngineer(user));
};