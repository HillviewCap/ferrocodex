export interface Branch {
  id: number;
  name: string;
  description: string | null;
  asset_id: number;
  parent_version_id: number;
  created_by: number;
  created_at: string;
  updated_at: string;
  is_active: boolean;
}

export interface BranchInfo {
  id: number;
  name: string;
  description: string | null;
  asset_id: number;
  parent_version_id: number;
  parent_version_number: string;
  parent_version_status: string;
  created_by: number;
  created_by_username: string;
  created_at: string;
  updated_at: string;
  is_active: boolean;
}

export interface CreateBranchRequest {
  name: string;
  description?: string;
  asset_id: number;
  parent_version_id: number;
}

export interface BranchVersion {
  id: number;
  branch_id: number;
  version_id: number;
  branch_version_number: string;
  is_latest: boolean;
  created_at: string;
}

export interface BranchVersionInfo {
  id: number;
  branch_id: number;
  branch_name: string;
  version_id: number;
  branch_version_number: string;
  is_latest: boolean;
  is_branch_latest: boolean;
  created_at: string;
  // Configuration version details
  asset_id: number;
  version_number: string;
  file_name: string;
  file_size: number;
  content_hash: string;
  author: number;
  author_username: string;
  notes: string;
  version_created_at: string;
}

export interface CreateBranchVersionRequest {
  branch_id: number;
  file_path: string;
  notes: string;
}

// Validation schemas
export const BranchValidation = {
  name: {
    minLength: 2,
    maxLength: 100,
    required: true,
    pattern: /^[a-zA-Z0-9\s\-_\.]+$/,
    message: 'Branch name must be 2-100 characters and contain only letters, numbers, spaces, hyphens, underscores, and periods'
  },
  description: {
    maxLength: 500,
    required: false,
    message: 'Description cannot exceed 500 characters'
  }
};

// Helper functions
export const validateBranchName = (name: string): string | null => {
  if (!name.trim()) {
    return 'Branch name is required';
  }
  if (name.length < BranchValidation.name.minLength) {
    return `Branch name must be at least ${BranchValidation.name.minLength} characters`;
  }
  if (name.length > BranchValidation.name.maxLength) {
    return `Branch name cannot exceed ${BranchValidation.name.maxLength} characters`;
  }
  if (!BranchValidation.name.pattern.test(name)) {
    return BranchValidation.name.message;
  }
  return null;
};

export const validateBranchDescription = (description: string): string | null => {
  if (description.length > BranchValidation.description.maxLength) {
    return BranchValidation.description.message;
  }
  return null;
};

export const formatBranchName = (name: string): string => {
  return name.trim().replace(/\s+/g, '-').toLowerCase();
};

export const getBranchStatusColor = (isActive: boolean): string => {
  return isActive ? 'green' : 'red';
};

export const getBranchStatusText = (isActive: boolean): string => {
  return isActive ? 'Active' : 'Inactive';
};

export const sortBranchesByCreated = (branches: BranchInfo[]): BranchInfo[] => {
  return [...branches].sort((a, b) => {
    return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
  });
};