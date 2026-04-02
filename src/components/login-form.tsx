import { cn } from "@/lib/utils"
import { Alert, AlertAction, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { AuthCard } from "@/components/auth-card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

type LoginFormProps = React.ComponentProps<"div"> & {
  account: string
  password: string
  rememberMe: boolean
  isLoading: boolean
  error?: string
  errorTitle?: string
  message?: string
  showDesktopControls: boolean
  showNetworkSettingsAction?: boolean
  isOpeningNetworkSettings?: boolean
  onAccountChange: (value: string) => void
  onPasswordChange: (value: string) => void
  onRememberMeChange: (value: boolean) => void
  onOpenNetworkSettings?: () => void
  onOpenSettings: () => void
  onFormSubmit: (event: React.FormEvent<HTMLFormElement>) => void
}

export function LoginForm({
  account,
  password,
  rememberMe,
  isLoading,
  error,
  errorTitle,
  message,
  showDesktopControls,
  showNetworkSettingsAction,
  isOpeningNetworkSettings,
  onAccountChange,
  onPasswordChange,
  onRememberMeChange,
  onOpenNetworkSettings,
  onOpenSettings,
  onFormSubmit,
  className,
  ...props
}: LoginFormProps) {
  return (
    <div className={cn("flex flex-col", className)} {...props}>
      <AuthCard
        title="EZLogin"
        description="校园网自动登录"
        mode="login"
        showDesktopControls={showDesktopControls}
        onToggleView={onOpenSettings}
      >
          <form onSubmit={onFormSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="account">账号</Label>
              <Input
                id="account"
                type="text"
                value={account}
                onChange={(event) => onAccountChange(event.currentTarget.value)}
                placeholder="请输入账号"
                autoComplete="username"
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">密码</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(event) => onPasswordChange(event.currentTarget.value)}
                placeholder="请输入密码"
                autoComplete="current-password"
                required
              />
            </div>

            <label className="flex items-center gap-2 text-sm text-muted-foreground">
              <input
                type="checkbox"
                checked={rememberMe}
                onChange={(event) => onRememberMeChange(event.currentTarget.checked)}
                className="size-4 rounded border border-input"
              />
              记住账号密码
            </label>

            <Button type="submit" disabled={isLoading} className="w-full">
              {isLoading ? "登录中..." : "登录"}
            </Button>
          </form>

          {error ? (
            <Alert variant="destructive" className="mt-4">
              <AlertTitle>{errorTitle ?? "登录失败"}</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
              {showNetworkSettingsAction ? (
                <AlertAction>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    disabled={Boolean(isOpeningNetworkSettings)}
                    onClick={onOpenNetworkSettings}
                  >
                    {isOpeningNetworkSettings ? "打开中..." : "打开网络设置"}
                  </Button>
                </AlertAction>
              ) : null}
            </Alert>
          ) : null}
          {message ? <Alert variant="success" className="mt-4">{message}</Alert> : null}
      </AuthCard>
    </div>
  )
}
