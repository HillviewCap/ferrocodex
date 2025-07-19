import { render, screen, fireEvent } from '@testing-library/react';
import { vi, expect, describe, it } from 'vitest';
import GoldenVersionIndicator from '../GoldenVersionIndicator';
import { ConfigurationVersionInfo } from '../../types/assets';

const mockGoldenVersion: ConfigurationVersionInfo = {
  id: 1,
  asset_id: 1,
  version_number: 'v3',
  file_name: 'golden-config.json',
  file_size: 2048,
  content_hash: 'def456',
  author: 1,
  author_username: 'jane.smith',
  notes: 'Golden master configuration for production',
  status: 'Golden',
  status_changed_by: 2,
  status_changed_at: '2025-01-15T14:30:00Z',
  created_at: '2025-01-15T10:00:00Z'
};

const mockProps = {
  goldenVersion: mockGoldenVersion,
  onViewDetails: vi.fn(),
  onViewHistory: vi.fn()
};

describe('GoldenVersionIndicator', () => {
  it('renders golden version information correctly', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    expect(screen.getByText('GOLDEN IMAGE')).toBeInTheDocument();
    expect(screen.getByText('Golden Image Version')).toBeInTheDocument();
    expect(screen.getByText('golden-config.json')).toBeInTheDocument();
    expect(screen.getByText('v3')).toBeInTheDocument();
    expect(screen.getByText('jane.smith')).toBeInTheDocument();
    expect(screen.getByText('Golden master configuration for production')).toBeInTheDocument();
  });

  it('displays file type icon based on file extension', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    // JSON files should show the config icon (🔧)
    expect(screen.getByText('🔧')).toBeInTheDocument();
  });

  it('shows relative time for golden status', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    // Should show relative time like "Golden X ago"
    expect(screen.getByText(/Golden \d+/)).toBeInTheDocument();
  });

  it('displays file size in human readable format', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    expect(screen.getByText('2 KB')).toBeInTheDocument();
  });

  it('renders view details button when callback provided', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    const viewDetailsButton = screen.getByText('View Details');
    expect(viewDetailsButton).toBeInTheDocument();
    
    fireEvent.click(viewDetailsButton);
    expect(mockProps.onViewDetails).toHaveBeenCalledWith(mockGoldenVersion);
  });

  it('renders view history button when callback provided', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    const viewHistoryButton = screen.getByText('View History');
    expect(viewHistoryButton).toBeInTheDocument();
    
    fireEvent.click(viewHistoryButton);
    expect(mockProps.onViewHistory).toHaveBeenCalledWith(mockGoldenVersion);
  });

  it('does not render buttons when callbacks not provided', () => {
    render(
      <GoldenVersionIndicator 
        goldenVersion={mockGoldenVersion}
        onViewDetails={undefined}
        onViewHistory={undefined}
      />
    );
    
    expect(screen.queryByText('View Details')).not.toBeInTheDocument();
    expect(screen.queryByText('View History')).not.toBeInTheDocument();
  });

  it('handles version without notes', () => {
    const versionWithoutNotes: ConfigurationVersionInfo = {
      ...mockGoldenVersion,
      notes: ''
    };
    
    render(<GoldenVersionIndicator goldenVersion={versionWithoutNotes} />);
    
    expect(screen.getByText('Golden Image Version')).toBeInTheDocument();
    expect(screen.queryByText('Golden master configuration for production')).not.toBeInTheDocument();
  });

  it('handles version without status_changed_at', () => {
    const versionWithoutStatusChanged: ConfigurationVersionInfo = {
      ...mockGoldenVersion,
      status_changed_at: undefined
    };
    
    render(<GoldenVersionIndicator goldenVersion={versionWithoutStatusChanged} />);
    
    // Should fall back to created_at for relative time
    expect(screen.getByText(/Golden \d+/)).toBeInTheDocument();
  });

  it('applies custom className and style', () => {
    const customClassName = 'custom-golden-indicator';
    const customStyle = { margin: '10px' };
    
    const { container } = render(
      <GoldenVersionIndicator 
        {...mockProps}
        className={customClassName}
        style={customStyle}
      />
    );
    
    const cardElement = container.querySelector('.custom-golden-indicator');
    expect(cardElement).toBeInTheDocument();
    expect(cardElement).toHaveStyle('margin: 10px');
  });

  it('displays appropriate file type icons for different extensions', () => {
    const testCases = [
      { fileName: 'config.xml', expectedIcon: '📄' },
      { fileName: 'config.yml', expectedIcon: '📝' },
      { fileName: 'config.yaml', expectedIcon: '📝' },
      { fileName: 'config.txt', expectedIcon: '📄' },
      { fileName: 'config.bin', expectedIcon: '💾' },
      { fileName: 'config.dat', expectedIcon: '💾' },
      { fileName: 'config.unknown', expectedIcon: '📄' }
    ];

    testCases.forEach(({ fileName, expectedIcon }) => {
      const version = { ...mockGoldenVersion, file_name: fileName };
      const { rerender } = render(<GoldenVersionIndicator goldenVersion={version} />);
      
      expect(screen.getByText(expectedIcon)).toBeInTheDocument();
      
      rerender(<div />); // Clean up for next iteration
    });
  });

  it('displays disaster recovery information', () => {
    render(<GoldenVersionIndicator {...mockProps} />);
    
    expect(screen.getByText(/This is the official master version for disaster recovery operations/)).toBeInTheDocument();
  });

  it('has proper golden styling and visual indicators', () => {
    const { container } = render(<GoldenVersionIndicator {...mockProps} />);
    
    // Check for golden ribbon
    expect(screen.getByText('GOLDEN IMAGE')).toBeInTheDocument();
    
    // Check for golden styling
    const card = container.querySelector('.ant-card');
    expect(card).toHaveStyle('border: 2px solid #fadb14');
  });
});