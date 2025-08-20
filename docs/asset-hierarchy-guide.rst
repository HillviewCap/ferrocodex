Asset Hierarchy Management Guide
=================================

The Asset Hierarchy Management system in Ferrocodex v0.5.0 provides comprehensive hierarchical organization capabilities for industrial assets, enabling intuitive folder-based structures with advanced metadata management and search functionality.

.. contents:: Table of Contents
   :local:
   :depth: 3

Overview
--------

The Asset Hierarchy Management system transforms how you organize and manage industrial equipment by providing:

* **Hierarchical Organization**: Create unlimited nested folder structures
* **Two Asset Types**: Folders (containers) and Devices (equipment)
* **Drag-and-Drop Management**: Intuitive reorganization of assets
* **Advanced Search**: Full-text search with sub-200ms performance
* **Custom Metadata**: Flexible field definitions with validation
* **Cybersecurity Compliance**: Secure naming conventions and validation
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
2. Click **"Add Asset"** button
3. Select **"Folder"** as asset type
4. Enter folder details:
   
   * **Name**: Following security naming pattern
   * **Description**: Purpose of the folder
   * **Parent Folder**: Location in hierarchy
   * **Metadata**: Custom fields as needed

5. Click **"Create"** to save

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

1. Navigate to target folder location
2. Click **"Add Device"** button
3. Complete device information:
   
   * **Name**: Unique identifier
   * **Type**: Equipment category
   * **Manufacturer**: Device vendor
   * **Model**: Specific model number
   * **Custom Metadata**: Additional fields

4. Click **"Create"** to save

Hierarchical Navigation
-----------------------

Tree View Interface
^^^^^^^^^^^^^^^^^^^

The tree navigation provides intuitive browsing of your asset hierarchy:

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

Reorganize assets easily with drag-and-drop:

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
* **Searchability**: All metadata is searchable
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

Advanced Search & Filtering
---------------------------

Full-Text Search
^^^^^^^^^^^^^^^^

The search system uses SQLite FTS5 for powerful text searching:

**Search Features:**

* **Instant Results**: Sub-200ms response time
* **Fuzzy Matching**: Tolerates typos
* **Relevance Ranking**: Best matches first
* **Highlighting**: Matched terms highlighted
* **Auto-complete**: Suggestions as you type

**Search Syntax:**

.. code-block:: text

   Simple search:
   pump                    # Find all assets containing "pump"
   
   Phrase search:
   "cooling pump"          # Exact phrase match
   
   Field-specific search:
   manufacturer:siemens    # Search specific field
   
   Wildcard search:
   PLC-*                   # Matches PLC-001, PLC-002, etc.
   
   Boolean operators:
   pump AND cooling        # Both terms required
   pump OR fan             # Either term
   pump NOT broken         # Exclude term

Filter Builder
^^^^^^^^^^^^^^

Create complex filters with the visual builder:

1. Click **"Advanced Filter"** button
2. Add filter conditions:
   
   * Select field
   * Choose operator
   * Enter value

3. Combine conditions:
   
   * AND: All conditions must match
   * OR: Any condition matches
   * NOT: Exclude matches

4. Save as preset for reuse

**Filter Examples:**

.. code-block:: text

   Recently Added Devices:
   - Asset Type = "Device"
   - AND Created Date > "30 days ago"
   
   Critical Production Equipment:
   - Location = "Production Floor"
   - AND Criticality = "High"
   - AND Status = "Active"
   
   Maintenance Due:
   - Next Service < "7 days from now"
   - OR Last Service > "365 days ago"

Search Integration
^^^^^^^^^^^^^^^^^^

Search is integrated throughout the application:

* **Global Search Bar**: Always accessible
* **Context Search**: Within current folder
* **Quick Filters**: Common searches
* **Search History**: Recent searches
* **Saved Searches**: Custom filter presets

Similar Asset Discovery
^^^^^^^^^^^^^^^^^^^^^^^

Find assets with similar characteristics:

1. Select reference asset
2. Click **"Find Similar"**
3. Choose similarity criteria:
   
   * Metadata fields
   * Asset type
   * Manufacturer
   * Location

4. Set similarity threshold
5. View matched assets

Cybersecurity-Compliant Naming
-------------------------------

Naming Requirements
^^^^^^^^^^^^^^^^^^^

All asset names must follow security best practices:

**Pattern Requirements:**

.. code-block:: regex

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

Assets can be tagged with security classifications:

* **Public**: No restrictions
* **Internal**: Company use only
* **Confidential**: Restricted access
* **Secret**: Highly restricted
* **Top Secret**: Maximum security

Classifications affect:

* Search result visibility
* Export permissions
* Audit logging detail
* Access control rules

Bulk Operations
---------------

Import Operations
^^^^^^^^^^^^^^^^^

Import multiple assets from external sources:

**CSV Import:**

1. Prepare CSV file with headers:
   
   .. code-block:: csv
   
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

Workflow Management
-------------------

Multi-Step Workflows
^^^^^^^^^^^^^^^^^^^^

Create structured workflows for complex operations:

**Workflow Components:**

* **Steps**: Sequential tasks
* **Conditions**: Branching logic
* **Validations**: Data checks
* **Approvals**: Review gates
* **Notifications**: Status updates

**Example Workflow - New Equipment Setup:**

1. **Asset Creation**
   
   * Create device asset
   * Apply metadata template
   * Set security classification

2. **Configuration**
   
   * Upload initial configuration
   * Link firmware version
   * Set baseline

3. **Identity Vault**
   
   * Create vault entry
   * Add credentials
   * Set rotation schedule

4. **Documentation**
   
   * Upload manuals
   * Add maintenance procedures
   * Link to vendor resources

5. **Validation**
   
   * Verify all required fields
   * Check security compliance
   * Confirm network settings

6. **Approval**
   
   * Submit for review
   * Administrator approval
   * Activate asset

Draft Management
^^^^^^^^^^^^^^^^

Save incomplete work as drafts:

**Draft Features:**

* **Auto-save**: Every 30 seconds
* **Manual Save**: Explicit draft creation
* **Resume Later**: Continue from any device
* **Version History**: Track draft changes
* **Sharing**: Collaborate on drafts

**Working with Drafts:**

1. Start creating/editing asset
2. Click **"Save as Draft"**
3. Add draft notes (optional)
4. Resume from **"My Drafts"** section
5. Publish when complete

Progress Tracking
^^^^^^^^^^^^^^^^^

Monitor workflow progress:

**Progress Indicators:**

* **Step Counter**: "Step 3 of 7"
* **Progress Bar**: Visual completion
* **Time Estimates**: Expected duration
* **Status Tags**: Pending, In Progress, Complete
* **Blockers**: Issues preventing progress

**Progress Dashboard:**

View all active workflows:

* Filter by status
* Sort by priority
* View assignees
* Check due dates
* Export reports

Performance Optimization
------------------------

Search Performance
^^^^^^^^^^^^^^^^^^

Optimizations for sub-200ms search:

**Database Optimizations:**

* SQLite FTS5 full-text indexing
* Optimized query plans
* Efficient JOIN strategies
* Index maintenance routines

**Caching Strategy:**

* Result caching with TTL
* Query plan caching
* Metadata caching
* Invalidation on updates

**Background Processing:**

* Async index updates
* Batch processing
* Queue management
* Priority scheduling

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
* Use search to navigate
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

Search Optimization
^^^^^^^^^^^^^^^^^^^

**Search Tips:**

1. Use specific terms
2. Leverage field-specific search
3. Save common searches
4. Use filters to narrow results
5. Learn search syntax

**Performance Tips:**

1. Index custom fields used in search
2. Archive old assets
3. Optimize metadata schemas
4. Regular database maintenance
5. Monitor search analytics

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

**Search Problems:**

* **Issue**: No search results
* **Solution**: Check spelling, try broader terms

* **Issue**: Slow search performance
* **Solution**: Rebuild search indexes in settings

* **Issue**: Missing metadata in results
* **Solution**: Verify metadata is indexed

**Drag-and-Drop Issues:**

* **Issue**: Cannot drop asset
* **Solution**: Check permissions for target folder

* **Issue**: Drag not working
* **Solution**: Ensure JavaScript enabled, try keyboard shortcuts

Performance Issues
^^^^^^^^^^^^^^^^^^

**Slow Tree Loading:**

1. Check asset count (Settings → Statistics)
2. Archive unused assets
3. Collapse large branches
4. Clear browser cache
5. Check network latency

**Search Performance:**

1. Rebuild search indexes
2. Optimize database (Admin → Maintenance)
3. Review custom metadata fields
4. Check system resources
5. Contact support if persists

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

The Asset Hierarchy Management system in Ferrocodex v0.5.0 provides powerful organizational capabilities for managing industrial equipment at scale. By leveraging hierarchical structures, custom metadata, advanced search, and bulk operations, you can efficiently organize and manage thousands of assets while maintaining security and compliance requirements.

For additional assistance, consult the :doc:`user-guide` or :doc:`troubleshooting` sections, or contact your system administrator.