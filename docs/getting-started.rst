Getting Started
===============

This guide will help you install Ferrocodex and get started with basic configuration management tasks.

System Requirements
-------------------

**Minimum Requirements:**

* **Operating System**: Windows 10/11, macOS 11+, or Ubuntu 20.04+
* **RAM**: 4GB minimum, 8GB recommended
* **Storage**: 500MB for application, additional space for configurations
* **Display**: 1280x720 minimum resolution

**Supported Platforms:**

* Windows (x64)
* macOS (Intel and Apple Silicon)
* Linux (x64)

Installation
------------

Windows
^^^^^^^

1. Download the latest ``.msi`` installer from the releases page
2. Double-click the installer and follow the setup wizard
3. Launch Ferrocodex from the Start Menu or Desktop shortcut

macOS
^^^^^

1. Download the appropriate ``.dmg`` file:
   
   * Intel Macs: ``Ferrocodex_x64.dmg``
   * Apple Silicon: ``Ferrocodex_aarch64.dmg``

2. Open the DMG file and drag Ferrocodex to your Applications folder
3. On first launch, right-click and select "Open" to bypass Gatekeeper

Linux
^^^^^

**AppImage (Recommended):**

1. Download the ``.AppImage`` file
2. Make it executable: ``chmod +x Ferrocodex_*.AppImage``
3. Run the application: ``./Ferrocodex_*.AppImage``

**Debian/Ubuntu:**

1. Download the ``.deb`` package
2. Install using: ``sudo dpkg -i ferrocodex_*.deb``
3. Launch from your application menu or run ``ferrocodex``

First Launch
------------

Initial Setup
^^^^^^^^^^^^^

When you first launch Ferrocodex, you'll need to:

1. **Accept the EULA**: Read and accept the End User License Agreement
2. **Create Master Password**: This encrypts your local database
3. **Create Administrator Account**: Set up your first user account

.. warning::
   Store your master password securely! It cannot be recovered if lost.

Creating Your First User
^^^^^^^^^^^^^^^^^^^^^^^^

1. Click "Create First User" on the welcome screen
2. Fill in the required information:
   
   * Username (unique identifier)
   * Full Name
   * Email Address
   * Password (minimum 8 characters)
   * Role (Administrator for first user)

3. Click "Create User" to complete setup

Basic Workflow
--------------

1. Managing Assets
^^^^^^^^^^^^^^^^^

**Adding an Asset:**

1. Navigate to the Assets page
2. Click "Add Asset"
3. Enter asset details:
   
   * Name (e.g., "PLC-01")
   * Type (e.g., "PLC")
   * Manufacturer
   * Model
   * Location

4. Click "Create" to save

**Organizing Assets:**

* Use descriptive names following your naming convention
* Group related assets by location or function
* Add detailed descriptions for easier searching

2. Configuration Management
^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Uploading Configurations:**

1. Select an asset from the Assets page
2. Click "Upload Configuration"
3. Choose your configuration file
4. Select file type and add optional notes
5. Click "Upload"

.. note::
   Ferrocodex accepts any file type for maximum flexibility with industrial equipment.

**Creating Branches:**

1. From an asset's configuration list, click "Create Branch"
2. Enter a branch name (e.g., "maintenance-2025-01")
3. The branch starts with the current configuration
4. Make changes without affecting the main configuration

3. User Management (Administrators Only)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Adding Users:**

1. Go to Settings → Users
2. Click "Create User"
3. Assign appropriate role:
   
   * **Administrator**: Full system access
   * **Engineer**: Configuration management only

4. Set initial password (user must change on first login)

**Managing Permissions:**

* Administrators can create/modify users and view audit logs
* Engineers can manage configurations but not users
* All actions are logged for security compliance

Next Steps
----------

* Read the :doc:`user-guide` for detailed feature documentation
* Review :doc:`security` for best practices
* Set up your asset hierarchy and naming conventions
* Configure regular backups of your database
* Train your team on proper configuration management workflows

Getting Help
------------

* **In-App Help**: Click the help icon for context-sensitive assistance
* **Documentation**: This guide and other resources
* **Support**: Contact your Ferrocodex representative for alpha support