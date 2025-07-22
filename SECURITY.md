# Security Policy

## Supported Versions

The following versions of Ferrocodex are currently supported with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take the security of Ferrocodex seriously, especially given its use in operational technology (OT) environments. If you discover a security vulnerability, please follow these steps:

### How to Report

1. **DO NOT** open a public issue
2. Email security vulnerabilities to: <zach@transformitadvisers.com>
3. Include the following information:
   - Description of the vulnerability
   - Steps to reproduce the issue
   - Potential impact
   - Any proof-of-concept code (if applicable)

### What to Expect

- **Initial Response**: We aim to acknowledge receipt within 48 hours
- **Investigation**: We'll investigate and provide updates within 7 days
- **Resolution Timeline**: Critical vulnerabilities will be addressed as soon as possible
- **Disclosure**: We'll coordinate disclosure timing with you

### Security Measures

Ferrocodex implements multiple security layers:

- AES-256 encryption for all stored data - Not implemented in the Alpha Build
- Role-based access control (RBAC)
- Session token validation
- Rate limiting on sensitive operations
- Input validation on frontend and backend
- Secure offline-first architecture

### Scope

The following are in scope for security reports:

- Authentication/authorization bypasses
- Data encryption vulnerabilities
- SQL injection or other injection attacks
- Cross-site scripting (XSS) in the desktop app
- Privilege escalation
- Information disclosure
- Any vulnerability affecting OT configuration integrity

### Recognition

We appreciate responsible disclosure and will acknowledge security researchers who help improve Ferrocodex's security in our release notes (with permission).
