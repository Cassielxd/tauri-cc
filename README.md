# tauri-cc

#### 介绍

rust 桌面端开发框架 Tauri 和Deno的融合，如果对性能要求比较高可以使用rust 开发 也可以使用 deno开发 tauri ，对前端开发友好
上手快,对于习惯使用Electron  js 开发桌面端的小伙伴，可以尝试使用 tauri-cc，融合了rust 和 js 两种方式；既有 rust 的 高性能也保留了
js 的灵活

#### tauri

Tauri 是一个为所有主流桌面平台构建小型、快速二进制文件的框架。开发人员可以集成任何编译成 HTML、 JS 和 CSS
的前端框架来构建他们的用户界面。应用程序的后端是一个 Rust 二进制文件，具有前端可以与之交互的 API。

#### deno

Deno 是一个 JavaScript / TypeScript 的运行时，它使用 V8 引擎和 Rust 编写。它内置了很多现代的特性，如异步
操作、模块化、TypeScript 等等。Deno 也是一个安全的运行时，它默认不允许访问文件、网络、环境变量等等,完全兼容
nodejs

#### 软件架构

软件架构说明
前端框架 vue3 + vite + tailwindcss
后端框架 deno + tauri

#### 开发环境

1：rust 最新版本
2：开发工具 RustRover
3: nodejs 最新版本18.0.0
4：tauri-cli 最新版本 2.0.0-rc.15
5：配置RUSTY_V8_MIRROR 环境变量
6: deno 版本于官方同步
<https://github.com/denoland/rusty_v8/releases> 下载v8 对应版本

### 使用说明

1. 进入plugin/tauri-plugin-deno目录运行： 执行 npm run build
2. 构建tauri： 根目录下执行 cargo build
3. 进入tauri-src目录运行： 执行 cargo run
4. 启动前端： 进入frontend目录，执行 npm install && npm run dev
5. 启动后端： 进入tauri-src目录，执行 cargo run

### 包结构说明

```
frontend   //前端目录
tauri-src  //tauri 后端
tauri-src/deno_demo  //deno代码目录 默认启动这个目录下的main.ts
```

#### 参与贡献

String <348040933@qq.com>
交流群:435604279



