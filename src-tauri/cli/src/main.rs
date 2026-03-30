use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use ezlogin_core::models::{LoginOptions, LoginResponse};

#[derive(Parser, Debug)]
#[command(name = "ezlogin", version, about = "EZLogin Ubuntu CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init(AuthArgs),
    Set(SetArgs),
    ShowConfig,
    Clear,
    Login(LoginArgs),
}

#[derive(Args, Debug)]
struct AuthArgs {
    #[arg(long)]
    account: String,
    #[arg(long)]
    password: String,
}

#[derive(Args, Debug)]
struct SetArgs {
    #[arg(long)]
    account: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    retries: Option<u32>,
    #[arg(long)]
    timeout: Option<u64>,
    #[arg(long)]
    probe_required: Option<bool>,
}

#[derive(Args, Debug)]
struct LoginArgs {
    #[arg(long)]
    account: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    retries: Option<u32>,
    #[arg(long)]
    timeout: Option<u64>,
    #[arg(long)]
    probe_required: Option<bool>,
    #[arg(long, default_value_t = true)]
    use_saved: bool,
    #[arg(long, default_value_t = false)]
    save_after_login: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => init_command(args),
        Commands::Set(args) => set_command(args),
        Commands::ShowConfig => show_config_command(),
        Commands::Clear => clear_command(),
        Commands::Login(args) => login_command(args).await,
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
    ezlogin_core::save_credentials(&args.account, &args.password)?;
    println!("初始化成功：已保存账号与密码");
    Ok(ExitCode::SUCCESS)
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
        "配置已更新: account={}, password_set={}, retries={}, timeout={}, probe_required={}",
        if account_updated { "updated" } else { "unchanged" },
        if password_updated { "yes" } else { "unchanged" },
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
