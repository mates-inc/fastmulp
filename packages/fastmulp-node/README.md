# fastmulp Node.js binding

Native Node.js package for `fastmulp`. The release workflow packages this template with the platform-specific `fastmulp.node` binary and TypeScript declarations.

```js
const { parse } = require("fastmulp-node-linux-x64");

const parts = parse(bodyBuffer, boundary);
const fileBytes = bodyBuffer.subarray(parts[0].bodyStart, parts[0].bodyEnd);
```
