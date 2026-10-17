import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import MetadataManagement from './MetadataManagement';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

// The three tab panes are full screens with their own data loading; stub them so this
// test exercises only the Tabs `items` wiring in MetadataManagement itself.
vi.mock('./FieldTemplateLibrary', () => ({
  default: () => <div data-testid="field-template-library">templates pane</div>,
}));
vi.mock('../forms/SchemaDesigner', () => ({
  default: () => <div data-testid="schema-designer">designer pane</div>,
}));
vi.mock('../search/MetadataSearchPage', () => ({
  default: () => <div data-testid="metadata-search-page">search pane</div>,
}));

describe('MetadataManagement', () => {
  useConsoleGuard();

  it('renders the heading and every tab label from items', () => {
    render(<MetadataManagement />);

    expect(screen.getByText('Metadata Management')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Field Templates/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Schema Designer/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Metadata Search/ })).toBeInTheDocument();
  });

  it('shows the templates pane by default and switches panes when a tab is clicked', () => {
    render(<MetadataManagement />);

    expect(screen.getByTestId('field-template-library')).toBeInTheDocument();
    expect(screen.queryByTestId('schema-designer')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: /Schema Designer/ }));
    expect(screen.getByTestId('schema-designer')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Schema Designer/ })).toHaveAttribute('aria-selected', 'true');

    fireEvent.click(screen.getByRole('tab', { name: /Metadata Search/ }));
    expect(screen.getByTestId('metadata-search-page')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Metadata Search/ })).toHaveAttribute('aria-selected', 'true');
  });
});
