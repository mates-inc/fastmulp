import assert from "node:assert/strict";
import { mkdtempSync, copyFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";

const releaseDir = resolve("target", "release");
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

const tempDir = mkdtempSync(join(tmpdir(), "fastmulp-napi-"));
const addonPath = join(tempDir, "fastmulp.node");
copyFileSync(nativeLibrary, addonPath);

try {
  const require = createRequire(import.meta.url);
  const addon = require(addonPath);

  assert.deepEqual(Object.keys(addon).sort(), [
    "boundaryFromContentType",
    "parse",
    "parseContentType",
  ]);

  const body = Buffer.from(
    '--abc123\r\nContent-Disposition: form-data; name="field"\r\n\r\npayload\r\n--abc123--\r\n',
  );
  const parts = addon.parse(body, "abc123");
  assert.equal(parts.length, 1);
  assert.equal(parts[0].name, "field");
  assert.equal(body.subarray(parts[0].bodyStart, parts[0].bodyEnd).toString(), "payload");
  assert.deepEqual(parts[0].headers, [
    {
      name: "Content-Disposition",
      value: 'form-data; name="field"',
    },
  ]);

  const escapedBoundaryBody = Buffer.from(
    '--abc:123\r\nContent-Disposition: form-data; name="field"\r\n\r\npayload\r\n--abc:123--\r\n',
  );
  const contentTypeParts = addon.parseContentType(
    escapedBoundaryBody,
    'multipart/form-data; boundary="abc\\:123"',
  );
  assert.equal(contentTypeParts[0].name, "field");
  assert.equal(
    addon.boundaryFromContentType('multipart/form-data; boundary="abc\\:123"'),
    "abc:123",
  );

  const invalidUtf8Name = Buffer.concat([
    Buffer.from('--abc123\r\nContent-Disposition: form-data; name="'),
    Buffer.from([0xff]),
    Buffer.from('"\r\n\r\npayload\r\n--abc123--\r\n'),
  ]);
  assert.throws(
    () => addon.parse(invalidUtf8Name, "abc123"),
    /name must be valid UTF-8/,
  );
} finally {
  rmSync(tempDir, { force: true, recursive: true });
}
