# Third-party notices

Codync's own license is in `LICENSE` and `NOTICE`. Where a file adapts or ports code from another project, the file's
header names it, and the project's notice is kept here.

## Mercury Agent

Used in: `host/src/agent/blocklist.rs` (the list of forbidden shell commands, ported from
`src/capabilities/shell/blocklist.ts`, with changes: literal-star semantics for the root patterns, extra variants, and
chain and wrapper handling).

Source: https://github.com/cosmicstack-labs/mercury-agent

MIT License

Copyright (c) 2026 Cosmic Stack

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
