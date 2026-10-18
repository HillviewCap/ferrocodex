Security Best Practices
=======================

This guide covers the security features of Ferrocodex and recommended practices for protecting your industrial configurations.

.. contents:: Table of Contents
   :local:
   :depth: 2

Security Architecture
---------------------

Core Security Features
^^^^^^^^^^^^^^^^^^^^^^

* **AES-256-GCM Encryption**: Configuration content, firmware files, and vault secrets encrypted at rest
* **Bcrypt Password Hashing**: Industry-standard password protection
* **Identity Vault**: Secure credential storage with rotation tracking
* **Role-Based Access Control**: Granular permission system
* **Audit Trail**: Immutable activity logging
* **Session Management**: Secure token-based authentication
* **Input Validation**: Protection against injection attacks
* **Asset Naming Validation**: Enforced naming pattern for assets
* **File Integrity Verification**: SHA-256 checksums for all uploads
* **Metadata Validation**: JSON schema validation for custom fields

Data Protection
^^^^^^^^^^^^^^^

**Encryption at Rest:**

* Configuration file content is encrypted with AES-256-GCM before it is stored in the database
* Firmware files are encrypted with AES-256-GCM before they are written to disk
* Vault secrets are encrypted with AES-256-GCM
* All of these use one random 256-bit data key, which is never written to disk in plaintext. Each user holds a copy wrapped with a key derived from their password (Argon2id), and one more copy is wrapped with the recovery key shown at setup. Copying the database without a password or the recovery key does not expose the data key.
* User passwords are stored as bcrypt hashes

The SQLite database file itself is not encrypted as a whole. Asset names, metadata, audit entries, and other records are stored in plaintext. Protect the application data directory with operating-system permissions and full-disk encryption.

**Encryption in Transit:**

* Ferrocodex does not transmit data over the network
* Communication between the user interface and the backend stays inside the application process
* Recovery bundles contain the configuration and firmware files in plaintext so they can be loaded onto devices; protect them in transit and at rest
* Vault secrets in vault exports and recovery bundles are re-encrypted with an export passphrase chosen at export time, and the same passphrase is needed to import them

Access Control
--------------

User Roles
^^^^^^^^^^

**Administrator Role:**

* Create and manage users
* View all audit logs
* System configuration
* Import/export capabilities
* Access all assets and configurations

**Engineer Role:**

* Manage assets and configurations
* Create branches
* View own audit entries
* Cannot manage users
* Cannot access system settings

Password Policies
^^^^^^^^^^^^^^^^^

**Requirements:**

* Minimum 8 characters
* Must include letters and numbers
* No common passwords allowed
* Password history enforcement
* Forced change on first login

**Best Practices:**

1. Use strong, unique passwords
2. Enable password managers
3. Regular password rotation
4. No password sharing
5. Avoid pattern-based passwords

Session Security
^^^^^^^^^^^^^^^^

* **Token-based authentication**
* **Automatic session timeout**
* **Concurrent session limits**
* **Session invalidation on logout**
* **Activity monitoring**

Asset Naming Security
---------------------

Naming Pattern
^^^^^^^^^^^^^^

Ferrocodex enforces a strict naming pattern to prevent injection and path-handling vulnerabilities:

**Naming Pattern Requirements:**

.. code-block:: text

   ^[A-Z0-9][A-Z0-9_-]{2,49}$

**Security Benefits:**

* **Path Traversal Prevention**: No ``../`` or special characters
* **Command Injection Protection**: No shell metacharacters
* **SQL Injection Prevention**: Restricted character set
* **Cross-Site Scripting (XSS) Prevention**: No HTML/JavaScript characters
* **Unicode Attack Prevention**: ASCII-only enforcement

**Reserved Name Blocking:**

The following Windows reserved names are automatically blocked:

* System devices: ``CON``, ``PRN``, ``AUX``, ``NUL``
* Serial ports: ``COM1`` through ``COM9``
* Parallel ports: ``LPT1`` through ``LPT9``

**File Upload Security:**

* **Filename Sanitization**: Automatic cleaning of uploaded filenames
* **Extension Validation**: Checked against security policies
* **Path Component Validation**: No directory traversal sequences
* **Length Limits**: Maximum 255 characters enforced
* **SHA-256 Verification**: Integrity checking for all uploads

Metadata Security
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Field Validation:**

* **Input Sanitization**: All metadata inputs sanitized
* **Type Enforcement**: Strict type checking for fields
* **Pattern Validation**: Regex patterns for text fields
* **Range Validation**: Min/max for numeric fields
* **JSON Schema Validation**: Structure validation for complex data

**SQL Injection Prevention:**

* **Parameterized Queries**: All metadata queries use parameters
* **Input Escaping**: Special characters properly escaped
* **Type Casting**: Explicit type conversion
* **Query Validation**: No dynamic SQL generation

**Cross-Site Scripting (XSS) Prevention:**

* **Output Encoding**: HTML entity encoding for display
* **Content Security Policy**: Strict CSP headers
* **Input Filtering**: Remove script tags and events
* **Template Sanitization**: Safe rendering of metadata

Security Classification System
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Assets can be tagged with security classifications affecting system behavior:

**Classification Levels:**

1. **Public**: No restrictions
2. **Internal**: Company-wide access
3. **Confidential**: Department-level access
4. **Secret**: Need-to-know basis
5. **Top Secret**: Maximum security

**Classification Enforcement:**

* **Search Filtering**: Results filtered by user clearance
* **Export Restrictions**: Higher classifications require approval
* **Audit Detail**: More logging for sensitive assets
* **Access Control**: Automatic permission adjustment
* **Compliance Tracking**: Classification-based reporting

Physical Security
-----------------

Workstation Security
^^^^^^^^^^^^^^^^^^^^

1. **Lock workstations** when unattended
2. **Encrypted hard drives** recommended
3. **Antivirus software** up-to-date
4. **Operating system** patches current
5. **Firewall** enabled

Installation Security
^^^^^^^^^^^^^^^^^^^^^

* Install only from official sources
* Verify digital signatures
* Restrict installation privileges
* Document installation locations
* Control application access

Operational Security
--------------------

Audit Log Management
^^^^^^^^^^^^^^^^^^^^

**Regular Review:**

1. Check failed login attempts
2. Monitor configuration changes
3. Verify user activities
4. Investigate anomalies
5. Export logs for archival

**Retention Policies:**

* Determine retention requirements
* Regular log exports
* Secure archive storage
* Compliance documentation

Backup and Recovery
^^^^^^^^^^^^^^^^^^^

**Backup Strategy:**

1. **Regular Exports**: Schedule periodic full exports
2. **Secure Storage**: Encrypt backup files
3. **Offsite Copies**: Store backups separately
4. **Test Recovery**: Verify backup integrity
5. **Document Process**: Clear recovery procedures

**Recovery Planning:**

* Administrator account continuity (passwords cannot be recovered)
* User account recovery
* Configuration restoration
* Audit trail preservation
* Business continuity planning

Network Security
----------------

Air-Gapped Operations
^^^^^^^^^^^^^^^^^^^^^

Ferrocodex is designed for air-gapped environments:

* No internet connectivity required
* No automatic updates
* No telemetry or analytics
* Complete offline functionality
* Manual update process

If Network Connected
^^^^^^^^^^^^^^^^^^^^

If using Ferrocodex on connected systems:

1. **Isolate on separate VLAN**
2. **Firewall rules** restricting access
3. **No internet access** from application
4. **Monitor network activity**
5. **Regular security scans**

Identity Vault Security
-----------------------

The Identity Vault protects credentials with encryption, access permissions, and audit logging.

Vault Encryption Architecture
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

.. figure:: _static/diagrams/vault-encryption-layers.svg
   :alt: Multi-layer encryption architecture diagram
   :align: center
   :width: 600px

   *Vault encryption layers showing defense in depth*

**Encryption Layers:**

1. **Data Key**: A random 256-bit key, generated when the administrator account is created, encrypts every secret
2. **Key Wrapping**: The data key is stored only in wrapped form, under a key derived with Argon2id from each user's password and under the recovery key. It is unwrapped into memory at login and discarded at logout
3. **Data Encryption**: All secrets are encrypted with AES-256-GCM
4. **Access Control**: Vault permissions are checked before any secret is decrypted

**Encryption Details:**

* **Algorithm**: AES-256 in GCM mode
* **Key Size**: 256-bit encryption keys
* **Nonce Generation**: Unique random nonce for each encryption operation
* **Authentication**: GCM provides built-in authentication of the ciphertext

Password Security Features
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Password Generation:**

* Cryptographically secure random generation
* Configurable complexity requirements
* Entropy calculation and display
* No predictable patterns
* Unique passwords enforced across vaults

**Password Storage:**

* Never stored in plaintext
* Encrypted immediately on entry
* Memory cleared after use
* No password logging
* Secure clipboard operations

**Password History:**

* Previous passwords retained encrypted
* Prevents reuse across rotations
* Configurable history depth
* Audit trail for all changes
* Compliance reporting support

Credential Rotation Security
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Rotation Policies:**

1. **By Asset Criticality**:

   * Critical Assets: 30-day rotation
   * Standard Assets: 60-day rotation
   * Low-Risk Assets: 90-day rotation

2. **Emergency Rotation**:

   * Immediate rotation capability
   * Batch rotation for incidents
   * Documented reason required
   * Notification system
   * Audit priority flagging

**Compliance Tracking:**

* Automated rotation reminders
* Overdue password alerts
* Compliance dashboard
* Export capabilities for audits
* Integration with audit system

Vault Access Control
^^^^^^^^^^^^^^^^^^^^

**Permission Model:**

1. **Granular Permissions**:

   * **Read**: View vault contents
   * **Write**: Modify secrets
   * **Export**: Include in bundles
   * **Share**: Grant permissions

2. **Access Patterns**:

   * Default deny principle
   * Explicit grant required
   * No permission inheritance
   * Time-limited access option

**Access Security:**

* All access attempts logged
* Failed access alerts
* Session-based access
* No cached credentials
* Automatic permission expiry

Standalone Credentials Security
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Isolation Features:**

* Separate from asset vaults
* Category-based organization
* Independent access control
* Dedicated audit trail
* No cross-contamination

**Best Practices:**

1. Use categories for access control
2. Regular access reviews
3. Separate production/test credentials
4. Document credential purpose
5. Implement rotation schedules

Export and Import Security
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Export Security:**

1. **Vault Data in Exports**:

   * Optional inclusion
   * Secrets re-encrypted with an export passphrase (Argon2id-derived key); share the passphrase separately from the file
   * Warning dialogs
   * Audit logging
   * Checksum verification

2. **Secure Handling**:

   * Encrypt export files
   * Secure transport methods
   * Limited distribution
   * Retention policies
   * Destruction procedures

**Import Security:**

* Verification before import
* Conflict resolution options
* Audit trail maintenance
* Permission preservation
* Rollback capability

Security Monitoring
^^^^^^^^^^^^^^^^^^^

**Vault-Specific Monitoring:**

1. **Access Monitoring**:

   * Real-time access logs
   * Unusual access patterns
   * Failed access attempts
   * Permission changes
   * Export operations

2. **Compliance Monitoring**:

   * Rotation compliance
   * Password strength
   * Access reviews
   * Policy violations
   * Trend analysis

**Alert Conditions:**

* Multiple failed access attempts
* Unexpected permission grants
* Overdue rotations
* Weak passwords detected
* Unusual export activity

Incident Response for Vaults
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Credential Compromise:**

1. **Immediate Actions**:

   * Identify affected credentials
   * Emergency rotation
   * Access review
   * Alert affected users
   * Document incident

2. **Investigation**:

   * Review access logs
   * Check export history
   * Analyze permission changes
   * Identify exposure window
   * Determine impact scope

3. **Remediation**:

   * Complete credential rotation
   * Update access controls
   * Security awareness training
   * Policy updates
   * Monitoring enhancements

Best Practices for Vault Security
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Organizational Policies:**

1. **Access Management**:

   * Principle of least privilege
   * Regular access reviews
   * Time-limited permissions
   * Segregation of duties
   * Documented approval process

2. **Password Policies**:

   * Minimum complexity requirements
   * Rotation schedules by risk
   * No password sharing
   * Unique passwords per system
   * Emergency rotation procedures

3. **Operational Security**:

   * Regular security training
   * Incident response drills
   * Compliance audits
   * Policy enforcement
   * Continuous improvement

**Technical Controls:**

1. **Preventive**:

   * Strong encryption
   * Access controls
   * Password policies
   * Input validation
   * Secure defaults

2. **Detective**:

   * Logging of all operations
   * Real-time monitoring
   * Anomaly detection
   * Compliance tracking
   * Regular audits

3. **Corrective**:

   * Emergency rotation
   * Access revocation
   * Incident response
   * Recovery procedures
   * Security updates

Incident Response
-----------------

Security Incident Types
^^^^^^^^^^^^^^^^^^^^^^^

* Unauthorized access attempts
* Lost or stolen devices
* Compromised passwords
* Suspicious audit entries
* Data corruption

Response Procedures
^^^^^^^^^^^^^^^^^^^

1. **Immediate Actions:**

   * Disable compromised accounts
   * Change affected passwords
   * Review audit logs
   * Document incident details

2. **Investigation:**

   * Determine scope of incident
   * Identify affected assets
   * Review access patterns
   * Check configuration integrity

3. **Remediation:**

   * Reset compromised credentials
   * Restore from clean backups
   * Update security measures
   * User security training

4. **Documentation:**

   * Complete incident report
   * Update security procedures
   * Notify stakeholders
   * Compliance reporting

Compliance and Standards
------------------------

Industry Standards
^^^^^^^^^^^^^^^^^^

Ferrocodex helps meet requirements for:

* **NERC CIP**: Critical infrastructure protection
* **IEC 62443**: Industrial network security
* **ISO 27001**: Information security management
* **NIST Framework**: Cybersecurity guidelines

Compliance Features
^^^^^^^^^^^^^^^^^^^

* Immutable audit trails
* Role-based access control
* Encryption standards
* Change management
* Configuration control

Security Checklist
------------------

Daily Tasks
^^^^^^^^^^^

- [ ] Review recent audit logs
- [ ] Check active user sessions
- [ ] Verify backup completion
- [ ] Monitor failed logins
- [ ] Check vault access attempts
- [ ] Review rotation compliance dashboard

Weekly Tasks
^^^^^^^^^^^^

- [ ] Export audit logs
- [ ] Review user permissions
- [ ] Check configuration changes
- [ ] Update security notices
- [ ] Review vault permissions
- [ ] Check overdue password rotations
- [ ] Analyze vault access patterns

Monthly Tasks
^^^^^^^^^^^^^

- [ ] Full security audit
- [ ] Password policy review
- [ ] Backup restoration test
- [ ] Security training
- [ ] Update documentation
- [ ] Vault access review
- [ ] Rotation compliance report
- [ ] Emergency response drill

Security Contact
----------------

Ferrocodex is intended for use in industrial environments, and security issues are treated accordingly.

* To report a vulnerability, email security@ferrocodex.com with details
* Do not open public issues for vulnerabilities
* General questions and non-sensitive bug reports can go to the `GitHub issue tracker <https://github.com/HillviewCap/ferrocodex/issues>`_
* Document all communications

Security is everyone's responsibility. Follow these practices to maintain the integrity of your industrial configuration management system.