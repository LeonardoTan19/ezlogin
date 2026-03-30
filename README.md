# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Ubuntu CLI（非 GUI 打包）

项目已提供独立 CLI 二进制，用于 Ubuntu 场景直接命令行登录并返回结构化结果。

### 构建 CLI

在项目根目录执行：

```bash
cargo build --manifest-path src-tauri/cli/Cargo.toml --release
```

产物位置：

- `src-tauri/cli/target/release/ezlogin-cli`

### CLI 子命令

- 初始化账号密码：

```bash
ezlogin init --account <账号> --password <密码>
```

- 更新配置（统一 `set --...`）：

```bash
ezlogin set --account <新账号>
ezlogin set --password <新密码>
ezlogin set --retries 5 --timeout 10 --probe-required false
ezlogin set --account <账号> --password <密码> --retries 3
```

- 查看当前配置：

```bash
ezlogin show-config
```

- 直接登录并返回 JSON 结果：

```bash
ezlogin login
```

也可临时指定参数登录：

```bash
ezlogin login --account <账号> --password <密码> --retries 3 --timeout 10 --probe-required false
```

### 前端配置界面

桌面端登录卡片已新增“配置管理”区，可直接修改并保存：

- 账号、密码
- 最大重试次数（`maxLoginRetries`）
- 超时秒数（`timeoutSecs`）
- 是否要求连通性检测通过（`probeRequired`）

### Ubuntu 打包

项目根目录提供打包脚本：

```bash
./scripts/package-cli-ubuntu.sh 0.1.0
```

生成目录：`dist-cli/`

- `ezlogin-cli_<version>_linux_<arch>.tar.gz`
- 若系统存在 `dpkg-deb`，额外生成 `ezlogin-cli_<version>_<arch>.deb`

脚本会将可执行文件安装名固定为 `ezlogin`（即安装后可直接执行 `ezlogin`）。
