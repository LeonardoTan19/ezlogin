import { ReactNode, MouseEvent } from "react"
import { ArrowLeft, Settings, Minus, X } from "lucide-react"
import { useWindowControls, WindowResizeDirection } from "@/lib/use-window-controls"
import { Button } from "@/components/ui/button"

type AuthCardProps = {
  title: string
  description: string
  children: ReactNode
  showDesktopControls: boolean
  mode: "login" | "settings"
  onToggleView: () => void
}

const RESIZE_HANDLES: { dir: WindowResizeDirection; className: string }[] = [
  { dir: "East", className: "absolute inset-y-2 right-0 w-2 cursor-e-resize" },
  { dir: "West", className: "absolute inset-y-2 left-0 w-2 cursor-w-resize" },
  { dir: "North", className: "absolute inset-x-2 top-0 h-2 cursor-n-resize" },
  { dir: "South", className: "absolute inset-x-2 bottom-0 h-2 cursor-s-resize" },
  { dir: "NorthEast", className: "absolute top-0 right-0 h-3 w-3 cursor-ne-resize" },
  { dir: "NorthWest", className: "absolute top-0 left-0 h-3 w-3 cursor-nw-resize" },
  { dir: "SouthEast", className: "absolute bottom-0 right-0 h-3 w-3 cursor-se-resize" },
  { dir: "SouthWest", className: "absolute bottom-0 left-0 h-3 w-3 cursor-sw-resize" },
]

export function AuthCard({
  title,
  description,
  children,
  showDesktopControls,
  mode,
  onToggleView,
}: AuthCardProps) {
  const { startDragging, minimizeWindow, closeWindow, resizeWindow } = useWindowControls()

  function handleDragMouseDown(event: MouseEvent<HTMLDivElement>) {
    if (event.button === 0) {
      void startDragging()
    }
  }

  return (
    <div className="relative flex flex-col gap-6 bg-background py-6 text-sm text-foreground">
      {showDesktopControls && (
        <div
          data-tauri-drag-region
          className="absolute inset-x-0 top-0 h-6 cursor-move select-none"
          onMouseDown={handleDragMouseDown}
        />
      )}
      <div className="grid auto-rows-min items-start gap-2 px-6">
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="font-heading text-base font-medium">{title}</div>
            <div className="text-sm text-muted-foreground">{description}</div>
          </div>
          <div className="flex items-center gap-1">
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              onClick={onToggleView}
              aria-label={mode === "login" ? "打开配置" : "返回登录"}
            >
              {mode === "login" ? <Settings className="size-4" /> : <ArrowLeft className="size-4" />}
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

      {showDesktopControls && (
        <>
          {RESIZE_HANDLES.map(({ dir, className }) => (
            <div
              key={dir}
              className={className}
              onMouseDown={() => void resizeWindow(dir)}
            />
          ))}
        </>
      )}
    </div>
  )
}
