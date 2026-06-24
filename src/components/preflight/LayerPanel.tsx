import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { LayerInfo } from '../../types'
import { t } from '../../i18n'

interface LayerPanelProps {
  filePath?: string
  onLayerChanged?: (newPath: string) => void
}

export default function LayerPanel({ filePath, onLayerChanged }: LayerPanelProps) {
  const [layers, setLayers] = useState<LayerInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busyId, setBusyId] = useState<number | null>(null)
  const [lastOutput, setLastOutput] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    if (!filePath) return
    setLoading(true)
    setError(null)
    try {
      const result = await invoke<LayerInfo[]>('list_layers', { path: filePath })
      setLayers(result)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [filePath])

  useEffect(() => {
    refresh()
  }, [refresh])

  const toggle = useCallback(
    async (layer: LayerInfo) => {
      if (!filePath) return
      setBusyId(layer.object_id)
      setError(null)
      try {
        const newVisible = !layer.visible
        const tmp = filePath.replace(/\.pdf$/i, `_layer_${layer.object_id}_${newVisible ? 'on' : 'off'}.pdf`)
        await invoke('set_layer_visibility', {
          path: filePath,
          objectId: layer.object_id,
          visible: newVisible,
          outputPath: tmp,
        })
        setLastOutput(tmp)
        if (onLayerChanged) onLayerChanged(tmp)
        setLayers((prev) =>
          prev.map((l) =>
            l.object_id === layer.object_id ? { ...l, visible: newVisible } : l
          )
        )
      } catch (e) {
        setError(String(e))
      } finally {
        setBusyId(null)
      }
    },
    [filePath, onLayerChanged]
  )

  if (!filePath) {
    return (
      <div className="layer-panel">
        <h3>{t('layer.title')}</h3>
        <p className="pdf-empty">Open a PDF to view its layers.</p>
      </div>
    )
  }

  return (
    <div className="layer-panel">
      <div className="layer-panel-header">
        <h3>{t('layer.title')}</h3>
        <button className="btn btn-secondary btn-small" onClick={refresh} disabled={loading}>
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>
      {error && <p className="pdf-error">{error}</p>}
      {lastOutput && (
        <p className="layer-panel-output">
          Last change written to <code>{lastOutput}</code>
        </p>
      )}
      {layers.length === 0 && !loading && (
        <p className="pdf-empty">This PDF has no Optional Content Groups (layers).</p>
      )}
      <ul className="layer-list">
        {layers.map((layer) => (
          <li key={layer.object_id} className="layer-item">
            <label>
              <input
                type="checkbox"
                checked={layer.visible}
                onChange={() => toggle(layer)}
                disabled={busyId === layer.object_id}
              />
              <span className="layer-name">
                {layer.name || `Layer ${layer.object_id}`}
              </span>
            </label>
            <span className="layer-id">OCG {layer.object_id}</span>
          </li>
        ))}
      </ul>
      <p className="layer-panel-help">
        Toggle visibility on each OCG. Each change writes a new file suffixed with the layer
        id and the new visibility so the source PDF is never overwritten.
      </p>
    </div>
  )
}
