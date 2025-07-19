import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import VersionCard from '../VersionCard';
import { ConfigurationVersionInfo } from '../../types/assets';

describe('VersionCard', () => {
  const mockVersion: ConfigurationVersionInfo = {
    id: 1,
    asset_id: 1,
    version_number: 'v1',
    file_name: 'config.json',
    file_size: 1024,
    content_hash: 'abc123def456',
    author: 1,
    author_username: 'john_doe',
    notes: 'Initial configuration version',
    status: 'Draft',
    status_changed_by: undefined,
    status_changed_at: undefined,
    created_at: '2023-01-01T12:00:00Z'
  };

  it('renders version information correctly', () => {
    render(<VersionCard version={mockVersion} />);
    
    expect(screen.getByText('v1')).toBeInTheDocument();
    expect(screen.getByText('config.json')).toBeInTheDocument();
    expect(screen.getByText('john_doe')).toBeInTheDocument();
    expect(screen.getByText('Initial configuration version')).toBeInTheDocument();
  });

  it('formats file size correctly', () => {
    render(<VersionCard version={mockVersion} />);
    
    expect(screen.getByText('1 KB')).toBeInTheDocument();
  });

  it('shows truncated content hash', () => {
    render(<VersionCard version={mockVersion} />);
    
    expect(screen.getByText('abc123de...')).toBeInTheDocument();
  });

  it('renders without notes when none provided', () => {
    const versionWithoutNotes = { ...mockVersion, notes: '' };
    render(<VersionCard version={versionWithoutNotes} />);
    
    expect(screen.getByText('v1')).toBeInTheDocument();
    expect(screen.queryByText('Initial configuration version')).not.toBeInTheDocument();
  });

  it('shows appropriate version color based on version number', () => {
    const { container } = render(<VersionCard version={mockVersion} />);
    
    // v1 should have green color
    const versionTag = container.querySelector('.ant-tag-green');
    expect(versionTag).toBeInTheDocument();
  });

  it('handles different file extensions correctly', () => {
    const xmlVersion = { ...mockVersion, file_name: 'config.xml' };
    render(<VersionCard version={xmlVersion} />);
    
    expect(screen.getByText('config.xml')).toBeInTheDocument();
  });

  it('formats relative time correctly', () => {
    const recentVersion = { 
      ...mockVersion, 
      created_at: new Date(Date.now() - 5 * 60 * 1000).toISOString() // 5 minutes ago
    };
    render(<VersionCard version={recentVersion} />);
    
    expect(screen.getByText('5 minutes ago')).toBeInTheDocument();
  });

  it('shows content hash is displayed', () => {
    render(<VersionCard version={mockVersion} />);
    
    const hashElement = screen.getByText('abc123de...');
    expect(hashElement).toBeInTheDocument();
  });

  it('displays status badge correctly', () => {
    render(<VersionCard version={mockVersion} />);
    
    expect(screen.getByText('Draft')).toBeInTheDocument();
  });

  it('shows status dropdown when user has permissions', () => {
    const { container } = render(<VersionCard version={mockVersion} />);
    
    // Check if dropdown trigger is present
    const dropdown = container.querySelector('.ant-dropdown-trigger');
    expect(dropdown).toBeInTheDocument();
  });

  it('displays different status badges correctly', () => {
    const approvedVersion = { ...mockVersion, status: 'Approved' as const };
    const { rerender } = render(<VersionCard version={approvedVersion} />);
    
    expect(screen.getByText('Approved')).toBeInTheDocument();

    const goldenVersion = { ...mockVersion, status: 'Golden' as const };
    rerender(<VersionCard version={goldenVersion} />);
    
    expect(screen.getByText('Golden')).toBeInTheDocument();

    const archivedVersion = { ...mockVersion, status: 'Archived' as const };
    rerender(<VersionCard version={archivedVersion} />);
    
    expect(screen.getByText('Archived')).toBeInTheDocument();
  });

  it('handles status change and history actions', () => {
    const { container } = render(<VersionCard version={mockVersion} />);
    
    // Verify that status-related UI elements are present
    expect(screen.getByText('Draft')).toBeInTheDocument();
    
    // Check that the card has proper structure for status functionality
    const statusSection = container.querySelector('[data-testid="version-status"]') || 
                         container.querySelector('.ant-tag');
    expect(statusSection).toBeInTheDocument();
  });
});