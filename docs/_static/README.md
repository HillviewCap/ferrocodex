# Static Assets for Documentation

This directory contains static assets (images, diagrams, etc.) for the Ferrocodex documentation.

## Directory Structure

```
_static/
├── images/           # Screenshots and UI images
│   ├── dashboard-overview.png
│   ├── error-dialog-example.png
│   ├── first-launch-screen.png
│   └── vault-creation-flow.png
└── diagrams/         # Architecture and workflow diagrams
    ├── golden-firmware-workflow.svg
    ├── migration-paths.svg
    ├── permission-hierarchy.svg
    └── vault-encryption-layers.svg
```

## Image Guidelines

### Screenshots
- Format: PNG
- Resolution: 1920x1080 or appropriate crop
- Include window chrome when relevant
- Annotate with arrows/highlights as needed
- Blur or redact sensitive information

### Diagrams
- Format: SVG (preferred) or PNG
- Created with: draw.io, Lucidchart, or similar
- Style: Clean, professional, consistent colors
- Font: Match documentation font
- Export at high resolution

## Placeholder Images

The documentation currently uses placeholder references. When creating actual images:

1. **Dashboard Overview** (`dashboard-overview.png`)
   - Show main dashboard with sample data
   - Highlight key metrics areas
   - Include navigation sidebar

2. **Vault Creation Flow** (`vault-creation-flow.png`)
   - Step-by-step vault creation process
   - Include form fields and buttons
   - Show success confirmation

3. **Error Dialog Example** (`error-dialog-example.png`)
   - Sample error with code ERR_VAULT_001
   - Show error message and action buttons
   - Include help link

4. **First Launch Screen** (`first-launch-screen.png`)
   - Welcome screen with logo
   - EULA acceptance
   - Admin setup form

## Diagram Descriptions

1. **Vault Encryption Layers** (`vault-encryption-layers.svg`)
   - Show nested encryption layers
   - Database → Vault → Secret encryption
   - Key derivation flow

2. **Permission Hierarchy** (`permission-hierarchy.svg`)
   - Admin vs Engineer roles
   - Vault permission types (Read, Write, Export, Share)
   - Time-limited access representation

3. **Golden Firmware Workflow** (`golden-firmware-workflow.svg`)
   - Testing → Approval → Promotion flow
   - Decision points
   - Rollback paths

4. **Migration Paths** (`migration-paths.svg`)
   - Version compatibility matrix
   - Automatic vs manual migration paths
   - Breaking change indicators

## Adding New Images

1. Create image following guidelines
2. Save in appropriate subdirectory
3. Use descriptive filename
4. Update this README
5. Reference in RST files using:

```rst
.. figure:: _static/images/your-image.png
   :alt: Descriptive alt text
   :align: center
   :width: 600px

   *Caption describing the image*
```

## Color Palette

For consistency across diagrams:
- Primary: #2E86AB (Ferrocodex Blue)
- Success: #27AE60 (Green)
- Warning: #F39C12 (Orange)
- Error: #E74C3C (Red)
- Neutral: #7F8C8D (Gray)
- Background: #ECF0F1 (Light Gray)