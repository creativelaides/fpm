# Changesets

This project uses [Changesets](https://github.com/changesets/changesets) to manage
versions and changelogs.

## Adding a changeset

Whenever you make a change that should be released (a new feature, a bug fix, a
breaking change), add a changeset:

```sh
pnpm changeset
```

This will prompt you to:

1. **Select the package** — for now there is only the root `fpm` package.
2. **Choose the bump type** — `major`, `minor`, or `patch`.
3. **Write a summary** — a human-readable description of the change. This goes
   into the changelog.

The command creates a new markdown file under `.changeset/` with a random name.
Commit that file alongside your code changes.

## Consuming changesets

When a PR with a changeset is merged into `main`, the Changesets GitHub Action
runs `pnpm version:prepare`, which:

1. Runs `changeset version` — consumes all pending changesets, bumps
   `package.json`, and updates `CHANGELOG.md`.
2. Runs `node .ci/sync-cargo-version.js` — syncs the new version from
   `package.json` into `Cargo.toml`.

If there are no pending changesets, the Action opens (or updates) a PR titled
"Version Packages" with the version bump. Merging that PR triggers the release.

## Version sync

The `version:prepare` script keeps `package.json` and `Cargo.toml` in sync.
Always bump the version via changesets — **never edit `Cargo.toml`'s version
field manually** unless you also update `package.json`.