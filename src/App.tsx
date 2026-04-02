import { FormEvent, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { LoginForm } from "./components/login-form";
import { SettingsPanel } from "./components/settings-panel";

type LoginResponse = {
  success: boolean;
  message: string;
  captchaText: string;
  confidence: number;
  attempt: number;
  probePassed: boolean;
  failureKind?:
    | "INVALID_CAPTCHA"
    | "INVALID_CREDENTIALS"
    | "INVALID_CREDENTIALS_OR_LOCKED"
    | "ACCOUNT_LOCKED"
    | "NETWORK_UNAVAILABLE"
    | "PORTAL_PAGE_UNREACHABLE"
    | "CONNECTIVITY_PROBE_FAILED"
    | "MAX_RETRIES_EXCEEDED"
    | "UNKNOWN";
};

type FailureKind = NonNullable<LoginResponse["failureKind"]>;

type SavedCredentials = {
  account: string;
  password: string;
};

type LoginOptions = {
  maxLoginRetries: number;
  probeRequired: boolean;
  timeoutSecs: number;
};

function App() {
  const [isMobilePlatform, setIsMobilePlatform] = useState(false);
  const [view, setView] = useState<"login" | "settings">("login");
  const [account, setAccount] = useState("");
  const [password, setPassword] = useState("");
  const [rememberMe, setRememberMe] = useState(true);
  const [retries, setRetries] = useState(5);
  const [timeoutSecs, setTimeoutSecs] = useState(10);
  const [probeRequired, setProbeRequired] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isSavingConfig, setIsSavingConfig] = useState(false);
  const [isOpeningNetworkSettings, setIsOpeningNetworkSettings] = useState(false);
  const [error, setError] = useState("");
  const [errorTitle, setErrorTitle] = useState("");
  const [failureKind, setFailureKind] = useState<FailureKind | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    const loadSaved = async () => {
      try {
        const [saved, options, mobile] = await Promise.all([
          invoke<SavedCredentials | null>("load_saved_credentials"),
          invoke<LoginOptions | null>("load_login_options"),
          invoke<boolean>("is_mobile_platform"),
        ]);

        setIsMobilePlatform(Boolean(mobile));

        if (saved) {
          setAccount(saved.account ?? "");
          setPassword(saved.password ?? "");
        }

        if (options) {
          setRetries(Math.max(1, options.maxLoginRetries ?? 5));
          setTimeoutSecs(Math.max(1, options.timeoutSecs ?? 10));
          setProbeRequired(Boolean(options.probeRequired));
        }
      } catch {
        // Ignore storage errors at startup and allow manual login.
      }
    };

    loadSaved();
  }, []);

  const showDesktopControls = !isMobilePlatform;
  const isLinuxDesktop =
    !isMobilePlatform &&
    typeof navigator !== "undefined" &&
    navigator.userAgent.toLowerCase().includes("linux");

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setErrorTitle("");
    setFailureKind(null);
    setMessage("");

    if (!account.trim() || !password.trim()) {
      setError("请输入账号和密码");
      return;
    }

    setIsLoading(true);
    try {
      const result = await invoke<LoginResponse>("portal_login_with_ocr", {
        account: account.trim(),
        password,
        options: {
          maxLoginRetries: retries,
          probeRequired,
          timeoutSecs,
        },
      });

      if (!result.success) {
        const kind = result.failureKind ?? null;
        setFailureKind(kind);
        setError(buildErrorMessage(result.message || "登录失败", kind, isLinuxDesktop));
        setErrorTitle(getFailureTitle(kind));
        return;
      }

      if (rememberMe) {
        await invoke("save_credentials", { account: account.trim(), password });
      } else {
        await invoke("clear_saved_credentials");
      }

      setMessage(
        `登录成功，尝试次数：${result.attempt}`,
      );
      setFailureKind(null);
      setErrorTitle("");
    } catch (e) {
      const text = typeof e === "string" ? e : "登录请求失败";
      setError(text);
      setErrorTitle("登录失败");
      setFailureKind(null);
    } finally {
      setIsLoading(false);
    }
  }

  const canOpenNetworkSettings =
    (failureKind === "NETWORK_UNAVAILABLE" || failureKind === "PORTAL_PAGE_UNREACHABLE") &&
    !isLinuxDesktop;

  async function handleOpenNetworkSettings() {
    setIsOpeningNetworkSettings(true);
    try {
      await invoke("open_network_settings");
    } catch (e) {
      const text = typeof e === "string" ? e : "打开网络设置失败";
      setError(text);
      setErrorTitle("无法打开网络设置");
    } finally {
      setIsOpeningNetworkSettings(false);
    }
  }

  async function handleSaveConfig() {
    setError("");
    setMessage("");
    setIsSavingConfig(true);

    try {
      if (!account.trim() || !password.trim()) {
        setError("保存配置前请填写账号和密码");
        return;
      }

      await invoke("save_credentials", {
        account: account.trim(),
        password,
      });

      await invoke("save_login_options", {
        options: {
          maxLoginRetries: Math.max(1, retries),
          timeoutSecs: Math.max(1, timeoutSecs),
          probeRequired,
        },
      });

      setMessage("配置已保存");
    } catch (e) {
      const text = typeof e === "string" ? e : "保存配置失败";
      setError(text);
    } finally {
      setIsSavingConfig(false);
    }
  }

  async function handleClearSaved() {
    setError("");
    setMessage("");

    try {
      await invoke("clear_saved_credentials");
      setMessage("已清除保存的账号密码");
    } catch (e) {
      const text = typeof e === "string" ? e : "清除失败";
      setError(text);
    }
  }

  return (
    <main className="h-full w-full bg-transparent p-2">
      {view === "login" ? (
        <LoginForm
          className="w-full"
          account={account}
          password={password}
          rememberMe={rememberMe}
          isLoading={isLoading}
          error={error}
          errorTitle={errorTitle}
          message={message}
          showDesktopControls={showDesktopControls}
          showNetworkSettingsAction={canOpenNetworkSettings}
          isOpeningNetworkSettings={isOpeningNetworkSettings}
          onAccountChange={setAccount}
          onPasswordChange={setPassword}
          onRememberMeChange={setRememberMe}
          onOpenNetworkSettings={handleOpenNetworkSettings}
          onOpenSettings={() => setView("settings")}
          onFormSubmit={handleSubmit}
        />
      ) : (
        <SettingsPanel
          className="w-full"
          retries={retries}
          timeoutSecs={timeoutSecs}
          probeRequired={probeRequired}
          isSaving={isSavingConfig}
          error={error}
          message={message}
          showDesktopControls={showDesktopControls}
          onRetriesChange={(value) => setRetries(Math.max(1, value))}
          onTimeoutSecsChange={(value) => setTimeoutSecs(Math.max(1, value))}
          onProbeRequiredChange={setProbeRequired}
          onSaveConfig={handleSaveConfig}
          onClearSaved={handleClearSaved}
          onBackToLogin={() => setView("login")}
        />
      )}
      </main>
  );
}

function getFailureTitle(failureKind: FailureKind | null): string {
  switch (failureKind) {
    case "NETWORK_UNAVAILABLE":
      return "当前未联网";
    case "PORTAL_PAGE_UNREACHABLE":
      return "认证网页不可达";
    case "CONNECTIVITY_PROBE_FAILED":
      return "连通性检测失败";
    case "ACCOUNT_LOCKED":
      return "账号已锁定";
    case "INVALID_CAPTCHA":
      return "验证码错误";
    case "INVALID_CREDENTIALS":
    case "INVALID_CREDENTIALS_OR_LOCKED":
      return "账号或密码错误";
    default:
      return "登录失败";
  }
}

function buildErrorMessage(
  message: string,
  failureKind: FailureKind | null,
  isLinuxDesktop: boolean,
): string {
  const isNetworkFailure =
    failureKind === "NETWORK_UNAVAILABLE" || failureKind === "PORTAL_PAGE_UNREACHABLE";

  if (!isLinuxDesktop || !isNetworkFailure) {
    return message;
  }

  return `${message}。可在终端执行：ping -c 1 192.168.200.127；curl -I --max-time 5 http://www.msftconnecttest.com/redirect`;
}

export default App;
