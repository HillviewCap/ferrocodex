import { message } from 'antd';

export interface ValidationRule {
  type: 'required' | 'minLength' | 'maxLength' | 'min' | 'max' | 'pattern' | 'email' | 'url' | 'custom';
  value?: any;
  message?: string;
  validator?: (value: any, allValues: Record<string, any>) => boolean | string;
}

export interface FieldValidationResult {
  isValid: boolean;
  error?: string;
}

export interface FormValidationResult {
  isValid: boolean;
  errors: Record<string, string>;
  warnings: string[];
}

export class FormValidator {
  private static emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  private static urlPattern = /^https?:\/\/.+/;

  static validateField(
    value: any, 
    rules: ValidationRule[], 
    allValues: Record<string, any> = {}
  ): FieldValidationResult {
    for (const rule of rules) {
      const result = this.validateSingleRule(value, rule, allValues);
      if (!result.isValid) {
        return result;
      }
    }
    return { isValid: true };
  }

  static validateSingleRule(
    value: any, 
    rule: ValidationRule, 
    allValues: Record<string, any>
  ): FieldValidationResult {
    const isEmpty = value === null || value === undefined || value === '';

    switch (rule.type) {
      case 'required':
        if (isEmpty) {
          return {
            isValid: false,
            error: rule.message || 'This field is required'
          };
        }
        break;

      case 'minLength':
        if (!isEmpty && typeof value === 'string' && value.length < rule.value) {
          return {
            isValid: false,
            error: rule.message || `Minimum length is ${rule.value} characters`
          };
        }
        break;

      case 'maxLength':
        if (!isEmpty && typeof value === 'string' && value.length > rule.value) {
          return {
            isValid: false,
            error: rule.message || `Maximum length is ${rule.value} characters`
          };
        }
        break;

      case 'min':
        if (!isEmpty && typeof value === 'number' && value < rule.value) {
          return {
            isValid: false,
            error: rule.message || `Minimum value is ${rule.value}`
          };
        }
        break;

      case 'max':
        if (!isEmpty && typeof value === 'number' && value > rule.value) {
          return {
            isValid: false,
            error: rule.message || `Maximum value is ${rule.value}`
          };
        }
        break;

      case 'pattern':
        if (!isEmpty && typeof value === 'string') {
          const regex = new RegExp(rule.value);
          if (!regex.test(value)) {
            return {
              isValid: false,
              error: rule.message || 'Invalid format'
            };
          }
        }
        break;

      case 'email':
        if (!isEmpty && typeof value === 'string' && !this.emailPattern.test(value)) {
          return {
            isValid: false,
            error: rule.message || 'Invalid email address'
          };
        }
        break;

      case 'url':
        if (!isEmpty && typeof value === 'string' && !this.urlPattern.test(value)) {
          return {
            isValid: false,
            error: rule.message || 'Invalid URL format'
          };
        }
        break;

      case 'custom':
        if (rule.validator) {
          const result = rule.validator(value, allValues);
          if (result !== true) {
            return {
              isValid: false,
              error: typeof result === 'string' ? result : (rule.message || 'Validation failed')
            };
          }
        }
        break;
    }

    return { isValid: true };
  }

  static validateForm(
    values: Record<string, any>, 
    fieldRules: Record<string, ValidationRule[]>
  ): FormValidationResult {
    const errors: Record<string, string> = {};
    const warnings: string[] = [];

    for (const [fieldName, rules] of Object.entries(fieldRules)) {
      const fieldValue = values[fieldName];
      const validationResult = this.validateField(fieldValue, rules, values);
      
      if (!validationResult.isValid && validationResult.error) {
        errors[fieldName] = validationResult.error;
      }
    }

    return {
      isValid: Object.keys(errors).length === 0,
      errors,
      warnings
    };
  }

  // Common validation rule builders
  static required(message?: string): ValidationRule {
    return { type: 'required', message };
  }

  static minLength(length: number, message?: string): ValidationRule {
    return { type: 'minLength', value: length, message };
  }

  static maxLength(length: number, message?: string): ValidationRule {
    return { type: 'maxLength', value: length, message };
  }

  static min(value: number, message?: string): ValidationRule {
    return { type: 'min', value, message };
  }

  static max(value: number, message?: string): ValidationRule {
    return { type: 'max', value, message };
  }

  static pattern(regex: string, message?: string): ValidationRule {
    return { type: 'pattern', value: regex, message };
  }

  static email(message?: string): ValidationRule {
    return { type: 'email', message };
  }

  static url(message?: string): ValidationRule {
    return { type: 'url', message };
  }

  static custom(
    validator: (value: any, allValues: Record<string, any>) => boolean | string,
    message?: string
  ): ValidationRule {
    return { type: 'custom', validator, message };
  }
}

// Cross-field validation utilities
export class CrossFieldValidator {
  static mustMatch(fieldA: string, fieldB: string, message?: string): ValidationRule {
    return FormValidator.custom(
      (value, allValues) => {
        return allValues[fieldA] === allValues[fieldB] || 
               (message || `Must match ${fieldB}`);
      }
    );
  }

  static mustBeDifferent(fieldA: string, fieldB: string, message?: string): ValidationRule {
    return FormValidator.custom(
      (value, allValues) => {
        return allValues[fieldA] !== allValues[fieldB] || 
               (message || `Must be different from ${fieldB}`);
      }
    );
  }

  static mustBeGreaterThan(otherField: string, message?: string): ValidationRule {
    return FormValidator.custom(
      (value, allValues) => {
        const otherValue = allValues[otherField];
        return (typeof value === 'number' && typeof otherValue === 'number' && value > otherValue) ||
               (message || `Must be greater than ${otherField}`);
      }
    );
  }

  static mustBeLessThan(otherField: string, message?: string): ValidationRule {
    return FormValidator.custom(
      (value, allValues) => {
        const otherValue = allValues[otherField];
        return (typeof value === 'number' && typeof otherValue === 'number' && value < otherValue) ||
               (message || `Must be less than ${otherField}`);
      }
    );
  }

  static requiredIf(conditionField: string, conditionValue: any, message?: string): ValidationRule {
    return FormValidator.custom(
      (value, allValues) => {
        if (allValues[conditionField] === conditionValue) {
          const isEmpty = value === null || value === undefined || value === '';
          return !isEmpty || (message || 'This field is required');
        }
        return true;
      }
    );
  }
}

// Common industrial/OT field validators
export class IndustrialValidators {
  static ipAddress(message?: string): ValidationRule {
    const ipPattern = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
    return FormValidator.pattern(
      ipPattern.source,
      message || 'Invalid IP address format'
    );
  }

  static macAddress(message?: string): ValidationRule {
    const macPattern = /^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/;
    return FormValidator.pattern(
      macPattern.source,
      message || 'Invalid MAC address format'
    );
  }

  static serialNumber(message?: string): ValidationRule {
    const serialPattern = /^[A-Z0-9\-_]{4,32}$/i;
    return FormValidator.pattern(
      serialPattern.source,
      message || 'Invalid serial number format'
    );
  }

  static modelNumber(message?: string): ValidationRule {
    const modelPattern = /^[A-Z0-9\-_.\/]{2,50}$/i;
    return FormValidator.pattern(
      modelPattern.source,
      message || 'Invalid model number format'
    );
  }

  static vlanId(message?: string): ValidationRule {
    return FormValidator.custom(
      (value) => {
        const num = parseInt(value, 10);
        return (num >= 1 && num <= 4094) || (message || 'VLAN ID must be between 1 and 4094');
      }
    );
  }

  static portNumber(message?: string): ValidationRule {
    return FormValidator.custom(
      (value) => {
        const num = parseInt(value, 10);
        return (num >= 1 && num <= 65535) || (message || 'Port number must be between 1 and 65535');
      }
    );
  }

  static firmwareVersion(message?: string): ValidationRule {
    const versionPattern = /^v?\d+(\.\d+)*(-[a-zA-Z0-9\-_]+)?$/;
    return FormValidator.pattern(
      versionPattern.source,
      message || 'Invalid firmware version format'
    );
  }

  static assetTag(message?: string): ValidationRule {
    const tagPattern = /^[A-Z0-9\-_]{3,20}$/i;
    return FormValidator.pattern(
      tagPattern.source,
      message || 'Asset tag must be 3-20 alphanumeric characters'
    );
  }
}

// Validation message utilities
export class ValidationMessages {
  static showFieldError(fieldName: string, error: string) {
    message.error(`${fieldName}: ${error}`);
  }

  static showFormErrors(errors: Record<string, string>) {
    const errorCount = Object.keys(errors).length;
    if (errorCount === 1) {
      const [field, error] = Object.entries(errors)[0];
      this.showFieldError(field, error);
    } else if (errorCount > 1) {
      message.error(`Form has ${errorCount} validation errors. Please check all fields.`);
    }
  }

  static showValidationSummary(result: FormValidationResult) {
    if (!result.isValid) {
      this.showFormErrors(result.errors);
    }
    
    if (result.warnings.length > 0) {
      result.warnings.forEach(warning => message.warning(warning));
    }
  }
}

