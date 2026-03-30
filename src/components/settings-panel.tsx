import { cn } from "@/lib/utils"
import { Alert } from "@/components/ui/alert"
import { AuthCard } from "@/components/auth-card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

type SettingsPanelProps = React.ComponentProps<"div"> & {
  retries: number
  timeoutSecs: number
  probeRequired: boolean
  isSaving: boolean
  error?: string
  message?: string
  showDesktopControls: boolean
  onRetriesChange: (value: number) => void
  onTimeoutSecsChange: (value: number) => void
  onProbeRequiredChange: (value: boolean) => void
  onSaveConfig: () => void
  onClearSaved: () => void
  onBackToLogin: () => void
}

export function SettingsPanel({
  retries,
  timeoutSecs,
  probeRequired,
  isSaving,
  error,
  message,
  showDesktopControls,
  onRetriesChange,
  onTimeoutSecsChange,
  onProbeRequiredChange,
  onSaveConfig,
  onClearSaved,
  onBackToLogin,
  className,
  ...props
}: SettingsPanelProps) {
  return (
    <div className={cn("flex flex-col", className)} {...props}>
      <AuthCard
        title="配置中心"
        description="管理登录策略和凭据"
        mode="settings"
        showDesktopControls={showDesktopControls}
        onToggleView={onBackToLogin}
      >
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="settings-retries">重试次数</Label>
              <Input
                id="settings-retries"
                type="number"
                min={1}
                value={retries}
                onChange={(event) => onRetriesChange(Number(event.currentTarget.value) || 1)}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="settings-timeout">超时(秒)</Label>
              <Input
                id="settings-timeout"
                type="number"
                min={1}
                value={timeoutSecs}
                onChange={(event) => onTimeoutSecsChange(Number(event.currentTarget.value) || 1)}
              />
            </div>
          </div>

          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            <input
              type="checkbox"
              checked={probeRequired}
              onChange={(event) => onProbeRequiredChange(event.currentTarget.checked)}
              className="size-4 rounded border border-input"
            />
            登录后要求连通性检测通过
          </label>

          <div className="grid grid-cols-2 gap-2">
            <Button type="button" variant="secondary" onClick={onSaveConfig} disabled={isSaving}>
              {isSaving ? "保存中..." : "保存"}
            </Button>
            <Button type="button" variant="outline" onClick={onClearSaved} disabled={isSaving}>
              清除凭据
            </Button>
          </div>

          {error ? <Alert variant="destructive">{error}</Alert> : null}
          {message ? <Alert variant="success">{message}</Alert> : null}
        </div>
      </AuthCard>
    </div>
  )
}
