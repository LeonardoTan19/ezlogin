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

## 发布与 Android 签名

仓库通过 `.github/workflows/build.yml` 在打 `v*` tag 时自动构建桌面端、CLI 与 Android APK，并发布到 GitHub Release。Android APK 由 CI 在构建时用 keystore 自动签名，**密钥不会进入仓库**。

### 必需的 GitHub Secrets

在仓库 `Settings → Secrets and variables → Actions → New repository secret` 中添加：

| Secret | 说明 |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | release keystore 的 base64：`base64 -w 0 release.jks` |
| `ANDROID_KEY_ALIAS` | keystore 中的 key alias |
| `ANDROID_KEY_PASSWORD` | 该 alias 的密码 |
| `ANDROID_KEYSTORE_PASSWORD` | keystore 整体密码 |

生成 base64 示例：

```bash
base64 -w 0 release.jks > release.jks.b64
# 复制 release.jks.b64 内容粘贴到 ANDROID_KEYSTORE_BASE64
```

### 本地 Android 构建

本地需要两个文件，都已被 `.gitignore` 屏蔽：

1. keystore 文件（推荐放在 `src-tauri/gen/android/keystore/ezlogin-release.jks`）
2. `src-tauri/gen/android/keystore.properties`：

```properties
keyAlias=<你的 alias>
keyPassword=<alias 密码>
storePassword=<keystore 密码>
storeFile=keystore/ezlogin-release.jks
```

之后即可：

```bash
pnpm tauri android build --apk --target aarch64
```