import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'

type SendFilesRequest = {
  targetDeviceId: string
  paths: string[]
}

export function getAppSnapshot<T>() {
  return invoke<T>('get_app_snapshot')
}

export function setSaveDirectory<T>(saveDirectory: string) {
  return invoke<T>('set_save_directory', { saveDirectory })
}

export function setAutoAccept<T>(autoAccept: boolean) {
  return invoke<T>('set_auto_accept', { autoAccept })
}

export function sendFiles<T>(request: SendFilesRequest) {
  return invoke<T>('send_files', { request })
}

export function listenTransferProgress<T>(handler: (payload: T) => void) {
  return listen<T>('transfer-progress', (event) => handler(event.payload))
}

export function openFilePicker(options: { multiple: boolean; directory: boolean }) {
  return open(options)
}

export function userFacingRuntimeError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error)
  if (!isDesktopRuntimeAvailable()) {
    return 'Airsend is waiting for the desktop runtime. Open the Tauri app to use local files and network transfer.'
  }
  return message
}

function isDesktopRuntimeAvailable() {
  return isTauri()
}
