# fastmulp wasm binding

Browser package for `fastmulp`. The release workflow builds the Rust wasm target, runs `wasm-bindgen --target bundler`, and packs the generated JavaScript, wasm, and TypeScript declarations.

```ts
import { parse } from "fastmulp-wasm";

const parts = parse(formBytes, boundary);
const fieldBytes = formBytes.subarray(parts[0].body_start, parts[0].body_end);
```
