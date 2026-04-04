# ezlogin

`ezlogin` 是一个校园网/门户自动登录工具，提供桌面端（Tauri + React）和命令行（CLI）两种使用方式。

## 核心功能

- 保存账号密码与登录参数
- 一键登录，支持失败重试与超时控制
- 可选登录前连通性检测（probe）
- CLI 输出结构化 JSON，便于脚本集成

## 快速使用

### 桌面端

在登录界面的“配置管理”中可设置：

- 账号、密码
- 最大重试次数 `maxLoginRetries`
- 超时秒数 `timeoutSecs`
- 是否要求连通性检测通过 `probeRequired`

保存后可直接执行登录。

### CLI

构建：

```bash
cargo build --manifest-path src-tauri/cli/Cargo.toml --release
```

常用命令：

```bash
# 初始化
ezlogin init --account <账号> --password <密码>

# 更新配置
ezlogin set --account <账号> --password <密码> --retries 3 --timeout 10 --probe-required false

# 查看配置
ezlogin show-config

# 登录（返回 JSON）
ezlogin login
```

也可临时覆盖参数：

```bash
ezlogin login --account <账号> --password <密码> --retries 3 --timeout 10 --probe-required false
```

## Ubuntu 打包（CLI）

```bash
./scripts/package-cli-ubuntu.sh 0.1.0
```

产物位于 `dist-cli/`：

- `ezlogin-cli_<version>_linux_<arch>.tar.gz`
- `ezlogin-cli_<version>_<arch>.deb`（系统安装了 `dpkg-deb` 时生成）

安装后命令名统一为 `ezlogin`。
