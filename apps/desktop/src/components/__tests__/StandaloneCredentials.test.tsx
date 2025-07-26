import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import StandaloneCredentials from '../StandaloneCredentials';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';

// Mock dependencies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../store/auth', () => ({
  default: vi.fn(),
}));

// Mock child components
vi.mock('../PasswordGenerator', () => ({
  default: ({ onGenerate }: { onGenerate: (password: string) => void }) => (
    <button onClick={() => onGenerate('generated-password')}>Generate Password</button>
  ),
}));

vi.mock('../PasswordInput', () => ({
  default: ({ 
    value, 
    onChange, 
    placeholder 
  }: { 
    value: string; 
    onChange: (value: string) => void; 
    placeholder?: string;
  }) => (
    <input
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      data-testid="password-input"
    />
  ),
}));

describe('StandaloneCredentials', () => {
  const mockInvoke = vi.mocked(invoke);
  const mockAuthStore = { token: 'test-token' };
  
  const mockCategories = [
    {
      category: {
        id: 1,
        name: 'Jump Hosts',
        description: 'SSH jump servers',
        parent_category_id: null,
        color_code: '#1890ff',
        icon: 'server',
        created_at: '2024-01-01',
      },
      children: [],
    },
    {
      category: {
        id: 2,
        name: 'Databases',
        description: 'Database servers',
        parent_category_id: null,
        color_code: '#52c41a',
        icon: 'database',
        created_at: '2024-01-01',
      },
      children: [
        {
          category: {
            id: 5,
            name: 'Production',
            description: 'Production databases',
            parent_category_id: 2,
            color_code: '#52c41a',
            icon: 'database',
            created_at: '2024-01-01',
          },
          children: [],
        },
      ],
    },
  ];

  const mockCredentials = [
    {
      credential: {
        id: 1,
        name: 'Test DB Password',
        description: 'Test database password',
        credential_type: 'Password',
        category_id: 2,
        created_by: 1,
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
        last_accessed: null,
        encrypted_data: 'encrypted',
      },
      category_name: 'Databases',
      created_by_username: 'testuser',
      tags: ['test', 'database'],
    },
    {
      credential: {
        id: 2,
        name: 'Jump Host SSH',
        description: 'SSH key for jump host',
        credential_type: 'Password',
        category_id: 1,
        created_by: 1,
        created_at: '2024-01-02T00:00:00',
        updated_at: '2024-01-02T00:00:00',
        last_accessed: null,
        encrypted_data: 'encrypted',
      },
      category_name: 'Jump Hosts',
      created_by_username: 'testuser',
      tags: ['ssh', 'production'],
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useAuthStore).mockReturnValue(mockAuthStore);
    
    // Default mock responses
    mockInvoke.mockImplementation((command: string) => {
      switch (command) {
        case 'get_credential_categories':
          return Promise.resolve(mockCategories);
        case 'search_credentials':
          return Promise.resolve({
            credentials: mockCredentials,
            total_count: mockCredentials.length,
          });
        case 'create_standalone_credential':
          return Promise.resolve({
            id: 3,
            name: 'New Credential',
            description: 'New credential description',
            credential_type: 'Password',
            category_id: 1,
            created_by: 1,
            created_at: '2024-01-03T00:00:00',
            updated_at: '2024-01-03T00:00:00',
            last_accessed: null,
            encrypted_data: 'encrypted',
          });
        case 'get_all_tags':
          return Promise.resolve(['test', 'database', 'ssh', 'production']);
        default:
          return Promise.resolve({});
      }
    });
  });

  it('renders standalone credentials interface', async () => {
    render(<StandaloneCredentials />);
    
    expect(screen.getByText('Standalone Credentials')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Search credentials...')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add credential/i })).toBeInTheDocument();
    
    // Wait for categories and credentials to load
    await waitFor(() => {
      expect(screen.getByText('Jump Hosts')).toBeInTheDocument();
      expect(screen.getByText('Databases')).toBeInTheDocument();
    });
  });

  it('displays categories in tree structure', async () => {
    render(<StandaloneCredentials />);
    
    await waitFor(() => {
      expect(screen.getByText('Jump Hosts')).toBeInTheDocument();
      expect(screen.getByText('Databases')).toBeInTheDocument();
    });
    
    // Expand Databases to see subcategory
    const databasesNode = screen.getByText('Databases');
    fireEvent.click(databasesNode);
    
    await waitFor(() => {
      expect(screen.getByText('Production')).toBeInTheDocument();
    });
  });

  it('displays credentials list', async () => {
    render(<StandaloneCredentials />);
    
    await waitFor(() => {
      expect(screen.getByText('Test DB Password')).toBeInTheDocument();
      expect(screen.getByText('Jump Host SSH')).toBeInTheDocument();
    });
    
    // Check tags are displayed
    expect(screen.getByText('test')).toBeInTheDocument();
    expect(screen.getByText('database')).toBeInTheDocument();
    expect(screen.getByText('ssh')).toBeInTheDocument();
    expect(screen.getByText('production')).toBeInTheDocument();
  });

  it('opens create credential modal', async () => {
    render(<StandaloneCredentials />);
    
    const newCredentialBtn = screen.getByRole('button', { name: /add credential/i });
    fireEvent.click(newCredentialBtn);
    
    await waitFor(() => {
      expect(screen.getByText('Create Standalone Credential')).toBeInTheDocument();
      expect(screen.getByLabelText('Name')).toBeInTheDocument();
      expect(screen.getByLabelText('Description')).toBeInTheDocument();
      expect(screen.getByLabelText('Type')).toBeInTheDocument();
      expect(screen.getByLabelText('Category')).toBeInTheDocument();
    });
  });

  it('creates a new credential', async () => {
    render(<StandaloneCredentials />);
    
    // Open modal
    const newCredentialBtn = screen.getByRole('button', { name: /add credential/i });
    fireEvent.click(newCredentialBtn);
    
    await waitFor(() => {
      expect(screen.getByText('Create Standalone Credential')).toBeInTheDocument();
    });
    
    // Fill form
    fireEvent.change(screen.getByLabelText('Name'), { 
      target: { value: 'New Test Credential' } 
    });
    fireEvent.change(screen.getByLabelText('Description'), { 
      target: { value: 'New test description' } 
    });
    
    // Use password input
    const passwordInput = screen.getByTestId('password-input');
    fireEvent.change(passwordInput, { target: { value: 'test-password' } });
    
    // Submit form
    const createBtn = screen.getByRole('button', { name: 'Create' });
    fireEvent.click(createBtn);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_standalone_credential', {
        token: 'test-token',
        request: expect.objectContaining({
          name: 'New Test Credential',
          description: 'New test description',
          credential_type: 'Password',
          value: 'test-password',
        }),
      });
    });
  });

  it('filters credentials by search query', async () => {
    render(<StandaloneCredentials />);
    
    await waitFor(() => {
      expect(screen.getByText('Test DB Password')).toBeInTheDocument();
    });
    
    // Search for credentials
    const searchInput = screen.getByPlaceholderText('Search credentials...');
    fireEvent.change(searchInput, { target: { value: 'database' } });
    
    // Add a small delay for debounce
    await new Promise(resolve => setTimeout(resolve, 600));
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('search_credentials', {
        token: 'test-token',
        request: expect.objectContaining({
          query: 'database',
        }),
      });
    });
  });

  it('filters credentials by category', async () => {
    render(<StandaloneCredentials />);
    
    await waitFor(() => {
      expect(screen.getByText('Jump Hosts')).toBeInTheDocument();
    });
    
    // Click on a category
    const jumpHostsCategory = screen.getByText('Jump Hosts');
    fireEvent.click(jumpHostsCategory);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('search_credentials', {
        token: 'test-token',
        request: expect.objectContaining({
          category_id: 1,
        }),
      });
    });
  });

  it('displays credential details when clicked', async () => {
    render(<StandaloneCredentials />);
    
    await waitFor(() => {
      expect(screen.getByText('Test DB Password')).toBeInTheDocument();
    });
    
    // Mock get_standalone_credential response
    mockInvoke.mockImplementation((command: string, args: any) => {
      if (command === 'get_standalone_credential' && args.credentialId === 1) {
        return Promise.resolve(mockCredentials[0]);
      }
      return Promise.resolve({});
    });
    
    // Click on a credential
    const credential = screen.getByText('Test DB Password');
    fireEvent.click(credential);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_standalone_credential', {
        token: 'test-token',
        credentialId: 1,
      });
    });
  });

  it('handles empty credentials list', async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'search_credentials') {
        return Promise.resolve({
          credentials: [],
          total_count: 0,
        });
      }
      if (command === 'get_credential_categories') {
        return Promise.resolve(mockCategories);
      }
      return Promise.resolve({});
    });
    
    render(<StandaloneCredentials />);
    
    await waitFor(() => {
      expect(screen.getByText(/no credentials found/i)).toBeInTheDocument();
    });
  });
});