import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import VersionHistoryList from '../VersionHistoryList';
import { ConfigurationVersionInfo } from '../../types/assets';

describe('VersionHistoryList', () => {
  const mockVersions: ConfigurationVersionInfo[] = [
    {
      id: 1,
      asset_id: 1,
      version_number: 'v1',
      file_name: 'config.json',
      file_size: 1024,
      content_hash: 'abc123def456',
      author: 1,
      author_username: 'john_doe',
      notes: 'Initial version',
      status: 'Draft',
      status_changed_by: undefined,
      status_changed_at: undefined,
      created_at: '2023-01-01T12:00:00Z'
    },
    {
      id: 2,
      asset_id: 1,
      version_number: 'v2',
      file_name: 'config.json',
      file_size: 2048,
      content_hash: 'def456ghi789',
      author: 2,
      author_username: 'jane_smith',
      notes: 'Updated configuration',
      status: 'Approved',
      status_changed_by: undefined,
      status_changed_at: undefined,
      created_at: '2023-01-02T12:00:00Z'
    },
    {
      id: 3,
      asset_id: 1,
      version_number: 'v3',
      file_name: 'config.json',
      file_size: 1536,
      content_hash: 'ghi789jkl012',
      author: 1,
      author_username: 'john_doe',
      notes: 'Bug fixes',
      status: 'Golden',
      status_changed_by: undefined,
      status_changed_at: undefined,
      created_at: '2023-01-03T12:00:00Z'
    }
  ];

  it('renders all versions', () => {
    render(<VersionHistoryList versions={mockVersions} />);
    
    expect(screen.getByText('v1')).toBeInTheDocument();
    expect(screen.getByText('v2')).toBeInTheDocument();
    expect(screen.getByText('v3')).toBeInTheDocument();
  });

  it('sorts versions in reverse chronological order', () => {
    render(<VersionHistoryList versions={mockVersions} />);
    
    const versionElements = screen.getAllByText(/v[123]/);
    expect(versionElements[0]).toHaveTextContent('v3'); // Latest first
    expect(versionElements[1]).toHaveTextContent('v2');
    expect(versionElements[2]).toHaveTextContent('v1');
  });

  it('shows pagination when there are more than 10 versions', () => {
    const manyVersions = Array.from({ length: 15 }, (_, i) => ({
      ...mockVersions[0],
      id: i + 1,
      version_number: `v${i + 1}`,
      status: 'Draft' as const,
      status_changed_by: undefined,
      status_changed_at: undefined,
      created_at: new Date(Date.now() - i * 24 * 60 * 60 * 1000).toISOString()
    }));

    render(<VersionHistoryList versions={manyVersions} />);
    
    expect(screen.getByText('1-10 of 15 versions')).toBeInTheDocument();
  });

  it('does not show pagination when there are 10 or fewer versions', () => {
    render(<VersionHistoryList versions={mockVersions} />);
    
    expect(screen.queryByText(/of \d+ versions/)).not.toBeInTheDocument();
  });

  it('renders empty list when no versions provided', () => {
    render(<VersionHistoryList versions={[]} />);
    
    // When the list is empty, Ant Design shows an empty state instead of a list
    const emptyElements = screen.getAllByText('No data');
    expect(emptyElements.length).toBeGreaterThan(0);
  });

  it('handles single version correctly', () => {
    const singleVersion = [mockVersions[0]];
    render(<VersionHistoryList versions={singleVersion} />);
    
    expect(screen.getByText('v1')).toBeInTheDocument();
    expect(screen.getByText('john_doe')).toBeInTheDocument();
    expect(screen.getByText('Initial version')).toBeInTheDocument();
  });

  it('maintains consistent spacing for list items', () => {
    const { container } = render(<VersionHistoryList versions={mockVersions} />);
    
    const listItems = container.querySelectorAll('.ant-list-item');
    expect(listItems).toHaveLength(3);
    
    listItems.forEach(item => {
      expect(item).toHaveStyle('padding: 12px 0');
    });
  });
});