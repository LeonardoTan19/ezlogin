import { cn } from "@/lib/utils"
import { Alert } from "@/components/ui/alert"
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
  message?: string
  showDesktopControls: boolean
  onAccountChange: (value: string) => void
  onPasswordChange: (value: string) => void
  onRememberMeChange: (value: boolean) => void
  onOpenSettings: () => void
  onFormSubmit: (event: React.FormEvent<HTMLFormElement>) => void
}

export function LoginForm({
  account,
  password,
  rememberMe,
  isLoading,
  error,
  message,
  showDesktopControls,
  onAccountChange,
  onPasswordChange,
  onRememberMeChange,
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

          {error ? <Alert variant="destructive" className="mt-4">{error}</Alert> : null}
          {message ? <Alert variant="success" className="mt-4">{message}</Alert> : null}
      </AuthCard>
    </div>
  )
}
