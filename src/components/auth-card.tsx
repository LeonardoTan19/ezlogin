import { ReactNode, MouseEvent } from "react"
import { ArrowLeft, Settings, Minus, X } from "lucide-react"
import { useWindowControls } from "@/lib/use-window-controls"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

type AuthCardProps = {
  title: string
  description: string
  children: ReactNode
  showDesktopControls: boolean
  mode: "login" | "settings"
  onToggleView: () => void
}

export function AuthCard({
  title,
  description,
  children,
  showDesktopControls,
  mode,
  onToggleView,
}: AuthCardProps) {
  const isLoginMode = mode === "login"
  const { startDragging, minimizeWindow, closeWindow, resizeWindow } = useWindowControls()

  function handleTitleMouseDown(event: MouseEvent<HTMLDivElement>) {
    if (!showDesktopControls) {
      return
    }

    // Left button only; skip interactive elements inside title area.
    if (event.button !== 0) {
      return
    }

    const interactive = (event.target as HTMLElement).closest("button, input, textarea, select, a, [role='button']")
    if (interactive) {
      return
    }

    void startDragging()
  }

  return (
    <div
      className={cn(
        "relative flex flex-col gap-6 bg-background py-6 text-sm text-foreground"
      )}
    >
      <div className="grid auto-rows-min items-start gap-2 px-6">
        <div className="flex items-start justify-between gap-3">
          <div
            data-tauri-drag-region={showDesktopControls ? true : undefined}
            className={showDesktopControls ? "select-none cursor-move" : undefined}
            onMouseDown={handleTitleMouseDown}
          >
            <div className="font-heading text-base font-medium">{title}</div>
            <div className="text-sm text-muted-foreground">{description}</div>
          </div>
          <div className="flex items-center gap-1">
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              onClick={onToggleView}
              aria-label={isLoginMode ? "打开配置" : "返回登录"}
            >
              {isLoginMode ? <Settings className="size-4" /> : <ArrowLeft className="size-4" />}
            </Button>
            {showDesktopControls ? (
              <>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => void minimizeWindow()}
                  aria-label="最小化"
                >
                  <Minus className="size-4" />
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  size="icon-sm"
                  onClick={() => void closeWindow()}
                  aria-label="关闭"
                >
                  <X className="size-4" />
                </Button>
              </>
            ) : null}
          </div>
        </div>
      </div>
      <div className="px-6">{children}</div>

      {showDesktopControls ? (
        <>
          <div
            className="absolute inset-y-2 right-0 w-2 cursor-e-resize"
            onMouseDown={() => void resizeWindow("East")}
          />
          <div
            className="absolute inset-y-2 left-0 w-2 cursor-w-resize"
            onMouseDown={() => void resizeWindow("West")}
          />
          <div
            className="absolute inset-x-2 top-0 h-2 cursor-n-resize"
            onMouseDown={() => void resizeWindow("North")}
          />
          <div
            className="absolute inset-x-2 bottom-0 h-2 cursor-s-resize"
            onMouseDown={() => void resizeWindow("South")}
          />
          <div
            className="absolute top-0 right-0 h-3 w-3 cursor-ne-resize"
            onMouseDown={() => void resizeWindow("NorthEast")}
          />
          <div
            className="absolute top-0 left-0 h-3 w-3 cursor-nw-resize"
            onMouseDown={() => void resizeWindow("NorthWest")}
          />
          <div
            className="absolute bottom-0 right-0 h-3 w-3 cursor-se-resize"
            onMouseDown={() => void resizeWindow("SouthEast")}
          />
          <div
            className="absolute bottom-0 left-0 h-3 w-3 cursor-sw-resize"
            onMouseDown={() => void resizeWindow("SouthWest")}
          />
        </>
      ) : null}
    </div>
  )
}
