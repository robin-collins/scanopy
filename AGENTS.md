# Repository Guidelines

## Project Structure & Module Organization

Scanopy is a Rust and SvelteKit application. `backend/src/server/` contains the HTTP API; `backend/src/daemon/` contains discovery and daemon code. Rust binaries live in `backend/src/bin/`, migrations in `backend/migrations/`, and integration tests in `backend/tests/integration/`. The UI is under `ui/src/`: routes are in `routes/`, feature code in `lib/features/`, shared code in `lib/shared/`, and Vitest tests in `ui/src/tests/`. Static files belong in `ui/static/`, product images in `media/`, and supporting scripts in `tools/`.

`ui/src/lib/data/*.json` (fixtures like `service-definitions.json`, `credential-types.json`, `host-os-groups.json`, `scan-settings.json`) and `ui/src/lib/paraglide/` are **generated, gitignored, and not committed**. They only exist after running `make generate-fixtures` / `npx paraglide-js compile`. A clean checkout (including every CI run) has none of them until those generators run.

## Build, Test, and Development Commands

- `make setup-db && make migrate-db`: start PostgreSQL 17 and apply migrations.
- `make dev-server`, `make dev-ui`, `make dev-daemon`: run each component locally with development settings. `dev-server` runs `generate-fixtures` first; the others do not.
- `make dev-container`: run the complete stack through `docker-compose.test.yml`.
- `make generate-fixtures`: regenerate `ui/src/lib/data/*.json` from the backend's `TypeMetadataProvider`/field-definition sources. Run this after touching any enum or struct that backs a fixture (see the CI gate section below) — **before** running frontend tests or `npm run check`, and before `node scripts/generate-meta-messages.js`.
- `make test-unit`: run Vitest and Rust library tests without integration infrastructure. **Does not regenerate fixtures.** If fixtures are stale or missing on disk, frontend results can differ from a clean CI checkout — run `make generate-fixtures` first whenever backend fixture-source data changed, don't rely on `test-unit` alone to catch fixture drift.
- `make test`: run the full frontend and backend/integration suite; it tears down development containers.
- `make format` and `make lint`: format, then check Rust, UI, Svelte types, and migrations.
- `make build`: build production server and daemon images.

## Coding Style & Naming Conventions

Use `cargo fmt` defaults for Rust. Keep modules and functions `snake_case`, types `PascalCase`, and constants `SCREAMING_SNAKE_CASE`. UI code follows `ui/.prettierrc`: tabs, single quotes, no trailing commas, and 100-column lines. Name Svelte components `PascalCase.svelte` and tests `*.test.ts`. Do not hand-edit generated API types or Paraglide output; use `make generate-types` or `make generate-messages`.

## Testing Guidelines

Add Rust unit tests beside the implementation and database/API scenarios under `backend/tests/integration/`. Add frontend tests under `ui/src/tests/` using Vitest. No numeric coverage threshold is documented, but behavior changes should include regression tests. Run `make test-unit` while iterating and `make test` before submission.

## CI Verification Gate — Run This Exact Sequence Before Pushing

Every push to `forkThat` touching `backend/**`, `ui/**`, or `messages/**` runs the `verify-source` job in `.github/workflows/publish-fork-images.yml`. It is **stricter than `make lint`/`make test`** — notably it adds `--locked` to every cargo command and runs `cargo audit`, neither of which the local Makefile targets do. Run the CI job's exact commands locally before pushing, not just `make lint`:

```bash
cd backend
cargo fmt --all --check
cargo clippy --locked --bin server -- -D warnings
cargo clippy --locked --bin daemon -- -D warnings
cargo clippy --locked --bin daemon --features ad-gssapi -- -D warnings
cargo test --locked --lib
cargo install cargo-audit --version 0.22.2 --locked   # once
cargo audit
cargo run --locked --bin generate-fixtures

cd ../ui
npm ci        # not `npm install` — must match package-lock.json exactly, same as CI
npm test
npm run check
npm run lint
```

## Recurring CI Failure Patterns (observed across this branch's run history — check these specifically)

1. **Unformatted Rust (`cargo fmt --all --check`) — the single most common failure.** Seen repeatedly (`winrm`/`ntlm`/`soap`/`credentials` modules, `categories` module, `storage/factory.rs`, etc.), including from commits whose *stated purpose* was to fix a previous CI failure. Always run `cargo fmt --all` (whole workspace, not `cargo fmt` on one file) immediately before every commit — don't trust an editor's format-on-save to have covered every file you touched.
2. **`clippy::large_enum_variant` after adding fields to a struct used inside an enum.** Adding fields to `CreateHostRequest` pushed `HostCreateRequestBody::New` past clippy's size-difference threshold against `HostCreateRequestBody::Legacy` — and boxing only the large variant just made the *other* variant the new outlier. When boxing one variant to fix this lint, check whether the enum has other variants close to the threshold and box those too. Run `cargo clippy --bin server -- -D warnings` (and `--bin daemon`, and `--bin daemon --features ad-gssapi`) after any field addition to a struct nested in an enum, not only after adding new code paths.
3. **Stale/missing `messages/en.json` `meta_*` keys.** Any change to a Rust enum/struct backing a `ui/src/lib/data/*.json` fixture (e.g. `HostOsGroup` variants, `ScanSettings::field_definitions()`) requires `make generate-fixtures && node ui/scripts/generate-meta-messages.js` before committing — in **both directions**: to add keys for new variants/fields, and to prune keys for removed/renamed ones. `ui/src/tests/i18n-meta-keys.test.ts` fails on either drift direction and is not caught by `make test-unit` if fixtures were already stale/missing on disk.
4. **New backend entity/table not registered in the entity-mapping test.** Adding a new `Storable`/`Entity` impl (new table) must be added to `get_entity_deserializers()` in `backend/src/server/shared/storage/tests.rs`, or `test_all_tables_have_entity_mapping` panics. This is easy to miss because the new entity compiles and works fine — only this specific integration test catches the missing registration.
5. **Single-word i18n values without a `common_` prefix.** `ui/src/tests/i18n-unused-keys.test.ts` requires any translation value with no spaces and no `{placeholder}` (e.g. `"Icon"`, `"Model"`, `"Built-in"`) to live under a `common_*` key rather than a feature-specific prefix, so identical single-word strings are shared across features. Check new message keys against this rule before adding them — `npm run check` (svelte-check) does **not** catch this; only `npm test` does.
6. **ESLint errors not caught by `svelte-check`.** `npm run check` (TypeScript/Svelte diagnostics) and `npm run lint` (ESLint, includes `@typescript-eslint/no-unused-vars`) check different things — a clean `npm run check` does not mean `npm run lint` is clean. Run both.
7. **`Cargo.lock` drift under `--locked`.** CI uses `--locked` on every cargo command (including inside the Dockerfile's `cargo chef` build stage); local `make lint`/`make test` do not pass `--locked` at all. After editing `Cargo.toml` (bumping a dependency, adding one), run a normal (non-locked) `cargo check` once to refresh `Cargo.lock`, then commit the lockfile change alongside the `Cargo.toml` edit — don't assume an unlocked local build passing means a `--locked` CI build will.
8. **`npm ci` requires an exactly-in-sync lockfile.** If you ran `npm install` locally (which tolerates minor `package-lock.json` drift), verify `npm ci` succeeds cleanly before pushing — CI fails on any mismatch `npm install` would silently absorb.
9. **Generated fixture/message files must never be committed, but must exist before running frontend checks.** `ui/src/lib/data/*.json` and `ui/src/lib/paraglide/` are gitignored. Running `npm test`/`npm run check` without first running `make generate-fixtures` (backend) and letting paraglide compile produces `Cannot find module`/`ENOENT` errors that look like missing files but are actually just "generator wasn't run yet" — the fix is to generate, not to create/commit the files.
10. **Multi-arch Docker build/scan failures are usually infra, not your code.** `netlink-packet-route`-style "failed to select a version" errors inside `cargo chef cook` come from the *release* build's own `--locked` resolution inside Docker — verify with a local `cargo build --locked --release` if the CI-only Docker build stage fails while local checks pass. Trivy "unable to find the specified image" errors on one architecture usually mean an earlier matrix job (build-server/build-daemon for that arch) failed or was skipped — check the *other* jobs in the same run before assuming the scan step itself is broken.

## Commit & Pull Request Guidelines

Recent commits use short, imperative, sentence-case subjects, such as `Fix host create wizard stuck on IP-addresses step`. Use focused `feature/...` or `fix/...` branches and keep one logical change per PR. Before opening a PR, run `make format`, `make lint`, and `make test` — then also run the CI Verification Gate sequence above, since it is stricter than the Makefile targets. Explain the problem, solution, testing, breaking changes, and linked issue; include screenshots for UI changes. Add review fixes as new commits rather than force-pushing.

A `.githooks/pre-commit` hook (when present/enabled via `core.hooksPath`) runs `make format && git add -u && make lint && make test` automatically — this is slow (several minutes) because it runs the *full* test suite, not just `test-unit`. Let it run to completion rather than interrupting it; killing it mid-run can leave a stale `.git/index.lock`.

## Database & Configuration Safety

Keep migrations backward-compatible because old and new application versions may run concurrently. Run `make lint-migrations` for migration changes. Never commit credentials, license keys, local database dumps, or populated `.env`/TOML configuration files.
