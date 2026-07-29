# Repository Guidelines

## Project Overview

ZxManager is a React 19 + Tauri 2 desktop console for local infrastructure management. The repository currently combines real, permission-scoped desktop capabilities with a mocked service-management prototype:

- System summary and detailed system information are collected by Rust commands.
- Diagnostic text is written through the Tauri clipboard plugin.
- Theme and locale preferences use Tauri Store, with legacy `localStorage` migration and fallback.
- Application restart uses the Tauri Process plugin.
- Network monitoring is opt-in and uses real platform interface counters, an in-memory realtime ring, and a local seven-day SQLite history.
- Application traffic attribution is deliberately unavailable in v1. Its abstraction, DTOs, and storage dimensions exist, but no estimate or mock traffic may be emitted.
- Dashboard service data and Start/Stop/Restart/Remove operations remain frontend-only mocks in `src/services/tauri/service-manager.ts`.

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

The Tauri backend lives in `src-tauri/`. Application commands and startup are under `src-tauri/src/`; the system information implementation is split across `src-tauri/src/system_information/`, and network monitoring is split across `src-tauri/src/network_monitor/`. Main-window capabilities are declared in `src-tauri/capabilities/`, and application-command permissions are in `src-tauri/permissions/`. Keep Rust command names, generated permissions, frontend `invoke()` calls, and TypeScript DTOs synchronized.

## Application Conventions

- Preserve the startup sequence in `src/main.tsx`: render the startup screen, load preferences, apply the theme, initialize i18n, and then render the application. Preference or localization failures must not prevent the shell from opening.
- Use `react-i18next` for all user-visible product text. Add matching keys to `src/i18n/locales/zh-CN.ts` and `src/i18n/locales/en-US.ts`; keep identifiers stable and translate dynamic labels at the rendering edge.
- Read and write UI preferences only through `src/services/storage/preferences-storage.ts`. Tauri Store is primary; the `localStorage` path is retained for migration and graceful browser/plugin fallback.
- Keep all frontend-to-desktop calls behind `src/services/tauri/`. Components and stores should not call `invoke()`, plugins, or browser storage directly.
- Treat system information command results as fallible and partially available. Preserve stable warning/error codes, nullable DTO fields, stale-data-on-refresh behavior, and request-order protection in the Zustand store.
- Diagnostic export must remain an explicit allowlist. Do not add host names, disk mount points, driver details, or future DTO fields to copied diagnostics without a privacy review.
- Use the existing shadcn/Base UI primitives and `cn()` helper rather than adding duplicate primitive components or another styling system.
- Keep page-specific header state in the route page and register it with `MainLayout`; do not move feature loading state into the layout merely to render header actions.
- Keep the network Manager disabled and its SQLite connection unopened until the user has configured monitoring and explicitly enables, queries, or clears it. Current-session state and the start-on-launch preference are independent.
- Network sampling uses the persisted 1/3/5/10-second preference, defaults to five seconds, and must not silently change based on realtime subscriber count.
- Network realtime consumers must discard old generations and out-of-order sequences, explicitly unsubscribe on page teardown, preserve `null` gaps, and keep bounded arrays.
- Never total physical, tunnel, and application layers together. Device totals include only operational non-loopback, non-virtual physical interfaces; VPN totals come from tunnel interfaces.
- Treat network interface counters as cumulative and reset the monotonic baseline after first samples, counter rollback, interface reconstruction, or long sleep gaps.

## Security and Tauri Boundaries

- Grant the narrowest Tauri capability required for each command or plugin action. Do not replace specific permissions with broad defaults for convenience.
- Run blocking system collection work outside the async UI path. Keep command errors serializable and suitable for localization at the frontend edge.
- Do not execute arbitrary shell strings from the WebView. Validate identifiers and arguments again in Rust when real service management is introduced.
- Keep service management mocked until a platform-specific, permission-scoped backend and corresponding tests exist.
- Network monitor commands are grouped into `allow-network-monitor-read`, `allow-network-monitor-control`, and `allow-network-monitor-clear`. Do not replace these sets with a broad default capability.
- The network database may store hashed interface identities and byte/time buckets only. Do not add raw MAC addresses, host names, remote addresses, domains, URLs, ports, packet contents, or command lines.
- Call out every dependency, capability, generated permission, or `tauri.conf.json` change in the final summary and pull request.

## Build, Test, and Development Commands

Use pnpm and commit `pnpm-lock.yaml` whenever JavaScript dependencies change. Commit `src-tauri/Cargo.lock` whenever Rust dependencies change.

Network monitoring uses `rusqlite` with bundled SQLite, `chrono-tz`, and `sha2`; Windows additionally uses `windows` and `winreg`, while macOS uses `libc`. Its eight commands include the control-scoped sampling interval setter; any changes to these dependencies, commands, the three permission sets, or the database schema must be called out explicitly.

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
