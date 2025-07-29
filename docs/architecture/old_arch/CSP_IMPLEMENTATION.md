# Content Security Policy (CSP) Implementation

## Overview
This document describes the Content Security Policy implementation for Ferrocodex, addressing issue #52.

## CSP Configuration

### Primary CSP (Tauri Configuration)
The main CSP is configured in `apps/desktop/src-tauri/tauri.conf.json`:

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self' tauri://localhost; object-src 'none'; base-uri 'self'; form-action 'self'"
    }
  }
}
```

### Fallback CSP (HTML Meta Tag)
A fallback CSP is also configured in `apps/desktop/index.html` as a meta tag with identical policy.

## CSP Directives Explained

| Directive | Value | Justification |
|-----------|-------|---------------|
| `default-src` | `'self'` | Default policy - only allow resources from same origin |
| `script-src` | `'self'` | Only allow scripts from same origin (no inline scripts) |
| `style-src` | `'self' 'unsafe-inline'` | Allow same-origin styles + inline styles (required for React components) |
| `img-src` | `'self' data: https:` | Allow same-origin images, data URLs, and HTTPS images |
| `font-src` | `'self'` | Only allow fonts from same origin |
| `connect-src` | `'self' tauri://localhost` | Allow connections to self and Tauri backend |
| `object-src` | `'none'` | Block all object/embed/applet elements |
| `base-uri` | `'self'` | Restrict base tag to same origin |
| `form-action` | `'self'` | Only allow form submissions to same origin |

## Security Relaxations

### `unsafe-inline` for Styles
**Required because:** The React codebase extensively uses inline styles via JSX `style` props throughout components.

**Files affected:** 40+ component files use inline styles
**Risk mitigation:** 
- Inline styles are less risky than inline scripts
- All style content is controlled by our React components
- No user-generated content is inserted into styles

**Future improvement:** Consider migrating to CSS modules or styled-components to eliminate `unsafe-inline`.

## CSP Violation Reporting

### Implementation
- CSP violations are captured via `SecurityPolicyViolationEvent` listener
- Violations are logged to console with structured format
- Reporting utility located in `src/utils/cspReporting.ts`
- Initialized in `src/main.tsx`

### Future Enhancements
- Integration with audit logging system
- Centralized violation reporting to backend
- Alert mechanism for critical violations

## Testing Instructions

### Browser Developer Tools
1. Start the application: `npm run dev`
2. Open browser developer tools (F12)
3. Check Console tab for CSP violation warnings
4. Check Security tab for CSP policy details
5. Verify Network tab shows CSP header in responses

### Manual Testing
1. Attempt to execute inline JavaScript (should be blocked)
2. Verify images from external sources load correctly
3. Test that Tauri IPC communication works
4. Confirm all existing functionality remains intact

## Files Modified

1. `apps/desktop/src-tauri/tauri.conf.json` - Primary CSP configuration
2. `apps/desktop/index.html` - Fallback CSP meta tag
3. `apps/desktop/src/utils/cspReporting.ts` - Violation reporting (new file)
4. `apps/desktop/src/main.tsx` - Initialize CSP reporting

## Security Impact

### Before Implementation
- ❌ No XSS protection via CSP
- ❌ Vulnerable to code injection attacks
- ❌ No resource loading restrictions

### After Implementation
- ✅ Strong XSS protection via CSP
- ✅ Restricted resource loading to trusted sources
- ✅ Violation detection and reporting
- ✅ Defense-in-depth security layer

## Compliance
This implementation addresses:
- Issue #52: Missing Content Security Policy Implementation
- Security architecture requirements
- OWASP security best practices for desktop applications

## Monitoring
CSP violations are logged and can be monitored for:
- Attempted XSS attacks
- Unexpected resource loading
- Configuration issues
- Security policy effectiveness