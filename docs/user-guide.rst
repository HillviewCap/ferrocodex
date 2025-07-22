User Guide
==========

This comprehensive guide covers all features and functionality in Ferrocodex.

.. contents:: Table of Contents
   :local:
   :depth: 2

Dashboard
---------

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

Workflow Optimization
^^^^^^^^^^^^^^^^^^^^^

1. **Naming Conventions**: Establish standards
2. **Tag System**: Use consistent tags
3. **Templates**: Create config templates
4. **Training**: Ensure team knowledge
5. **Documentation**: Keep notes updated