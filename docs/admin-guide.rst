Administrator Guide
===================

This guide covers administrative functions and best practices for managing Ferrocodex in your organization, including new v0.5.0 features for asset hierarchy management, metadata schemas, and enhanced security compliance.

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
* **Asset Hierarchy Management** (v0.5.0): Overseeing organizational structures and naming compliance
* **Metadata Schema Administration** (v0.5.0): Defining and managing custom field templates
* **Search Performance** (v0.5.0): Optimizing search indexes and monitoring performance
* **Audit Oversight**: Monitoring system activity and investigating incidents
* **Data Management**: Overseeing backups, exports, and data retention
* **Compliance**: Ensuring the system meets organizational and regulatory requirements

Exclusive Capabilities
^^^^^^^^^^^^^^^^^^^^^^

Administrators have access to functions not available to Engineers:

* Create, modify, and delete user accounts
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

2. **Reset Password**:
   
   * Generate new temporary password
   * Force change on next login
   * Notify user via email
   * Document reason in audit log

3. **Lock/Unlock Account**:
   
   * Temporarily disable access
   * Preserves user data and permissions
   * Useful for leave of absence
   * Automatic unlock option

4. **Delete Account**:
   
   * Permanent removal (use cautiously)
   * Audit history retained
   * Cannot be undone
   * Consider locking instead

User Role Management
^^^^^^^^^^^^^^^^^^^^

**Role Capabilities:**

.. list-table::
   :header-rows: 1

   * - Feature
     - Administrator
     - Engineer
   * - Create/Edit Assets
     - ✓
     - ✓
   * - Upload Configurations
     - ✓
     - ✓
   * - Manage Branches
     - ✓
     - ✓
   * - Access Identity Vaults
     - ✓
     - With Permission
   * - Create Users
     - ✓
     - ✗
   * - View All Audit Logs
     - ✓
     - Own Only
   * - System Settings
     - ✓
     - ✗
   * - Grant Vault Permissions
     - ✓
     - ✗

**Changing User Roles:**

1. Consider impact on existing permissions
2. Document reason for role change
3. Review vault access after change
4. Notify user of new capabilities
5. Update training if needed

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

Asset Hierarchy Administration (v0.5.0)
----------------------------------------

Asset Naming Security
^^^^^^^^^^^^^^^^^^^^^

**Cybersecurity-Compliant Naming Enforcement:**

Ferrocodex v0.5.0 enforces strict naming conventions for security:

.. code-block:: regex

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
   * Check search indexing
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

Search Performance Management
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Search Index Administration:**

1. **Index Monitoring**:
   
   * Navigate to **Settings** → **Search Performance**
   * View metrics:
     
     - Index size and fragmentation
     - Query response times
     - Cache hit rates
     - Slow query log

2. **Optimization Tasks**:
   
   * **Rebuild Index**: Full reconstruction
   * **Optimize Index**: Defragmentation
   * **Clear Cache**: Reset search cache
   * **Analyze Queries**: Performance profiling

3. **Configuration**:
   
   * Set cache size limits
   * Configure background optimization
   * Adjust relevance scoring
   * Manage stopwords

**Performance Tuning:**

.. code-block:: text

   Recommended Settings for Different Scales:
   
   Small (< 1,000 assets):
   - Cache: 100MB
   - Background optimization: Weekly
   - Full-text index: All fields
   
   Medium (1,000 - 10,000 assets):
   - Cache: 500MB
   - Background optimization: Daily
   - Full-text index: Selected fields
   
   Large (> 10,000 assets):
   - Cache: 1GB+
   - Background optimization: Continuous
   - Full-text index: Optimized fields only

**Search Analytics:**

* Popular search terms
* Failed searches (no results)
* User search patterns
* Performance trends
* Optimization recommendations

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
   * Use search for navigation
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

1. **Session Timeout**:
   
   * Default: 30 minutes
   * Range: 5 minutes to 8 hours
   * Consider security vs. usability
   * Different for admin accounts

2. **Concurrent Sessions**:
   
   * Limit per user
   * Force single session
   * Device restrictions
   * Geographic limitations

3. **Login Security**:
   
   * Failed attempt lockout
   * Lockout duration
   * CAPTCHA after failures
   * IP allowlisting

**Master Password Protection:**

* Cannot be recovered if lost
* Consider key escrow procedures
* Document in security policies
* Test recovery procedures

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
   * **Asset hierarchies** (v0.5.0)
   * **Custom metadata schemas** (v0.5.0)
   * **Search indexes** (v0.5.0)
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

**Comprehensive Access:**

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

3. **Asset Compliance** (v0.5.0):
   
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
   * Approval workflows
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
- [ ] Check search performance metrics (v0.5.0)
- [ ] Review asset naming violations (v0.5.0)

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
- [ ] Optimize search indexes (v0.5.0)
- [ ] Review metadata schema usage (v0.5.0)
- [ ] Audit asset hierarchy structure (v0.5.0)

Quarterly Tasks
^^^^^^^^^^^^^^^

- [ ] Complete security audit
- [ ] Policy review and update
- [ ] Disaster recovery test
- [ ] Vendor security updates
- [ ] Compliance assessment
- [ ] Technology roadmap review

Remember: As an administrator, you are the guardian of your organization's critical configuration data. Your diligence in following these procedures ensures the security, integrity, and availability of the Ferrocodex system.