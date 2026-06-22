import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  getAppSnapshot,
  listenTransferProgress,
  openFilePicker,
  sendFiles,
  setAutoAccept,
  setSaveDirectory,
  userFacingRuntimeError,
} from './desktopRuntime'
import './App.css'

type Device = {
  deviceId: string
  displayName: string
  fingerprint: string
  protocolVersion: number
  appVersion: string
  controlPort: number | null
  online: boolean
  trusted: boolean
  lastSeen: string | null
  addresses: string[]
}

type Settings = {
  saveDirectory: string | null
  autoAccept: boolean
  parallelWorkers: number
}

type TransferHistoryRecord = {
  transferId: string
  peerId: string
  peerName: string
  direction: 'incoming' | 'outgoing'
  fileCount: number
  totalBytes: number
  status: 'queued' | 'waitingForApproval' | 'transferring' | 'completed' | 'failed' | 'cancelled'
  createdAt: string
  completedAt: string | null
}

type NetworkInterfaceSummary = {
  name: string
  ip: string
  allowedForBind: boolean
}

type FirewallGuidance = {
  title: string
  message: string
  platformHint: string
}

type AppSnapshot = {
  localDevice: Device
  devices: Device[]
  settings: Settings
  transferHistory: TransferHistoryRecord[]
  interfaces: NetworkInterfaceSummary[]
  firewallGuidance: FirewallGuidance
}

type TransferProgressEvent = {
  transferId: string
  status: string
  transferredBytes: number
  totalBytes: number
  bytesPerSecond: number
}

const emptySnapshot: AppSnapshot = {
  localDevice: {
    deviceId: 'loading',
    displayName: 'Airsend',
    fingerprint: '',
    protocolVersion: 1,
    appVersion: '0.1.0',
    controlPort: null,
    online: true,
    trusted: true,
    lastSeen: null,
    addresses: [],
  },
  devices: [],
  settings: {
    saveDirectory: null,
    autoAccept: false,
    parallelWorkers: 4,
  },
  transferHistory: [],
  interfaces: [],
  firewallGuidance: {
    title: 'Local network access needed',
    message: 'Airsend listens only on private LAN interfaces.',
    platformHint: 'Allow private local network access if your OS asks.',
  },
}

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot>(emptySnapshot)
  const [selectedPaths, setSelectedPaths] = useState<string[]>([])
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('')
  const [error, setError] = useState<string | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [latestProgress, setLatestProgress] = useState<TransferProgressEvent | null>(null)

  const nearbyDevices = snapshot.devices
  const onlineDevices = nearbyDevices.filter((device) => device.online)
  const targetDeviceId = selectedDeviceId
  const targetDevice = nearbyDevices.find((device) => device.deviceId === targetDeviceId)
  const safeInterfaces = snapshot.interfaces.filter((iface) => iface.allowedForBind)
  const blockedInterfaces = snapshot.interfaces.length - safeInterfaces.length
  const queuedBytes = useMemo(
    () => snapshot.transferHistory.reduce((total, item) => total + item.totalBytes, 0),
    [snapshot.transferHistory],
  )
  const canSend = selectedPaths.length > 0 && Boolean(targetDevice?.online)
  const selectionSummary =
    selectedPaths.length > 0 ? compactPathList(selectedPaths) : 'No files selected'

  const refreshSnapshot = useCallback(async () => {
    try {
      setError(null)
      const next = await getAppSnapshot<AppSnapshot>()
      setSnapshot(next)
      setSelectedDeviceId((currentDeviceId) => {
        const selectedDeviceStillOnline = next.devices.some(
          (device) => device.deviceId === currentDeviceId && device.online,
        )
        if (selectedDeviceStillOnline) return currentDeviceId
        return next.devices.find((device) => device.online)?.deviceId ?? ''
      })
    } catch (err) {
      setError(userFacingRuntimeError(err))
    }
  }, [])

  useEffect(() => {
    void refreshSnapshot()
  }, [refreshSnapshot])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void listenTransferProgress<TransferProgressEvent>((payload) => {
      setLatestProgress(payload)
    }).then((cleanup) => {
      unlisten = cleanup
    }).catch((err) => {
      setError(userFacingRuntimeError(err))
    })

    return () => {
      unlisten?.()
    }
  }, [])

  async function chooseFiles() {
    try {
      const result = await openFilePicker({ multiple: true, directory: false })
      setSelectedPaths(normalizeDialogResult(result))
      setError(null)
    } catch (err) {
      setError(userFacingRuntimeError(err))
    }
  }

  async function chooseFolder() {
    try {
      const result = await openFilePicker({ multiple: false, directory: true })
      setSelectedPaths(normalizeDialogResult(result))
      setError(null)
    } catch (err) {
      setError(userFacingRuntimeError(err))
    }
  }

  async function chooseSaveDirectory() {
    try {
      const result = await openFilePicker({ multiple: false, directory: true })
      const [saveDirectory] = normalizeDialogResult(result)
      if (!saveDirectory) return
      const next = await setSaveDirectory<AppSnapshot>(saveDirectory)
      setSnapshot(next)
      setError(null)
    } catch (err) {
      setError(userFacingRuntimeError(err))
    }
  }

  async function toggleAutoAccept() {
    try {
      const next = await setAutoAccept<AppSnapshot>(!snapshot.settings.autoAccept)
      setSnapshot(next)
      setError(null)
    } catch (err) {
      setError(userFacingRuntimeError(err))
    }
  }

  async function queueTransfer() {
    if (selectedPaths.length === 0) {
      setError('Choose at least one file or folder first.')
      return
    }
    if (!targetDevice?.online) {
      setError('Choose an available nearby device first.')
      return
    }

    try {
      const next = await sendFiles<AppSnapshot>({
        targetDeviceId: targetDevice.deviceId,
        paths: selectedPaths,
      })
      setSnapshot(next)
      setLatestProgress({
        transferId: next.transferHistory[0]?.transferId ?? 'queued',
        status: 'Queued for local transfer engine',
        transferredBytes: 0,
        totalBytes: next.transferHistory[0]?.totalBytes ?? 0,
        bytesPerSecond: 0,
      })
      setError(null)
    } catch (err) {
      setError(userFacingRuntimeError(err))
    }
  }

  return (
    <main
      className={isDragging ? 'app-shell is-dragging' : 'app-shell'}
      onDragOver={(event) => {
        event.preventDefault()
        setIsDragging(true)
      }}
      onDragLeave={() => setIsDragging(false)}
      onDrop={(event) => {
        event.preventDefault()
        setIsDragging(false)
        setError('Use Choose files or Choose folder so Airsend can receive exact local paths.')
      }}
    >
      <header className="app-header">
        <div className="brand-lockup" aria-label="Airsend local device">
          <div className="app-mark">A</div>
          <div>
            <h1>Airsend</h1>
            <p>{snapshot.localDevice.displayName}</p>
          </div>
        </div>
        <div className="status-strip" aria-label="Local network status">
          <span className="status-pill good">{safeInterfaces.length} LAN ready</span>
          <span className={blockedInterfaces > 0 ? 'status-pill warn' : 'status-pill'}>
            {blockedInterfaces} blocked
          </span>
          <button className="ghost-button" onClick={refreshSnapshot}>
            Refresh
          </button>
        </div>
      </header>

      <section className="alerts" aria-live="polite">
        {error && (
          <div className="notice error" role="alert">
            <strong>Action needed</strong>
            <span>{error}</span>
          </div>
        )}
        <div className="notice">
          <strong>{snapshot.firewallGuidance.title}</strong>
          <span>{snapshot.firewallGuidance.platformHint}</span>
        </div>
      </section>

      <section className="workspace" aria-label="File transfer workspace">
        <section className="send-panel">
          <div className="drop-zone" aria-label="Select files or folders to send">
            <div className="drop-icon" aria-hidden="true">
              ↑
            </div>
            <h2>{isDragging ? 'Release to continue with the picker' : 'Send files on this network'}</h2>
            <p>
              Choose files or a folder, select a nearby device, then queue the transfer locally.
            </p>
            <div className="actions">
              <button onClick={chooseFiles}>Choose files</button>
              <button onClick={chooseFolder}>Choose folder</button>
            </div>
          </div>

          <div className="selection-bar">
            <div>
              <span>{selectedPaths.length} selected</span>
              <strong title={selectionSummary}>{selectionSummary}</strong>
            </div>
            <button className="primary" onClick={queueTransfer} disabled={!canSend}>
              {targetDevice ? `Send to ${targetDevice.displayName}` : 'Choose a device'}
            </button>
          </div>

          {latestProgress && (
            <div className="progress-card">
              <div>
                <strong>{latestProgress.status}</strong>
                <span>
                  {formatBytes(latestProgress.transferredBytes)} of{' '}
                  {formatBytes(latestProgress.totalBytes)}
                  {latestProgress.bytesPerSecond > 0
                    ? ` · ${formatBytes(latestProgress.bytesPerSecond)}/s`
                    : ''}
                </span>
              </div>
              <div className="progress-track">
                <span style={{ width: `${progressPercent(latestProgress)}%` }}></span>
              </div>
            </div>
          )}
        </section>

        <aside className="panel device-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Nearby devices</p>
              <h2>
                {nearbyDevices.length === 0
                  ? 'Looking on this network'
                  : `${onlineDevices.length} available`}
              </h2>
            </div>
            <span className="live-dot" aria-label="Discovery active"></span>
          </div>
          <div className="device-list" aria-live="polite">
            {nearbyDevices.length === 0 ? (
              <div className="empty-state">
                <div className="empty-icon" aria-hidden="true">
                  {initials(snapshot.localDevice.displayName)}
                </div>
                <strong>No nearby devices</strong>
                <span>Keep Airsend open on another device connected to the same Wi-Fi.</span>
              </div>
            ) : (
              nearbyDevices.map((device) => (
                <button
                  className={device.deviceId === targetDeviceId ? 'device-row active' : 'device-row'}
                  aria-pressed={device.deviceId === targetDeviceId}
                  disabled={!device.online}
                  key={device.deviceId}
                  onClick={() => setSelectedDeviceId(device.deviceId)}
                >
                  <span className="device-initials">{initials(device.displayName)}</span>
                  <div>
                    <strong>{device.displayName}</strong>
                    <small>
                      {device.trusted ? 'Trusted' : 'Pairing required'} ·{' '}
                      {device.online ? 'Online' : 'Offline'}
                    </small>
                  </div>
                </button>
              ))
            )}
          </div>
        </aside>

        <aside className="panel settings-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Receiving</p>
              <h2>Rules and location</h2>
            </div>
          </div>
          <div className="settings-list">
            <label>
              <span>
                <strong>Auto accept trusted peers</strong>
                <small>Skip confirmation only for paired devices.</small>
              </span>
              <input
                type="checkbox"
                checked={snapshot.settings.autoAccept}
                onChange={toggleAutoAccept}
              />
            </label>
            <div>
              <span>Save directory</span>
              <strong>{snapshot.settings.saveDirectory ?? 'Ask every time'}</strong>
              <button className="secondary" onClick={chooseSaveDirectory}>
                Change
              </button>
            </div>
            <div>
              <span>Transfer workers</span>
              <strong>{snapshot.settings.parallelWorkers} parallel connections</strong>
            </div>
          </div>
        </aside>
      </section>

      <section className="lower-grid">
        <div className="panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Transfer queue</p>
              <h2>Recent activity</h2>
            </div>
            <span className="panel-total">{formatBytes(queuedBytes)} total</span>
          </div>
          <div className="history-list">
            {snapshot.transferHistory.length === 0 ? (
              <div className="empty-row">
                <strong>No transfers yet</strong>
                <span>Queued sends and received files will appear here.</span>
              </div>
            ) : (
              snapshot.transferHistory.map((item) => (
                <div className="history-row" key={item.transferId}>
                  <div>
                    <strong>{item.fileCount} item{item.fileCount === 1 ? '' : 's'}</strong>
                    <span>{item.direction} · {statusLabel(item.status)}</span>
                  </div>
                  <span className={`status-text ${item.status}`}>{formatBytes(item.totalBytes)}</span>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">LAN guard</p>
              <h2>Bind targets</h2>
            </div>
          </div>
          <div className="interface-list">
            {snapshot.interfaces.length === 0 ? (
              <div className="empty-row">
                <strong>No interfaces reported</strong>
                <span>Airsend will only listen on private local addresses.</span>
              </div>
            ) : (
              snapshot.interfaces.map((iface) => (
                <div className="interface-row" key={`${iface.name}-${iface.ip}`}>
                  <div>
                    <strong>{iface.name}</strong>
                    <span>{iface.ip}</span>
                  </div>
                  <span className={iface.allowedForBind ? 'safe' : 'blocked'}>
                    {iface.allowedForBind ? 'LAN safe' : 'Blocked'}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </section>
    </main>
  )
}

function statusLabel(status: TransferHistoryRecord['status']) {
  const labels: Record<TransferHistoryRecord['status'], string> = {
    queued: 'Queued',
    waitingForApproval: 'Waiting for approval',
    transferring: 'Transferring',
    completed: 'Completed',
    failed: 'Failed',
    cancelled: 'Cancelled',
  }
  return labels[status]
}

function normalizeDialogResult(result: string | string[] | null): string[] {
  if (!result) return []
  return Array.isArray(result) ? result : [result]
}

function initials(name: string) {
  return name
    .split(/[\s.-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join('') || 'A'
}

function compactPathList(paths: string[]) {
  if (paths.length === 1) return paths[0]
  return `${paths[0]} + ${paths.length - 1} more`
}

function formatBytes(bytes: number) {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  return `${(bytes / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`
}

function progressPercent(event: TransferProgressEvent) {
  if (event.totalBytes === 0) return 0
  return Math.min(100, (event.transferredBytes / event.totalBytes) * 100)
}

export default App
