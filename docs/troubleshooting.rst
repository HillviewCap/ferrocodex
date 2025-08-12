Troubleshooting
===============

This guide helps you resolve common issues with Ferrocodex. If you can't find a solution here, contact your support representative.

.. contents:: Table of Contents
   :local:
   :depth: 2

Installation Issues
-------------------

Windows Installation
^^^^^^^^^^^^^^^^^^^^

**Issue: "Windows protected your PC" warning**

*Solution:*

1. Click "More info" on the SmartScreen dialog
2. Click "Run anyway"
3. This occurs because the app isn't yet widely recognized

**Issue: Installation fails with permission error**

*Solution:*

1. Right-click the installer
2. Select "Run as administrator"
3. Ensure you have local admin rights

macOS Installation
^^^^^^^^^^^^^^^^^^

**Issue: "Cannot be opened because it is from an unidentified developer"**

*Solution:*

1. Right-click the application
2. Select "Open" from the context menu
3. Click "Open" in the dialog
4. Or: System Preferences → Security & Privacy → "Open Anyway"

**Issue: App crashes on Apple Silicon Macs**

*Solution:*

1. Ensure you downloaded the correct version:
   * Intel: ``Ferrocodex_x64.dmg``
   * Apple Silicon: ``Ferrocodex_aarch64.dmg``
2. Verify in "About This Mac" → "Processor"

Linux Installation
^^^^^^^^^^^^^^^^^^

**Issue: AppImage won't run**

*Solution:*

.. code-block:: bash

   chmod +x Ferrocodex_*.AppImage
   ./Ferrocodex_*.AppImage

**Issue: Missing dependencies on Debian/Ubuntu**

*Solution:*

.. code-block:: bash

   sudo apt update
   sudo apt install libwebkit2gtk-4.0-37 libgtk-3-0

Login and Authentication
------------------------

Cannot Login
^^^^^^^^^^^^

**Issue: "Invalid credentials" error**

*Checklist:*

1. Verify username (case-sensitive)
2. Check Caps Lock key
3. Ensure correct password
4. Try copy-paste to avoid typos

**Issue: "Account locked" message**

*Solution:*

* Contact administrator to unlock
* Wait for lockout period (default: 15 minutes)
* Check audit log for failed attempts

Forgotten Password
^^^^^^^^^^^^^^^^^^

**User Password:**

1. Contact your administrator
2. Admin can reset your password
3. You'll need to change it on next login

**Master Password (First Launch):**

.. warning::
   The master password cannot be recovered. If lost, you must reinstall and lose all data.

Session Timeout
^^^^^^^^^^^^^^^

**Issue: Frequently logged out**

*Solution:*

1. Check Settings → Security → Session Timeout
2. Default is 30 minutes of inactivity
3. Administrator can adjust timeout
4. Activity extends session automatically

Performance Issues
------------------

Slow Application Start
^^^^^^^^^^^^^^^^^^^^^^

**Common Causes:**

1. **Large database**: Many assets/configurations
2. **Antivirus scanning**: Add exception for Ferrocodex
3. **Disk performance**: Check available space
4. **Memory constraints**: Close other applications

*Solutions:*

* Archive old configurations
* Add antivirus exception
* Ensure 10% free disk space
* Restart application

Slow Search Results (Enhanced in v0.5.0)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**v0.5.0 Search Performance:**

With SQLite FTS5, searches should complete in under 200ms. If experiencing slowness:

*Solutions:*

1. **Rebuild Search Index**:
   
   * Navigate to Settings → Search Performance
   * Click "Rebuild Index"
   * Wait for completion (5-10 minutes)

2. **Optimize Search Queries**:
   
   * Use specific field searches: ``manufacturer:siemens``
   * Avoid wildcards at start: ``*pump`` (slow) vs ``pump*`` (fast)
   * Use filters to narrow scope

3. **Check Index Health**:
   
   * Settings → Search Performance → Index Health
   * Look for fragmentation warnings
   * Run "Optimize Index" if needed

4. **Clear Search Cache**:
   
   * Settings → Search Performance → Clear Cache
   * Helps with stale results

Slow Tree Navigation (v0.5.0)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Asset hierarchy tree is slow**

*Solutions:*

1. **Collapse Unused Branches**: Reduce rendered nodes
2. **Use Search Instead**: Navigate directly to assets
3. **Archive Old Assets**: Move inactive assets to archive
4. **Check Asset Count**: Limit folders to < 500 items
4. Search within specific categories

Database Performance
^^^^^^^^^^^^^^^^^^^^

**Issue: Operations take long time**

*Solutions:*

1. Check database size in Settings
2. Export and archive old data
3. Compact database (Settings → Maintenance)
4. Ensure adequate disk space

Asset Hierarchy Issues (v0.5.0)
--------------------------------

Asset Naming Errors
^^^^^^^^^^^^^^^^^^^

**Issue: "Invalid asset name" error**

*v0.5.0 Requirements:*

Asset names must follow the pattern ``^[A-Z0-9][A-Z0-9_-]{2,49}$``

*Solutions:*

1. Use UPPERCASE letters only
2. Remove spaces (use hyphen or underscore)
3. Ensure 3-50 character length
4. Avoid reserved names (CON, PRN, AUX, etc.)

*Examples:*

* ❌ ``plc-west-01`` → ✅ ``PLC-WEST-01``
* ❌ ``PLC WEST 01`` → ✅ ``PLC-WEST-01``
* ❌ ``_sensor`` → ✅ ``SENSOR-01``

Cannot Create Asset
^^^^^^^^^^^^^^^^^^^

**Issue: "Asset creation failed"**

*Common Causes:*

1. **Duplicate Name**: Name already exists in same folder
2. **Invalid Parent**: Parent folder doesn't exist
3. **Permissions**: Insufficient rights
4. **Validation Error**: Metadata field validation failed

*Solutions:*

* Use unique name within folder
* Verify parent folder exists
* Check metadata field requirements
* Review validation error messages

Drag-and-Drop Not Working
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Cannot drag assets in tree**

*Solutions:*

1. **Check Browser**: Use Chrome, Firefox, or Edge
2. **Permissions**: Verify edit rights on assets
3. **Target Folder**: Ensure target allows children
4. **Circular Reference**: Cannot move folder into itself

Metadata Field Issues
^^^^^^^^^^^^^^^^^^^^^

**Issue: "Invalid metadata value"**

*Solutions:*

1. Check field type requirements
2. Verify pattern matching (for text fields)
3. Ensure date format is correct
4. Check numeric ranges

**Issue: Cannot add custom fields**

*Solutions:*

1. Administrator privileges may be required
2. Check metadata schema restrictions
3. Verify field name uniqueness
4. Review JSON schema if applicable

Search Not Finding Assets
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Search returns no results**

*v0.5.0 Search Tips:*

1. **Check Syntax**:
   
   * Simple: ``pump``
   * Field-specific: ``location:west``
   * Boolean: ``pump AND cooling``

2. **Rebuild Index** (Admin):
   
   * Settings → Search Performance
   * Click "Rebuild Index"

3. **Check Permissions**: 
   
   * Only assets you can access are searchable

4. **Clear Cache**:
   
   * Settings → Search Performance → Clear Cache

Configuration Management
------------------------

Upload Failures
^^^^^^^^^^^^^^^

**Issue: "Upload failed" error**

*Common Causes:*

1. **File too large**: Check size limits
2. **Invalid characters**: in filename (v0.5.0 stricter)
3. **Permissions**: Insufficient user rights
4. **Disk space**: Storage full

*Solutions:*

* Ensure filename follows security rules (v0.5.0)
* Remove special characters
* Check available disk space
* Verify user has Engineer/Admin role
* Try smaller file or compress

Cannot Download Configuration
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Download button not working**

*Checklist:*

1. Check browser download settings
2. Verify file still exists
3. Check user permissions
4. Try different browser

**Issue: Downloaded file is corrupted**

*Solutions:*

* Clear browser cache
* Try download again
* Check original upload integrity
* Verify from different user account

Branch Operations
^^^^^^^^^^^^^^^^^

**Issue: Cannot create branch**

*Requirements:*

* Must have configuration to branch from
* Engineer or Administrator role
* Unique branch name

**Issue: Merge conflicts**

*Best Practices:*

1. Document changes in branch
2. Communicate with team
3. Download both versions first
4. Manually resolve if needed

Display Issues
--------------

UI Elements Missing
^^^^^^^^^^^^^^^^^^^

**Common Fixes:**

1. Refresh page (F5)
2. Clear browser cache
3. Check zoom level (Ctrl/Cmd + 0)
4. Update graphics drivers
5. Try different display resolution

Text Too Small/Large
^^^^^^^^^^^^^^^^^^^^

**Adjustments:**

* Windows: Ctrl + Plus/Minus
* macOS: Cmd + Plus/Minus
* Settings → Display → Font Size

Dark Mode Issues
^^^^^^^^^^^^^^^^

**If UI elements are incorrect:**

1. Toggle theme off/on
2. Restart application
3. Check system theme settings
4. Report specific elements affected

Data and Storage
----------------

Running Out of Space
^^^^^^^^^^^^^^^^^^^^

**Check Storage:**

1. Settings → System → Storage Info
2. Shows database size
3. Configuration storage usage

**Free Up Space:**

1. Export old configurations
2. Delete from system
3. Archive audit logs
4. Compact database

Backup Failures
^^^^^^^^^^^^^^^

**Issue: Export fails**

*Solutions:*

1. Check destination has space
2. Verify write permissions
3. Try smaller export (date range)
4. Export without audit logs

Import Problems
^^^^^^^^^^^^^^^

**Issue: Import doesn't work**

*Requirements:*

* Valid Ferrocodex export file
* Matching version format
* Administrator privileges
* No corrupt ZIP file

Identity Vault Issues
---------------------

Cannot Create Vault
^^^^^^^^^^^^^^^^^^^

**Issue: "Create Vault" button disabled or missing**

*Solutions:*

1. Verify you have Engineer or Administrator role
2. Check if vault already exists for the asset
3. Ensure asset is saved before creating vault
4. Refresh the page and try again

**Issue: Vault creation fails with error**

*Common Causes:*

* Database space limitations
* Concurrent modification conflict
* Browser compatibility issues

*Solutions:*

1. Check available disk space
2. Close and reopen asset view
3. Try different browser
4. Contact administrator if persists

Password Generation Problems
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Generate button not working**

*Solutions:*

1. Check browser JavaScript is enabled
2. Clear browser cache
3. Try manual password entry
4. Verify password policy settings

**Issue: Generated password rejected**

*Causes:*

* Password policy requirements changed
* Special characters not allowed
* Length requirements not met

*Solutions:*

1. Review password requirements
2. Adjust generation settings
3. Try shorter/longer password
4. Remove special characters if needed

Vault Access Denied
^^^^^^^^^^^^^^^^^^^

**Issue: "Access Denied" when opening vault**

*Solutions:*

1. Verify you have vault permissions:
   
   * Ask administrator for access
   * Check permission expiration
   * Review audit log for changes

2. If recently granted access:
   
   * Log out and back in
   * Clear browser cache
   * Wait 5 minutes for sync

**Issue: Cannot see vault contents**

*Requirements:*

* Read permission on specific vault
* Active user session
* Asset access rights

Password Rotation Failures
^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Rotation wizard won't complete**

*Common Problems:*

1. **Current password incorrect**:
   
   * Verify caps lock
   * Check password history
   * Try copy/paste

2. **New password invalid**:
   
   * Check complexity requirements
   * Avoid password reuse
   * Try generated password

3. **Network/timing issues**:
   
   * Check session hasn't expired
   * Retry the operation
   * Save work frequently

**Issue: Batch rotation stuck**

*Solutions:*

1. Cancel batch operation
2. Rotate passwords individually
3. Check for locked vaults
4. Review error messages

Compliance and Alerts
^^^^^^^^^^^^^^^^^^^^^

**Issue: Not receiving rotation reminders**

*Check:*

1. Notification settings enabled
2. Rotation schedule configured
3. Email address correct
4. Check spam folder

**Issue: Compliance dashboard empty**

*Solutions:*

1. Verify rotation schedules set
2. Check user has reporting access
3. Refresh dashboard data
4. Review date range filters

Standalone Credentials Issues
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Cannot create categories**

*Requirements:*

* Administrator or Engineer role
* Unique category name
* Valid parent category

*Solutions:*

1. Check role permissions
2. Use different category name
3. Create at root level first

**Issue: Search not finding credentials**

*Tips:*

1. Use partial search terms
2. Check category filters
3. Clear all filters and retry
4. Verify credential exists

Export/Import with Vaults
^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Vault data not included in export**

*Checklist:*

1. "Include vault data" checked
2. Export permissions granted
3. Vaults contain data
4. No active vault locks

**Issue: Import fails with vault data**

*Common Causes:*

* Version incompatibility
* Corrupted export file
* Insufficient permissions
* Duplicate vault conflicts

*Solutions:*

1. Verify export file integrity
2. Check Ferrocodex versions match
3. Use Administrator account
4. Choose merge strategy

Permission Management Problems
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Cannot grant vault permissions**

*Requirements:*

* Administrator role required
* Target user must exist
* Vault must be created

**Issue: Time-limited access not expiring**

*Solutions:*

1. Check system time/timezone
2. Review expiration settings
3. Manually revoke if needed
4. Check audit log

Vault Performance Issues
^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Vault operations slow**

*Optimizations:*

1. Limit vault size (< 100 entries)
2. Archive old passwords
3. Clear browser cache
4. Check database performance

**Issue: Search within vault slow**

*Tips:*

1. Use specific search terms
2. Search by label first
3. Use filters effectively
4. Paginate large results

Error Messages
--------------

Common Error Codes
^^^^^^^^^^^^^^^^^^

.. figure:: _static/images/error-dialog-example.png
   :alt: Example error dialog with error code
   :align: center
   :width: 500px

   *Example error dialog showing error code and message*

.. list-table::
   :header-rows: 1

   * - Error
     - Meaning
     - Solution
   * - ERR_AUTH_001
     - Authentication failed
     - Check credentials
   * - ERR_PERM_001
     - Insufficient permissions
     - Contact administrator
   * - ERR_FILE_001
     - File operation failed
     - Check disk space/permissions
   * - ERR_DB_001
     - Database error
     - Restart application
   * - ERR_SESS_001
     - Session expired
     - Login again
   * - ERR_VAULT_001
     - Vault access denied
     - Check vault permissions
   * - ERR_VAULT_002
     - Vault already exists
     - Use existing vault
   * - ERR_VAULT_003
     - Password policy violation
     - Meet complexity requirements
   * - ERR_VAULT_004
     - Rotation failed
     - Check current password
   * - ERR_VAULT_005
     - Export permission denied
     - Request export permission

Getting Help
------------

Before Contacting Support
^^^^^^^^^^^^^^^^^^^^^^^^^

1. **Document the issue:**
   
   * Exact error message
   * Steps to reproduce
   * Screenshot if possible
   * Time of occurrence

2. **Check basics:**
   
   * Application version
   * Operating system
   * Available disk space
   * User role/permissions

3. **Try standard fixes:**
   
   * Restart application
   * Reboot computer
   * Check for updates
   * Review this guide

Collecting Diagnostic Info
^^^^^^^^^^^^^^^^^^^^^^^^^^

**For support tickets:**

1. Go to Settings → About
2. Click "Copy System Info"
3. Include in support request
4. Attach relevant screenshots
5. Export recent audit logs

Support Channels
^^^^^^^^^^^^^^^^

During the alpha phase:

* Primary: Your designated support contact
* Include: System info, steps to reproduce
* Severity: Mark urgent issues appropriately
* Response: Check your agreed SLA

Emergency Procedures
^^^^^^^^^^^^^^^^^^^^

For critical issues:

1. **Document everything** immediately
2. **Stop using** affected features
3. **Contact support** urgently
4. **Prepare rollback** if needed
5. **Communicate** with team

Remember: Most issues have simple solutions. Work through this guide systematically before escalating.