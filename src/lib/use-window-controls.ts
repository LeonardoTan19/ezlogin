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

async function safely(fn: () => Promise<unknown>) {
  try {
    await fn()
  } catch {}
}

export function useWindowControls() {
  return {
    startDragging: () => safely(() => getCurrentWindow().startDragging()),
    minimizeWindow: () => safely(() => getCurrentWindow().minimize()),
    closeWindow: () => safely(() => getCurrentWindow().close()),
    resizeWindow: (direction: WindowResizeDirection) =>
      safely(() => getCurrentWindow().startResizeDragging(direction)),
  }
}
