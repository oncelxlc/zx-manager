# Repository Guidelines

## Project Overview

ZxManager is a React 19 + Tauri 2 desktop console for local infrastructure management. The repository currently combines real, permission-scoped desktop capabilities with a mocked service-management prototype:

- System summary and detailed system information are collected by Rust commands.
- Diagnostic text is written through the Tauri clipboard plugin.
- Theme and locale preferences use Tauri Store, with legacy `localStorage` migration and fallback.
- A static, permissionless splashscreen is shown while the hidden main window loads preferences, theme, localization, and the initial route. The main window performs an explicit permission-scoped handoff after its layout mounts.
- Application restart uses the Tauri Process plugin.
- Dashboard Nginx status and Start/Stop/Restart operations use the real, permission-scoped Nginx backend. Other service rows and their operations remain frontend-only mocks in `src/services/tauri/service-manager.ts`.
- Nginx uses a strict singleton registry, an app-lifetime two-second status Channel, and a shared non-queuing operation lock. Windows portable builds additionally support confirmed, signature-verified upgrades with critical snapshots and automatic rollback.

Keep that boundary explicit. Never describe a mocked Dashboard action as real system control. Any new host-level operation requires an auditable Rust command or official Tauri plugin plus the narrowest practical capability permission.

## Project Structure & Module Organization

The React and TypeScript application lives in `src/`:

- `src/app/` contains route definitions.
- `src/layouts/` contains shared layouts and the route-aware header contract.
- `src/pages/<feature>/` contains route-level screens.
- `src/components/<feature>/` contains application and feature components.
- `src/data/` contains stable mock data.
- `src/services/tauri/` is the frontend boundary for Tauri and mocked desktop operations.
- `src/services/storage/` owns preference persistence and migration.
- `src/stores/` contains Zustand stores.
- `src/types/`, `src/utils/`, and `src/i18n/` contain domain types, pure helpers, and localization.
- `src/test/` contains shared test setup and fixtures.
- Imported assets and global styles belong in `src/assets/` and `src/styles/`; unchanged public files belong in `public/`.

The reusable shadcn/Base UI foundation lives in the root `@/` directory. Primitives are in `@/components/ui/`, theme support is in `@/components/theme-provider.tsx`, hooks are in `@/hooks/`, and shared helpers are in `@/lib/`. Keep application-specific behavior in `src/`. Import the UI layer through `@/…` and application code through `src/…`. Both aliases are configured in `tsconfig.json` and `vite.config.ts`; update both files together if this convention changes.

The Tauri backend lives in `src-tauri/`. Application commands and startup are under `src-tauri/src/`; the system information implementation is split across `src-tauri/src/system_information/`. Main-window capabilities are declared in `src-tauri/capabilities/`, and application-command permissions are in `src-tauri/permissions/`. Keep Rust command names, generated permissions, frontend `invoke()` calls, and TypeScript DTOs synchronized.

## Application Conventions

- Preserve the two-window startup sequence: show `public/splashscreen.html`, keep `main` hidden, render the React fallback, load preferences, apply the theme, initialize i18n, mount the initial route, and then invoke the idempotent startup handoff. Preference or localization failures must not prevent the shell from opening.
- Use `react-i18next` for all user-visible product text. Add matching keys to `src/i18n/locales/zh-CN.ts` and `src/i18n/locales/en-US.ts`; keep identifiers stable and translate dynamic labels at the rendering edge.
- Read and write UI preferences only through `src/services/storage/preferences-storage.ts`. Tauri Store is primary; the `localStorage` path is retained for migration and graceful browser/plugin fallback.
- Keep all frontend-to-desktop calls behind `src/services/tauri/`. Components and stores should not call `invoke()`, plugins, or browser storage directly.
- Treat system information command results as fallible and partially available. Preserve stable warning/error codes, nullable DTO fields, stale-data-on-refresh behavior, and request-order protection in the Zustand store.
- Diagnostic export must remain an explicit allowlist. Do not add host names, disk mount points, driver details, or future DTO fields to copied diagnostics without a privacy review.
- Use the existing shadcn/Base UI primitives and `cn()` helper rather than adding duplicate primitive components or another styling system.
- Keep route pages as orchestration layers: data ownership, persistence and external side effects belong in a page controller, store, or service; repeated presentation and independent interaction state belong in focused feature components. Split a component when it crosses those boundaries instead of only splitting by file length.
- Production React components in `src/**/*.tsx` must have no more than 200 effective lines. `pnpm check:component-size` excludes import declarations, blank lines, and pure comment lines; test files and hooks/pure utilities are not component gate targets. Run it with the normal frontend checks (it is also part of `pnpm typecheck`).
- Split oversized components first at data/control, independent interaction, and presentation-domain boundaries. An unavoidable exception requires an immediately adjacent `// component-size-exception: <specific safety or integration reason>` comment above the component declaration; never use a generic length-based reason.
- Add short comments for non-obvious lifecycle, asynchronous ordering, virtualization fallback, pagination/sorting, and security boundaries. Do not annotate self-evident JSX or ordinary props.
- Derive values from props, store state, or local inputs with pure functions and `useMemo` when needed; do not mirror them in `useState`. Use `useEffect` only to synchronize with external systems (Tauri, subscriptions, timers, browser APIs, or async persistence), never to keep derived UI state in sync.
- Keep route-level lazy loading. Load heavy charts, PDF viewers, and code editors with `React.lazy`/`Suspense` and a layout-stable fallback. Current Dashboard charts follow this rule; PDF and editor modules do not yet exist.
- Use `@tanstack/react-virtual` for unbounded service, log, or file lists, retaining semantic table/list controls and bounded overscan. The current service table is virtualized; logs and files need an auditable, permission-scoped backend before a UI is added.
- Use Tauri `Channel` for high-throughput streams such as logs and Nginx status. Keep a single-flight subscription boundary, explicit cleanup, ordering/generation checks, and bounded frontend buffers.
- Keep page-specific header state in the route page and register it with `MainLayout`; do not move feature loading state into the layout merely to render header actions.

## Security and Tauri Boundaries

- Grant the narrowest Tauri capability required for each command or plugin action. Do not replace specific permissions with broad defaults for convenience.
- Keep the `splashscreen` window outside every capability. Only `main` may invoke `complete_startup`, which must show the main window before closing the splashscreen and remain safe to call more than once.
- Run blocking system collection work outside the async UI path. Keep command errors serializable and suitable for localization at the frontend edge.
- Do not execute arbitrary shell strings from the WebView. Validate identifiers and arguments again in Rust when real service management is introduced.
- Keep non-Nginx service management mocked until a platform-specific, permission-scoped backend and corresponding tests exist. Never route Nginx Dashboard actions through the mock service manager.
- Nginx registry v2 stores at most one instance. A legacy v1 registry with multiple entries must block control, configuration reads, and upgrades until the user selects one record to keep; migration must archive v1 and never delete Nginx files.
- Nginx status subscriptions use generation/sequence ordering and explicit cleanup. Portable runtime detection must verify PID ownership and executable identity; an unverified process using the same binary is a conflict and must block start.
- Managed Nginx upgrades are Windows portable-only and user-confirmed. Accept release URLs only from cached nginx.org metadata, verify the detached signature against pinned full fingerprints, reject unsafe ZIP entries, snapshot only `nginx.exe`, `conf/**`, and `modules/**`, and automatically roll back failures after replacement.
- Call out every dependency, capability, generated permission, or `tauri.conf.json` change in the final summary and pull request.

## Build, Test, and Development Commands

Use pnpm and commit `pnpm-lock.yaml` whenever JavaScript dependencies change. Commit `src-tauri/Cargo.lock` whenever Rust dependencies change.

- `pnpm install` installs JavaScript dependencies and configures Husky hooks.
- `pnpm dev` starts Vite on port 1420.
- `pnpm tauri:dev` runs the complete desktop application with hot reload.
- `pnpm typecheck` runs TypeScript without emitting files.
- `pnpm test` runs the Vitest suite once.
- `pnpm test:watch` runs Vitest in watch mode.
- `pnpm build` type-checks and builds the frontend.
- `pnpm tauri:build` creates platform-specific desktop packages.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` verifies Rust formatting.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` treats Rust lints as errors.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs Rust tests.

## Coding Style & Naming Conventions

Follow the existing TypeScript style: two-space indentation, double quotes, semicolons, and explicit imports. TypeScript strict mode and unused checks are enabled. Name React components and component files in PascalCase (`MainLayout.tsx`), page components with a `Page` suffix, and functions or variables in camelCase. Prefer named exports for feature components, explicit type imports, and `satisfies` where it preserves useful static checking.

Keep Rust compatible with `rustfmt`; use snake_case for functions and modules. Prefer typed DTOs and structured errors over free-form JSON or strings.

No ESLint or Prettier configuration is present. Avoid unrelated formatting churn and match surrounding files.

## Testing Guidelines

Frontend tests use Vitest, jsdom, React Testing Library, `user-event`, and jest-dom. Shared setup is in `src/test/setup.ts`. Place tests beside their implementation using `*.test.ts` or `*.test.tsx`, and use Tauri mocks rather than invoking the host in jsdom.

- For frontend logic or UI changes, run `pnpm typecheck`, `pnpm test`, and `pnpm build`.
- For visible UI changes, smoke-test the desktop flow with `pnpm tauri:dev`. When responsive layout changes, also use the Web frontend to check Sidebar behavior below 1024px; tables must scroll inside their own container rather than the document body.
- For Rust changes, run `cargo fmt`, Clippy with warnings denied, and `cargo test`.
- For Tauri command, plugin, or capability changes, also run the frontend checks and exercise the affected desktop flow.
- Add Rust unit tests beside the implementation using `#[cfg(test)]`. Preserve coverage for DTO normalization, GPU deduplication, frontend request ordering, failure fallback, and diagnostic filtering when those areas change.
- Documentation-only changes do not require application builds, but commands, paths, links, and Markdown formatting must be checked against the current repository.

## Documentation Guidelines

Keep `README.md` user-facing and `AGENTS.md` contributor/agent-facing. Update both whenever project capabilities, mock-versus-real boundaries, scripts, persistence, directory conventions, or Tauri permissions materially change. Do not claim cross-platform validation for platforms that were not actually tested.

## Commit & Pull Request Guidelines

Commits are enforced by Commitlint and follow Conventional Commits, such as `feat: add settings route` or `fix: handle failed Tauri invoke`. Keep each commit focused. Pull requests should summarize behavior changes, list verification commands, link related issues, and include screenshots or recordings for visible UI changes. Explicitly call out changes to Tauri capabilities, configuration, generated permissions, or dependencies.
