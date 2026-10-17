import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FileUploadStep from './FileUploadStep';
import { AssetInfo } from '../../types/associations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const selectedAsset: AssetInfo = {
  id: 2,
  name: 'PLC-001',
  description: 'Primary PLC',
  assetType: 'Device',
  sortOrder: 0,
  createdBy: 1,
  createdByUsername: 'admin',
  createdAt: '2024-01-01T00:00:00Z',
  versionCount: 0,
};

// The uploaded-file list uses antd `List`, which antd 6.6 marks deprecated and which the
// migration commit explicitly left for a follow-up. These tests cover the instructions
// alert and drop zone without uploading, so the List is not rendered.
describe('FileUploadStep', () => {
  useConsoleGuard();

  it('renders the instructions alert and drop zone for the selected asset', () => {
    render(
      <FileUploadStep selectedAsset={selectedAsset} fileType="Firmware" onFilesSelected={vi.fn()} onUploadProgress={vi.fn()} />,
    );

    expect(screen.getByText('Upload Files for PLC-001')).toBeInTheDocument();
    expect(screen.getByText('File Upload Instructions')).toBeInTheDocument();
    expect(screen.getByText(/Upload firmware files to associate with/)).toBeInTheDocument();
    const alert = document.querySelector('.ant-alert') as HTMLElement;
    expect(alert.textContent).toContain('Accepted file types: .bin,.hex,.srec,.elf,.img,.rom,.fw');
    expect(alert.textContent).toContain('Maximum file size: 2GB');
    expect(screen.getByText('Click or drag files to this area to upload')).toBeInTheDocument();
    expect(document.querySelector('input[type="file"]')).toHaveAttribute('accept', '.bin,.hex,.srec,.elf,.img,.rom,.fw');
    expect(screen.queryByText('Uploaded Files')).not.toBeInTheDocument();
  });

  it('accepts both file families when no type is given', () => {
    render(<FileUploadStep selectedAsset={selectedAsset} onFilesSelected={vi.fn()} onUploadProgress={vi.fn()} />);

    expect(screen.getByText(/Upload configuration or firmware files/)).toBeInTheDocument();
    expect((document.querySelector('.ant-alert') as HTMLElement).textContent).toContain('Maximum file size: 100MB');
    expect(document.querySelector('input[type="file"]')).toHaveAttribute(
      'accept',
      '.json,.xml,.yaml,.yml,.cfg,.conf,.ini,.txt,.bin,.hex,.srec,.elf,.img,.rom,.fw',
    );
  });
});
