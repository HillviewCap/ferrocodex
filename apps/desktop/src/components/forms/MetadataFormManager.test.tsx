import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import MetadataFormManager from './MetadataFormManager';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('./DynamicMetadataForm', () => ({
  default: ({ schemaId, readonly }: { schemaId: number; readonly?: boolean }) => (
    <div data-testid={readonly ? 'metadata-form-readonly' : 'metadata-form'}>schema {schemaId}</div>
  ),
}));
vi.mock('./SchemaDesigner', () => ({
  default: ({ onSave }: { onSave: (id: number) => void }) => (
    <div data-testid="schema-designer">
      <button onClick={() => onSave(5)}>save schema</button>
    </div>
  ),
}));

describe('MetadataFormManager', () => {
  useConsoleGuard();

  it('renders the three tabs from items and starts on the designer', () => {
    render(<MetadataFormManager />);

    expect(screen.getByText('Metadata Management')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Metadata Form/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Schema Designer/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: /Form Preview/ })).toBeInTheDocument();
    expect(screen.getByTestId('schema-designer')).toBeInTheDocument();
  });

  it('shows the empty form/preview states and switches tabs', () => {
    render(<MetadataFormManager />);

    fireEvent.click(screen.getByRole('tab', { name: /Metadata Form/ }));
    expect(screen.getByText('No schema selected. Please create or select a metadata schema first.')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: /Form Preview/ }));
    expect(screen.getByText('No schema to preview. Please create a schema first.')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: /Metadata Form/ }));
    fireEvent.click(screen.getByRole('button', { name: /Create Schema/ }));
    expect(screen.getByRole('tab', { name: /Schema Designer/ })).toHaveAttribute('aria-selected', 'true');
  });

  it('switches to the form once a schema is saved and renders the preview', async () => {
    render(<MetadataFormManager assetId={3} />);

    fireEvent.click(screen.getByText('save schema'));

    expect(await screen.findByTestId('metadata-form')).toHaveTextContent('schema 5');
    expect(screen.getByRole('tab', { name: /Metadata Form/ })).toHaveAttribute('aria-selected', 'true');

    fireEvent.click(screen.getByRole('tab', { name: /Form Preview/ }));
    expect(screen.getByTestId('metadata-form-readonly')).toHaveTextContent('schema 5');
  });

  it('renders only the designer in designer mode and a close button when onClose is given', () => {
    const onClose = vi.fn();
    const { rerender } = render(<MetadataFormManager mode="designer" />);
    expect(screen.getByTestId('schema-designer')).toBeInTheDocument();
    expect(screen.queryByRole('tab')).not.toBeInTheDocument();

    rerender(<MetadataFormManager onClose={onClose} />);
    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(onClose).toHaveBeenCalled();
  });
});
