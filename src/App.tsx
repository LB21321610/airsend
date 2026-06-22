import { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
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
  const targetDeviceId = selectedDeviceId || nearbyDevices[0]?.deviceId || ''
  const safeInterfaces = snapshot.interfaces.filter((iface) => iface.allowedForBind)
  const queuedBytes = useMemo(
    () => snapshot.transferHistory.reduce((total, item) => total + item.totalBytes, 0),
    [snapshot.transferHistory],
  )

  const refreshSnapshot = useCallback(async () => {
    try {
      setError(null)
      const next = await invoke<AppSnapshot>('get_app_snapshot')
      setSnapshot(next)
      if (!selectedDeviceId && next.devices.length > 0) {
        setSelectedDeviceId(next.devices[0].deviceId)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [selectedDeviceId])

  useEffect(() => {
    void refreshSnapshot()
  }, [refreshSnapshot])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void listen<TransferProgressEvent>('transfer-progress', (event) => {
      setLatestProgress(event.payload)
    }).then((cleanup) => {
      unlisten = cleanup
    })

    return () => {
      unlisten?.()
    }
  }, [])

  async function chooseFiles() {
    const result = await open({ multiple: true, directory: false })
    setSelectedPaths(normalizeDialogResult(result))
  }

  async function chooseFolder() {
    const result = await open({ multiple: false, directory: true })
    setSelectedPaths(normalizeDialogResult(result))
  }

  async function chooseSaveDirectory() {
    const result = await open({ multiple: false, directory: true })
    const [saveDirectory] = normalizeDialogResult(result)
    if (!saveDirectory) return
    const next = await invoke<AppSnapshot>('set_save_directory', { saveDirectory })
    setSnapshot(next)
  }

  async function toggleAutoAccept() {
    const next = await invoke<AppSnapshot>('set_auto_accept', {
      autoAccept: !snapshot.settings.autoAccept,
    })
    setSnapshot(next)
  }

  async function queueTransfer() {
    if (selectedPaths.length === 0) {
      setError('Choose at least one file or folder first.')
      return
    }
    if (!targetDeviceId) {
      setError('No nearby peer is available yet. Start Airsend on another device first.')
      return
    }

    const next = await invoke<AppSnapshot>('send_files', {
      request: {
        targetDeviceId,
        paths: selectedPaths,
      },
    })
    setSnapshot(next)
    setLatestProgress({
      transferId: next.transferHistory[0]?.transferId ?? 'queued',
      status: 'Queued for local transfer engine',
      transferredBytes: 0,
      totalBytes: next.transferHistory[0]?.totalBytes ?? 0,
      bytesPerSecond: 0,
    })
  }

  return (
    <main
      className={isDragging ? 'app-shell dragging' : 'app-shell'}
      onDragOver={(event) => {
        event.preventDefault()
        setIsDragging(true)
      }}
      onDragLeave={() => setIsDragging(false)}
      onDrop={(event) => {
        event.preventDefault()
        setIsDragging(false)
      }}
    >
      <section className="hero-band">
        <div>
          <p className="eyebrow">Local P2P transfer</p>
          <h1>Airsend</h1>
          <p className="hero-copy">
            Fast LAN file transfer with local-only discovery, private interface guards, and a
            resumable transfer core taking shape underneath.
          </p>
        </div>
        <div className="device-orbit" aria-label="Local device">
          <span className="pulse-ring"></span>
          <div className="device-avatar">{initials(snapshot.localDevice.displayName)}</div>
          <strong>{snapshot.localDevice.displayName}</strong>
          <small>{safeInterfaces.length} safe LAN interface{safeInterfaces.length === 1 ? '' : 's'}</small>
        </div>
      </section>

      {error && <div className="notice error">{error}</div>}
      <div className="notice">
        <strong>{snapshot.firewallGuidance.title}</strong>
        <span>{snapshot.firewallGuidance.platformHint}</span>
      </div>

      <section className="workspace">
        <aside className="panel">
          <div className="panel-heading">
            <p className="eyebrow">Nearby</p>
            <button onClick={refreshSnapshot}>Refresh</button>
          </div>
          <div className="device-list">
            {nearbyDevices.length === 0 ? (
              <div className="empty-state">
                <div className="mini-orbit">{initials(snapshot.localDevice.displayName)}</div>
                <strong>No peers yet</strong>
                <span>Discovery hooks are ready; mDNS runtime lands next.</span>
              </div>
            ) : (
              nearbyDevices.map((device) => (
                <button
                  className={device.deviceId === targetDeviceId ? 'device-row active' : 'device-row'}
                  key={device.deviceId}
                  onClick={() => setSelectedDeviceId(device.deviceId)}
                >
                  <span>{initials(device.displayName)}</span>
                  <div>
                    <strong>{device.displayName}</strong>
                    <small>{device.trusted ? 'Trusted' : 'Pairing required'}</small>
                  </div>
                </button>
              ))
            )}
          </div>
        </aside>

        <section className="drop-panel">
          <div className="drop-zone">
            <div className="drop-icon">Up</div>
            <h2>Drop files here</h2>
            <p>
              Desktop drag/drop animation is in place. Use the picker for real local paths while
              Tauri file-drop handling is wired in.
            </p>
            <div className="actions">
              <button onClick={chooseFiles}>Choose files</button>
              <button onClick={chooseFolder}>Choose folder</button>
            </div>
          </div>

          <div className="selection-bar">
            <div>
              <span>{selectedPaths.length} selected</span>
              <strong>{selectedPaths.length > 0 ? compactPathList(selectedPaths) : 'Ready when you are'}</strong>
            </div>
            <button className="primary" onClick={queueTransfer}>
              Queue transfer
            </button>
          </div>

          {latestProgress && (
            <div className="progress-card">
              <div>
                <strong>{latestProgress.status}</strong>
                <span>{formatBytes(latestProgress.totalBytes)} prepared</span>
              </div>
              <div className="progress-track">
                <span style={{ width: `${progressPercent(latestProgress)}%` }}></span>
              </div>
            </div>
          )}
        </section>

        <aside className="panel">
          <p className="eyebrow">Settings</p>
          <div className="settings-list">
            <label>
              <span>Auto accept trusted peers</span>
              <input
                type="checkbox"
                checked={snapshot.settings.autoAccept}
                onChange={toggleAutoAccept}
              />
            </label>
            <div>
              <span>Save directory</span>
              <strong>{snapshot.settings.saveDirectory ?? 'Ask every time'}</strong>
              <button onClick={chooseSaveDirectory}>Change</button>
            </div>
            <div>
              <span>Workers</span>
              <strong>{snapshot.settings.parallelWorkers}</strong>
            </div>
          </div>
        </aside>
      </section>

      <section className="lower-grid">
        <div className="panel">
          <div className="panel-heading">
            <p className="eyebrow">Transfer history</p>
            <span>{formatBytes(queuedBytes)} total</span>
          </div>
          <div className="history-list">
            {snapshot.transferHistory.length === 0 ? (
              <p className="muted">No transfers yet.</p>
            ) : (
              snapshot.transferHistory.map((item) => (
                <div className="history-row" key={item.transferId}>
                  <div>
                    <strong>{item.fileCount} item{item.fileCount === 1 ? '' : 's'}</strong>
                    <span>{item.direction} - {item.status}</span>
                  </div>
                  <span>{formatBytes(item.totalBytes)}</span>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="panel">
          <p className="eyebrow">LAN guard</p>
          <div className="interface-list">
            {snapshot.interfaces.length === 0 ? (
              <p className="muted">No interfaces reported yet.</p>
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
