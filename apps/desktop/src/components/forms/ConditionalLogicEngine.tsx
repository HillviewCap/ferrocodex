import React, { createContext, useContext, useState, ReactNode } from 'react';

// JSON Logic implementation for conditional rules
interface ConditionalRule {
  id: string;
  condition: JsonLogicExpression;
  actions: ConditionalAction[];
  priority?: number;
}

interface ConditionalAction {
  type: 'show' | 'hide' | 'require' | 'disable' | 'setValue';
  targetFields: string[];
  value?: any; // For setValue action
}

interface JsonLogicExpression {
  [operator: string]: any;
}

interface ConditionalLogicState {
  hiddenFields: Set<string>;
  disabledFields: Set<string>;
  requiredFields: Set<string>;
  fieldValues: Record<string, any>;
}

interface ConditionalLogicContextType {
  state: ConditionalLogicState;
  evaluateRules: (formValues: Record<string, any>) => void;
  isFieldVisible: (fieldName: string) => boolean;
  isFieldDisabled: (fieldName: string) => boolean;
  isFieldRequired: (fieldName: string) => boolean;
  addRules: (rules: ConditionalRule[]) => void;
  clearRules: () => void;
}

const ConditionalLogicContext = createContext<ConditionalLogicContextType | null>(null);

interface ConditionalLogicProviderProps {
  children: ReactNode;
  initialRules?: ConditionalRule[];
}

export const ConditionalLogicProvider: React.FC<ConditionalLogicProviderProps> = ({
  children,
  initialRules = []
}) => {
  const [rules, setRules] = useState<ConditionalRule[]>(initialRules);
  const [state, setState] = useState<ConditionalLogicState>({
    hiddenFields: new Set(),
    disabledFields: new Set(),
    requiredFields: new Set(),
    fieldValues: {}
  });

  // JSON Logic implementation
  const evaluateJsonLogic = (expression: JsonLogicExpression, data: Record<string, any>): any => {
    if (typeof expression !== 'object' || expression === null) {
      return expression;
    }

    const operator = Object.keys(expression)[0];
    const operand = expression[operator];

    switch (operator) {
      case 'var':
        return data[operand] !== undefined ? data[operand] : null;

      case '==':
      case '===':
        return evaluateJsonLogic(operand[0], data) === evaluateJsonLogic(operand[1], data);

      case '!=':
      case '!==':
        return evaluateJsonLogic(operand[0], data) !== evaluateJsonLogic(operand[1], data);

      case '>':
        return evaluateJsonLogic(operand[0], data) > evaluateJsonLogic(operand[1], data);

      case '>=':
        return evaluateJsonLogic(operand[0], data) >= evaluateJsonLogic(operand[1], data);

      case '<':
        return evaluateJsonLogic(operand[0], data) < evaluateJsonLogic(operand[1], data);

      case '<=':
        return evaluateJsonLogic(operand[0], data) <= evaluateJsonLogic(operand[1], data);

      case 'and':
        return operand.every((condition: JsonLogicExpression) => 
          evaluateJsonLogic(condition, data)
        );

      case 'or':
        return operand.some((condition: JsonLogicExpression) => 
          evaluateJsonLogic(condition, data)
        );

      case 'not':
        return !evaluateJsonLogic(operand, data);

      case 'in':
        const value = evaluateJsonLogic(operand[0], data);
        const array = evaluateJsonLogic(operand[1], data);
        return Array.isArray(array) ? array.includes(value) : false;

      case 'contains':
        const container = evaluateJsonLogic(operand[0], data);
        const item = evaluateJsonLogic(operand[1], data);
        if (Array.isArray(container)) {
          return container.includes(item);
        }
        if (typeof container === 'string') {
          return container.includes(String(item));
        }
        return false;

      case 'if':
        const condition = evaluateJsonLogic(operand[0], data);
        return condition ? 
          evaluateJsonLogic(operand[1], data) : 
          (operand[2] !== undefined ? evaluateJsonLogic(operand[2], data) : null);

      case 'empty':
        const val = evaluateJsonLogic(operand, data);
        return val === null || val === undefined || val === '' || 
               (Array.isArray(val) && val.length === 0);

      case 'count':
        const arr = evaluateJsonLogic(operand, data);
        return Array.isArray(arr) ? arr.length : 0;

      case '+':
        if (Array.isArray(operand)) {
          return operand.reduce((sum, op) => sum + evaluateJsonLogic(op, data), 0);
        }
        return evaluateJsonLogic(operand, data);

      case '-':
        if (Array.isArray(operand)) {
          const values = operand.map(op => evaluateJsonLogic(op, data));
          return values.reduce((diff, val, index) => index === 0 ? val : diff - val);
        }
        return -evaluateJsonLogic(operand, data);

      case '*':
        if (Array.isArray(operand)) {
          return operand.reduce((product, op) => product * evaluateJsonLogic(op, data), 1);
        }
        return evaluateJsonLogic(operand, data);

      case '/':
        if (Array.isArray(operand)) {
          const values = operand.map(op => evaluateJsonLogic(op, data));
          return values.reduce((quotient, val, index) => index === 0 ? val : quotient / val);
        }
        return evaluateJsonLogic(operand, data);

      case '%':
        const dividend = evaluateJsonLogic(operand[0], data);
        const divisor = evaluateJsonLogic(operand[1], data);
        return dividend % divisor;

      default:
        console.warn(`Unknown JSON Logic operator: ${operator}`);
        return false;
    }
  };

  const evaluateRules = (formValues: Record<string, any>) => {
    const newState: ConditionalLogicState = {
      hiddenFields: new Set(),
      disabledFields: new Set(),
      requiredFields: new Set(),
      fieldValues: formValues
    };

    // Sort rules by priority (higher priority first)
    const sortedRules = [...rules].sort((a, b) => (b.priority || 0) - (a.priority || 0));

    sortedRules.forEach(rule => {
      try {
        const conditionResult = evaluateJsonLogic(rule.condition, formValues);
        
        if (conditionResult) {
          rule.actions.forEach(action => {
            action.targetFields.forEach(fieldName => {
              switch (action.type) {
                case 'hide':
                  newState.hiddenFields.add(fieldName);
                  break;
                case 'show':
                  newState.hiddenFields.delete(fieldName);
                  break;
                case 'disable':
                  newState.disabledFields.add(fieldName);
                  break;
                case 'require':
                  newState.requiredFields.add(fieldName);
                  break;
                case 'setValue':
                  // This would need to be handled by the form component
                  // as we can't modify form values from here
                  break;
              }
            });
          });
        }
      } catch (error) {
        console.error(`Error evaluating rule ${rule.id}:`, error);
      }
    });

    setState(newState);
  };

  const isFieldVisible = (fieldName: string): boolean => {
    return !state.hiddenFields.has(fieldName);
  };

  const isFieldDisabled = (fieldName: string): boolean => {
    return state.disabledFields.has(fieldName);
  };

  const isFieldRequired = (fieldName: string): boolean => {
    return state.requiredFields.has(fieldName);
  };

  const addRules = (newRules: ConditionalRule[]) => {
    setRules(prevRules => [...prevRules, ...newRules]);
  };

  const clearRules = () => {
    setRules([]);
    setState({
      hiddenFields: new Set(),
      disabledFields: new Set(),
      requiredFields: new Set(),
      fieldValues: {}
    });
  };

  const contextValue: ConditionalLogicContextType = {
    state,
    evaluateRules,
    isFieldVisible,
    isFieldDisabled,
    isFieldRequired,
    addRules,
    clearRules
  };

  return (
    <ConditionalLogicContext.Provider value={contextValue}>
      {children}
    </ConditionalLogicContext.Provider>
  );
};

export const useConditionalLogic = (): ConditionalLogicContextType => {
  const context = useContext(ConditionalLogicContext);
  if (!context) {
    throw new Error('useConditionalLogic must be used within a ConditionalLogicProvider');
  }
  return context;
};

// Helper functions for creating common conditional rules
export class ConditionalRuleBuilder {
  private rule: Partial<ConditionalRule>;

  constructor(id: string) {
    this.rule = { id, actions: [] };
  }

  static create(id: string): ConditionalRuleBuilder {
    return new ConditionalRuleBuilder(id);
  }

  // Condition builders
  when(field: string, operator: '==' | '!=' | '>' | '>=' | '<' | '<=' | 'in' | 'contains', value: any): ConditionalRuleBuilder {
    switch (operator) {
      case '==':
        this.rule.condition = { '==': [{ var: field }, value] };
        break;
      case '!=':
        this.rule.condition = { '!=': [{ var: field }, value] };
        break;
      case '>':
        this.rule.condition = { '>': [{ var: field }, value] };
        break;
      case '>=':
        this.rule.condition = { '>=': [{ var: field }, value] };
        break;
      case '<':
        this.rule.condition = { '<': [{ var: field }, value] };
        break;
      case '<=':
        this.rule.condition = { '<=': [{ var: field }, value] };
        break;
      case 'in':
        this.rule.condition = { in: [{ var: field }, value] };
        break;
      case 'contains':
        this.rule.condition = { contains: [{ var: field }, value] };
        break;
    }
    return this;
  }

  whenEmpty(field: string): ConditionalRuleBuilder {
    this.rule.condition = { empty: { var: field } };
    return this;
  }

  whenNotEmpty(field: string): ConditionalRuleBuilder {
    this.rule.condition = { not: { empty: { var: field } } };
    return this;
  }

  whenAny(...conditions: JsonLogicExpression[]): ConditionalRuleBuilder {
    this.rule.condition = { or: conditions };
    return this;
  }

  whenAll(...conditions: JsonLogicExpression[]): ConditionalRuleBuilder {
    this.rule.condition = { and: conditions };
    return this;
  }

  // Action builders
  show(...fields: string[]): ConditionalRuleBuilder {
    this.rule.actions!.push({ type: 'show', targetFields: fields });
    return this;
  }

  hide(...fields: string[]): ConditionalRuleBuilder {
    this.rule.actions!.push({ type: 'hide', targetFields: fields });
    return this;
  }

  require(...fields: string[]): ConditionalRuleBuilder {
    this.rule.actions!.push({ type: 'require', targetFields: fields });
    return this;
  }

  disable(...fields: string[]): ConditionalRuleBuilder {
    this.rule.actions!.push({ type: 'disable', targetFields: fields });
    return this;
  }

  setValue(field: string, value: any): ConditionalRuleBuilder {
    this.rule.actions!.push({ type: 'setValue', targetFields: [field], value });
    return this;
  }

  withPriority(priority: number): ConditionalRuleBuilder {
    this.rule.priority = priority;
    return this;
  }

  build(): ConditionalRule {
    if (!this.rule.condition) {
      throw new Error('Rule must have a condition');
    }
    if (!this.rule.actions || this.rule.actions.length === 0) {
      throw new Error('Rule must have at least one action');
    }
    return this.rule as ConditionalRule;
  }
}

// Common conditional rule patterns
export const CommonRules = {
  // Show field B when field A equals specific value
  showWhenEquals: (fieldA: string, value: any, ...fieldsToShow: string[]): ConditionalRule => 
    ConditionalRuleBuilder
      .create(`show_${fieldsToShow.join('_')}_when_${fieldA}_equals_${value}`)
      .when(fieldA, '==', value)
      .show(...fieldsToShow)
      .build(),

  // Hide field B when field A is empty
  hideWhenEmpty: (fieldA: string, ...fieldsToHide: string[]): ConditionalRule =>
    ConditionalRuleBuilder
      .create(`hide_${fieldsToHide.join('_')}_when_${fieldA}_empty`)
      .whenEmpty(fieldA)
      .hide(...fieldsToHide)
      .build(),

  // Require field B when field A has value
  requireWhenNotEmpty: (fieldA: string, ...fieldsToRequire: string[]): ConditionalRule =>
    ConditionalRuleBuilder
      .create(`require_${fieldsToRequire.join('_')}_when_${fieldA}_not_empty`)
      .whenNotEmpty(fieldA)
      .require(...fieldsToRequire)
      .build(),

  // Disable field B when field A is checked
  disableWhenChecked: (checkboxField: string, ...fieldsToDisable: string[]): ConditionalRule =>
    ConditionalRuleBuilder
      .create(`disable_${fieldsToDisable.join('_')}_when_${checkboxField}_checked`)
      .when(checkboxField, '==', true)
      .disable(...fieldsToDisable)
      .build()
};

export type { ConditionalRule, ConditionalAction, JsonLogicExpression };