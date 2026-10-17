Ferrocodex Documentation
========================

Ferrocodex is an offline-first configuration management application for operational technology (OT) environments. It manages configuration files, firmware, and credentials for industrial equipment, stores all data locally, and does not require a network connection.

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   getting-started
   user-guide
   asset-hierarchy-guide
   firmware-guide
   admin-guide
   security
   troubleshooting
   migration-guide

Overview
--------

Ferrocodex helps industrial organizations manage their equipment configurations. Key features include:

* **Asset Hierarchy**: Organize equipment in a tree of folders and devices, with drag-and-drop and bulk import, export, and rename
* **Search and Filtering**: SQLite FTS5 full-text search across assets and metadata, with saved searches and filter presets
* **Custom Metadata**: Define equipment-specific fields with JSON Schema validation and field templates
* **Offline-First Design**: Work without network connectivity in air-gapped environments
* **Configuration Versioning**: Branching, version history, and promotion from working versions to approved ("golden") configurations
* **Firmware Management**: Firmware files stored alongside their configurations, with analysis and integrity checks
* **Identity Vault**: Encrypted credential storage with password rotation, history, and per-user access permissions
* **Role-Based Access**: Administrator and Engineer roles, session management, and login rate limiting
* **Encryption at Rest**: Configuration content, firmware files, and vault secrets are encrypted with AES-256-GCM
* **Audit Trail**: Every configuration change and data access is recorded
* **Cross-Platform**: Native builds for Windows, macOS, and Linux

Changelog
---------

Release notes for each version are published on the
`GitHub releases page <https://github.com/HillviewCap/ferrocodex/releases>`_.
For upgrade instructions between versions, see the :doc:`migration-guide`.

Who Should Use Ferrocodex?
--------------------------

* **OT Engineers**: Managing configurations for PLCs, SCADA systems, and industrial equipment
* **System Administrators**: Overseeing configuration management processes and user access
* **Security Teams**: Ensuring compliance and tracking all configuration changes
* **Operations Teams**: Maintaining consistent configurations across facilities

Quick Links
-----------

* :doc:`getting-started` - Install and set up Ferrocodex
* :doc:`user-guide` - Learn how to use all features
* :doc:`admin-guide` - Administrator functions and management
* :doc:`security` - Understand security features and practices
* :doc:`troubleshooting` - Solve common issues

Support
-------

* This documentation
* `GitHub issue tracker <https://github.com/HillviewCap/ferrocodex/issues>`_ for bug reports and feature requests

Please do not open public issues for security vulnerabilities; email security@ferrocodex.com instead.

Indices and tables
==================

* :ref:`genindex`
* :ref:`search`
