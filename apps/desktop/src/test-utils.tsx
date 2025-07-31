import React from 'react';
import { render, RenderOptions, act } from '@testing-library/react';
import { vi } from 'vitest';
import { App } from 'antd';

// Custom render function that wraps components with necessary providers
const AllTheProviders: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <App>
      <div>{children}</div>
    </App>
  );
};

// Custom render with act wrapper
const customRender = (ui: React.ReactElement, options?: Omit<RenderOptions, 'wrapper'>) => {
  let renderResult: ReturnType<typeof render>;
  
  act(() => {
    renderResult = render(ui, { wrapper: AllTheProviders, ...options });
  });
  
  return renderResult!;
};

// Helper function to wrap async operations in act
export const actAsync = async (callback: () => Promise<void>) => {
  await act(async () => {
    await callback();
  });
};

// Helper function to create mock form instance for Ant Design
export const createMockForm = () => ({
  getFieldValue: vi.fn(),
  getFieldsValue: vi.fn(() => ({})),
  setFieldsValue: vi.fn(),
  setFieldValue: vi.fn(),
  resetFields: vi.fn(),
  validateFields: vi.fn(() => Promise.resolve({})),
  getFieldInstance: vi.fn(),
  submit: vi.fn(),
  scrollToField: vi.fn(),
  __INTERNAL__: {
    name: 'test-form',
  },
});

// Helper function to wait for component to stabilize
export const waitForStabilization = () => new Promise(resolve => setTimeout(resolve, 0));

// Mock auth store helper
export const createMockAuthStore = (overrides = {}) => ({
  token: 'test-token',
  user: { id: 1, username: 'testuser', role: 'Administrator' },
  isAuthenticated: true,
  ...overrides,
});

// Re-export everything from testing-library
export * from '@testing-library/react';
export { customRender as render };