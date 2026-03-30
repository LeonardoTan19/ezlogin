import { getCurrentWindow } from "@tauri-apps/api/window"

export type WindowResizeDirection =
  | "East"
  | "West"
  | "North"
  | "South"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest"

export function useWindowControls() {
  async function startDragging() {
    try {
      await getCurrentWindow().startDragging()
    } catch {
      // Ignore drag failures on non-desktop runtime.
    }
  }

  async function minimizeWindow() {
    try {
      await getCurrentWindow().minimize()
    } catch {
      // Ignore desktop API errors on non-desktop runtime.
    }
  }

  async function closeWindow() {
    try {
      await getCurrentWindow().close()
    } catch {
      // Ignore desktop API errors on non-desktop runtime.
    }
  }

  async function resizeWindow(direction: WindowResizeDirection) {
    try {
      await getCurrentWindow().startResizeDragging(direction)
    } catch {
      // Ignore resize drag failures on non-desktop runtime.
    }
  }

  return {
    startDragging,
    minimizeWindow,
    closeWindow,
    resizeWindow,
  }
}
