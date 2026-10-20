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
* **Per-User Key Access and Key Rotation**: Each user opens the data key through their own X25519 keypair, and administrators can replace the data key (see :ref:`data-key-rotation`)
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
* Vault secrets are encrypted with AES-256-GCM. A secret's label and username are not: they are stored in plaintext so they can be listed and searched without decrypting anything
* All of these use one random 256-bit data key, which is never written to disk in plaintext. Copying the database without a password or the recovery key does not expose the data key.
* User passwords are stored as bcrypt hashes

The SQLite database file itself is not encrypted as a whole. Asset names, metadata, audit entries, and other records are stored in plaintext. Protect the application data directory with operating-system permissions and full-disk encryption.

**Key Management:**

* **The data key**: One random 256-bit key, generated when the first administrator logs in. It exists in plaintext only in memory, inside a session. It carries a version number (the first key is version 1), and every encrypted item records the version of the key that encrypted it.
* **Per-user keypairs**: Each user has an X25519 keypair. The private key is wrapped (encrypted with AES-256-GCM) under a key derived from the user's login password with Argon2id. The public key is stored in the clear.
* **Public-key wraps**: The data key is sealed to each user's public key (an ephemeral X25519 exchange, HKDF-SHA256, and AES-256-GCM, bound to the user and the key version). At login the user's password opens their private key, which opens their wrap of the active data key. Because sealing needs only the public key, an administrator can give every user the new key during a rotation without knowing any password.
* **Recovery-key wrap**: One more copy of the data key is wrapped under the recovery key shown at setup. Anyone holding the recovery key can reset any account's password, which gives that account a fresh keypair.
* **Lazy migration at login**: Accounts created before keypairs existed still have a copy of the data key wrapped directly under their password. At their next login Ferrocodex opens that copy, creates a keypair, stores the public-key wrap, and deletes the old copy in one transaction. The upgrade is recorded in the audit log. An active user who has not logged in since the upgrade has no keypair, which matters for rotation (see :ref:`data-key-rotation`).
* **Key versions**: Each configuration version, vault secret, standalone credential, and firmware file records the data-key version that encrypted it. Only one version is active. While a rotation runs, the old version is retiring, and it is retired once everything has been re-encrypted.

**Encryption in Transit:**

* Ferrocodex does not transmit data over the network
* Communication between the user interface and the backend stays inside the application process
* Recovery bundles contain the configuration and firmware files in plaintext so they can be loaded onto devices; protect them in transit and at rest
* Vault secrets in vault exports and recovery bundles are re-encrypted with an export passphrase chosen at export time, and the same passphrase is needed to import them

.. _data-key-rotation:

Data-Key Rotation
-----------------

An administrator can replace the data key with a new one and re-encrypt everything under it. This is called rotation. It is run from **User Management** with **Rotate Encryption Key**; the step-by-step procedure is in the :doc:`admin-guide`.

When to Rotate
^^^^^^^^^^^^^^

Rotate the data key:

* After deactivating a user who had access to encrypted data. Deactivation removes the user's key access from the current database right away (see below), but anyone who kept a copy of the data key, or a copy of the database from before the deactivation, can still read old data. Rotation makes the key they hold useless against the current database.
* After a suspected compromise of the data key, of an administrator password, of a session, or of the workstation while a session was unlocked.
* After the recovery key has been lost or exposed. **Replace Recovery Key** issues a new code but keeps the same data key, so the old code still opens any older copy of the database. Rotation replaces the data key as well as the recovery key.

What Rotation Does
^^^^^^^^^^^^^^^^^^

1. Checks that no firmware analysis jobs are running (each holds a copy of the key) and that every active user has a keypair, or that the administrator acknowledges the users who do not.
2. Generates a new data key with the next version number. The old key is kept only while the rotation runs, encrypted under the new key.
3. Seals the new key to the public key of every active user, so those users get access without any password being entered.
4. Deletes every other copy of the old key: legacy password wraps, old public-key wraps, and the keypairs of deactivated users.
5. Replaces the recovery key. The new recovery key is shown once.
6. Ends every other session and blocks access to encrypted data for everyone, including the administrator who started it, until the rotation is finished.
7. Re-encrypts every configuration version, vault secret, standalone credential, and firmware file under the new key, in small batches. Firmware files are written to a new file first; the old file is deleted only after the database points at the new one.
8. Retires the old key, flushes the database write-ahead log, and removes leftover firmware files. Encrypted data becomes available again.

If the application closes part-way, the rotation is resumed from the next administrator login. Re-encryption is safe to repeat.

.. warning::
   **Copies and backups taken before rotation stay readable with the old key.** Any database copy or backup made before rotation (together with the firmware files, if those were copied too) can be opened forever with the old data key. That includes a copy taken before a user was deactivated, opened with that user's password, and any copy opened with the old recovery key. Rotation cannot reach backward. Destroy or secure old backups yourself.

   **Rotation does not protect plaintext that has already been exported.** Exported configurations, firmware exports, recovery bundles, and vault exports whose passphrase is known stay readable no matter what happens to the data key.

Threat Model
^^^^^^^^^^^^

**What rotation protects against:**

* **Old key plus the current database.** Once rotation finishes, someone holding the old data key can't read anything in the current database file or the firmware directory. That applies however they got the old key: an old database copy plus their password, the old recovery key plus an old copy, or process memory. Data that existed before rotation is covered too, because it was re-encrypted, unless they kept their own copy of that ciphertext.
* **Old recovery key.** It no longer opens anything in the current database.

**What deactivation alone protects against** (no rotation needed):

* **A deactivated user's password plus the current database.** Once their key copies are deleted, their password opens nothing in the current database. This relies on the database deleting data securely and on the write-ahead log having been flushed, which Ferrocodex does when it deactivates the user.

**What neither protects against:**

* **Copies taken before rotation.** Any database copy or backup taken before rotation (with firmware files, if those were copied too) stays readable forever with the old key. That includes a pre-deactivation copy plus that user's password, or plus the old recovery key. Rotation cannot reach backward.
* **Plaintext that has left the app.** Exported configurations, firmware exports, recovery bundles, and vault exports whose passphrase is known.
* **Columns that were never encrypted.** Asset and vault names, labels, credential usernames, metadata, the audit log, and firmware analysis results. The database as a whole is not encrypted.
* **Trusted parties turning hostile.** A malicious or compromised active administrator, or malware on the host while a session is unlocked.
* **Leftover blocks on disk.** Ciphertext remnants on SSD or filesystem blocks after files are rewritten or deleted.
* **Tampering by someone who can write to the database.** Ciphertexts carry no binding to their row, so someone with write access to the database file could swap them between rows.

Deactivation Revokes Key Access
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Deactivating a user now takes effect on encrypted data immediately:

* The user's keypair and every copy of the data key they hold are deleted in the same transaction as the deactivation, so their password opens nothing in the current database.
* The user's live sessions end at once, because a session holds the unlocked key in memory.
* The write-ahead log is flushed so the deleted copies do not linger in it.
* Reactivating the account requires an administrator to set a temporary password, because the administrator does not know the old one. That gives the user a new keypair and a fresh copy of the key.

The same rule applies to a password reset by an administrator: the user's sessions end and a new keypair is created under the temporary password.

Deactivation does not change the data key, so it does not protect against copies made earlier. Rotate the key afterwards if that matters.

Users Who Lose Access
^^^^^^^^^^^^^^^^^^^^^

Rotation can only give the new key to active users who have a keypair. An active user without one (an account that has not logged in since keypairs were introduced) can still log in after a rotation but cannot open encrypted data. The preflight lists these users and requires the administrator to acknowledge them. An administrator restores their access with **Reset password**, which gives them a keypair holding the new key. An administrator can also reset any account with the recovery key.

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

Security Monitoring Dashboard
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Administrators can open **Security Dashboard** from the main menu. Everything on it is read from the audit log and the database at the moment it loads; nothing is estimated or simulated. Choose the reporting period (last 24 hours, 7 days, or 30 days) at the top. The dashboard refreshes every 30 seconds, and **Refresh** reloads it immediately.

**What It Shows:**

* **Status banner**: OK, Warning, or Critical, set by the most serious finding, with a count of findings at each severity
* **Failed logins**: failed logins and account lockouts in the period, successful logins, and any accounts locked right now
* **Vault access denied**: denied vault requests in the period, vault exports and secret reads, and pending access requests
* **Users**: active users, active administrators, inactive users, and role changes in the period
* **Encryption key**: the active data-key version, its age, whether a rotation is in progress, and when the last rotation finished
* **Findings**: issues the dashboard detected, each with a severity and an explanation
* **Security classification distribution**: how many assets have each classification, from Public to Secret
* **Recent security events**: up to 15 of the newest security-relevant audit events in the period. Routine successful logins and logouts are left out. **Open audit log** goes to the full audit log.

**Findings:**

* **Critical: key rotation in progress**: a data-key rotation was started and not finished. Encrypted data is unavailable to every user until an administrator finishes it.
* **Warning: accounts locked**: one or more accounts are locked out right now (they are named in the finding).
* **Warning: failed logins**: five or more failed logins in the period, or any account lockout.
* **Warning: single administrator**: only one active administrator exists. If that account is lost, nobody can administer the system.
* **Warning: legacy key wraps**: some users still have an older form of encryption-key access. It is upgraded automatically the next time each of them logs in.
* **Warning: vault access denied**: at least one vault request was denied in the period.
* **Info: data key age**: the active data key is a year old or more. Consider rotating it.
* **Info: pending permission requests**: vault access requests are waiting for review.
* **Info: grants expiring**: vault permission grants expire within the next seven days.

**Export Report** saves the dashboard data, together with every security-relevant audit event in the period (up to 5,000), as a JSON file you choose. The export itself is recorded in the audit log.

Audit Log Management
^^^^^^^^^^^^^^^^^^^^

Successful logins, failed logins, and account lockouts are recorded in the audit log, along with user, vault, and encryption-key changes.

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

1. **Data Key**: A random 256-bit key, generated when the first administrator logs in, encrypts every secret
2. **Key Wrapping**: The data key is stored only in wrapped form: sealed to each user's X25519 public key (the private key is wrapped under the user's password with Argon2id) and wrapped under the recovery key. It is unwrapped into memory at login and discarded at logout
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
* Stored byte-for-byte as typed: secret values are not trimmed, stripped, or filtered, so leading or trailing spaces and special characters survive. The only checks are that the value is not empty and is at most 4096 bytes
* Never written to logs
* Memory cleared after use
* Secure clipboard operations

**Password History:**

* Previous passwords are retained only as bcrypt hashes, used to prevent reuse
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

2. **Who Holds What**:

   * **Administrators** hold every permission on every vault
   * **The vault's creator** holds Read, Write, and Export on their own vault without a grant, so they can manage and back up its secrets. Creating a vault does not give Share
   * **Everyone else** needs an explicit, active, unexpired grant for each permission on each vault
   * **Share is administrator-only**: only administrators can grant vault permissions

3. **Access Patterns**:

   * Default deny principle
   * No permission inheritance between permissions: a Read grant does not imply Write or Export
   * Time-limited access option

**Server-Side Enforcement:**

The backend checks the permission on every vault and secret operation, not only in the user interface. A user without the permission gets an "Access denied" error, and the denial is recorded in the vault access log and the audit log.

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