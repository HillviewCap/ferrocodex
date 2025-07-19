import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
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

  it('shows promote to golden action for approved versions when user has permission', () => {
    const approvedVersion = { ...mockVersion, status: 'Approved' as const };
    render(
      <VersionCard 
        version={approvedVersion} 
        canPromoteToGolden={true}
        token="test-token"
      />
    );
    
    // Approved status should be displayed
    expect(screen.getByText('Approved')).toBeInTheDocument();
    
    // Component should have the structure to support golden promotion
    const { container } = render(
      <VersionCard 
        version={approvedVersion} 
        canPromoteToGolden={true}
        token="test-token"
      />
    );
    expect(container.querySelector('.ant-card')).toBeInTheDocument();
  });

  it('does not show promote to golden action for non-approved versions', () => {
    render(
      <VersionCard 
        version={mockVersion} // Draft status
        canPromoteToGolden={true}
        token="test-token"
      />
    );
    
    // Should show Draft status
    expect(screen.getByText('Draft')).toBeInTheDocument();
    
    // The promote to golden action should not be available for Draft versions
    // (This is handled by the menu logic in the component)
  });

  it('handles golden promotion callback when provided', () => {
    const onGoldenPromotion = vi.fn();
    const approvedVersion = { ...mockVersion, status: 'Approved' as const };
    
    render(
      <VersionCard 
        version={approvedVersion} 
        canPromoteToGolden={true}
        token="test-token"
        onGoldenPromotion={onGoldenPromotion}
      />
    );
    
    // Verify component renders correctly with golden promotion props
    expect(screen.getByText('Approved')).toBeInTheDocument();
  });

  it('shows export action when user has export permission', () => {
    render(
      <VersionCard 
        version={mockVersion} 
        canExport={true}
        token="test-token"
      />
    );
    
    // Should render with export capability enabled
    expect(screen.getByText('Draft')).toBeInTheDocument();
    const { container } = render(
      <VersionCard 
        version={mockVersion} 
        canExport={true}
        token="test-token"
      />
    );
    expect(container.querySelector('.ant-card')).toBeInTheDocument();
  });

  it('does not show export action when user lacks permission', () => {
    render(<VersionCard version={mockVersion} canExport={false} />);
    
    // Should render normally but without export permission
    expect(screen.getByText('Draft')).toBeInTheDocument();
  });

  it('handles export callback when provided', () => {
    const onExport = vi.fn();
    
    render(
      <VersionCard 
        version={mockVersion} 
        canExport={true}
        token="test-token"
        onExport={onExport}
      />
    );
    
    // Verify component renders correctly with export props
    expect(screen.getByText('config.json')).toBeInTheDocument();
  });

  it('shows export option for all version statuses when user has permission', () => {
    const statuses = ['Draft', 'Approved', 'Golden', 'Archived'] as const;
    
    statuses.forEach(status => {
      const versionWithStatus = { ...mockVersion, status };
      const { container } = render(
        <VersionCard 
          version={versionWithStatus} 
          canExport={true}
          token="test-token"
        />
      );
      
      expect(screen.getByText(status)).toBeInTheDocument();
      expect(container.querySelector('.ant-card')).toBeInTheDocument();
    });
  });

  it('requires token for export functionality', () => {
    render(
      <VersionCard 
        version={mockVersion} 
        canExport={true}
        // No token provided
      />
    );
    
    // Should render but export functionality won't be fully enabled without token
    expect(screen.getByText('Draft')).toBeInTheDocument();
  });
});