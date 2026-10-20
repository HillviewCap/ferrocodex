Asset Hierarchy Management Guide
=================================

The Asset Hierarchy Management system in Ferrocodex organizes industrial assets in folder-based structures with custom metadata and tree filtering.

.. contents:: Table of Contents
   :local:
   :depth: 3

Overview
--------

The Asset Hierarchy Management system provides:

* **Hierarchical Organization**: Create unlimited nested folder structures
* **Two Asset Types**: Folders (containers) and Devices (equipment)
* **Drag-and-Drop Management**: Reorganization of assets
* **Tree Filtering**: Narrow the tree by name, description, and asset type
* **Custom Metadata**: Flexible field definitions with validation
* **Naming Validation**: Enforced naming pattern and input validation
* **Bulk Operations**: Efficient management of multiple assets

Understanding Asset Types
-------------------------

Folder Assets
^^^^^^^^^^^^^

Folder assets act as organizational containers for grouping related equipment:

* **Purpose**: Organize devices into logical groups
* **Nesting**: Unlimited depth for complex hierarchies
* **Examples**:

  * Production Lines
  * Facility Areas
  * Equipment Categories
  * Geographic Locations

**Creating Folder Assets:**

1. Navigate to **Assets** from the sidebar
2. Click **"Create Folder"**, or right-click a folder in the tree and choose **"Create Folder"**
3. Enter folder details:

   * **Folder Name**: Required
   * **Description**: Purpose of the folder
   * **Parent Folder**: Defaults to the folder you right-clicked, or the root level. Hidden until at least one folder exists.
   * **Security Classification**: Pre-filled from the parent folder; it can only be raised (see `Security Classification`_)

4. Click **"Create Folder"** to save

Device Assets
^^^^^^^^^^^^^

Device assets represent actual industrial equipment:

* **Purpose**: Individual equipment records
* **Location**: Can exist in folders or at root level
* **Configuration Association**: Link to configuration files
* **Examples**:

  * PLCs (PLC-WEST-01)
  * HMIs (HMI-LINE2-03)
  * SCADA Systems (SCADA-MAIN-01)
  * Network Equipment (SWITCH-PROD-05)

**Creating Device Assets:**

1. Click **"Create Device"**, or right-click a folder and choose **"Create Device"**
2. Choose the parent folder, or leave it empty for the root level (skipped when no folders exist)
3. Complete device information:

   * **Device Name**: Unique identifier
   * **Description**: Additional details
   * **Security Classification**: Pre-filled from the parent folder; it can only be raised

4. Review and click **"Create Device"** to save

Hierarchical Navigation
-----------------------

Tree View Interface
^^^^^^^^^^^^^^^^^^^

The tree navigation provides browsing of your asset hierarchy:

.. figure:: _static/images/asset-tree-view.png
   :alt: Asset hierarchy tree navigation
   :align: center
   :width: 800px

   *Hierarchical tree view with folders and devices*

**Tree View Features:**

* **Expand/Collapse**: Click arrows to navigate folders
* **Visual Indicators**:

  * Folder icons for containers
  * Device icons for equipment
  * Configuration count badges
  * Security status indicators

* **Context Menus**: Right-click for quick actions
* **Breadcrumb Navigation**: Path display at top
* **Performance**: Virtualized rendering for large trees

Drag-and-Drop Organization
^^^^^^^^^^^^^^^^^^^^^^^^^^

Reorganize assets with drag-and-drop:

1. **Select Asset**: Click to select source asset
2. **Drag**: Hold and drag to target location
3. **Drop Indicators**: Visual feedback shows valid targets
4. **Confirm**: Release to move asset
5. **Undo**: Available if needed

**Drag-and-Drop Rules:**

* Devices can move between any folders
* Folders can be nested within other folders
* Multiple selection supported (Ctrl/Cmd+Click)
* Cross-hierarchy moves allowed
* Validation prevents circular references

Keyboard Navigation
^^^^^^^^^^^^^^^^^^^

Power users can navigate entirely via keyboard:

.. list-table:: Keyboard Shortcuts
   :header-rows: 1
   :widths: 30 70

   * - Shortcut
     - Action
   * - ``Arrow Up/Down``
     - Navigate between assets
   * - ``Arrow Left``
     - Collapse folder or move to parent
   * - ``Arrow Right``
     - Expand folder or move to first child
   * - ``Enter``
     - Open asset details
   * - ``Space``
     - Select/deselect asset
   * - ``Ctrl+A``
     - Select all visible assets
   * - ``Delete``
     - Delete selected assets
   * - ``F2``
     - Rename selected asset
   * - ``Ctrl+X/C/V``
     - Cut/Copy/Paste assets
   * - ``/``
     - Focus search bar

Custom Metadata System
----------------------

Understanding Metadata
^^^^^^^^^^^^^^^^^^^^^^

Metadata allows capturing equipment-specific information beyond basic fields:

* **Flexibility**: Define fields specific to your needs
* **Validation**: Ensure data quality with rules
* **Templates**: Reusable field configurations

Field Types
^^^^^^^^^^^

Ferrocodex supports various metadata field types:

.. list-table:: Available Field Types
   :header-rows: 1
   :widths: 20 40 40

   * - Type
     - Description
     - Example Use
   * - **Text**
     - Single-line text input
     - Serial numbers, names
   * - **Text Area**
     - Multi-line text input
     - Notes, descriptions
   * - **Number**
     - Numeric values
     - Counts, measurements
   * - **Date**
     - Date picker
     - Install dates, warranties
   * - **Dropdown**
     - Predefined options
     - Status, categories
   * - **Checkbox**
     - Boolean values
     - Feature flags, compliance
   * - **IP Address**
     - IPv4/IPv6 validation
     - Network addresses
   * - **URL**
     - Web address validation
     - Documentation links

Creating Custom Fields
^^^^^^^^^^^^^^^^^^^^^^

**Adding Fields to Assets:**

1. Select an asset or create new
2. Click **"Manage Metadata"**
3. Click **"Add Field"**
4. Configure field properties:

   * **Field Name**: Internal identifier
   * **Display Label**: User-friendly name
   * **Field Type**: Select from available types
   * **Required**: Mark as mandatory
   * **Validation Rules**: Pattern, range, etc.
   * **Default Value**: Pre-filled value
   * **Help Text**: User guidance

5. Click **"Save Field"**

Field Templates
^^^^^^^^^^^^^^^

Ferrocodex includes pre-built field templates:

**Network Equipment Template:**

* IP Address (IPv4/IPv6)
* Subnet Mask
* Gateway
* VLAN ID
* MAC Address
* Port Configuration

**Location Template:**

* Facility Name
* Building
* Floor
* Room
* Rack/Cabinet
* GPS Coordinates

**Maintenance Template:**

* Install Date
* Last Service Date
* Next Service Due
* Service Contract
* Warranty Expiration
* Service Notes

**Applying Templates:**

1. Click **"Apply Template"**
2. Select desired template
3. Review fields to be added
4. Customize as needed
5. Click **"Apply"**

Validation Rules
^^^^^^^^^^^^^^^^

Ensure data quality with validation:

**Text Validation:**

* Pattern matching (regex)
* Length constraints
* Character restrictions
* Case requirements

**Number Validation:**

* Minimum/maximum values
* Decimal places
* Step increments
* Positive only

**Date Validation:**

* Date ranges
* Future/past only
* Business days only
* Relative constraints

**Custom Validation:**

* Cross-field validation
* External system checks
* Business rule enforcement

Filtering the Asset Tree
------------------------

The search box above the asset tree narrows the tree to matching assets:

* **Text match**: Type part of an asset name or description; matching is case-insensitive
* **Type filter**: Show all assets, only folders, or only devices
* **Context**: The parent folders of each match are expanded so you can see where it sits in the hierarchy
* **Clear**: Use the clear button to restore the full tree

The filter matches asset names and descriptions only. Custom metadata values are not searched.

Asset Naming Rules
------------------

Naming Requirements
^^^^^^^^^^^^^^^^^^^

All asset names must match the naming pattern:

**Pattern Requirements:**

.. code-block:: text

   ^[A-Z0-9][A-Z0-9_-]{2,49}$

**Rules Explained:**

* Start with letter or number (A-Z, 0-9)
* Contain only uppercase letters, numbers, underscore, hyphen
* Length between 3-50 characters
* No spaces or special characters
* No leading/trailing underscores or hyphens

**Valid Examples:**

* ``PLC-WEST-01``
* ``HMI_LINE2_03``
* ``SCADA-MAIN-01``
* ``PUMP-STATION-A5``
* ``SENSOR_TEMP_001``

**Invalid Examples:**

* ``plc-west-01`` (lowercase not allowed)
* ``PLC WEST 01`` (spaces not allowed)
* ``_PLC-WEST`` (leading underscore)
* ``PLC.WEST.01`` (dots not allowed)
* ``AB`` (too short)

Reserved Names
^^^^^^^^^^^^^^

The following Windows reserved names are blocked:

* ``CON``, ``PRN``, ``AUX``, ``NUL``
* ``COM1`` through ``COM9``
* ``LPT1`` through ``LPT9``

These restrictions prevent system conflicts and security issues.

File Upload Validation
^^^^^^^^^^^^^^^^^^^^^^

When uploading files to assets:

1. **Filename Sanitization**: Automatic cleaning
2. **Extension Validation**: Checked against allowlist
3. **Path Traversal Prevention**: No ``../`` sequences
4. **Unicode Normalization**: Consistent encoding
5. **Length Limits**: Maximum 255 characters

Security Classification
^^^^^^^^^^^^^^^^^^^^^^^

Every folder and device has one of five security classifications, from lowest to highest:

* **Public**: Can be shared freely
* **Internal**: Internal use only (the default)
* **Confidential**: Controlled access
* **Restricted**: Highly sensitive, limited access
* **Secret**: Highest level

The classification appears as a tag in the asset tree and the details panel. Folders pass it down:

* **Inheritance**: A new asset starts at its parent folder's classification, or **Internal** at the root. It can be set higher, never lower.
* **Moves**: Moving an asset into a folder classified above it, or above anything inside it, is rejected with a message naming the levels. Nothing is raised automatically.
* **Changes**: Click **"Change"** next to the tag in the details panel. An asset can't go below its parent folder. Raising a folder raises everything inside it that is classified lower, in one step that is recorded in the audit log; the dialog shows how many items will be raised first. Only Administrators can lower a classification, and lowering a folder leaves its contents unchanged.
* **Existing assets**: Assets created before classifications were introduced start at **Internal** and are left as they are.

Bulk Operations
---------------

Import Operations
^^^^^^^^^^^^^^^^^

Import multiple assets from external sources:

**CSV Import:**

1. Prepare CSV file with headers:

   .. code-block:: text

      Name,Type,Manufacturer,Model,Location,IP_Address
      PLC-001,Device,Siemens,S7-1500,Line 1,192.168.1.10
      HMI-001,Device,Rockwell,PanelView,Line 1,192.168.1.11

2. Navigate to **Import/Export** section
3. Select **"Import from CSV"**
4. Map CSV columns to fields
5. Preview import data
6. Confirm import

**JSON Import:**

For complex hierarchies with metadata:

.. code-block:: json

   {
     "assets": [
       {
         "name": "PRODUCTION-LINE-1",
         "type": "folder",
         "children": [
           {
             "name": "PLC-LINE1-01",
             "type": "device",
             "metadata": {
               "ip_address": "192.168.1.10",
               "install_date": "2024-01-15"
             }
           }
         ]
       }
     ]
   }

Export Operations
^^^^^^^^^^^^^^^^^

Export assets for backup or migration:

**Export Options:**

* **Format**: CSV, JSON, or ZIP bundle
* **Scope**: Selected assets or entire hierarchy
* **Include**: Metadata, configurations, audit trails
* **Encryption**: Optional AES-256 encryption

**Export Process:**

1. Select assets to export
2. Click **"Export"** button
3. Choose export format
4. Configure options
5. Download export file

Bulk Rename
^^^^^^^^^^^

Rename multiple assets efficiently:

1. Select target assets
2. Click **"Bulk Rename"**
3. Choose rename pattern:

   * Find and replace
   * Add prefix/suffix
   * Sequential numbering
   * Case conversion

4. Preview changes
5. Confirm rename operation

**Rename Examples:**

.. code-block:: text

   Add prefix:
   Selected: [PUMP-01, PUMP-02]
   Pattern: Add prefix "MAIN-"
   Result: [MAIN-PUMP-01, MAIN-PUMP-02]

   Sequential numbering:
   Selected: [SENSOR, SENSOR, SENSOR]
   Pattern: Append number "-%03d"
   Result: [SENSOR-001, SENSOR-002, SENSOR-003]

Batch Metadata Updates
^^^^^^^^^^^^^^^^^^^^^^

Update metadata across multiple assets:

1. Select target assets
2. Click **"Update Metadata"**
3. Choose update mode:

   * **Replace**: Overwrite existing values
   * **Append**: Add to existing values
   * **Merge**: Combine with existing

4. Enter new values
5. Preview changes
6. Apply updates

Multi-Asset Selection
^^^^^^^^^^^^^^^^^^^^^

Select multiple assets for bulk operations:

* **Click**: Select single asset
* **Ctrl+Click**: Add to selection
* **Shift+Click**: Select range
* **Ctrl+A**: Select all visible
* **Selection Box**: Drag to select area

Performance Optimization
------------------------

Tree Rendering
^^^^^^^^^^^^^^

Efficient rendering for large hierarchies:

**Virtualization:**

* Only visible nodes rendered
* Dynamic loading on scroll
* Memory-efficient data structures
* Lazy loading of children

**Performance Tips:**

* Collapse unused branches
* Use the tree filter to navigate
* Filter to reduce nodes
* Archive old assets

Bulk Operation Performance
^^^^^^^^^^^^^^^^^^^^^^^^^^

Handling large-scale operations:

**Batch Processing:**

* Operations in chunks
* Progress reporting
* Pause/resume capability
* Error recovery

**Resource Management:**

* Memory limits enforced
* CPU throttling available
* Network optimization
* Database connection pooling

Best Practices
--------------

Organizational Structure
^^^^^^^^^^^^^^^^^^^^^^^^

**Recommended Hierarchy:**

.. code-block:: text

   Root
   ├── FACILITIES
   │   ├── PLANT-NORTH
   │   │   ├── PRODUCTION-LINE-1
   │   │   │   ├── PLC-LINE1-01
   │   │   │   ├── HMI-LINE1-01
   │   │   │   └── SENSORS
   │   │   │       ├── TEMP-SENSOR-001
   │   │   │       └── PRESS-SENSOR-001
   │   │   └── PRODUCTION-LINE-2
   │   └── PLANT-SOUTH
   └── INFRASTRUCTURE
       ├── NETWORK
       │   ├── SWITCHES
       │   └── ROUTERS
       └── SERVERS

**Guidelines:**

1. Use consistent naming conventions
2. Limit hierarchy depth to 5-7 levels
3. Group by function or location
4. Separate infrastructure from production
5. Archive inactive assets

Metadata Management
^^^^^^^^^^^^^^^^^^^

**Best Practices:**

1. **Standardize Fields**: Use templates
2. **Required Fields**: Minimize mandatory fields
3. **Validation**: Implement appropriate rules
4. **Documentation**: Provide help text
5. **Review**: Regular metadata audits

**Common Metadata Fields:**

* **Identification**: Serial, asset tag, barcode
* **Network**: IP, MAC, hostname, port
* **Location**: Building, floor, room, rack
* **Maintenance**: Install date, warranty, service
* **Compliance**: Certification, audit date
* **Financial**: Cost, depreciation, owner

Security Considerations
^^^^^^^^^^^^^^^^^^^^^^^

**Security Guidelines:**

1. **Naming**: Follow security patterns
2. **Classification**: Tag sensitive assets
3. **Access Control**: Limit permissions
4. **Audit**: Review access logs
5. **Validation**: Enforce input rules

**Compliance Requirements:**

1. Document security classifications
2. Regular security audits
3. Access review cycles
4. Incident response procedures
5. Training on security practices

Troubleshooting
---------------

Common Issues
^^^^^^^^^^^^^

**Asset Creation Failures:**

* **Issue**: "Invalid asset name"
* **Solution**: Ensure name follows pattern ``^[A-Z0-9][A-Z0-9_-]{2,49}$``

* **Issue**: "Parent folder not found"
* **Solution**: Verify folder exists and you have access

* **Issue**: "Duplicate asset name"
* **Solution**: Use unique names within same parent

**Tree Filter Problems:**

* **Issue**: No assets match the filter
* **Solution**: Check spelling and set the type filter to "All"; the filter matches names and descriptions, not metadata values

**Drag-and-Drop Issues:**

* **Issue**: Cannot drop asset
* **Solution**: Check permissions for target folder

* **Issue**: Drag not working
* **Solution**: Restart the application, or use the keyboard shortcuts

Performance Issues
^^^^^^^^^^^^^^^^^^

**Slow Tree Loading:**

1. Check asset count (Settings → Statistics)
2. Archive unused assets
3. Collapse large branches
4. Restart the application

Import/Export Problems
^^^^^^^^^^^^^^^^^^^^^^

**Import Failures:**

* Validate CSV/JSON format
* Check for invalid characters
* Verify required fields present
* Review error log for details
* Try smaller batches

**Export Issues:**

* Check available disk space
* Verify export permissions
* Try different format
* Export smaller selection
* Check audit logs

Conclusion
----------

The Asset Hierarchy Management system organizes industrial equipment at scale. Hierarchical structures, custom metadata, and bulk operations let you manage thousands of assets while maintaining security and compliance requirements.

For additional assistance, consult the :doc:`user-guide` or :doc:`troubleshooting` sections, or contact your system administrator.