import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const mode = process.argv.includes("--pack") ? "pack" : "dry-run";
const distRoot = join(root, "dist", "bindings");
const artifactsDir = join(distRoot, "artifacts");

const version = workspaceVersion();

rmSync(distRoot, { force: true, recursive: true });
mkdirSync(artifactsDir, { recursive: true });

const packages = [prepareNodePackage(), prepareWasmPackage()];
for (const packageDir of packages) {
  packPackage(packageDir);
}

function prepareNodePackage() {
  const packageDir = join(distRoot, "fastmulp-node");
  cpSync(join(root, "packages", "fastmulp-node"), packageDir, { recursive: true });
  copyFileSync(join(root, "LICENSE"), join(packageDir, "LICENSE"));
  copyFileSync(nativeLibraryPath(), join(packageDir, "fastmulp.node"));

  const platform = npmPlatform();
  const arch = npmArch();
  updatePackageJson(packageDir, {
    name: `fastmulp-node-${platform}-${arch}`,
    version,
    os: [platform],
    cpu: [arch],
  });
  return packageDir;
}

function prepareWasmPackage() {
  const packageDir = join(distRoot, "fastmulp-wasm");
  cpSync(join(root, "packages", "fastmulp-wasm"), packageDir, { recursive: true });
  copyFileSync(join(root, "LICENSE"), join(packageDir, "LICENSE"));

  const wasmInput = join(
    root,
    "target",
    "wasm32-unknown-unknown",
    "release",
    "fastmulp_wasm.wasm",
  );
  if (!existsSync(wasmInput)) {
    throw new Error(`wasm build output was not found at ${wasmInput}`);
  }

  execFileSync(
    "wasm-bindgen",
    [wasmInput, "--target", "bundler", "--out-dir", packageDir, "--out-name", "fastmulp_wasm"],
    { cwd: root, stdio: "inherit" },
  );
  updatePackageJson(packageDir, { version });
  return packageDir;
}

function packPackage(packageDir) {
  const args = ["pack", "--json"];
  if (mode === "pack") {
    args.push("--pack-destination", artifactsDir);
  } else {
    args.push("--dry-run");
  }

  const output = execFileSync("npm", args, {
    cwd: packageDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });
  const [packed] = JSON.parse(output);
  const action = mode === "pack" ? "packed" : "validated";
  console.log(`${action} ${packed.name}@${packed.version}`);
}

function updatePackageJson(packageDir, updates) {
  const packageJsonPath = join(packageDir, "package.json");
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  Object.assign(packageJson, updates);
  writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
}

function workspaceVersion() {
  const cargoToml = readFileSync(join(root, "Cargo.toml"), "utf8");
  const workspacePackage = cargoToml.match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/);
  const versionMatch = workspacePackage?.[1].match(/^\s*version = "([^"]+)"/m);
  if (!versionMatch) {
    throw new Error("workspace package version was not found in Cargo.toml");
  }
  return versionMatch[1];
}

function nativeLibraryPath() {
  const releaseDir = join(root, "target", "release");
  const candidates =
    process.platform === "darwin"
      ? ["libfastmulp_napi.dylib"]
      : process.platform === "win32"
        ? ["fastmulp_napi.dll", "fastmulp-napi.dll"]
        : ["libfastmulp_napi.so"];
  const nativeLibrary = candidates
    .map((candidate) => join(releaseDir, candidate))
    .find((candidate) => existsSync(candidate));

  if (!nativeLibrary) {
    throw new Error(`fastmulp napi build output was not found in ${releaseDir}`);
  }
  return nativeLibrary;
}

function npmPlatform() {
  if (process.platform === "win32") {
    return "win32";
  }
  return process.platform;
}

function npmArch() {
  if (process.arch === "x64" || process.arch === "arm64") {
    return process.arch;
  }
  throw new Error(`unsupported Node package architecture: ${process.arch}`);
}
