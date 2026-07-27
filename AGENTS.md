# Repository Guidelines

## Project Overview

ZxManager is a React 19 + Tauri 2 desktop console for local infrastructure management. The present dashboard is a frontend prototype: service data and Start/Stop/Restart operations are mocked in `src/services/tauri/service-manager.ts`. The Rust backend currently exposes only the template `greet` command. Do not represent or implement a UI-only flow as real system control without adding an explicit, permission-scoped Tauri command.

## Project Structure & Module Organization

The React 19 and TypeScript frontend lives in `src/`. Put route definitions in `src/app/`, shared layouts in `src/layouts/`, and route-level screens in `src/pages/<feature>/`. Put feature components in `src/components/<feature>/`, domain data in `src/data/`, service boundaries in `src/services/`, types in `src/types/`, and translations in `src/i18n/`. Imported images and styles belong in `src/assets/` and `src/styles/`; files that must be copied unchanged belong in `public/`.

The shadcn/Base UI foundation lives in the root `@/` directory: primitives are in `@/components/ui/`, theme support is in `@/components/theme-provider.tsx`, and shared helpers are in `@/lib/`. Keep that layer reusable and framework-oriented; put application-specific UI in `src/`. Import it through `@/…`. The separate `src/…` alias is for application code. These aliases are configured in both `tsconfig.json` and `vite.config.ts`; update both files together if the convention changes.

The Tauri 2 backend lives in `src-tauri/`. Rust commands and application startup code are under `src-tauri/src/`, permissions are declared in `src-tauri/capabilities/`, and distributable icons are in `src-tauri/icons/`. Keep frontend-to-Rust command names synchronized with calls to `invoke()`.

## Application Conventions

- The app bootstraps preferences, theme, and i18n before rendering in `src/main.tsx`. Preserve this order to avoid language and theme flash.
- Use `react-i18next` for all new user-visible product text. Add matching keys to `src/i18n/locales/zh-CN.ts` and `src/i18n/locales/en-US.ts`; keep identifiers stable and translate dynamic labels at the rendering edge.
- Persist UI preferences through `src/services/storage/preferences-storage.ts`, unless the project intentionally adopts a Tauri storage solution.
- Keep service-management calls behind `src/services/tauri/`. When real functionality is introduced, implement an allowlisted Rust command and the required capability before replacing the Mock implementation.
- Use existing shadcn/Base UI primitives and the `cn()` helper rather than introducing duplicate primitive components or new styling systems.

## Build, Test, and Development Commands

Use pnpm; commit changes to `pnpm-lock.yaml` whenever dependencies change.

- `pnpm install` installs JavaScript dependencies and configures Husky hooks.
- `pnpm dev` starts the Vite frontend on port 1420.
- `pnpm tauri:dev` runs the complete desktop application with hot reload.
- `pnpm build` type-checks the frontend and creates the Vite production bundle.
- `pnpm tauri build` creates platform-specific desktop packages.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs Rust tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` verifies Rust formatting.

Run `pnpm build` for every frontend change. For visible UI changes, also smoke-test the relevant flow using `pnpm tauri:dev` at desktop widths; the sidebar changes behavior below 1024px and tables should scroll within their own container rather than the document body.

## Coding Style & Naming Conventions

Follow the existing TypeScript style: two-space indentation, double quotes, semicolons, and explicit imports. TypeScript strict mode is enabled; do not leave unused locals or parameters. Name React components and component files in PascalCase (`MainLayout.tsx`), page components with a `Page` suffix, and functions or variables in camelCase. Keep Rust code compatible with `rustfmt`; use snake_case for functions and modules.

No ESLint or Prettier configuration is currently present. Avoid unrelated formatting churn and match the surrounding file. Prefer named exports for feature components, preserve explicit type imports, and use `satisfies` where it strengthens static data or handler maps without widening their types.

## Testing Guidelines

The frontend currently has no configured test runner or coverage threshold. For every change, run `pnpm build` and manually exercise affected flows through `pnpm tauri:dev`. Add Rust unit tests beside the implementation using `#[cfg(test)]`. If introducing frontend tests, first add a documented test script and use `*.test.ts` or `*.test.tsx`. Run `cargo fmt --manifest-path src-tauri/Cargo.toml --check` with Rust changes, and run `cargo test --manifest-path src-tauri/Cargo.toml` when Rust behavior changes.

## Commit & Pull Request Guidelines

Commits are enforced by Commitlint and follow Conventional Commits, such as `feat: add settings route` or `fix: handle failed Tauri invoke`. Keep each commit focused. Pull requests should summarize behavior changes, list verification commands, link related issues, and include screenshots or recordings for visible UI changes. Call out changes to Tauri capabilities, configuration, or dependencies explicitly.
