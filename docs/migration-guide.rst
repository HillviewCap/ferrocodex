Migration Guide
===============

This guide provides detailed instructions for migrating between Ferrocodex versions, handling data migrations, and managing breaking changes.

.. contents:: Table of Contents
   :local:
   :depth: 2

Overview
--------

Migration planning is essential for maintaining data integrity and system availability when upgrading Ferrocodex. This guide covers migrations from version to version, including major releases with breaking changes.

Migration Principles
^^^^^^^^^^^^^^^^^^^^

* **Data Preservation**: No data loss during migration
* **Rollback Capability**: Ability to revert if issues arise
* **Minimal Downtime**: Efficient migration process
* **Compatibility**: Handle version differences gracefully
* **Audit Trail**: Maintain compliance records

Version Compatibility
^^^^^^^^^^^^^^^^^^^^^

.. figure:: _static/diagrams/migration-paths.svg
   :alt: Version migration paths diagram
   :align: center
   :width: 700px

   *Supported migration paths between Ferrocodex versions*

.. list-table::
   :header-rows: 1

   * - From Version
     - To Version
     - Migration Type
     - Downtime Required
   * - 0.1.x
     - 0.2.x
     - Automatic
     - None
   * - 0.2.x
     - 0.3.x
     - Automatic
     - None
   * - 0.3.x
     - 0.4.x
     - Semi-automatic
     - Minimal
   * - Any
     - 0.4.x
     - Manual possible
     - Extended

Pre-Migration Planning
----------------------

System Assessment
^^^^^^^^^^^^^^^^^

Before beginning migration:

1. **Current State Documentation**:
   
   * Current version number
   * Number of users
   * Number of assets
   * Database size
   * Custom configurations

2. **Resource Requirements**:
   
   * Available disk space (2x current)
   * Backup storage location
   * Migration window duration
   * Personnel availability

3. **Risk Assessment**:
   
   * Critical operations impact
   * User access requirements
   * Compliance considerations
   * Rollback triggers

Backup Procedures
^^^^^^^^^^^^^^^^^

**Complete System Backup:**

1. **Stop Application**:
   
   .. code-block:: bash
   
      # Windows
      taskkill /F /IM Ferrocodex.exe
      
      # macOS/Linux
      pkill Ferrocodex

2. **Export Data**:
   
   * Use Admin → Export → Full System
   * Include all options:
     - Assets and configurations
     - User accounts
     - Audit logs
     - Vault data (if applicable)
     - System settings

3. **Database Backup**:
   
   .. code-block:: bash
   
      # Locate database file
      # Windows: %APPDATA%\Ferrocodex\data.db
      # macOS: ~/Library/Application Support/Ferrocodex/data.db
      # Linux: ~/.config/ferrocodex/data.db
      
      # Create backup copy
      cp data.db data.db.backup-$(date +%Y%m%d)

4. **Configuration Backup**:
   
   * Copy settings files
   * Document customizations
   * Save license information

Migration Testing
^^^^^^^^^^^^^^^^^

**Test Environment Setup:**

1. Create isolated test system
2. Restore backup to test
3. Perform test migration
4. Verify functionality
5. Document issues
6. Plan remediation

**Test Checklist:**

- [ ] Application starts correctly
- [ ] Users can login
- [ ] Assets display properly
- [ ] Configurations accessible
- [ ] Vault data intact (v0.4.0+)
- [ ] Audit logs preserved
- [ ] Performance acceptable

Version-Specific Migrations
---------------------------

Migrating to v0.4.0 (Asset Identity Vault)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Major Changes:**

* New Identity Vault feature
* Enhanced permission system
* Standalone credentials
* Password rotation tracking

**Migration Steps:**

1. **Pre-Migration**:
   
   * Backup current system
   * Document vault requirements
   * Plan permission structure
   * Notify users of new features

2. **Installation**:
   
   .. code-block:: bash
   
      # 1. Stop current version
      # 2. Backup database
      # 3. Install new version
      # 4. Do NOT start yet

3. **Database Migration**:
   
   * Automatic on first launch
   * Creates vault tables
   * Migrates permissions
   * Adds rotation tracking

4. **Post-Migration**:
   
   * Verify vault creation ability
   * Test permission grants
   * Configure rotation policies
   * Train users on features

**New Configuration Options:**

.. code-block:: json

   {
     "vault": {
       "passwordPolicy": {
         "minLength": 12,
         "requireUppercase": true,
         "requireNumbers": true,
         "requireSpecial": true,
         "historyDepth": 5
       },
       "rotation": {
         "defaultDays": 90,
         "warningDays": 7,
         "criticalAssetDays": 30
       }
     }
   }

Migrating from v0.2.x to v0.3.x
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Major Changes:**

* Firmware management added
* Enhanced branching
* Performance improvements

**Migration Process:**

1. Export configurations
2. Install new version
3. Import configurations
4. Enable firmware features
5. Update user training

Migrating from v0.1.x to v0.2.x
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Major Changes:**

* Improved audit system
* Branch management
* Enhanced security

**Simple Upgrade:**

1. Backup database
2. Install new version
3. Automatic migration
4. Verify functionality

Data Migration Procedures
-------------------------

Export/Import Method
^^^^^^^^^^^^^^^^^^^^

For major version jumps or clean installations:

1. **Export from Old Version**:
   
   .. code-block:: text
   
      1. Login as Administrator
      2. Navigate to Settings → Export
      3. Select "Full System Export"
      4. Choose all options:
         ☑ Assets
         ☑ Configurations  
         ☑ Users
         ☑ Audit Logs
         ☑ Settings
      5. Save export file

2. **Prepare New System**:
   
   * Fresh installation
   * Initial admin setup
   * Basic configuration

3. **Import to New Version**:
   
   .. code-block:: text
   
      1. Login to new system
      2. Settings → Import
      3. Select export file
      4. Review import preview
      5. Handle conflicts:
         - Skip existing
         - Overwrite
         - Merge
      6. Execute import

4. **Verification**:
   
   * Count records
   * Spot check data
   * Test functionality
   * Validate permissions

Database Migration
^^^^^^^^^^^^^^^^^^

For supported version upgrades:

**Automatic Migration:**

1. Install new version
2. Start application
3. Migration prompt appears
4. Confirm to proceed
5. Wait for completion
6. Verify success

**Manual Migration:**

If automatic fails:

.. code-block:: bash

   # Run migration tool
   Ferrocodex.exe --migrate --from 0.3.0 --to 0.4.0
   
   # With verbose logging
   Ferrocodex.exe --migrate --verbose --log migration.log

**Migration Verification:**

.. code-block:: sql

   -- Check version
   SELECT * FROM schema_version;
   
   -- Verify record counts
   SELECT COUNT(*) FROM assets;
   SELECT COUNT(*) FROM configurations;
   SELECT COUNT(*) FROM users;

Handling Migration Issues
-------------------------

Common Problems
^^^^^^^^^^^^^^^

**Database Locked:**

*Symptom:* Migration fails with "database locked" error

*Solution:*

1. Ensure application stopped
2. Check for hung processes
3. Restart system if needed
4. Retry migration

**Insufficient Space:**

*Symptom:* Migration fails partway

*Solution:*

1. Free disk space (need 2x database size)
2. Move to larger disk
3. Archive old data first
4. Retry migration

**Permission Errors:**

*Symptom:* Cannot write to database

*Solution:*

1. Run as administrator
2. Check file permissions
3. Verify ownership
4. Fix permissions

**Schema Conflicts:**

*Symptom:* Table already exists errors

*Solution:*

1. Backup current database
2. Use clean database
3. Import after migration
4. Merge manually if needed

Rollback Procedures
^^^^^^^^^^^^^^^^^^^

If migration fails:

1. **Immediate Rollback**:
   
   .. code-block:: bash
   
      # Stop application
      # Restore database backup
      mv data.db data.db.failed
      cp data.db.backup-20250127 data.db
      
      # Reinstall previous version
      # Start application

2. **Data Recovery**:
   
   * Export partial data
   * Manual correction
   * Selective import
   * Verify integrity

3. **Investigation**:
   
   * Review migration logs
   * Identify failure point
   * Plan remediation
   * Retry with fixes

Post-Migration Tasks
--------------------

Verification Checklist
^^^^^^^^^^^^^^^^^^^^^^

**System Functionality:**

- [ ] Application starts normally
- [ ] No error messages on startup
- [ ] Dashboard loads correctly
- [ ] Navigation works properly

**Data Integrity:**

- [ ] User accounts present
- [ ] All assets visible
- [ ] Configurations intact
- [ ] Audit history preserved
- [ ] Vault data accessible (v0.4.0+)

**Feature Testing:**

- [ ] Upload configuration
- [ ] Create branch
- [ ] User management
- [ ] Audit log search
- [ ] Export functions
- [ ] Vault operations (v0.4.0+)

**Performance:**

- [ ] Login time acceptable
- [ ] Search responsive
- [ ] Upload speeds normal
- [ ] Database queries fast

User Communication
^^^^^^^^^^^^^^^^^^

**Pre-Migration Notice:**

.. code-block:: text

   Subject: Ferrocodex Upgrade Scheduled
   
   Team,
   
   We will be upgrading Ferrocodex to version [X.X.X] on [DATE].
   
   Downtime: [START] to [END]
   New Features: [List key features]
   Action Required: [Any user actions]
   
   Please complete any critical work before [TIME].
   
   Contact [ADMIN] with questions.

**Post-Migration Notice:**

.. code-block:: text

   Subject: Ferrocodex Upgrade Complete
   
   Team,
   
   Ferrocodex has been successfully upgraded to version [X.X.X].
   
   New Features Available:
   - [Feature 1]
   - [Feature 2]
   
   Training: [Schedule/resources]
   Documentation: [Updated guides]
   
   Report any issues to [ADMIN].

Training Updates
^^^^^^^^^^^^^^^^

After migration:

1. **Update Documentation**:
   
   * New feature guides
   * Changed workflows
   * Updated screenshots
   * FAQ additions

2. **Conduct Training**:
   
   * Admin changes
   * New features
   * Best practices
   * Q&A session

3. **Gather Feedback**:
   
   * User experience
   * Performance issues
   * Feature requests
   * Training needs

Best Practices
--------------

Migration Planning
^^^^^^^^^^^^^^^^^^

1. **Schedule Wisely**:
   
   * Low-activity periods
   * Maintenance windows
   * Holiday avoidance
   * Team availability

2. **Communicate Early**:
   
   * Two-week notice
   * Reminder emails
   * Feature previews
   * Training schedule

3. **Test Thoroughly**:
   
   * Full test migration
   * Performance testing
   * Feature validation
   * Rollback testing

Migration Execution
^^^^^^^^^^^^^^^^^^^

1. **Follow Runbook**:
   
   * Step-by-step procedures
   * Checkpoint verification
   * Go/no-go decisions
   * Communication plan

2. **Monitor Progress**:
   
   * Migration status
   * Error watching
   * Performance metrics
   * User reports

3. **Document Everything**:
   
   * Actions taken
   * Issues encountered
   * Resolutions applied
   * Lessons learned

Post-Migration
^^^^^^^^^^^^^^

1. **Monitor Closely**:
   
   * First 24 hours critical
   * Performance tracking
   * Error monitoring
   * User feedback

2. **Support Users**:
   
   * Available for questions
   * Quick issue resolution
   * Training reinforcement
   * Positive messaging

3. **Plan Next Steps**:
   
   * Feature adoption
   * Process updates
   * Future migrations
   * Improvement ideas

Emergency Contacts
------------------

During migration:

* **Primary Admin**: [Name, Contact]
* **Backup Admin**: [Name, Contact]
* **Vendor Support**: [Contact Info]
* **Emergency Line**: [24/7 Number]

Keep this guide updated with each migration experience to improve future upgrades.