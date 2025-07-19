import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import ConfigurationStatusBadge from '../ConfigurationStatusBadge';
import { ConfigurationStatus } from '../../types/assets';

describe('ConfigurationStatusBadge', () => {
  it('renders Draft status correctly', () => {
    render(<ConfigurationStatusBadge status="Draft" />);
    
    expect(screen.getByText('Draft')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /edit/i })).toBeInTheDocument();
  });

  it('renders Approved status correctly', () => {
    render(<ConfigurationStatusBadge status="Approved" />);
    
    expect(screen.getByText('Approved')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /check-circle/i })).toBeInTheDocument();
  });

  it('renders Golden status correctly', () => {
    render(<ConfigurationStatusBadge status="Golden" />);
    
    expect(screen.getByText('Golden')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /star/i })).toBeInTheDocument();
  });

  it('renders Archived status correctly', () => {
    render(<ConfigurationStatusBadge status="Archived" />);
    
    expect(screen.getByText('Archived')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /folder/i })).toBeInTheDocument();
  });

  it('renders small size correctly', () => {
    const { container } = render(<ConfigurationStatusBadge status="Draft" size="small" />);
    
    const badge = container.querySelector('.ant-tag');
    expect(badge).toHaveStyle('font-size: 11px');
  });

  it('renders default size correctly', () => {
    const { container } = render(<ConfigurationStatusBadge status="Draft" />);
    
    const badge = container.querySelector('.ant-tag');
    expect(badge).not.toHaveStyle('font-size: 11px');
  });

  it('shows tooltip with description on hover', async () => {
    render(<ConfigurationStatusBadge status="Draft" showTooltip />);
    
    const badge = screen.getByText('Draft');
    expect(badge).toBeInTheDocument();
  });

  it('does not show tooltip when showTooltip is false', () => {
    render(<ConfigurationStatusBadge status="Draft" showTooltip={false} />);
    
    const badge = screen.getByText('Draft');
    expect(badge).toBeInTheDocument();
  });

  it('applies correct color for each status', () => {
    const { container: draftContainer } = render(<ConfigurationStatusBadge status="Draft" />);
    const { container: approvedContainer } = render(<ConfigurationStatusBadge status="Approved" />);
    const { container: goldenContainer } = render(<ConfigurationStatusBadge status="Golden" />);
    const { container: archivedContainer } = render(<ConfigurationStatusBadge status="Archived" />);

    expect(draftContainer.querySelector('.ant-tag-default')).toBeInTheDocument();
    expect(approvedContainer.querySelector('.ant-tag-success')).toBeInTheDocument();
    expect(goldenContainer.querySelector('.ant-tag-gold')).toBeInTheDocument();
    expect(archivedContainer.querySelector('.ant-tag-warning')).toBeInTheDocument();
  });

  it('handles unknown status gracefully', () => {
    render(<ConfigurationStatusBadge status={'Unknown' as ConfigurationStatus} />);
    
    expect(screen.getByText('Unknown')).toBeInTheDocument();
  });
});