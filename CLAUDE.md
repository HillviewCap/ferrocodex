# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ferrocodex is an offline-first OT (Operational Technology) configuration management desktop app built with Tauri 2.0: a Rust backend (`apps/desktop/src-tauri`) and a React 18 + TypeScript + Vite + Ant Design + Zustand frontend (`apps/desktop/src`). It is an npm-workspaces/Turborepo monorepo with two packages: `apps/desktop` (the app) and `apps/website` (the static ferrocodex.com site, plain HTML/CSS/JS under `public/`, deployed as Cloudflare Worker static assets with `npm run deploy -w apps/website`; see its README).

## Commands

```bash
# From root
npm install            # always from root; the root package-lock.json is the only lockfile
npm run dev            # turbo → vite dev server only (port 1420, strictPort)
npm run build          # turbo → vite build
npm run test           # turbo → runs build first (turbo.json dependsOn), then vitest

# From apps/desktop
npm run tauri:dev      # full app: Vite + Rust backend
npm run tauri:build    # production bundle
npm run test:run       # vitest once (`npm run test` is watch mode)
npm run test:run -- src/components/Dashboard.test.tsx   # single file
npm run test:run -- -t "test name"                        # single test by name
npm run typecheck      # tsc --noEmit (also run in CI)

# Backend (from root)
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml -- vault::   # tests under one module
```

There is no linter configured: `npm run lint` resolves to a turbo task that no package defines. CI (`.github/workflows/test.yml`) runs `npm run test` in `apps/desktop` and `cargo test` on macOS, Ubuntu 22.04, and Windows. On Linux, building requires `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

## Architecture

### Backend (Rust)

- **Entry point**: `src-tauri/src/lib.rs` declares all modules, registers managed state, and lists every command in `generate_handler!` (~176 active). A new command must be added there or the frontend can't call it. The workflow, association, and enhanced tree-navigation commands are currently commented out ("temporarily disabled").
- **Managed state** (`lib.rs`): `DatabaseState = Mutex<Option<Database>>` (it is `None` until the frontend calls `initialize_database`), `SessionManagerState`, `LoginAttemptTrackerState`, and `RateLimiterState`.
- **Commands** (`src/commands/*_commands.rs`, glob re-exported from `commands/mod.rs`) hold the real request logic. The usual shape is: take `token: String` as the first argument, lock `SessionManagerState` and call `validate_session(&token)`, drop the guard, sanitize inputs with `validation::InputSanitizer`, check roles, lock the DB, call a repository, write an audit event, and return `Result<T, String>`. Vault commands also enforce permissions server-side with `require_vault_permission` / `require_secret_permission` (`commands/vault_commands.rs`): admin has everything, a vault's creator has Read/Write/Export, anyone else needs an unexpired grant. Secret values go through `InputSanitizer::validate_secret_value`, never `sanitize_string`/`is_potentially_malicious`, so they are stored byte-for-byte and never logged.
- **`src/handlers/`** is mostly dead code. Only `workflow_commands.rs` (which is disabled) imports from it. Put new logic in commands or domain modules, not handlers.
- **Domain modules** (`assets/`, `configurations/`, `vault/`, `firmware/`, ...): most keep models, an `XRepository` trait, and a `SqliteXRepository<'a>` wrapping `&Connection` together in `mod.rs`. Only some (`metadata/`, `workflow/`, `firmware_analysis/`) have a separate `repository.rs`.
- **Schema**: there is no migration runner. Each repository has an `initialize_schema()` using `CREATE TABLE IF NOT EXISTS`, and all of them are called in order from `database::Database::initialize_schema()`. Data fixes go in `Database::run_data_migrations()`. A new table must be registered in that function. The `.sql` files in `src-tauri/migrations/` are not referenced by code.
- **Encryption**: the SQLite DB (`<app_data_dir>/ferrocodex.db`, WAL mode) is **not** encrypted as a whole. AES-256-GCM (`encryption::FileEncryption`) encrypts specific payloads: configuration file content, firmware files on disk, vault secrets, standalone credentials. All of them use one random install-wide `DataKey`, which carries a `version` (rows in `data_keys`: `active`, `retiring` during a rotation, `retired`). `keystore/` never stores it in plaintext. Each user has an X25519 keypair (`user_keypairs`, private key wrapped under their login password with Argon2id) and the key is sealed to their public key in `data_key_public_wraps`; one more copy is wrapped under the recovery code shown at setup (`data_key_wraps`, kind `recovery`). Users from before keypairs have a legacy password wrap (`data_key_wraps`, kind `user`) that `unlock_user` converts to a keypair at their next login. Login unwraps the key into the in-memory session; commands get it with `session.data_key()?` and pass `&DataKey` to repository methods that encrypt or decrypt. While a rotation is in progress (`keystore/rotation.rs`, `commands/key_rotation_commands.rs`) `data_key()` errors for every session; only the rotation commands and Replace Recovery Key use `session.rotation_key()`. A new user needs `KeyStoreRepository::set_user_keys` (it replaced `grant_user`), or their session has no key; `deactivate_user` calls `revoke_user`. New code that writes ciphertext must set `key_version` (`key.version()`) on the row, or rotation will not find it. Vault exports and recovery bundles re-encrypt vault secrets under an export passphrase (`vault/export.rs`), because the data key never leaves the install. Design and threat model: `docs/data-key-rotation-design.md`.
- **Roles**: only `Administrator` and `Engineer`. They are enforced in commands and mirrored in the frontend by `src/utils/roleUtils.ts` and `components/RoleGuard.tsx`.

### Frontend

- **IPC**: components and Zustand stores (`src/store/*.ts`) call `invoke('snake_case_command', { camelCaseArgs })` directly. Tauri maps camelCase keys to the Rust snake_case parameters. Almost every call passes the session `token` from `useAuthStore`, which is persisted with zustand `persist`.
- **Types**: `src/types/*.ts` mirror the Rust serde structs by hand. If you change a Rust struct, update its TS type too.
- **Routing**: `App.tsx` has only `/setup` (first launch, admin creation), `/login`, and `/dashboard`. Feature screens are switched inside `Dashboard.tsx`, not through routes.
- **Tests**: `src/test-setup.ts` already mocks `@tauri-apps/api/core` `invoke` globally, along with `matchMedia`, `ResizeObserver`, and `getComputedStyle` for Ant Design. In tests, use `vi.mocked(invoke).mockResolvedValue(...)` rather than re-mocking. `tsconfig.json` excludes test files from type-checking.

## Versioning & Release

- The version string is duplicated in `package.json`, `apps/desktop/package.json`, `apps/desktop/src-tauri/Cargo.toml`, and `apps/desktop/src-tauri/tauri.conf.json`.
- Pushing a `v*` tag runs `release.yml`. It uses `tauri-action` to build Windows, macOS (arm64 and x64), and Linux, and creates a **draft** GitHub release.
- Windows installers are signed with Azure Artifact Signing (`artifact-signing-cli`) when the `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, and `AZURE_TENANT_ID` secrets exist; otherwise the Windows build is unsigned and the job logs a warning. The `signCommand` lives in `src-tauri/tauri.windows-signing.conf.json`, which only `release.yml` loads (via `--config`), so local builds don't need Azure credentials. Manual `workflow_dispatch` runs of `release.yml` build only and upload workflow artifacts (no release), which is how to test signing on a branch: `gh workflow run release.yml --ref <branch>`.
- When a push to `main` contains a Dependabot-authored commit, `post-dependabot-bump.yml` runs `.github/scripts/bump-version.mjs`. The script bumps the patch version in all four files plus `package-lock.json` and `Cargo.lock`. The workflow then commits, tags, and pushes, which triggers a release. It pushes with the `RELEASE_TOKEN` secret (an admin PAT), because `GITHUB_TOKEN` can't get past branch protection and its tag pushes don't trigger other workflows.

## Other Repo Contents

- `docs/`: Sphinx (`.rst`) user and admin docs published to Read the Docs (`.readthedocs.yaml`), plus PRDs and design notes in Markdown.
- `.bmad-core/`, `_bmad/`, `_bmad-output/`, `.claude/agents|commands`: BMAD story-driven workflow tooling, not application code.
- `src-tauri/*.txt` (`build_errors.txt`, etc.): stray build logs, not source.
