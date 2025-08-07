import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { AssetTypeSelector } from './AssetTypeSelector';
import { AssetType } from '../../types/assets';

describe('AssetTypeSelector', () => {
  it('renders both folder and device options', () => {
    render(<AssetTypeSelector />);
    
    expect(screen.getByText('Folder')).toBeInTheDocument();
    expect(screen.getByText('Device')).toBeInTheDocument();
    expect(screen.getByText(/container that can hold other folders/)).toBeInTheDocument();
    expect(screen.getByText(/individual piece of equipment/)).toBeInTheDocument();
  });

  it('calls onChange when folder is selected', () => {
    const mockOnChange = vi.fn();
    render(<AssetTypeSelector onChange={mockOnChange} />);
    
    const folderCard = screen.getByText('Folder').closest('.ant-card');
    expect(folderCard).toBeInTheDocument();
    
    fireEvent.click(folderCard!);
    expect(mockOnChange).toHaveBeenCalledWith('Folder');
  });

  it('calls onChange when device is selected', () => {
    const mockOnChange = vi.fn();
    render(<AssetTypeSelector onChange={mockOnChange} />);
    
    const deviceCard = screen.getByText('Device').closest('.ant-card');
    expect(deviceCard).toBeInTheDocument();
    
    fireEvent.click(deviceCard!);
    expect(mockOnChange).toHaveBeenCalledWith('Device');
  });

  it('shows selected state for folder', () => {
    render(<AssetTypeSelector value="Folder" />);
    
    const folderCard = screen.getByText('Folder').closest('.ant-card');
    expect(folderCard).toHaveStyle('border: 2px solid #1890ff');
  });

  it('shows selected state for device', () => {
    render(<AssetTypeSelector value="Device" />);
    
    const deviceCard = screen.getByText('Device').closest('.ant-card');
    expect(deviceCard).toHaveStyle('border: 2px solid #1890ff');
  });

  it('disables interaction when disabled prop is true', () => {
    const mockOnChange = vi.fn();
    render(<AssetTypeSelector disabled onChange={mockOnChange} />);
    
    const folderCard = screen.getByText('Folder').closest('.ant-card');
    expect(folderCard).toHaveStyle('cursor: not-allowed');
    expect(folderCard).toHaveStyle('opacity: 0.6');
    
    fireEvent.click(folderCard!);
    expect(mockOnChange).not.toHaveBeenCalled();
  });

  it('renders radio buttons with correct values', () => {
    render(<AssetTypeSelector />);
    
    const folderRadio = screen.getByDisplayValue('Folder');
    const deviceRadio = screen.getByDisplayValue('Device');
    
    expect(folderRadio).toBeInTheDocument();
    expect(deviceRadio).toBeInTheDocument();
    expect(folderRadio).toHaveAttribute('type', 'radio');
    expect(deviceRadio).toHaveAttribute('type', 'radio');
  });

  it('shows proper icons for each asset type', () => {
    render(<AssetTypeSelector />);
    
    // Check that folder and tool icons are present
    const icons = document.querySelectorAll('.anticon');
    expect(icons.length).toBeGreaterThanOrEqual(2);
  });
});