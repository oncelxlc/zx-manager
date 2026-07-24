# Repository Guidelines

## Project Structure & Module Organization

The React 19 and TypeScript frontend lives in `src/`. Put route definitions in `src/app/`, shared layouts in `src/layouts/`, and route-level screens in `src/pages/<feature>/`. Imported images and styles belong in `src/assets/` and `src/styles/`; files that must be copied unchanged belong in `public/`.

The Tauri 2 backend lives in `src-tauri/`. Rust commands and application startup code are under `src-tauri/src/`, permissions are declared in `src-tauri/capabilities/`, and distributable icons are in `src-tauri/icons/`. Keep frontend-to-Rust command names synchronized with calls to `invoke()`.

## Build, Test, and Development Commands

Use pnpm; commit changes to `pnpm-lock.yaml` whenever dependencies change.

- `pnpm install` installs JavaScript dependencies and configures Husky hooks.
- `pnpm dev` starts the Vite frontend on port 1420.
- `pnpm tauri:dev` runs the complete desktop application with hot reload.
- `pnpm build` type-checks the frontend and creates the Vite production bundle.
- `pnpm tauri build` creates platform-specific desktop packages.
- `cargo test --manifest-path src-tauri/Cargo.toml` runs Rust tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check` verifies Rust formatting.

## Coding Style & Naming Conventions

Follow the existing TypeScript style: two-space indentation, double quotes, semicolons, and explicit imports. TypeScript strict mode is enabled; do not leave unused locals or parameters. Name React components and component files in PascalCase (`MainLayout.tsx`), page components with a `Page` suffix, and functions or variables in camelCase. Keep Rust code compatible with `rustfmt`; use snake_case for functions and modules.

No ESLint or Prettier configuration is currently present. Avoid unrelated formatting churn and match the surrounding file.

## Testing Guidelines

The frontend currently has no configured test runner or coverage threshold. For every change, run `pnpm build` and manually exercise affected flows through `pnpm tauri:dev`. Add Rust unit tests beside the implementation using `#[cfg(test)]`. If introducing frontend tests, first add a documented test script and use `*.test.ts` or `*.test.tsx`.

## Commit & Pull Request Guidelines

Commits are enforced by Commitlint and follow Conventional Commits, such as `feat: add settings route` or `fix: handle failed Tauri invoke`. Keep each commit focused. Pull requests should summarize behavior changes, list verification commands, link related issues, and include screenshots or recordings for visible UI changes. Call out changes to Tauri capabilities, configuration, or dependencies explicitly.
