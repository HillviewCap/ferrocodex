import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ComplianceStatusIndicator from './ComplianceStatusIndicator';
import { ComplianceStatus } from '../../types/security';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const clean: ComplianceStatus = { level: 'excellent', score: 100, issues: [], recommendations: [] };
const withFindings: ComplianceStatus = {
  level: 'fair',
  score: 62,
  issues: ['Name contains spaces'],
  recommendations: ['Use underscores instead of spaces'],
};

describe('ComplianceStatusIndicator', () => {
  useConsoleGuard();

  it('renders the status badge with score and progress', () => {
    render(<ComplianceStatusIndicator status={clean} showProgress />);

    expect(screen.getByText('Excellent')).toBeInTheDocument();
    expect(screen.getByText('(100%)')).toBeInTheDocument();
    expect(document.querySelector('.ant-progress')).not.toBeNull();
  });

  it('renders issues and recommendations inline when showDetails is set', () => {
    render(<ComplianceStatusIndicator status={withFindings} showDetails />);

    expect(screen.getByText('Fair')).toBeInTheDocument();
    expect(screen.getByText(/Issues \(1\):/)).toBeInTheDocument();
    expect(screen.getByText('Name contains spaces')).toBeInTheDocument();
    expect(screen.getByText(/Recommendations \(1\):/)).toBeInTheDocument();
    expect(screen.getByText('Use underscores instead of spaces')).toBeInTheDocument();
  });

  it('shows the findings in a tooltip on hover when details are collapsed', async () => {
    render(<ComplianceStatusIndicator status={withFindings} />);

    fireEvent.mouseEnter(screen.getByText('Fair'));

    expect(await screen.findByText('Some security requirements need attention')).toBeInTheDocument();
    expect(screen.getByText('Issues:')).toBeInTheDocument();
    expect(screen.getByText('Name contains spaces')).toBeInTheDocument();
    expect(screen.getByText('Recommendations:')).toBeInTheDocument();
  });
});
