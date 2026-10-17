import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import PasswordStrengthIndicator from './PasswordStrengthIndicator';
import { PasswordStrength } from '../types/vault';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const strong: PasswordStrength = {
  score: 85,
  entropy: 72.46,
  has_uppercase: true,
  has_lowercase: true,
  has_numbers: true,
  has_special: false,
  length: 16,
  feedback: ['Add a special character'],
};

describe('PasswordStrengthIndicator', () => {
  useConsoleGuard();

  it('renders the score bar and label', () => {
    render(<PasswordStrengthIndicator strength={strong} />);

    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(screen.getByText('Excellent (85/100)')).toBeInTheDocument();
    expect(screen.queryByText('Length:')).not.toBeInTheDocument();
  });

  it('renders the detail tags and recommendations when showDetails is set', () => {
    render(<PasswordStrengthIndicator strength={strong} showDetails size="small" />);

    expect(screen.getByText('Length:')).toBeInTheDocument();
    expect(screen.getByText('16 chars')).toBeInTheDocument();
    expect(screen.getByText('Entropy:')).toBeInTheDocument();
    expect(screen.getByText('72.5 bits')).toBeInTheDocument();
    for (const label of ['Uppercase', 'Lowercase', 'Numbers', 'Special']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByText('Recommendations:')).toBeInTheDocument();
    expect(screen.getByText('Add a special character')).toBeInTheDocument();
  });

  it('labels weak passwords', () => {
    render(<PasswordStrengthIndicator strength={{ ...strong, score: 20, length: 5, feedback: [] }} showDetails />);

    expect(screen.getByText('Weak (20/100)')).toBeInTheDocument();
    expect(screen.getByText('5 chars')).toBeInTheDocument();
    expect(screen.queryByText('Recommendations:')).not.toBeInTheDocument();
  });
});
