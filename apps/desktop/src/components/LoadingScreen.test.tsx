import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import LoadingScreen from './LoadingScreen';
import { useConsoleGuard } from '../test-utils/consoleGuard';

describe('LoadingScreen', () => {
  useConsoleGuard();

  it('renders the brand, spinner and default message', () => {
    render(<LoadingScreen />);

    expect(screen.getByText('Ferrocodex')).toBeInTheDocument();
    expect(screen.getByText('Initializing Ferrocodex...')).toBeInTheDocument();
    expect(document.querySelector('.ant-spin')).not.toBeNull();
    // Vertical Space layout survives the direction -> orientation rename
    expect(document.querySelectorAll('.ant-space-vertical').length).toBe(2);
  });

  it('renders a custom message', () => {
    render(<LoadingScreen message="Opening database..." />);
    expect(screen.getByText('Opening database...')).toBeInTheDocument();
  });
});
