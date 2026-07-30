# Repository Guidelines

## Project Structure & Module Organization

Scanopy is a Rust and SvelteKit application. `backend/src/server/` contains the HTTP API; `backend/src/daemon/` contains discovery and daemon code. Rust binaries live in `backend/src/bin/`, migrations in `backend/migrations/`, and integration tests in `backend/tests/integration/`. The UI is under `ui/src/`: routes are in `routes/`, feature code in `lib/features/`, shared code in `lib/shared/`, and Vitest tests in `ui/src/tests/`. Static files belong in `ui/static/`, product images in `media/`, and supporting scripts in `tools/`.

## Build, Test, and Development Commands

- `make setup-db && make migrate-db`: start PostgreSQL 17 and apply migrations.
- `make dev-server`, `make dev-ui`, `make dev-daemon`: run each component locally with development settings.
- `make dev-container`: run the complete stack through `docker-compose.test.yml`.
- `make test-unit`: run Vitest and Rust library tests without integration infrastructure.
- `make test`: run the full frontend and backend/integration suite; it tears down development containers.
- `make format` and `make lint`: format, then check Rust, UI, Svelte types, and migrations.
- `make build`: build production server and daemon images.

## Coding Style & Naming Conventions

Use `cargo fmt` defaults for Rust. Keep modules and functions `snake_case`, types `PascalCase`, and constants `SCREAMING_SNAKE_CASE`. UI code follows `ui/.prettierrc`: tabs, single quotes, no trailing commas, and 100-column lines. Name Svelte components `PascalCase.svelte` and tests `*.test.ts`. Do not hand-edit generated API types or Paraglide output; use `make generate-types` or `make generate-messages`.

## Testing Guidelines

Add Rust unit tests beside the implementation and database/API scenarios under `backend/tests/integration/`. Add frontend tests under `ui/src/tests/` using Vitest. No numeric coverage threshold is documented, but behavior changes should include regression tests. Run `make test-unit` while iterating and `make test` before submission.

## Commit & Pull Request Guidelines

Recent commits use short, imperative, sentence-case subjects, such as `Fix host create wizard stuck on IP-addresses step`. Use focused `feature/...` or `fix/...` branches and keep one logical change per PR. Before opening a PR, run `make format`, `make lint`, and `make test`. Explain the problem, solution, testing, breaking changes, and linked issue; include screenshots for UI changes. Add review fixes as new commits rather than force-pushing.

## Database & Configuration Safety

Keep migrations backward-compatible because old and new application versions may run concurrently. Run `make lint-migrations` for migration changes. Never commit credentials, license keys, local database dumps, or populated `.env`/TOML configuration files.
