use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use ezlogin_core::models::{LoginOptions, LoginResponse};

#[derive(Parser, Debug)]
#[command(
    name = "ezlogin",
    version,
    about = "EZLogin 校园网自动登录工具",
    long_about = "EZLogin 是面向 Ubuntu 系统的校园网门户自动登录 CLI 工具。\n\
                  支持 OCR 自动识别验证码、本地加密保存凭据，以及灵活的登录选项配置。\n\
                  \n\
                  退出码说明:\n\
                  \x20 0  操作成功\n\
                  \x20 1  程序错误（配置缺失、IO 异常等）\n\
                  \x20 2  登录失败（凭据错误、验证码识别失败等）"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 初始化并保存登录凭据（账号与密码）
    #[command(long_about = "将账号与密码加密保存到本地配置文件。\n\
                            若省略 --password，将在终端交互式输入密码（不回显）。")]
    Init(AuthArgs),

    /// 更新已保存的账号、密码或登录选项
    #[command(long_about = "修改已保存的一项或多项配置。\n\
                            至少需要指定一个选项，未指定的字段保持不变。")]
    Set(SetArgs),

    /// 显示当前保存的配置信息
    ShowConfig,

    /// 清除已保存的账号密码
    Clear,

    /// 执行校园网门户登录（省略子命令时的默认行为）
    #[command(long_about = "连接校园网门户并完成登录，自动 OCR 识别验证码。\n\
                            若未指定账号密码，将优先读取 init/set 保存的凭据。\n\
                            \n\
                            退出码: 0=成功, 1=程序错误, 2=登录失败")]
    Login(LoginArgs),

    /// 输出指定 Shell 的命令补全脚本（供安装使用）
    #[command(
        hide = true,
        long_about = "向标准输出打印指定 Shell 的自动补全脚本。\n\
                      \n\
                      安装示例:\n\
                      \x20 Bash:  ezlogin completions bash >> ~/.bashrc\n\
                      \x20 Zsh:   ezlogin completions zsh  > ~/.zfunc/_ezlogin\n\
                      \x20 Fish:  ezlogin completions fish > ~/.config/fish/completions/ezlogin.fish"
    )]
    Completions {
        /// Shell 类型：bash / zsh / fish / elvish / powershell
        #[arg(value_enum)]
        shell: Shell,
    },

    /// 向标准输出打印 ezlogin(1) man page（roff 格式）
    #[command(
        hide = true,
        long_about = "向标准输出打印 roff 格式的 man page。\n\
                      \n\
                      安装示例:\n\
                      \x20 ezlogin man | gzip > /usr/local/share/man/man1/ezlogin.1.gz\n\
                      \x20 man ./ezlogin.1"
    )]
    Man,
}

#[derive(Args, Debug)]
struct AuthArgs {
    /// 登录账号（学号或用户名）
    #[arg(long)]
    account: String,
    /// 登录密码（省略则交互式输入，不回显）
    #[arg(long)]
    password: Option<String>,
}

#[derive(Args, Debug)]
struct SetArgs {
    /// 更新账号
    #[arg(long)]
    account: Option<String>,
    /// 更新密码
    #[arg(long)]
    password: Option<String>,
    /// 最大登录重试次数（最小值 1）
    #[arg(long)]
    retries: Option<u32>,
    /// 请求超时秒数（最小值 1）
    #[arg(long)]
    timeout: Option<u64>,
    /// 登录前是否检测网络连通性
    #[arg(long)]
    probe_required: Option<bool>,
}

#[derive(Args, Debug)]
struct LoginArgs {
    /// 登录账号（覆盖已保存的账号）
    #[arg(long)]
    account: Option<String>,
    /// 登录密码（覆盖已保存的密码）
    #[arg(long)]
    password: Option<String>,
    /// 最大登录重试次数（覆盖已保存配置，最小值 1）
    #[arg(long)]
    retries: Option<u32>,
    /// 请求超时秒数（覆盖已保存配置，最小值 1）
    #[arg(long)]
    timeout: Option<u64>,
    /// 登录前是否检测网络连通性（覆盖已保存配置）
    #[arg(long)]
    probe_required: Option<bool>,
    /// 优先使用本地已保存的凭据
    #[arg(long, default_value_t = true)]
    use_saved: bool,
    /// 登录成功后将本次使用的凭据保存到本地
    #[arg(long, default_value_t = false)]
    save_after_login: bool,
}

impl Default for LoginArgs {
    fn default() -> Self {
        Self {
            account: None,
            password: None,
            retries: None,
            timeout: None,
            probe_required: None,
            use_saved: true,
            save_after_login: false,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command.unwrap_or_else(|| Commands::Login(LoginArgs::default())) {
        Commands::Init(args) => init_command(args),
        Commands::Set(args) => set_command(args),
        Commands::ShowConfig => show_config_command(),
        Commands::Clear => clear_command(),
        Commands::Login(args) => login_command(args).await,
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "ezlogin", &mut io::stdout());
            Ok(ExitCode::SUCCESS)
        }
        Commands::Man => man_command(),
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn init_command(args: AuthArgs) -> Result<ExitCode, String> {
    let password = match args.password {
        Some(p) => p,
        None => prompt_password("请输入密码: ")?,
    };
    ezlogin_core::save_credentials(&args.account, &password)?;
    println!("初始化成功：已保存账号与密码");
    Ok(ExitCode::SUCCESS)
}

fn prompt_password(prompt: &str) -> Result<String, String> {
    if !io::stdin().is_terminal() {
        return Err("无 TTY，无法交互输入密码，请通过 --password 传入".to_string());
    }
    rpassword::prompt_password(prompt).map_err(|e| format!("读取密码失败: {e}"))
}

fn set_command(args: SetArgs) -> Result<ExitCode, String> {
    let account_updated = args.account.is_some();
    let password_updated = args.password.is_some();
    let retries_updated = args.retries.is_some();
    let timeout_updated = args.timeout.is_some();
    let probe_updated = args.probe_required.is_some();

    if !account_updated && !password_updated && !retries_updated && !timeout_updated && !probe_updated {
        return Err(
            "未提供可更新项，请至少指定 --account/--password/--retries/--timeout/--probe-required"
                .to_string(),
        );
    }

    if account_updated || password_updated {
        let current = ezlogin_core::load_credentials()?;
        let mut account = current.as_ref().map(|v| v.account.clone()).unwrap_or_default();
        let mut password = current.as_ref().map(|v| v.password.clone()).unwrap_or_default();

        if let Some(v) = args.account {
            account = v;
        }
        if let Some(v) = args.password {
            password = v;
        }

        if account.is_empty() || password.is_empty() {
            return Err("更新账号或密码时需保证账号和密码都存在，可先使用 init 初始化".to_string());
        }

        ezlogin_core::save_credentials(&account, &password)?;
    }

    let mut options = ezlogin_core::load_login_options()?.unwrap_or_default();

    if let Some(retries) = args.retries {
        options.max_login_retries = retries.max(1);
    }
    if let Some(timeout) = args.timeout {
        options.timeout_secs = timeout.max(1);
    }
    if let Some(probe_required) = args.probe_required {
        options.probe_required = probe_required;
    }

    if retries_updated || timeout_updated || probe_updated {
        ezlogin_core::save_login_options(&options)?;
    }

    println!(
        "配置已更新: account={}, password={}, retries={}, timeout={}, probe_required={}",
        if account_updated { "updated" } else { "unchanged" },
        if password_updated { "updated" } else { "unchanged" },
        options.max_login_retries,
        options.timeout_secs,
        options.probe_required
    );
    Ok(ExitCode::SUCCESS)
}

fn show_config_command() -> Result<ExitCode, String> {
    let creds = ezlogin_core::load_credentials()?;
    let options = ezlogin_core::load_login_options()?.unwrap_or_default();

    let account = creds.as_ref().map(|v| v.account.as_str()).unwrap_or("<未设置>");
    let password_state = if creds.is_some() { "已设置" } else { "未设置" };

    println!("账号: {account}");
    println!("密码: {password_state}");
    println!("max_login_retries: {}", options.max_login_retries);
    println!("timeout_secs: {}", options.timeout_secs);
    println!("probe_required: {}", options.probe_required);
    Ok(ExitCode::SUCCESS)
}

fn clear_command() -> Result<ExitCode, String> {
    ezlogin_core::clear_credentials()?;
    println!("已清除账号密码");
    Ok(ExitCode::SUCCESS)
}

async fn login_command(args: LoginArgs) -> Result<ExitCode, String> {
    let (account, password) = resolve_auth(&args)?;
    let options = resolve_options(&args)?;

    // Warm up OCR engine in parallel with network init.
    tokio::task::spawn_blocking(|| { let _ = ezlogin_core::init_ocr_engine(); });

    let response = ezlogin_core::login_with_ocr(account.clone(), password.clone(), Some(options)).await?;

    print_login_response(&response)?;

    if response.success && args.save_after_login {
        ezlogin_core::save_credentials(&account, &password)?;
    }

    if response.success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(2))
    }
}

fn resolve_auth(args: &LoginArgs) -> Result<(String, String), String> {
    if let (Some(account), Some(password)) = (args.account.as_ref(), args.password.as_ref()) {
        return Ok((account.to_string(), password.to_string()));
    }

    if args.use_saved {
        if let Some(saved) = ezlogin_core::load_credentials()? {
            let account = args.account.clone().unwrap_or(saved.account);
            let password = args.password.clone().unwrap_or(saved.password);
            if !account.is_empty() && !password.is_empty() {
                return Ok((account, password));
            }
        }
    }

    if let Some(account) = args.account.clone() {
        let password = prompt_password(&format!("请输入 {account} 的密码: "))?;
        if !password.is_empty() {
            return Ok((account, password));
        }
    }

    Err("缺少登录账号或密码，请使用 --account/--password，或先执行 init 保存凭据".to_string())
}

fn resolve_options(args: &LoginArgs) -> Result<LoginOptions, String> {
    let mut options = ezlogin_core::load_login_options()?.unwrap_or_default();

    if let Some(retries) = args.retries {
        options.max_login_retries = retries.max(1);
    }
    if let Some(timeout) = args.timeout {
        options.timeout_secs = timeout.max(1);
    }
    if let Some(probe_required) = args.probe_required {
        options.probe_required = probe_required;
    }

    Ok(options)
}

fn print_login_response(response: &LoginResponse) -> Result<(), String> {
    let mut out = io::BufWriter::new(io::stdout());
    let json = serde_json::to_string_pretty(response).map_err(|e| e.to_string())?;
    out.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    out.write_all(b"\n").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

fn man_command() -> Result<ExitCode, String> {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf).map_err(|e| e.to_string())?;
    io::stdout().write_all(&buf).map_err(|e| e.to_string())?;
    Ok(ExitCode::SUCCESS)
}
