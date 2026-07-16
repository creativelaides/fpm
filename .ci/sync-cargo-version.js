// Syncs the version from package.json into Cargo.toml's [package] version field.
// Run as part of `pnpm version:prepare` after `changeset version`.
//
// This ensures Cargo.toml stays in sync with the npm/changesets-managed version.

const fs = require("node:fs");
const path = require("node:path");

const ROOT = path.resolve(__dirname, "..");

function main() {
  // Read version from package.json
  const pkgPath = path.join(ROOT, "package.json");
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  const { version } = pkg;

  if (!version) {
    console.error("sync-cargo-version: no version found in package.json");
    process.exit(1);
  }

  // Read Cargo.toml
  const cargoPath = path.join(ROOT, "Cargo.toml");
  let cargo = fs.readFileSync(cargoPath, "utf8");

  // Replace the version field in [package] section.
  // Matches: version = "x.y.z"  (first occurrence, which is in [package])
  const versionRe = /^version\s*=\s*"[^"]*"/m;
  if (!versionRe.test(cargo)) {
    console.error("sync-cargo-version: could not find version field in Cargo.toml");
    process.exit(1);
  }

  cargo = cargo.replace(versionRe, `version = "${version}"`);

  fs.writeFileSync(cargoPath, cargo, "utf8");
  console.log(`sync-cargo-version: Cargo.toml version set to ${version}`);
}

main();