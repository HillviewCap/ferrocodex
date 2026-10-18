Administrator Guide
===================

This guide covers administrative functions and recommended practices for managing Ferrocodex in your organization, including asset hierarchy management, metadata schemas, and security compliance.

.. contents:: Table of Contents
   :local:
   :depth: 2

Administrator Role Overview
---------------------------

Responsibilities
^^^^^^^^^^^^^^^^

As a Ferrocodex administrator, you are responsible for:

* **User Management**: Creating accounts, resetting passwords, managing roles
* **Access Control**: Granting and revoking permissions, including vault access
* **System Configuration**: Managing settings, security policies, and preferences
* **Asset Hierarchy Management**: Overseeing organizational structures and naming compliance
* **Metadata Schema Administration**: Defining and managing custom field templates
* **Audit Oversight**: Monitoring system activity and investigating incidents
* **Data Management**: Overseeing backups, exports, and data retention
* **Compliance**: Ensuring the system meets organizational and regulatory requirements

Exclusive Capabilities
^^^^^^^^^^^^^^^^^^^^^^

Administrators have access to functions not available to Engineers:

* Create and modify user accounts, and deactivate or reactivate them
* View all audit logs (not just their own)
* Manage vault permissions for all users
* Configure system-wide settings
* Export complete system data
* View all assets and configurations
* Manage password policies
* Access compliance reports

User Management
---------------

Creating User Accounts
^^^^^^^^^^^^^^^^^^^^^^

**To create a new user:**

1. Navigate to **Settings** → **User Management**
2. Click **"Create User"** button
3. Fill in required information:

   * **Username**: Unique identifier (cannot be changed)
   * **Full Name**: User's display name
   * **Email**: For notifications and password resets
   * **Role**: Select Administrator or Engineer
   * **Initial Password**: Set temporary password

4. Configure optional settings:

   * Force password change on first login
   * Set account expiration date
   * Add user notes or department

5. Click **"Create"** to save

**Best Practices:**

* Use consistent username formats (e.g., firstname.lastname)
* Document the purpose of admin accounts
* Limit administrator accounts to essential personnel
* Set strong initial passwords
* Enable forced password change

Managing Existing Users
^^^^^^^^^^^^^^^^^^^^^^^

**User Account Actions:**

1. **Edit User Details**:

   * Update display name or email
   * Change role (with caution)
   * Modify account settings
   * Add administrative notes

2. **Reset Password** (row action in **User Management**):

   * Available for active accounts other than your own
   * Enter a temporary password that meets the password policy and give it to the user through a secure channel
   * The account gets a new encryption keypair holding the current data key, so it can open encrypted data again
   * The user's open sessions end immediately
   * The reset is recorded in the audit log
   * This is also how you restore access for a user who lost it after a key rotation (see :ref:`rotation-lost-access`)

3. **Deactivate**:

   * Available for engineer accounts; administrator accounts cannot be deactivated
   * The user's sessions end immediately
   * Their access to encrypted data is revoked at once: their keypair and every copy of the data key they hold are deleted, so their password opens nothing in the current database
   * User data, audit history, and permissions records are kept
   * Deactivation does not change the data key. Backups made earlier stay readable with the old key. To cut off a copy of the key someone may already hold, follow it with a key rotation (see :ref:`rotating-the-encryption-key`)

4. **Reactivate** (shown in place of Deactivate for inactive accounts):

   * Requires you to enter a temporary password, because the old password no longer opens anything
   * Gives the account a new keypair and a fresh copy of the data key
   * Give the temporary password to the user through a secure channel

User Role Management
^^^^^^^^^^^^^^^^^^^^

**Role Capabilities:**

.. list-table::
   :header-rows: 1

   * - Feature
     - Administrator
     - Engineer
   * - Create/Edit Assets
     - Yes
     - Yes
   * - Upload Configurations
     - Yes
     - Yes
   * - Manage Branches
     - Yes
     - Yes
   * - Access Identity Vaults
     - Yes
     - With Permission
   * - Create Users
     - Yes
     - No
   * - View All Audit Logs
     - Yes
     - Own Only
   * - System Settings
     - Yes
     - No
   * - Grant Vault Permissions
     - Yes
     - No

**Changing User Roles:**

1. Consider impact on existing permissions
2. Document reason for role change
3. Review vault access after change
4. Notify user of new capabilities
5. Update training if needed

.. _rotating-the-encryption-key:

Encryption Key Rotation
-----------------------

Rotation replaces the data key that encrypts configurations, firmware, and vault secrets, and re-encrypts all of it under the new key. Read :ref:`data-key-rotation` in the security guide first: it explains when to rotate and what rotation does not protect (in particular, backups made before the rotation stay readable with the old key).

Before You Start
^^^^^^^^^^^^^^^^

* Pick a quiet time. While a rotation runs, nobody can read or write encrypted data, and large firmware libraries can take several minutes.
* Make sure no firmware analysis is running. Rotation is refused while analysis jobs are queued or running.
* Have your administrator password ready.
* Have a safe place ready to store the new recovery key. The old one stops working.
* Tell engineers beforehand. Their sessions end when the rotation starts.

Rotating the Key
^^^^^^^^^^^^^^^^

1. Go to **User Management** and click **Rotate Encryption Key**.
2. Read the preflight summary. It lists any active users who have no encryption keypair. Those users cannot be given the new key and will lose access to encrypted data until you reset their password. Tick the acknowledgement checkbox to proceed with them. Ferrocodex refuses to start without it.
3. Enter your administrator password. Ferrocodex checks it and uses it to open your own copy of the key.
4. Click to start. Ferrocodex generates the new key, gives it to every active user who has a keypair, and replaces the recovery key.
5. **Store the new recovery key.** It is shown once, in the same dialog as at setup. Write it down or print it and keep it offline, away from the workstation. Do not continue until you have saved it, because it cannot be shown again. Anyone holding it can reset any account's password.
6. Watch the progress bar. Ferrocodex re-encrypts configuration versions, vault secrets, standalone credentials, and firmware files in turn. Keep the application open until it reports that it has finished.
7. When nothing is left to re-encrypt and nothing failed, the rotation finishes on its own: the old key is retired and encrypted data becomes available again. The audit log records when the rotation started and when it completed.

.. note::
   Starting a rotation ends every other session, including other administrators. Your own session carries on with the new key.

If the Application Closes Part-Way
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

A rotation survives a restart, a crash, or a power cut. The old key is kept encrypted under the new one until the rotation finishes, and re-encryption can safely be repeated.

1. Start Ferrocodex and log in as an administrator with your normal password. The login reports that a rotation is in progress, and encrypted data stays blocked for everyone.
2. Open **User Management** and click **Rotate Encryption Key** again. It picks up where it stopped, without asking for a new recovery key.
3. Let it run to the end.

Any administrator who has a keypair can resume it, not only the one who started it.

If Items Cannot Be Re-Encrypted
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

If some items cannot be decrypted with the old key (for example, a corrupted record), Ferrocodex records them as failures, finishes everything else, and then stops with the failures listed. It does not finish on its own.

* Review the list. Each item is shown with the reason it could not be decrypted under the old key.
* If you can restore them another way, do that first (for example, re-upload the configuration), then continue.
* To complete the rotation anyway, click **Finalize and discard unreadable items**. The old key is then destroyed, so those items stay unreadable permanently. The rows are left in place but cannot be opened. This cannot be undone.

If an error appears while writing a re-encrypted firmware file (for example, the disk is full), fix the problem and click **Rotate Encryption Key** again to resume.

What Engineers See During a Rotation
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

* Their sessions end when the rotation starts, so they must log in again.
* They can log in, and the dashboard shows that a rotation is in progress.
* Anything that reads or writes encrypted data (configuration content, firmware, vault secrets, standalone credentials) fails with "Encryption key rotation in progress; an administrator must finish it".
* Once the rotation is finished, sessions opened during it work again without logging in again. Engineers who have no copy of the new key (see below) stay without access.

While a rotation is in progress, these administrator actions are blocked because they need the data key: creating an engineer account, reactivating a user, and **Reset password**. Resetting a password with the recovery key from the login screen still works, as do deactivating a user and **Replace Recovery Key**.

.. _rotation-lost-access:

Users Who Lose Access After a Rotation
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

An active user who had no keypair when the rotation started (typically an account that had not logged in since keypairs were introduced) has no copy of the new key. They can log in, but encrypted data is unavailable to them. The preflight listed them.

To restore a user:

1. Wait until the rotation has finished.
2. In **User Management**, click **Reset password** on the user's row and enter a temporary password.
3. Give them the temporary password. They log in with it and have access again.

If every administrator lost access, use **Forgot password?** on the login screen with the new recovery key. The old recovery key no longer works.

Rotation also deletes the keypairs of deactivated users. They get new ones when you reactivate them with a temporary password.

Identity Vault Administration
-----------------------------

Managing Vault Permissions
^^^^^^^^^^^^^^^^^^^^^^^^^^

.. figure:: _static/diagrams/permission-hierarchy.svg
   :alt: Vault permission hierarchy diagram
   :align: center
   :width: 700px

   *Permission hierarchy showing inheritance and access levels*

**Granting Vault Access:**

1. Navigate to **User Management**
2. Select the user requiring access
3. Click **"Manage Vault Permissions"**
4. Search for specific vaults:

   * By asset name
   * By vault creation date
   * By current permissions

5. Grant appropriate permissions:

   * **Read**: View vault contents only
   * **Write**: Add/modify secrets
   * **Export**: Include in recovery bundles
   * **Share**: Grant permissions to others

6. Set access duration:

   * Permanent access
   * Time-limited (with expiration)
   * Recurring (for contractors)

**Best Practices:**

* Follow principle of least privilege
* Document reason for access grants
* Set expiration for temporary staff
* Regular access reviews (monthly)
* Revoke unused permissions

Vault Access Requests
^^^^^^^^^^^^^^^^^^^^^

**Reviewing Requests:**

1. Check **Settings** → **Access Requests**
2. Review pending requests showing:

   * Requesting user
   * Target vault
   * Requested permissions
   * Business justification

3. Evaluate request:

   * Verify business need
   * Check user's role
   * Consider security implications
   * Review similar requests

4. Take action:

   * **Approve**: Grant requested access
   * **Modify**: Grant partial permissions
   * **Deny**: Reject with reason
   * **Defer**: Request more information

**Request Handling Guidelines:**

* Respond within 24 hours
* Always provide reason for denials
* Consider time-limiting approvals
* Document special circumstances
* Follow organizational policies

Vault Security Oversight
^^^^^^^^^^^^^^^^^^^^^^^^

**Monitoring Vault Usage:**

1. **Access Reports**:

   * Who accessed which vaults
   * Frequency of access
   * Failed access attempts
   * Permission changes

2. **Rotation Compliance**:

   * View rotation dashboard
   * Identify overdue rotations
   * Track compliance percentages
   * Generate audit reports

3. **Security Alerts**:

   * Multiple failed access attempts
   * Unusual access patterns
   * Emergency rotations
   * Export operations

**Regular Reviews:**

* Weekly: Check access logs
* Monthly: Review all permissions
* Quarterly: Full security audit
* Annually: Policy review

Password Policy Management
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Configuring Password Policies:**

1. Navigate to **Settings** → **Security** → **Password Policy**
2. Set requirements:

   * Minimum length (8-64 characters)
   * Character requirements:

     - Uppercase letters
     - Lowercase letters
     - Numbers
     - Special characters

   * History depth (prevent reuse)
   * Maximum age (force rotation)

3. Configure by asset type:

   * Critical assets: Strictest policy
   * Standard assets: Balanced security
   * Test assets: Relaxed requirements

**Enforcement Options:**

* Block weak passwords
* Force immediate compliance
* Grace period for updates
* Exemption management

Asset Hierarchy Administration
----------------------------------------

Asset Naming Security
^^^^^^^^^^^^^^^^^^^^^

**Naming Enforcement:**

Ferrocodex enforces a strict naming pattern for assets:

.. code-block:: text

   ^[A-Z0-9][A-Z0-9_-]{2,49}$

**Administrator Controls:**

1. **Naming Policy Configuration**:

   * Navigate to **Settings** → **Asset Security**
   * Configure naming rules:

     - Enforce uppercase only
     - Block reserved names (CON, PRN, AUX, etc.)
     - Set minimum/maximum length
     - Custom regex patterns

2. **Validation Rules**:

   * Real-time validation during creation
   * Bulk rename validation
   * Import sanitization
   * Audit non-compliant names

3. **Reserved Name Management**:

   * View blocked Windows names
   * Add organization-specific restrictions
   * Configure exceptions (with logging)
   * Generate compliance reports

**File Upload Security:**

* Automatic filename sanitization
* Path traversal prevention
* Extension validation
* SHA-256 integrity verification
* Quarantine suspicious files

Metadata Schema Management
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Creating Organization-Wide Schemas:**

1. Navigate to **Settings** → **Metadata Schemas**
2. Click **"Create Schema Template"**
3. Define schema structure:

   * Field definitions
   * Validation rules
   * Default values
   * Required fields
   * Conditional logic

4. Set schema properties:

   * **Name**: Template identifier
   * **Asset Types**: Applicable to folders/devices
   * **Mandatory**: Force on asset creation
   * **Version**: Track schema changes

**Field Type Configuration:**

.. list-table:: Available Field Types and Validation
   :header-rows: 1
   :widths: 20 30 50

   * - Field Type
     - Validation Options
     - Use Cases
   * - Text
     - Pattern, length, character set
     - Serial numbers, descriptions
   * - Number
     - Min/max, decimals, step
     - Counts, measurements
   * - Date
     - Range, relative constraints
     - Install dates, warranties
   * - Dropdown
     - Option list, multi-select
     - Categories, status
   * - IP Address
     - IPv4/IPv6, subnet validation
     - Network configuration
   * - JSON
     - Schema validation
     - Complex structured data

**Schema Deployment:**

1. **Testing**:

   * Create test assets
   * Validate field behavior
   * Verify validation rules

2. **Rollout**:

   * Apply to asset types
   * Migrate existing data
   * Set as default template
   * Monitor adoption

3. **Maintenance**:

   * Version control schemas
   * Track field usage
   * Update validation rules
   * Archive obsolete fields

Asset Security Classification
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Managing Security Tags:**

1. **Classification Levels**:

   * Public
   * Internal
   * Confidential
   * Secret
   * Top Secret

2. **Classification Impact**:

   * Search result filtering
   * Export restrictions
   * Audit detail level
   * Access control enforcement
   * Compliance reporting

3. **Bulk Classification**:

   * Select multiple assets
   * Apply classification
   * Cascade to children
   * Generate reports

Hierarchy Management Best Practices
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Organizational Structure Guidelines:**

1. **Design Principles**:

   * Logical grouping (function/location)
   * Consistent depth (5-7 levels max)
   * Clear naming conventions
   * Avoid circular references

2. **Performance Considerations**:

   * Archive inactive branches
   * Limit folder contents (< 500 items)
   * Use the tree filter for navigation
   * Regular structure review

3. **Access Control**:

   * Inherit permissions from parent
   * Override at specific levels
   * Document exceptions
   * Regular permission audits

System Configuration
--------------------

Security Settings
^^^^^^^^^^^^^^^^^

**Session Management:**

1. **Session Lifetime**:

   * Sessions expire 24 hours after login
   * Logging out invalidates the session immediately
   * Users should log out when leaving a workstation

2. **Login Security**:

   * Five failed login attempts lock the account
   * The lockout lasts 15 minutes
   * Failed attempts are recorded in the audit log

**Administrator Credential Protection:**

* Passwords are stored as bcrypt hashes and cannot be recovered
* Encryption keys for configuration content, firmware files, and vault secrets are derived from user credentials
* Document credential handling in security policies
* Test the backup and restore procedure regularly

System Maintenance
^^^^^^^^^^^^^^^^^^

**Database Management:**

1. **Size Monitoring**:

   * Check Settings → System → Storage
   * Monitor growth trends
   * Plan for capacity
   * Set size alerts

2. **Performance Optimization**:

   * Database compaction
   * Index optimization
   * Archive old data
   * Regular maintenance windows

3. **Cleanup Tasks**:

   * Old audit logs
   * Orphaned files
   * Temporary data
   * Export archives

Import/Export Configuration
^^^^^^^^^^^^^^^^^^^^^^^^^^^

**System-Wide Exports:**

1. **Full System Backup**:

   * All assets and configurations
   * Asset hierarchies
   * Custom metadata schemas
   * User accounts (without passwords)
   * Audit logs
   * System settings
   * Optionally vault data

2. **Selective Exports**:

   * Date range filtering
   * Specific asset types
   * User activity only
   * Configuration subsets

**Import Procedures:**

1. **Preparation**:

   * Verify source compatibility
   * Check available space
   * Notify users of downtime
   * Backup current data

2. **Import Process**:

   * Validate import file
   * Preview contents
   * Handle conflicts
   * Verify completion

3. **Post-Import**:

   * Verify data integrity
   * Check user access
   * Test critical functions
   * Document changes

Audit Log Management
--------------------

Viewing Audit Logs
^^^^^^^^^^^^^^^^^^

**Access:**

Administrators can view all system activity:

* User authentication events
* Configuration changes
* Vault access attempts
* Permission modifications
* System configuration changes
* Export/import operations

**Filtering and Search:**

1. **By User**: Track specific user activity
2. **By Date**: Focus on time periods
3. **By Action**: Filter event types
4. **By Resource**: Asset or vault specific
5. **By Result**: Success or failure

**Advanced Queries:**

* Failed login patterns
* After-hours access
* Privilege escalation
* Mass operations
* Anomaly detection

Audit Log Analysis
^^^^^^^^^^^^^^^^^^

**Security Investigations:**

1. **Incident Response**:

   * Identify timeline
   * Determine scope
   * Track user actions
   * Find root cause
   * Document findings

2. **Pattern Recognition**:

   * Unusual access times
   * Repeated failures
   * Permission abuse
   * Data exfiltration
   * Policy violations

**Regular Reviews:**

* Daily: Failed authentications
* Weekly: Permission changes
* Monthly: Access patterns
* Quarterly: Compliance audit

Audit Log Retention
^^^^^^^^^^^^^^^^^^^

**Retention Policies:**

1. **Storage Considerations**:

   * Regulatory requirements
   * Storage capacity
   * Performance impact
   * Legal hold needs

2. **Archive Procedures**:

   * Export before deletion
   * Secure archive storage
   * Maintain searchability
   * Document locations

3. **Compliance Requirements**:

   * Industry regulations
   * Internal policies
   * Audit trail integrity
   * Long-term accessibility

Compliance and Reporting
------------------------

Compliance Dashboard
^^^^^^^^^^^^^^^^^^^^

**Key Metrics:**

1. **Password Compliance**:

   * Rotation adherence
   * Policy compliance
   * Weak passwords
   * Overdue changes

2. **Access Control**:

   * Active permissions
   * Unused access
   * Time-limited expiry
   * Segregation violations

3. **Asset Compliance**:

   * Naming convention adherence
   * Metadata completeness
   * Security classification coverage
   * Hierarchy organization standards

4. **System Security**:

   * Failed login trends
   * Security incidents
   * Policy violations
   * Audit completeness

Report Generation
^^^^^^^^^^^^^^^^^

**Available Reports:**

1. **User Reports**:

   * User activity summary
   * Permission matrix
   * Login history
   * Role distribution

2. **Security Reports**:

   * Vault access logs
   * Password age analysis
   * Compliance status
   * Incident summary

3. **System Reports**:

   * Configuration changes
   * Asset inventory
   * Storage utilization
   * Performance metrics

**Report Scheduling:**

* Automated generation
* Email distribution
* Format options (PDF, CSV)
* Custom parameters

Regulatory Compliance
^^^^^^^^^^^^^^^^^^^^^

**Supporting Compliance:**

1. **Documentation**:

   * Policy enforcement
   * Audit trail integrity
   * Access controls
   * Change management

2. **Evidence Collection**:

   * Export capabilities
   * Report generation
   * Log preservation
   * Timestamp accuracy

3. **Compliance Features**:

   * Role segregation
   * Immutable logs
   * Encryption standards

Best Practices
--------------

Administrative Security
^^^^^^^^^^^^^^^^^^^^^^^

1. **Account Protection**:

   * Use strong, unique passwords
   * Enable all security features
   * Regular password rotation
   * Limit admin accounts

2. **Operational Security**:

   * Document all changes
   * Follow change procedures
   * Peer review for critical changes
   * Regular security training

3. **Monitoring**:

   * Daily log reviews
   * Alert configuration
   * Trend analysis
   * Incident preparedness

Training and Documentation
^^^^^^^^^^^^^^^^^^^^^^^^^^

1. **User Training**:

   * Role-based training programs
   * Security awareness
   * Feature updates
   * Best practices

2. **Documentation**:

   * Maintain procedures
   * Update policies
   * Record decisions
   * Knowledge transfer

3. **Continuous Improvement**:

   * User feedback
   * Security assessments
   * Process refinement
   * Technology updates

Emergency Procedures
^^^^^^^^^^^^^^^^^^^^

1. **Account Compromise**:

   * Immediate lockout
   * Password reset
   * Permission review
   * Incident documentation

2. **Data Recovery**:

   * Backup restoration
   * Point-in-time recovery
   * Verification procedures
   * User communication

3. **System Issues**:

   * Escalation procedures
   * Vendor contact
   * Workaround documentation
   * Status communication

Administrative Checklist
------------------------

Daily Tasks
^^^^^^^^^^^

- [ ] Review failed login attempts
- [ ] Check vault access requests
- [ ] Monitor system alerts
- [ ] Verify backup completion
- [ ] Review critical audit entries
- [ ] Review asset naming violations

Weekly Tasks
^^^^^^^^^^^^

- [ ] Process access requests
- [ ] Review user permissions
- [ ] Check password compliance
- [ ] Analyze access patterns
- [ ] Update user documentation

Monthly Tasks
^^^^^^^^^^^^^

- [ ] Full permission audit
- [ ] Generate compliance reports
- [ ] Review security policies
- [ ] User account cleanup
- [ ] System performance review
- [ ] Security training updates
- [ ] Review metadata schema usage
- [ ] Audit asset hierarchy structure

Quarterly Tasks
^^^^^^^^^^^^^^^

- [ ] Complete security audit
- [ ] Policy review and update
- [ ] Disaster recovery test
- [ ] Vendor security updates
- [ ] Compliance assessment
- [ ] Technology roadmap review

Following these procedures helps maintain the security, integrity, and availability of your organization's configuration data.