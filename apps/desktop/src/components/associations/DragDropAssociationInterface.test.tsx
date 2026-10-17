import { render, screen } from '@testing-library/react';
import { describe, it, expect, beforeEach } from 'vitest';
import DragDropAssociationInterface from './DragDropAssociationInterface';
import useAssociationStore from '../../store/associations';
import { AssetInfo } from '../../types/associations';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const targetAsset: AssetInfo = {
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

// The file queue uses antd `List`, which antd 6.6 marks deprecated and which the
// migration commit explicitly left for a follow-up. These tests cover the drop zone and
// alerts without queueing files, so the List is not rendered.
describe('DragDropAssociationInterface', () => {
  useConsoleGuard();

  beforeEach(() => {
    useAssociationStore.getState().reset();
  });

  it('renders the target asset alert, drop zone and usage instructions', () => {
    render(<DragDropAssociationInterface targetAsset={targetAsset} allowedFileTypes={['Configuration']} />);

    expect(screen.getByText('Drag & Drop File Association')).toBeInTheDocument();
    expect(screen.getByText('Target Asset')).toBeInTheDocument();
    expect(screen.getAllByText('PLC-001').length).toBeGreaterThan(0);
    expect(screen.getByText('- Primary PLC')).toBeInTheDocument();
    expect(screen.getByText('| Allowed types: Configuration')).toBeInTheDocument();

    expect(screen.getByText(/Drop files here to associate with/)).toBeInTheDocument();
    expect(document.querySelector('.ant-upload-drag')).not.toBeNull();

    expect(screen.getByText('How to Use')).toBeInTheDocument();
    expect(screen.getByText(/Drag files from your file explorer/)).toBeInTheDocument();
    expect(screen.queryByText('File Association Queue')).not.toBeInTheDocument();
  });

  it('disables the drop zone while the store is loading', () => {
    useAssociationStore.setState({ loading: true });
    render(<DragDropAssociationInterface targetAsset={targetAsset} />);

    expect(document.querySelector('.ant-upload-disabled')).not.toBeNull();
    expect(screen.queryByText(/Allowed types/)).not.toBeInTheDocument();
  });
});
