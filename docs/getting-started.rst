Getting Started
===============

This guide will help you install Ferrocodex and get started with basic configuration management tasks.

System Requirements
-------------------

**Minimum Requirements:**

* **Operating System**: Windows 10/11, macOS 10.15 or later, or a Linux distribution with WebKitGTK 4.1 (for example Ubuntu 22.04 or later)
* **RAM**: 4GB minimum, 8GB recommended
* **Storage**: 500MB for application, additional space for configurations
* **Display**: 1280x720 minimum resolution

**Supported Platforms:**

* Windows (x64)
* macOS (Intel and Apple Silicon)
* Linux (x64)

Installation
------------

Pre-built installers for every release are published on the
`GitHub releases page <https://github.com/HillviewCap/ferrocodex/releases>`_.

Windows
^^^^^^^

1. Download the ``.msi`` or ``.exe`` installer from the releases page
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

**AppImage:**

1. Download the ``.AppImage`` file
2. Make it executable: ``chmod +x Ferrocodex_*.AppImage``
3. Run the application: ``./Ferrocodex_*.AppImage``

**Debian/Ubuntu:**

1. Download the ``.deb`` package
2. Install using: ``sudo dpkg -i ferrocodex_*.deb``
3. Launch from your application menu or run ``ferrocodex``

**Fedora/RHEL:**

1. Download the ``.rpm`` package
2. Install using: ``sudo rpm -i ferrocodex-*.rpm``
3. Launch from your application menu or run ``ferrocodex``

Building from Source
^^^^^^^^^^^^^^^^^^^^

Prerequisites:

* Node.js 22.12 or newer
* Rust (stable toolchain)
* Platform dependencies:

  * **Linux**: ``libwebkit2gtk-4.1-dev``, ``libappindicator3-dev``, ``librsvg2-dev``, ``patchelf``, ``build-essential``, ``curl``, ``wget``, ``file``, ``libgtk-3-dev``
  * **macOS**: Xcode Command Line Tools
  * **Windows**: Microsoft Visual Studio C++ Build Tools

.. code-block:: bash

   git clone https://github.com/HillviewCap/ferrocodex.git
   cd ferrocodex
   npm install

   # Production bundle for the current platform
   cd apps/desktop
   npm run tauri:build

On Debian or Ubuntu, the Linux dependencies can be installed with:

.. code-block:: bash

   sudo apt update
   sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
     build-essential curl wget file libgtk-3-dev

Data Storage
------------

Ferrocodex stores its SQLite database and encrypted files in the platform application-data directory:

* **Windows**: ``%APPDATA%\com.ferrocodex.app\``
* **macOS**: ``~/Library/Application Support/com.ferrocodex.app/``
* **Linux**: ``~/.config/com.ferrocodex.app/``

The database (``ferrocodex.db``) is created on first launch and opened in WAL mode. Configuration content, firmware files, and vault secrets are encrypted with AES-256-GCM using keys derived from user credentials. The database file itself is not encrypted as a whole, so protect the data directory with operating-system permissions and full-disk encryption.

First Launch
------------

.. figure:: _static/images/first-launch-screen.png
   :alt: Ferrocodex first launch screen
   :align: center
   :width: 600px

   *The Ferrocodex welcome screen on first launch*

Initial Setup
^^^^^^^^^^^^^

When you first launch Ferrocodex, you'll need to:

1. **Accept the EULA**: Read and accept the End User License Agreement
2. **Create the Administrator Account**: Set up the first user account, which has the Administrator role

.. warning::
   There is no password recovery. If the administrator password is lost, the account and the data encrypted with its credentials cannot be recovered. Keep a secure record of the credentials.

Creating Your First User
^^^^^^^^^^^^^^^^^^^^^^^^

1. On the welcome screen, enter a username (letters, numbers, hyphens, and underscores)
2. Enter a password of at least 8 characters, using a mix of letters, numbers, and symbols
3. Confirm the password and click **"Create Admin Account"**

You can then sign in with these credentials. The first account has the Administrator role; additional users are created from **User Management** after login.

Basic Workflow
--------------

1. Managing Assets
^^^^^^^^^^^^^^^^^^

**Understanding Asset Types:**

Ferrocodex supports two asset types for organization:

* **Folders**: Organizational containers for grouping related equipment
* **Devices**: Actual industrial equipment (PLCs, HMIs, etc.)

**Creating Your First Asset Hierarchy:**

1. **Create a Folder Structure:**

   a. Navigate to the Assets page
   b. Click "Add Asset" → "Folder"
   c. Name it following the naming pattern (e.g., "PRODUCTION-LINE-1")
   d. Click "Create"

2. **Add a Device to the Folder:**

   a. Select your folder in the tree view
   b. Click "Add Asset" → "Device"
   c. Enter device details:

      * Name: Must follow ``^[A-Z0-9][A-Z0-9_-]{2,49}$`` (e.g., "PLC-LINE1-01")
      * Type: Equipment category
      * Manufacturer: Device vendor
      * Model: Specific model
      * Custom Metadata: Add fields as needed

   d. Click "Create" to save

**Asset Naming Requirements:**

All asset names must follow the naming pattern:

* Use UPPERCASE letters and numbers only
* Can include underscore (_) and hyphen (-)
* Length: 3-50 characters
* Examples: ``PLC-001``, ``HMI_MAIN_01``, ``SENSOR-TEMP-001``

**Organizing Assets with Hierarchy:**

* Create logical folder structures (by location, function, or type)
* Use drag-and-drop to reorganize assets
* Navigate with the tree view or filter it by name
* Add custom metadata fields for better organization

2. Configuration Management
^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Uploading Configurations:**

1. Select an asset from the Assets page
2. Click "Upload Configuration"
3. Choose your configuration file
4. Select file type and add optional notes
5. Click "Upload"

.. note::
   Ferrocodex accepts any file type so that it can store configurations for any industrial equipment.

**Creating Branches:**

1. From an asset's configuration list, click "Create Branch"
2. Enter a branch name (e.g., "maintenance-2025-01")
3. The branch starts with the current configuration
4. Make changes without affecting the main configuration

3. User Management (Administrators Only)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**Adding Users:**

1. Go to **User Management**
2. Click "Create Engineer Account"
3. Enter a username and an initial password, and share the password with the user securely

Ferrocodex has two roles:

* **Administrator**: Full system access, including user management and all audit logs
* **Engineer**: Configuration management only

The account created at first launch is the Administrator; accounts created from User Management have the Engineer role.

**Managing Permissions:**

* Administrators can create, deactivate, and reactivate users and view audit logs
* Engineers can manage configurations but not users
* All actions are logged for security compliance

Next Steps
----------

* Read the :doc:`user-guide` for detailed feature documentation
* Review :doc:`security` for recommended practices
* Set up your asset hierarchy and naming conventions
* Configure regular backups of your database
* Train your team on proper configuration management procedures

Getting Help
------------

* **Documentation**: This guide and the other sections of this site
* **Issue Tracker**: Report bugs and request features at https://github.com/HillviewCap/ferrocodex/issues
