User Guide
==========

This comprehensive guide covers all features and functionality in Ferrocodex.

.. contents:: Table of Contents
   :local:
   :depth: 2

Dashboard
---------

.. figure:: _static/images/dashboard-overview.png
   :alt: Ferrocodex dashboard showing system statistics
   :align: center
   :width: 800px

   *The Ferrocodex dashboard with key metrics and recent activity*

The dashboard provides an at-a-glance view of your configuration management system:

* **Total Assets**: Number of equipment items in the system
* **Active Configurations**: Current configuration files
* **Recent Activity**: Latest configuration changes
* **System Status**: User sessions and system health

Navigation
^^^^^^^^^^

The main navigation sidebar includes:

* **Dashboard**: System overview
* **Assets**: Equipment management
* **Configurations**: File management and branching
* **Identity Vault**: Secure credential management
* **Standalone Credentials**: Non-PLC credential storage
* **Audit Log**: Activity tracking
* **Settings**: User and system configuration

Asset Management
----------------

Understanding Assets
^^^^^^^^^^^^^^^^^^^^

Assets represent your industrial equipment and devices:

* PLCs (Programmable Logic Controllers)
* HMIs (Human Machine Interfaces)
* SCADA systems
* Network devices
* Any configurable industrial equipment

Creating Assets
^^^^^^^^^^^^^^^

1. Navigate to **Assets** from the sidebar
2. Click **"Add Asset"** button
3. Complete the form:

   * **Name**: Unique identifier (e.g., "PLC-WEST-01")
   * **Type**: Equipment category
   * **Manufacturer**: Device manufacturer
   * **Model**: Specific model number
   * **Location**: Physical or logical location
   * **Description**: Additional details

4. Click **"Create"** to save

Best Practices for Asset Naming
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

* Use consistent naming conventions
* Include location identifiers
* Add equipment type prefixes
* Avoid special characters
* Keep names descriptive but concise

Example naming convention::

    [TYPE]-[LOCATION]-[NUMBER]
    PLC-WEST-01
    HMI-LINE2-03
    SCADA-MAIN-01

Editing Assets
^^^^^^^^^^^^^^

1. Click on any asset in the list
2. Select **"Edit"** from the actions menu
3. Update information as needed
4. Save changes

.. note::
   Asset deletion is restricted if configurations exist. Archive configurations first.

Configuration Management
------------------------

Uploading Configurations
^^^^^^^^^^^^^^^^^^^^^^^^

1. Select an asset from the Assets page
2. Click **"Upload Configuration"**
3. **Drag and drop** or **browse** for files
4. Add configuration details:

   * **File Type**: Select appropriate type
   * **Version Notes**: Describe changes
   * **Tags**: Add searchable keywords

5. Click **"Upload"** to save

Supported File Types
^^^^^^^^^^^^^^^^^^^^

Ferrocodex accepts **all file types** without restriction. This alpha build is designed to support the widest variety of industrial equipment configurations for testing purposes.

Common file types include:

* PLC programs (``.acd``, ``.rss``, ``.l5x``, ``.apb``, ``.zpj``)
* HMI projects (``.mer``, ``.apa``, ``.hmi``, ``.gef``)
* SCADA configurations (``.scada``, ``.proj``, ``.s7p``)
* Text-based configs (``.xml``, ``.ini``, ``.cfg``, ``.conf``)
* Documentation (``.pdf``, ``.docx``, ``.xlsx``)
* Binary files and proprietary formats
* Compressed archives (``.zip``, ``.tar``, ``.gz``)
* Any other file format used by your industrial equipment

.. note::
   The alpha build intentionally accepts all file types to ensure compatibility with various SCADA and industrial control systems during testing.

Viewing Configurations
^^^^^^^^^^^^^^^^^^^^^^

1. Click on an asset to view its configurations
2. The configuration list shows:

   * File name and type
   * Upload date and time
   * Uploaded by (user)
   * Version number
   * File size

3. Click on any configuration to:

   * Download the file
   * View metadata
   * Create a branch
   * See related audit entries

Branching and Merging
^^^^^^^^^^^^^^^^^^^^^

**Creating a Branch:**

1. From a configuration, click **"Create Branch"**
2. Enter a descriptive branch name
3. The branch copies the current configuration
4. Work on the branch without affecting main

**Working with Branches:**

* Upload new versions to the branch
* Track changes separately
* Test configurations safely
* Multiple branches per configuration

**Merging Branches:**

1. Review branch changes
2. Click **"Merge to Main"**
3. Add merge notes
4. Confirm the merge

.. warning::
   Merging replaces the main configuration. Download current version first if needed.

Import and Export
-----------------

Bulk Import
^^^^^^^^^^^

For migrating existing configurations:

1. Go to **Settings** → **Import/Export**
2. Select **"Bulk Import"**
3. Choose import type:

   * **Configurations Only**: Just files
   * **Full Import**: Assets and configurations

4. Select ZIP file containing configurations
5. Map files to assets
6. Review and confirm import

Export Options
^^^^^^^^^^^^^^

**Single Asset Export:**

1. Select asset
2. Choose **"Export"** from actions
3. Includes all configurations and metadata

**System Export:**

1. **Settings** → **Import/Export**
2. Select **"Export All"**
3. Choose export options:

   * Include audit logs
   * Include user data (admins only)
   * Encryption options

Asset Identity Vault
--------------------

The Asset Identity Vault provides secure storage for all authentication information related to your industrial equipment. Each asset can have its own vault containing passwords, network information, and security keys.

Understanding the Identity Vault
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

The Identity Vault is a secure container that stores:

* **Passwords**: Login credentials for equipment access
* **IP Addresses**: Network addressing information
* **VPN Keys**: Virtual private network credentials
* **License Files**: Software licensing information

All vault contents are encrypted using AES-256 encryption and include complete version history and audit trails.

Creating an Identity Vault
^^^^^^^^^^^^^^^^^^^^^^^^^^

.. figure:: _static/images/vault-creation-flow.png
   :alt: Identity Vault creation workflow
   :align: center
   :width: 700px

   *Creating an Identity Vault for an asset*

1. Navigate to an asset's detail view
2. Click the **"Identity Vault"** tab
3. Click **"Create Vault"** (first time only)
4. The vault is now ready to store secrets

Adding Secrets to the Vault
^^^^^^^^^^^^^^^^^^^^^^^^^^^

1. Click **"Add Secret"** button
2. Select the secret type:
   
   * **Password**: For login credentials
   * **IP Address**: For network information
   * **VPN Key**: For VPN credentials
   * **License File**: For software licenses

3. Enter secret details:

   * **Label**: Descriptive name (e.g., "Admin Login", "Maintenance Account")
   * **Value**: The actual secret information
   * **Notes**: Optional additional information

4. Click **"Save"** to store securely

Password Management Features
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Generating Strong Passwords:**

1. When adding a password secret, click **"Generate"**
2. Customize password requirements:
   
   * Length: 8-64 characters
   * Include uppercase letters
   * Include lowercase letters
   * Include numbers
   * Include special characters

3. The system shows password strength in real-time
4. Generated passwords are cryptographically secure

**Password Strength Indicators:**

* **Weak** (Red): Low entropy, easily guessable
* **Fair** (Orange): Moderate entropy
* **Good** (Yellow): Strong entropy
* **Excellent** (Green): Very high entropy

**Password History:**

* View complete history of password changes
* See who changed passwords and when
* Prevent password reuse
* Track rotation compliance

Password Rotation
^^^^^^^^^^^^^^^^^

Regular password rotation is crucial for security. Ferrocodex provides guided rotation workflows.

**Setting Rotation Schedules:**

1. Select a password in the vault
2. Click **"Set Rotation Schedule"**
3. Choose rotation interval:
   
   * 30 days
   * 60 days
   * 90 days
   * Custom interval

4. Enable automated reminders

**Rotating Passwords:**

1. Click **"Rotate Password"** on any credential
2. Follow the rotation wizard:
   
   a. Verify current password
   b. Generate or enter new password
   c. Add rotation reason/notes
   d. Confirm the change

3. The old password is retained in history
4. Audit trail records the rotation

**Batch Rotation:**

For coordinated password changes:

1. Select multiple related passwords
2. Click **"Batch Rotate"**
3. Generate new passwords for all
4. Complete rotation with notes

**Emergency Rotation:**

For security incidents:

1. Use **"Emergency Rotate"** option
2. All selected passwords immediately rotated
3. Incident notes are required
4. Notifications sent to administrators

Standalone Credentials
^^^^^^^^^^^^^^^^^^^^^^

Store credentials for non-PLC equipment like jump hosts, databases, and network devices.

**Creating Standalone Credentials:**

1. Navigate to **"Standalone Credentials"** from main menu
2. Click **"Add Credential"**
3. Select or create a category:
   
   * Jump Hosts
   * Databases
   * Network Equipment
   * Custom categories

4. Enter credential details:
   
   * Name
   * Username
   * Password
   * Host/URL
   * Port
   * Notes

5. Save the credential

**Organizing with Categories:**

* Create hierarchical categories
* Drag and drop to reorganize
* Bulk operations on categories
* Export/import category structures

**Searching Credentials:**

* Full-text search across all fields
* Filter by category
* Filter by credential type
* Recent access shortcuts

Vault Access Control
^^^^^^^^^^^^^^^^^^^^

Administrators control who can access each vault through granular permissions.

**Permission Types:**

* **Read**: View vault contents
* **Write**: Add/modify secrets
* **Export**: Include in recovery bundles
* **Share**: Grant access to others

**Granting Access (Administrators):**

1. Go to **User Management**
2. Select a user
3. Click **"Manage Vault Permissions"**
4. Search for specific vaults
5. Grant required permissions
6. Optionally set expiration date

**Requesting Access (Engineers):**

1. Navigate to a restricted vault
2. Click **"Request Access"**
3. Select needed permissions
4. Add justification
5. Submit request
6. Wait for administrator approval

**Time-Limited Access:**

* Grant temporary access for contractors
* Automatic expiration
* No manual revocation needed
* Audit trail of all access

Secure Export and Import
^^^^^^^^^^^^^^^^^^^^^^^^

**Exporting with Vault Data:**

1. Create a recovery bundle
2. Check **"Include vault data"**
3. **Security Warning**: Acknowledge sensitive data export
4. Bundle includes encrypted vault contents
5. Store bundle in secure location

**Importing Vault Data:**

1. Select recovery bundle to import
2. Preview included vault data
3. Choose import options:
   
   * Merge with existing
   * Replace existing
   * Skip conflicts

4. Verify successful import

**Security Considerations:**

* Vault data remains encrypted in bundles
* Physical security of bundles is critical
* Use air-gapped systems for sensitive imports
* Audit all import operations

Compliance and Auditing
^^^^^^^^^^^^^^^^^^^^^^^

**Rotation Compliance Dashboard:**

1. Access from Identity Vault main page
2. View rotation status:
   
   * Compliant (green)
   * Due Soon (yellow)
   * Overdue (red)

3. Filter by asset or rotation policy
4. Export compliance reports

**Vault Audit Trail:**

All vault operations are logged:

* Secret creation/modification
* Access attempts (successful and failed)
* Permission changes
* Export operations
* Rotation events

**Compliance Reports:**

* Password age reports
* Rotation compliance
* Access audit reports
* Failed access attempts
* Permission usage analysis

Best Practices for Vault Management
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Security Guidelines:**

1. **Unique Passwords**: Never reuse passwords across assets
2. **Regular Rotation**: Follow your organization's policy
3. **Access Control**: Grant minimum necessary permissions
4. **Audit Reviews**: Regularly review access logs
5. **Secure Export**: Protect exported bundles physically

**Organizational Tips:**

1. **Naming Conventions**: Use clear, consistent labels
2. **Documentation**: Add notes for special procedures
3. **Categories**: Organize standalone credentials logically
4. **Training**: Ensure team understands vault features
5. **Emergency Plans**: Document incident response procedures

**Common Workflows:**

1. **New Asset Setup**:
   
   * Create asset
   * Create identity vault
   * Add all known credentials
   * Set rotation schedules
   * Grant team access

2. **Contractor Access**:
   
   * Create user account
   * Grant time-limited vault access
   * Monitor access logs
   * Access auto-expires

3. **Security Incident**:
   
   * Emergency rotate affected passwords
   * Document incident in notes
   * Review audit logs
   * Update access permissions
   * Generate incident report

Firmware Management
-------------------

Ferrocodex provides integrated firmware management alongside configuration files, allowing you to track and manage firmware versions for your industrial equipment.

Understanding Firmware Management
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Firmware files are managed separately from configurations but can be linked together:

* **Version Tracking**: Complete history of all firmware versions
* **File Validation**: Checksum verification for integrity
* **Metadata Storage**: Version numbers, release notes, compatibility
* **Analysis Reports**: Security and compliance scanning results
* **Golden Firmware**: Mark verified versions as golden standard

Uploading Firmware
^^^^^^^^^^^^^^^^^^

1. Navigate to an asset's detail view
2. Click the **"Firmware"** tab
3. Click **"Upload Firmware"**
4. Select firmware file(s):
   
   * Binary files (.bin, .hex, .fw)
   * Compressed archives (.zip, .tar.gz)
   * Vendor-specific formats
   * Any firmware file type

5. Enter firmware details:
   
   * **Version**: Firmware version number
   * **Release Date**: When firmware was released
   * **Release Notes**: Changes and improvements
   * **Compatibility**: Supported hardware versions
   * **Criticality**: Security/bug fix priority

6. Click **"Upload"** to save

Firmware Version Management
^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Version List Features:**

* Chronological version history
* Current deployed version indicator
* Golden version highlighting
* File size and upload information
* Quick actions menu

**Version Actions:**

* **Download**: Get firmware file
* **View Details**: See complete metadata
* **Set as Current**: Mark as deployed version
* **Promote to Golden**: Designate as verified
* **Link to Configuration**: Associate with config
* **Delete**: Remove old versions

Firmware Analysis
^^^^^^^^^^^^^^^^^

**Automatic Analysis:**

When firmware is uploaded, Ferrocodex can perform:

1. **File Integrity Check**:
   
   * Calculate checksums
   * Verify file structure
   * Detect corruption

2. **Metadata Extraction**:
   
   * Embedded version info
   * Build timestamps
   * Compiler information

3. **Security Scanning**:
   
   * Known vulnerability checks
   * Suspicious patterns
   * Compliance validation

**Analysis Reports:**

* View detailed scan results
* Security risk assessment
* Compliance status
* Recommendations

Golden Firmware Workflow
^^^^^^^^^^^^^^^^^^^^^^^^

**Promoting to Golden:**

1. Thoroughly test firmware version
2. Verify compatibility
3. Document test results
4. Click **"Promote to Golden"**
5. Add promotion notes
6. Confirm the action

**Golden Version Benefits:**

* Visual distinction in lists
* Deployment recommendations
* Rollback target
* Audit trail entry
* Compliance evidence

Linking Firmware to Configurations
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Creating Associations:**

1. From configuration view:
   
   * Click **"Link Firmware"**
   * Select firmware version
   * Document relationship

2. From firmware view:
   
   * Click **"Link to Config"**
   * Choose configuration
   * Add linking notes

**Benefits of Linking:**

* Track firmware-config pairs
* Ensure compatibility
* Simplify deployments
* Aid troubleshooting
* Support rollbacks

Firmware Deployment Tracking
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Deployment States:**

* **Available**: Uploaded but not deployed
* **Current**: Currently running version
* **Previous**: Replaced versions
* **Golden**: Verified and recommended
* **Deprecated**: Should not be used

**Deployment History:**

* When firmware was deployed
* Who performed deployment
* Associated configurations
* Rollback information
* Success/failure status

Best Practices for Firmware
^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Version Control:**

1. Use semantic versioning
2. Document all changes
3. Test before uploading
4. Keep release notes updated
5. Archive old versions

**Security Practices:**

1. Verify firmware sources
2. Check digital signatures
3. Scan for vulnerabilities
4. Monitor vendor advisories
5. Plan emergency updates

**Organizational Tips:**

1. Standardize version formats
2. Regular firmware audits
3. Deployment procedures
4. Rollback planning
5. Team training

Search and Filter
-----------------

Global Search
^^^^^^^^^^^^^

Use the search bar in the top navigation to find:

* Assets by name, type, or location
* Configurations by filename
* Users (administrators only)
* Audit entries

Advanced Filters
^^^^^^^^^^^^^^^^

On list pages, use filters to narrow results:

* **Date Range**: Filter by creation/modification date
* **Type**: Filter by asset or file type
* **User**: Filter by who created/modified
* **Tags**: Filter by custom tags

Audit Trail
-----------

Viewing Audit Logs
^^^^^^^^^^^^^^^^^^

The audit system tracks all system activities:

1. Navigate to **Audit Log** from sidebar
2. View comprehensive activity list:

   * User actions
   * Configuration changes
   * Login attempts
   * System events

3. Each entry shows:

   * Timestamp
   * User
   * Action performed
   * Affected resource
   * Result (success/failure)

Filtering Audit Logs
^^^^^^^^^^^^^^^^^^^^

* Filter by date range
* Search by user
* Filter by action type
* Export filtered results

Compliance Features
^^^^^^^^^^^^^^^^^^^

* Immutable audit trail
* Cryptographic verification
* Regulatory compliance support
* Retention policies

User Settings
-------------

Profile Management
^^^^^^^^^^^^^^^^^^

Access your profile from the user menu:

1. Click your username (top right)
2. Select **"Profile"**
3. Update:

   * Display name
   * Email address
   * Password
   * Preferences

Password Requirements
^^^^^^^^^^^^^^^^^^^^^

* Minimum 8 characters
* Mix of letters and numbers
* Regular password changes recommended
* No password reuse for 5 changes

Session Management
^^^^^^^^^^^^^^^^^^

* Sessions expire after inactivity
* Concurrent session limits
* Manual logout recommended
* Session activity in audit log

Keyboard Shortcuts
------------------

.. list-table::
   :header-rows: 1

   * - Shortcut
     - Action
   * - ``Ctrl/Cmd + K``
     - Quick search
   * - ``Ctrl/Cmd + N``
     - New asset
   * - ``Ctrl/Cmd + U``
     - Upload configuration
   * - ``Ctrl/Cmd + V``
     - Open Identity Vault
   * - ``Ctrl/Cmd + G``
     - Generate password
   * - ``Ctrl/Cmd + R``
     - Rotate selected password
   * - ``Esc``
     - Close dialog/modal
   * - ``?``
     - Show keyboard shortcuts

Tips and Best Practices
-----------------------

Configuration Management
^^^^^^^^^^^^^^^^^^^^^^^^

1. **Version Everything**: Upload configs regularly
2. **Use Branches**: Test changes safely
3. **Document Changes**: Add detailed notes
4. **Regular Backups**: Export data periodically

Security Best Practices
^^^^^^^^^^^^^^^^^^^^^^^

1. **Strong Passwords**: Use complex passwords
2. **Limit Access**: Only necessary permissions
3. **Regular Audits**: Review audit logs
4. **Logout**: When finished working
5. **Secure Storage**: Protect exported files
6. **Vault Security**: Never share vault passwords outside the system
7. **Rotation Compliance**: Follow password rotation schedules
8. **Access Reviews**: Regularly review vault permissions
9. **Emergency Plans**: Have incident response procedures ready

Workflow Optimization
^^^^^^^^^^^^^^^^^^^^^

1. **Naming Conventions**: Establish standards
2. **Tag System**: Use consistent tags
3. **Templates**: Create config templates
4. **Training**: Ensure team knowledge
5. **Documentation**: Keep notes updated