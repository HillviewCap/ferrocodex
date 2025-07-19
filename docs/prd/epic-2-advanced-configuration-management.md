# Epic 2 Advanced Configuration Management

**Epic Goal:** Empower engineers with advanced workflows to safely manage configuration changes using branching, status promotion, and one-click restore capabilities. This epic transforms the platform from a passive "vault" into an active, powerful tool for change management and disaster recovery.

## Story 2.1: Create a Branch

As an Engineer, I want to create a branch from an existing configuration version, so that I can safely experiment with changes without affecting the main line of development.

- **Acceptance Criteria:**
    
    1. From the "History View" of an asset, the user can select any version.
        
    2. An action is available to "Create Branch from this Version."
        
    3. The user must provide a name for the new branch (e.g., "test-new-parameters").
        
    4. The new branch appears in the UI, clearly showing its relationship to the parent version it was branched from.
        

## Story 2.2: Add New Version to a Branch

As an Engineer, I want to add a new configuration version to the branch I created, so that I can save my work-in-progress.

- **Acceptance Criteria:**
    
    1. The user can select an active branch.
        
    2. The user can use an "Update Branch" action to import a new version of the configuration file.
        
    3. The user must add notes for the new version.
        
    4. The new version is added to the history of that specific branch only.
        
    5. The main version history of the asset is not affected.
        

## Story 2.3: Assign Configuration Status

As an Engineer, I want to assign a status to a configuration version, so that I and others know its state of readiness.

- **Acceptance Criteria:**
    
    1. In the history view, each version has a visible status (defaulting to `Draft`).
        
    2. A user can change the status of a version to `Approved`.
        
    3. The status is clearly displayed with a distinct color or icon.
        
    4. Only an `Administrator` can change a status back from `Approved` to `Draft`.
        

## Story 2.4: Promote Version to "Golden Image"

As an Engineer, I want to use a guided wizard to promote an approved configuration to the official "Golden Image", so that it is clearly marked as the trusted, master version for disaster recovery.

- **Acceptance Criteria:**
    
    1. A "Promote to Golden" action is available only on versions with an `Approved` status.
        
    2. Activating this starts a wizard that explains the action and asks for final confirmation.
        
    3. Upon confirmation, the version's status is changed to `Golden`.
        
    4. If another version of the same asset was previously `Golden`, its status is automatically changed to `Archived`.
        
    5. The `Golden` version is prominently displayed on the asset's main view.
        

## Story 2.5: Revert to a Previous Version

As an Engineer, I want to quickly restore any previous configuration version, so that I can rapidly recover from a failed deployment or equipment failure.

- **Acceptance Criteria:**
    
    1. Every version in the history view has a "Restore" or "Export" button.
        
    2. Clicking the button prompts the user for confirmation.
        
    3. Upon confirmation, the system exports the exact file for that selected version to a location the user chooses on their local machine.
        
    4. The export process is completed in under 2 seconds.