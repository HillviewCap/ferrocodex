Troubleshooting
===============

This guide helps you resolve common issues with Ferrocodex. If you can't find a solution here, report the problem on the `GitHub issue tracker <https://github.com/HillviewCap/ferrocodex/issues>`_.

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
   sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0

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

**Engineer Password:**

1. Contact your administrator
2. The administrator uses **Reset password** on your row in **User Management** to set a temporary password
3. Log in with the temporary password

**Administrator Password:**

1. On the login screen, click **Forgot password?**
2. Enter the username, the recovery key saved at setup, and a new password
3. Ferrocodex sets the new password and shows a new recovery key; the old one stops working. Store the new key offline.

An administrator who still knows their password can issue a new recovery key from **User Management** with **Replace Recovery Key**.

.. warning::
   Passwords are stored as bcrypt hashes and cannot be recovered. If every administrator password and the recovery key are lost, configuration content, firmware files, and vault secrets cannot be decrypted.

Session Timeout
^^^^^^^^^^^^^^^

**Issue: Frequently logged out**

*Solution:*

1. Sessions expire 24 hours after login
2. Logging out ends the session immediately
3. Log in again to continue working

Encryption and Key Rotation
---------------------------

Rotation in Progress
^^^^^^^^^^^^^^^^^^^^

**Issue: "Encryption key rotation in progress; an administrator must finish it"**

*Cause:*

An administrator started a data-key rotation and it has not finished. While it runs, nobody can read or write encrypted data (configuration content, firmware, vault secrets, standalone credentials). The block also applies after a restart if the rotation was interrupted.

*Solution (engineers):*

1. Wait, or ask an administrator to finish the rotation
2. Retry after it completes; no new login is needed

*Solution (administrators):*

1. Log in and open **User Management**
2. Click **Rotate Encryption Key** to resume the rotation
3. Let it run until it finishes. If it stops with a list of failed items, see :ref:`rotating-the-encryption-key`

See :ref:`rotating-the-encryption-key` in the administrator guide for the full procedure.

Rotation Stops With Failed Items
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: "N item(s) could not be decrypted with the old key and were recorded as failures..."**

*Cause:*

Some items did not decrypt with the old key, a firmware file is missing, or a firmware path points outside the firmware folder. The rotation re-encrypted everything else and is waiting for a decision.

*Solution:*

1. Read the reason shown for each failed item
2. If you can fix it (for example, restore a missing firmware file from a backup to the same place), do so and click **Retry failed items**, then let the rotation run again
3. Only if the items are truly lost, click **Finalize and discard unreadable items**. This destroys the old key, so they stay unreadable permanently

**Issue: "N item(s) recorded as unreadable can be decrypted now. Retry the failed items instead of discarding them."**

*Cause:*

Before discarding, Ferrocodex checks every failed item again. At least one of them now decrypts with the old key (for example, a restored file), or was already re-encrypted. Discarding would destroy data that can still be saved.

*Solution:*

Click **Retry failed items** and let the rotation run until it finishes.

**Issue: "Some failed items could not be checked before discarding (...)"**

*Cause:*

A failed firmware file could not be read during the check (for example, missing permissions or a locked file), so Ferrocodex cannot confirm it is unreadable.

*Solution:*

Fix the access problem named in the message and try again.

Rotation Stops With a Read or Write Error
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: "Failed to re-encrypt firmware N: ... Fix the problem and continue the rotation."**

*Cause:*

A firmware file could not be read or the re-encrypted copy could not be written (another program has the file open, missing permissions, a full disk). The data is not damaged and nothing was recorded as a failure.

*Solution:*

1. Fix the problem named in the message
2. Click **Rotate Encryption Key** again to resume

Writes Refused With an Outdated Key
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: "Your session holds an outdated encryption key; log in again"**

*Cause:*

The data key was rotated after this session (or a running task, such as a firmware upload or analysis) obtained its key. Ferrocodex refuses to store anything under the old key, because it would be unreadable.

*Solution:*

Log in again and repeat the action.

Cannot Open Encrypted Data After a Rotation
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: "Encrypted data is unavailable for this account. Ask an administrator to reset your password in User Management."**

*Cause:*

The user can log in but has no copy of the active data key. After a rotation this affects active users who had no encryption keypair when it started (for example, accounts that had not logged in since keypairs were introduced).

*Solution:*

1. Wait until the rotation has finished
2. An administrator opens **User Management** and uses **Reset password** on the user's row, entering a temporary password
3. The user logs in with the temporary password.

If the affected user is the only administrator, use **Forgot password?** on the login screen with the recovery key shown when the rotation started. The recovery key from before the rotation no longer works.

Rotation Refused: Analysis Jobs Running
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: "N firmware analysis job(s) are queued or running; wait for them to finish before rotating the encryption key"**

*Cause:*

Each firmware analysis job holds a copy of the current key. Rotating while one runs would leave a job working with a key that is being replaced. (A job queued just after the rotation starts does not use its old key: it fails with "Analysis failed: Encryption key rotation in progress" and can be retried once the rotation has finished.)

*Solution:*

1. Wait for the analysis jobs to finish
2. If none appear to be running, restart Ferrocodex to clear the job count, then log in and try again

Rotation Refused: Users Without Keypairs
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: "N active user(s) have no encryption keypair and will lose access to encrypted data until an administrator resets their password... Acknowledge this to continue."**

*Cause:*

Those users have no keypair to seal the new key to. They get one the next time they log in with their own password, or when an administrator resets their password.

*Solutions:*

1. To avoid any loss of access, ask those users to log in once, then start the rotation again
2. Or tick the acknowledgement checkbox and rotate now, then use **Reset password** for each listed user afterwards

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

Slow Tree Navigation
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Asset hierarchy tree is slow**

*Solutions:*

1. **Collapse Unused Branches**: Reduce rendered nodes
2. **Use the Tree Filter**: Type part of the asset name to narrow the tree
3. **Archive Old Assets**: Move inactive assets to archive
4. **Check Asset Count**: Limit folders to < 500 items
5. **Search Within Categories**: Narrow the tree to the relevant folder

Database Performance
^^^^^^^^^^^^^^^^^^^^

**Issue: Operations take long time**

*Solutions:*

1. Check database size in Settings
2. Export and archive old data
3. Compact database (Settings → Maintenance)
4. Ensure adequate disk space

Asset Hierarchy Issues
--------------------------------

Asset Naming Errors
^^^^^^^^^^^^^^^^^^^

**Issue: "Invalid asset name" error**

*Requirements:*

Asset names must match the pattern ``^[A-Z0-9][A-Z0-9_-]{2,49}$``

*Solutions:*

1. Use UPPERCASE letters only
2. Remove spaces (use hyphen or underscore)
3. Ensure 3-50 character length
4. Avoid reserved names (CON, PRN, AUX, etc.)

*Examples:*

* ``plc-west-01`` → ``PLC-WEST-01``
* ``PLC WEST 01`` → ``PLC-WEST-01``
* ``_sensor`` → ``SENSOR-01``

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

1. **Restart the Application**: Reload the asset tree
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

Tree Filter Not Finding Assets
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: The asset tree filter shows no matches**

*Tips:*

1. **Check the Term**: The filter matches any part of the asset name or description, case-insensitively
2. **Check the Type Filter**: Set it to "All" if it is limited to folders or devices
3. **Metadata Is Not Searched**: Custom metadata values do not match the filter; use the name or description
4. **Clear and Retry**: Use the clear button and type the term again

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

* Ensure filename follows security rules
* Remove special characters
* Check available disk space
* Verify user has Engineer/Admin role
* Try smaller file or compress

Cannot Download Configuration
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Download button not working**

*Checklist:*

1. Check that the export location is writable
2. Verify file still exists
3. Check user permissions
4. Restart the application and try again

**Issue: Downloaded file is corrupted**

*Solutions:*

* Try the export again
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

1. Restart the application
2. Check zoom level (Ctrl/Cmd + 0)
3. Update graphics drivers
4. Try different display resolution

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

*Solutions:*

1. Check available disk space
2. Close and reopen asset view
3. Contact administrator if persists

Password Generation Problems
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Issue: Generate button not working**

*Solutions:*

1. Restart the application
2. Try manual password entry
3. Verify password policy settings

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

**Issue: "Access denied" on a vault operation (open, add or edit a secret, view a secret, export, share)**

*Cause:*

Vault permissions are enforced by the backend on every operation. You do not hold the permission the operation needs:

* Administrators hold every permission
* The creator of a vault holds Read, Write, and Export on it
* Everyone else needs an explicit, unexpired grant for that permission on that vault
* Share is administrator-only

A Read grant does not allow editing or exporting, so a user can open a vault and still be denied when they try to change it. The denial is recorded in the vault access log and the audit log.

*Solutions:*

1. Verify you have vault permissions:

   * Ask an administrator to grant the specific permission you need
   * Check permission expiration
   * Review audit log for changes

2. If recently granted access:

   * Log out and back in
   * Check the audit log for the permission grant

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

* Wrong export passphrase. Vault secrets in an export are encrypted with the passphrase chosen at export time, and the import needs the same one.
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
3. Check database performance

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

**For issue reports:**

1. Go to Settings → About
2. Click "Copy System Info"
3. Include it in the issue report
4. Attach relevant screenshots
5. Export recent audit logs

Support Channels
^^^^^^^^^^^^^^^^

* **Documentation**: This site
* **Issue Tracker**: https://github.com/HillviewCap/ferrocodex/issues
* Include the application version, operating system, steps to reproduce, and the exact error message
* Do not post vulnerabilities publicly; email security@ferrocodex.com instead

Emergency Procedures
^^^^^^^^^^^^^^^^^^^^

For critical issues:

1. **Document everything** immediately
2. **Stop using** affected features
3. **Report the issue** on the issue tracker
4. **Prepare rollback** if needed
5. **Communicate** with team

Remember: Most issues have simple solutions. Work through this guide systematically before escalating.