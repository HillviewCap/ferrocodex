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

Slow Search Results
^^^^^^^^^^^^^^^^^^^

**Optimizations:**

1. Use specific search terms
2. Apply filters before searching
3. Limit date ranges
4. Search within specific categories

Database Performance
^^^^^^^^^^^^^^^^^^^^

**Issue: Operations take long time**

*Solutions:*

1. Check database size in Settings
2. Export and archive old data
3. Compact database (Settings → Maintenance)
4. Ensure adequate disk space

Configuration Management
------------------------

Upload Failures
^^^^^^^^^^^^^^^

**Issue: "Upload failed" error**

*Common Causes:*

1. **File too large**: Check size limits
2. **Invalid characters**: in filename
3. **Permissions**: Insufficient user rights
4. **Disk space**: Storage full

*Solutions:*

* Rename file (remove special characters)
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

Error Messages
--------------

Common Error Codes
^^^^^^^^^^^^^^^^^^

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