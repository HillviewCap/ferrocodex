# Epic 1 Foundation & Core Versioning

**Epic Goal:** Establish the secure, cross-platform application foundation, including user authentication and the core ability to import configurations and track their complete version history. This epic delivers the initial core value: a secure, auditable "vault" for vital configurations.

## Story 1.1: Project Initialization

As a Project Owner, I want the initial application structure set up, so that developers have a clean, consistent foundation to start building on.

- **Acceptance Criteria:**
    
    1. A new monorepo is created.
        
    2. A Tauri application is initialized with a Rust backend and a React/TypeScript frontend.
        
    3. The application compiles and launches to a blank window on Windows, macOS, and Linux.
        
    4. The local, encrypted database (e.g., SQLite) is included as a dependency.
        

## Story 1.2: Initial User Account & Login

As an Administrator, I want to create the first user account and log in, so that the application is secured from unauthorized access.

- **Acceptance Criteria:**
    
    1. On first launch, the application presents a "Create Admin Account" screen.
        
    2. A new user account is created and stored in the local database with a securely hashed password.
        
    3. After account creation, the user is directed to a Login Screen.
        
    4. The user can successfully log in using the created credentials.
        
    5. Upon successful login, the user is presented with a main (currently empty) dashboard screen.
        

## Story 1.3: User Management

As an Administrator, I want to manage Engineer accounts, so that I can control who has access to the platform.

- **Acceptance Criteria:**
    
    1. When logged in as an `Administrator`, a "User Management" area is visible.
        
    2. The Administrator can create new `Engineer` user accounts (username and initial password).
        
    3. The Administrator can deactivate and reactivate existing `Engineer` accounts.
        
    4. `Engineer` users do not see the "User Management" area.
        

## Story 1.4: Import Configuration and Create First Version

As an Engineer, I want to import a configuration file for a new asset, so that it is securely stored and versioned in the platform.

- **Acceptance Criteria:**
    
    1. From the main dashboard, a logged-in user can select an "Import Configuration" action.
        
    2. The user is prompted to select a file from their local system.
        
    3. The user must provide a name for the asset/device (e.g., "PLC-Line5") and can add initial commit notes.
        
    4. Upon saving, the file is stored encrypted in the local database.
        
    5. A new version record (v1) is created, associated with the asset, and includes the author's username, timestamp, and notes.
        
    6. The newly created asset appears on the main dashboard.
        

## Story 1.5: View Configuration History

As an Engineer, I want to view the complete version history of an asset, so that I have a full audit trail of all changes.

- **Acceptance Criteria:**
    
    1. Clicking on an asset from the dashboard navigates to a "History View."
        
    2. The History View displays a list of all saved versions in reverse chronological order.
        
    3. Each entry in the list clearly shows the version number, author, timestamp, and the user's notes for that change.
        
    4. The view is performant and loads in under 2 seconds.