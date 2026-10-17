import React from 'react';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { vi } from 'vitest';
import { App } from 'antd';
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

// The component uses App.useApp() for messages, so render it inside <App> as the real app does
const renderComponent = () => render(
  <App>
    <StandaloneCredentials />
  </App>
);

// Role/label queries over the whole page (table, tree, modals) are slow in jsdom,
// so scope them to the element under test
const findCreateModal = async () =>
  (await screen.findByText('Create Standalone Credential')).closest('.ant-modal') as HTMLElement;

// First mounts of the page and the create modal pay Ant Design's style-generation cost
const SLOW_TEST_TIMEOUT = 20000;

const rowActionButton = (row: HTMLElement, icon: string) =>
  row.querySelector(`.anticon-${icon}`)?.closest('button') as HTMLElement | null;

describe('StandaloneCredentials', () => {
  const mockInvoke = vi.mocked(invoke);
  // createCredential needs the user id for created_by
  const mockAuthStore = { token: 'test-token', user: { id: 1, username: 'testuser' } };
  
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
      credential_count: 1,
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
          credential_count: 0,
        },
      ],
      credential_count: 1,
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
            page: 1,
            page_size: 50,
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
    renderComponent();
    
    expect(screen.getByText('Vault')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Search credentials...')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add credential/i })).toBeInTheDocument();
    
    // Wait for categories and credentials to load
    await waitFor(() => {
      expect(screen.getByText('Jump Hosts')).toBeInTheDocument();
      expect(screen.getByText('Databases')).toBeInTheDocument();
    });
  }, SLOW_TEST_TIMEOUT);

  it('displays categories in tree structure', async () => {
    renderComponent();
    
    await waitFor(() => {
      expect(screen.getByText('Jump Hosts')).toBeInTheDocument();
      expect(screen.getByText('Databases')).toBeInTheDocument();
    });
    
    // The tree is expanded by default, so subcategories are visible without clicking
    expect(screen.getByText('Production')).toBeInTheDocument();
  });

  it('displays credentials list', async () => {
    renderComponent();
    
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
    renderComponent();
    
    const newCredentialBtn = screen.getByRole('button', { name: /add credential/i });
    fireEvent.click(newCredentialBtn);
    
    const modal = await findCreateModal();
    expect(within(modal).getByLabelText('Name')).toBeInTheDocument();
    expect(within(modal).getByLabelText('Description')).toBeInTheDocument();
    expect(within(modal).getByLabelText('Type')).toBeInTheDocument();
    expect(within(modal).getByLabelText('Category')).toBeInTheDocument();
  }, SLOW_TEST_TIMEOUT);

  it('creates a new credential', async () => {
    renderComponent();
    
    // Open modal
    const newCredentialBtn = screen.getByRole('button', { name: /add credential/i });
    fireEvent.click(newCredentialBtn);
    
    const modal = await findCreateModal();
    
    // Fill form
    fireEvent.change(within(modal).getByLabelText('Name'), { 
      target: { value: 'New Test Credential' } 
    });
    fireEvent.change(within(modal).getByLabelText('Description'), { 
      target: { value: 'New test description' } 
    });

    // Type is required; pick Password from the Select
    fireEvent.mouseDown(within(modal).getByLabelText('Type'));
    const passwordOption = await waitFor(() => {
      const option = document.querySelector('.ant-select-item-option[title="Password"]')
        ?? Array.from(document.querySelectorAll('.ant-select-item-option'))
          .find(el => el.textContent?.trim() === 'Password');
      expect(option).toBeTruthy();
      return option as HTMLElement;
    });
    fireEvent.click(passwordOption);
    
    // Use password input
    const passwordInput = within(modal).getByTestId('password-input');
    fireEvent.change(passwordInput, { target: { value: 'test-password' } });
    
    // Submit form (the modal uses Ant Design's default OK button)
    fireEvent.click(within(modal).getByText('OK').closest('button')!);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_standalone_credential', {
        token: 'test-token',
        credentialRequest: expect.objectContaining({
          name: 'New Test Credential',
          description: 'New test description',
          credential_type: 'Password',
          value: 'test-password',
          created_by: 1,
        }),
      });
    });
  }, SLOW_TEST_TIMEOUT);

  it('filters credentials by search query', async () => {
    renderComponent();
    
    await waitFor(() => {
      expect(screen.getByText('Test DB Password')).toBeInTheDocument();
    });
    
    // Search runs when the filter form is submitted
    const searchInput = screen.getByPlaceholderText('Search credentials...');
    fireEvent.change(searchInput, { target: { value: 'database' } });
    fireEvent.click(screen.getByText('Filter').closest('button')!);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('search_credentials', {
        token: 'test-token',
        searchRequest: expect.objectContaining({
          query: 'database',
        }),
      });
    });
  });

  it('filters credentials by category', async () => {
    renderComponent();
    
    await waitFor(() => {
      expect(screen.getByText('Jump Hosts')).toBeInTheDocument();
    });
    
    // Click on a category
    const jumpHostsCategory = screen.getByText('Jump Hosts');
    fireEvent.click(jumpHostsCategory);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('search_credentials', {
        token: 'test-token',
        searchRequest: expect.objectContaining({
          category_id: 1,
        }),
      });
    });
  });

  it('decrypts and reveals a credential value when View is clicked', async () => {
    const baseImpl = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command: string, args?: any) => {
      if (command === 'decrypt_standalone_credential' && args?.credentialId === 1) {
        return Promise.resolve('s3cret-value');
      }
      return baseImpl(command, args);
    });

    renderComponent();
    
    const row = (await screen.findByText('Test DB Password')).closest('tr')!;
    
    fireEvent.click(rowActionButton(row, 'eye')!);
    
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('decrypt_standalone_credential', {
        token: 'test-token',
        credentialId: 1,
      });
    });

    // Once decrypted, a Copy action appears and the value is shown in the row's expanded area
    await waitFor(() => {
      expect(rowActionButton(row, 'copy')).toBeInTheDocument();
    });
    fireEvent.click(row.querySelector('.ant-table-row-expand-icon')!);
    expect(await screen.findByDisplayValue('s3cret-value')).toBeInTheDocument();
  });

  it('handles empty credentials list', async () => {
    mockInvoke.mockImplementation((command: string) => {
      if (command === 'search_credentials') {
        return Promise.resolve({
          credentials: [],
          total_count: 0,
          page: 1,
          page_size: 50,
        });
      }
      if (command === 'get_credential_categories') {
        return Promise.resolve(mockCategories);
      }
      return Promise.resolve({});
    });
    
    renderComponent();
    
    await waitFor(() => {
      expect(screen.getByText(/no credentials found/i)).toBeInTheDocument();
    });
  });
});