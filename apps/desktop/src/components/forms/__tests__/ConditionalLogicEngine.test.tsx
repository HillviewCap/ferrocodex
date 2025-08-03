import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { vi, describe, it, expect } from 'vitest';
import {
  ConditionalLogicProvider,
  useConditionalLogic,
  ConditionalRuleBuilder,
  CommonRules
} from '../ConditionalLogicEngine';

// Test component to interact with conditional logic
const TestComponent: React.FC = () => {
  const {
    state,
    evaluateRules,
    isFieldVisible,
    isFieldDisabled,
    isFieldRequired,
    addRules,
    clearRules
  } = useConditionalLogic();

  const [formData, setFormData] = React.useState({
    device_type: '',
    has_display: false,
    display_size: '',
    ip_address: '',
    use_dhcp: false
  });

  const handleEvaluate = () => {
    evaluateRules(formData);
  };

  const handleAddTestRules = () => {
    const rules = [
      // Show display_size when device_type is HMI
      ConditionalRuleBuilder
        .create('show_display_size_for_hmi')
        .when('device_type', '==', 'HMI')
        .show('display_size')
        .build(),

      // Hide IP address fields when use_dhcp is true
      ConditionalRuleBuilder
        .create('hide_ip_when_dhcp')
        .when('use_dhcp', '==', true)
        .hide('ip_address')
        .build(),

      // Require display_size when has_display is true
      ConditionalRuleBuilder
        .create('require_display_size')
        .when('has_display', '==', true)
        .require('display_size')
        .build(),

      // Disable ip_address when use_dhcp is true
      ConditionalRuleBuilder
        .create('disable_ip_when_dhcp')
        .when('use_dhcp', '==', true)
        .disable('ip_address')
        .build()
    ];

    addRules(rules);
  };

  return (
    <div>
      <button onClick={handleAddTestRules}>Add Test Rules</button>
      <button onClick={handleEvaluate}>Evaluate Rules</button>
      <button onClick={clearRules}>Clear Rules</button>
      
      <div>
        <label>
          Device Type:
          <select 
            value={formData.device_type}
            onChange={(e) => setFormData({ ...formData, device_type: e.target.value })}
          >
            <option value="">Select...</option>
            <option value="PLC">PLC</option>
            <option value="HMI">HMI</option>
            <option value="Switch">Switch</option>
          </select>
        </label>
      </div>

      <div>
        <label>
          <input
            type="checkbox"
            checked={formData.has_display}
            onChange={(e) => setFormData({ ...formData, has_display: e.target.checked })}
          />
          Has Display
        </label>
      </div>

      <div>
        <label>
          <input
            type="checkbox"
            checked={formData.use_dhcp}
            onChange={(e) => setFormData({ ...formData, use_dhcp: e.target.checked })}
          />
          Use DHCP
        </label>
      </div>

      {isFieldVisible('display_size') && (
        <div>
          <label>
            Display Size:
            <input
              type="text"
              value={formData.display_size}
              onChange={(e) => setFormData({ ...formData, display_size: e.target.value })}
              disabled={isFieldDisabled('display_size')}
              required={isFieldRequired('display_size')}
            />
          </label>
        </div>
      )}

      {isFieldVisible('ip_address') && (
        <div>
          <label>
            IP Address:
            <input
              type="text"
              value={formData.ip_address}
              onChange={(e) => setFormData({ ...formData, ip_address: e.target.value })}
              disabled={isFieldDisabled('ip_address')}
            />
          </label>
        </div>
      )}

      <div data-testid="field-states">
        <div>display_size visible: {isFieldVisible('display_size').toString()}</div>
        <div>display_size required: {isFieldRequired('display_size').toString()}</div>
        <div>ip_address visible: {isFieldVisible('ip_address').toString()}</div>
        <div>ip_address disabled: {isFieldDisabled('ip_address').toString()}</div>
      </div>
    </div>
  );
};

describe('ConditionalLogicEngine', () => {
  it('should provide conditional logic context', () => {
    render(
      <ConditionalLogicProvider>
        <TestComponent />
      </ConditionalLogicProvider>
    );

    expect(screen.getByText('Add Test Rules')).toBeInTheDocument();
    expect(screen.getByText('Evaluate Rules')).toBeInTheDocument();
    expect(screen.getByText('Clear Rules')).toBeInTheDocument();
  });

  it('should show/hide fields based on conditions', () => {
    render(
      <ConditionalLogicProvider>
        <TestComponent />
      </ConditionalLogicProvider>
    );

    // Add test rules
    fireEvent.click(screen.getByText('Add Test Rules'));

    // Initially, display_size should be visible (no rules applied yet)
    expect(screen.getByText('display_size visible: true')).toBeInTheDocument();

    // Select HMI device type
    fireEvent.change(screen.getByLabelText('Device Type:'), {
      target: { value: 'HMI' }
    });

    // Evaluate rules
    fireEvent.click(screen.getByText('Evaluate Rules'));

    // display_size should still be visible for HMI
    expect(screen.getByText('display_size visible: true')).toBeInTheDocument();

    // Select PLC device type (not HMI)
    fireEvent.change(screen.getByLabelText('Device Type:'), {
      target: { value: 'PLC' }
    });

    // Evaluate rules
    fireEvent.click(screen.getByText('Evaluate Rules'));

    // display_size should be hidden for PLC
    expect(screen.queryByLabelText('Display Size:')).not.toBeInTheDocument();
  });

  it('should handle checkbox conditions', () => {
    render(
      <ConditionalLogicProvider>
        <TestComponent />
      </ConditionalLogicProvider>
    );

    // Add test rules
    fireEvent.click(screen.getByText('Add Test Rules'));

    // Check "Use DHCP"
    fireEvent.click(screen.getByLabelText('Use DHCP'));

    // Evaluate rules
    fireEvent.click(screen.getByText('Evaluate Rules'));

    // IP address field should be hidden and disabled when DHCP is enabled
    expect(screen.getByText('ip_address visible: false')).toBeInTheDocument();
    expect(screen.getByText('ip_address disabled: true')).toBeInTheDocument();
  });

  it('should handle required field conditions', () => {
    render(
      <ConditionalLogicProvider>
        <TestComponent />
      </ConditionalLogicProvider>
    );

    // Add test rules
    fireEvent.click(screen.getByText('Add Test Rules'));

    // Check "Has Display"
    fireEvent.click(screen.getByLabelText('Has Display'));

    // Evaluate rules
    fireEvent.click(screen.getByText('Evaluate Rules'));

    // display_size should be required when has_display is true
    expect(screen.getByText('display_size required: true')).toBeInTheDocument();
  });

  it('should clear rules when requested', () => {
    render(
      <ConditionalLogicProvider>
        <TestComponent />
      </ConditionalLogicProvider>
    );

    // Add test rules
    fireEvent.click(screen.getByText('Add Test Rules'));

    // Select HMI and evaluate
    fireEvent.change(screen.getByLabelText('Device Type:'), {
      target: { value: 'PLC' }
    });
    fireEvent.click(screen.getByText('Evaluate Rules'));

    // Clear rules
    fireEvent.click(screen.getByText('Clear Rules'));

    // Evaluate again - should reset to default state
    fireEvent.click(screen.getByText('Evaluate Rules'));

    // All fields should be visible by default
    expect(screen.getByText('display_size visible: true')).toBeInTheDocument();
    expect(screen.getByText('ip_address visible: true')).toBeInTheDocument();
  });

  describe('ConditionalRuleBuilder', () => {
    it('should build simple equality rules', () => {
      const rule = ConditionalRuleBuilder
        .create('test_rule')
        .when('field1', '==', 'value1')
        .show('field2', 'field3')
        .build();

      expect(rule.id).toBe('test_rule');
      expect(rule.condition).toEqual({ '==': [{ var: 'field1' }, 'value1'] });
      expect(rule.actions).toHaveLength(1);
      expect(rule.actions[0].type).toBe('show');
      expect(rule.actions[0].targetFields).toEqual(['field2', 'field3']);
    });

    it('should build complex conditions with multiple actions', () => {
      const rule = ConditionalRuleBuilder
        .create('complex_rule')
        .when('status', '==', 'active')
        .show('details')
        .require('name')
        .disable('archive_button')
        .withPriority(10)
        .build();

      expect(rule.actions).toHaveLength(3);
      expect(rule.actions[0].type).toBe('show');
      expect(rule.actions[1].type).toBe('require');
      expect(rule.actions[2].type).toBe('disable');
      expect(rule.priority).toBe(10);
    });

    it('should build empty condition rules', () => {
      const rule = ConditionalRuleBuilder
        .create('empty_rule')
        .whenEmpty('optional_field')
        .hide('dependent_field')
        .build();

      expect(rule.condition).toEqual({ empty: { var: 'optional_field' } });
    });

    it('should build not empty condition rules', () => {
      const rule = ConditionalRuleBuilder
        .create('not_empty_rule')
        .whenNotEmpty('required_field')
        .show('additional_options')
        .build();

      expect(rule.condition).toEqual({ not: { empty: { var: 'required_field' } } });
    });

    it('should handle OR conditions', () => {
      const condition1 = { '==': [{ var: 'type' }, 'A'] };
      const condition2 = { '==': [{ var: 'type' }, 'B'] };

      const rule = ConditionalRuleBuilder
        .create('or_rule')
        .whenAny(condition1, condition2)
        .show('type_specific_field')
        .build();

      expect(rule.condition).toEqual({ or: [condition1, condition2] });
    });

    it('should handle AND conditions', () => {
      const condition1 = { '==': [{ var: 'enabled' }, true] };
      const condition2 = { '!=': [{ var: 'status' }, 'disabled'] };

      const rule = ConditionalRuleBuilder
        .create('and_rule')
        .whenAll(condition1, condition2)
        .show('controls')
        .build();

      expect(rule.condition).toEqual({ and: [condition1, condition2] });
    });
  });

  describe('CommonRules', () => {
    it('should create showWhenEquals rule', () => {
      const rule = CommonRules.showWhenEquals('device_type', 'HMI', 'display_options');

      expect(rule.condition).toEqual({ '==': [{ var: 'device_type' }, 'HMI'] });
      expect(rule.actions[0].type).toBe('show');
      expect(rule.actions[0].targetFields).toEqual(['display_options']);
    });

    it('should create hideWhenEmpty rule', () => {
      const rule = CommonRules.hideWhenEmpty('optional_field', 'dependent_field');

      expect(rule.condition).toEqual({ empty: { var: 'optional_field' } });
      expect(rule.actions[0].type).toBe('hide');
      expect(rule.actions[0].targetFields).toEqual(['dependent_field']);
    });

    it('should create requireWhenNotEmpty rule', () => {
      const rule = CommonRules.requireWhenNotEmpty('parent_field', 'child_field');

      expect(rule.condition).toEqual({ not: { empty: { var: 'parent_field' } } });
      expect(rule.actions[0].type).toBe('require');
      expect(rule.actions[0].targetFields).toEqual(['child_field']);
    });

    it('should create disableWhenChecked rule', () => {
      const rule = CommonRules.disableWhenChecked('auto_mode', 'manual_controls');

      expect(rule.condition).toEqual({ '==': [{ var: 'auto_mode' }, true] });
      expect(rule.actions[0].type).toBe('disable');
      expect(rule.actions[0].targetFields).toEqual(['manual_controls']);
    });
  });
});