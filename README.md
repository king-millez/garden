# Garden

Messing about with WebGPU because I am too stupid to get WSL -> Windows 10 X server forwarding working and I am not going to install Windows 11.

## Dependencies

You will need to install the following:

- [`pnpm`](https://pnpm.io/installation)
- [Node.js](https://nodejs.org/en/download)
- [Rust](https://rust-lang.org/tools/install/)
- [`wasm-pack`](https://github.com/wasm-bindgen/wasm-pack) (note the docs are cooked at the time of writing - see the top GitHub issue).

## Setup

I'm pretty sure everything will work if you do:

```bash
$ pnpm i --frozen-lockfile
$ pnpm run dev:recomp
```
