import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";

const wasmInput = resolve(
  "target",
  "wasm32-unknown-unknown",
  "release",
  "fastmulp_wasm.wasm",
);

if (!existsSync(wasmInput)) {
  throw new Error(`fastmulp wasm build output was not found at ${wasmInput}`);
}

const tempDir = mkdtempSync(join(tmpdir(), "fastmulp-wasm-"));

try {
  execFileSync(
    "wasm-bindgen",
    [wasmInput, "--target", "nodejs", "--out-dir", tempDir, "--out-name", "fastmulp_wasm"],
    { stdio: "inherit" },
  );

  const require = createRequire(import.meta.url);
  const wasm = require(join(tempDir, "fastmulp_wasm.js"));

  assert.deepEqual(Object.keys(wasm).sort(), [
    "boundary_from_content_type",
    "parse",
    "parseContentType",
  ]);

  const body = Buffer.from(
    '--abc123\r\nContent-Disposition: form-data; name="field"\r\n\r\npayload\r\n--abc123--\r\n',
  );
  const parts = wasm.parse(body, "abc123");
  assert.equal(parts.length, 1);
  assert.equal(parts[0].name, "field");
  assert.equal(body.subarray(parts[0].body_start, parts[0].body_end).toString(), "payload");

  const escapedBoundaryBody = Buffer.from(
    '--abc:123\r\nContent-Disposition: form-data; name="field"\r\n\r\npayload\r\n--abc:123--\r\n',
  );
  const contentTypeParts = wasm.parseContentType(
    escapedBoundaryBody,
    'multipart/form-data; boundary="abc\\:123"',
  );
  assert.equal(contentTypeParts[0].name, "field");
  assert.equal(
    wasm.boundary_from_content_type('multipart/form-data; boundary="abc\\:123"'),
    "abc:123",
  );
} finally {
  rmSync(tempDir, { force: true, recursive: true });
}
