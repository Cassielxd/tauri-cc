# tauri-cc

#### Introduction

A fusion of the Tauri framework for desktop development in Rust and Deno. If you have high performance requirements, you can develop using Rust or Deno for Tauri, making it friendly for frontend developers. It is easy to get started, especially for those accustomed to developing desktop applications with Electron and JavaScript. Tauri-cc combines Rust and JavaScript, offering both high performance from Rust and the flexibility of JavaScript.

#### Tauri

Tauri is a framework for building small, fast binaries for all major desktop platforms. Developers can integrate any frontend framework that compiles to HTML, JS, and CSS to build their user interfaces. The backend of the application is a Rust binary with an API that the frontend can interact with.

#### Deno

Deno is a runtime for JavaScript/TypeScript that uses the V8 engine and is written in Rust. It comes with many modern features such as asynchronous operations, modularity, and TypeScript support. Deno is also a secure runtime, defaulting to not allowing access to files, the network, environment variables, etc., and is fully compatible with Node.js.

#### Software Architecture

- Frontend Framework: Vue3 + Vite + TailwindCSS
- Backend Framework: Deno + Tauri

#### Development Environment

1. Latest version of Rust
2. Development tool: RustRover
3. Latest version of Node.js (18.0.0)
4. Latest version of tauri-cli (2.0.0-rc.15)
5. Configure RUSTY_V8_MIRROR environment variable
6. Deno version synchronized with official releases
   - Download the corresponding V8 version from <https://github.com/denoland/rusty_v8/releases>

### Usage Instructions

1. Enter the `plugin/tauri-plugin-deno` directory and run: `npm run build`
2. Build Tauri: Run `cargo build` in the root directory
3. Enter the `tauri-src` directory and run: `cargo run`
4. Start the frontend: Enter the `frontend` directory and run `npm install && npm run dev`
5. Start the backend: Enter the `tauri-src` directory and run `cargo run`

### Package Structure Explanation

#### Contributing

Contact: String <348040933@qq.com>
Discussion Group: 435604279