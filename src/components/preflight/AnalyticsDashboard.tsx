import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { AnalyticsSummary } from '../../types'

interface ClientPassRate {
  client_name: string
  runs: number
  errors: number
  warnings: number
  pass_rate: number
}

interface AnalyticsDashboardData {
  summary: AnalyticsSummary
  client_pass_rates: ClientPassRate[]
  average_turnaround_hours: number
  common_error_categories: [string, number][]
}

interface AnalyticsDashboardProps {
  clientId?: number
  refreshKey?: number
}

function formatPercent(n: number, total: number): string {
  if (total === 0) return '0%'
  return `${((n / total) * 100).toFixed(1)}%`
}

function formatPercentFrac(n: number): string {
  return `${(n * 100).toFixed(1)}%`
}

function formatNumber(n: number): string {
  return n.toLocaleString()
}

function formatHours(n: number): string {
  if (n <= 0) return 'n/a'
  if (n < 1) return `${(n * 60).toFixed(0)} min`
  if (n < 48) return `${n.toFixed(1)} h`
  return `${(n / 24).toFixed(1)} d`
}

export default function AnalyticsDashboard({ clientId, refreshKey = 0 }: AnalyticsDashboardProps) {
  const [data, setData] = useState<AnalyticsDashboardData | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      // Prefer the combined dashboard endpoint when present; fall
      // back to the legacy summary when the Tauri side hasn't been
      // updated yet.
      try {
        const result = await invoke<AnalyticsDashboardData>('get_analytics_dashboard')
        setData(result)
        return
      } catch (_e) {
        const summary = await invoke<AnalyticsSummary>('get_analytics_summary')
        setData({
          summary,
          client_pass_rates: [],
          average_turnaround_hours: 0,
          common_error_categories: summary.most_common_errors,
        })
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh, refreshKey])

  if (loading && !data) {
    return (
      <div className="analytics-dashboard">
        <h3>Preflight Analytics</h3>
        <p>Loading…</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="analytics-dashboard">
        <h3>Preflight Analytics</h3>
        <p className="pdf-error">Error: {error}</p>
        <button className="btn btn-secondary" onClick={refresh}>
          Retry
        </button>
      </div>
    )
  }

  if (!data) {
    return (
      <div className="analytics-dashboard">
        <h3>Preflight Analytics</h3>
        <p>No data yet.</p>
        <button className="btn btn-secondary" onClick={refresh}>
          Refresh
        </button>
      </div>
    )
  }

  const passRate = data.summary.total_preflight_runs > 0
    ? formatPercent(
        data.summary.total_preflight_runs - data.summary.total_errors,
        data.summary.total_preflight_runs
      )
    : 'n/a'

  return (
    <div className="analytics-dashboard">
      <div className="analytics-header">
        <h3>Preflight Analytics</h3>
        <button className="btn btn-secondary btn-small" onClick={refresh} disabled={loading}>
          {loading ? 'Refreshing…' : 'Refresh'}
        </button>
      </div>
      {clientId && (
        <p className="analytics-scope">Scope: client #{clientId}</p>
      )}

      <div className="analytics-cards">
        <div className="analytics-card">
          <span className="analytics-card-label">Total jobs</span>
          <span className="analytics-card-value">{formatNumber(data.summary.total_jobs)}</span>
        </div>
        <div className="analytics-card">
          <span className="analytics-card-label">Preflight runs</span>
          <span className="analytics-card-value">{formatNumber(data.summary.total_preflight_runs)}</span>
        </div>
        <div className="analytics-card analytics-card--error">
          <span className="analytics-card-label">Errors</span>
          <span className="analytics-card-value">{formatNumber(data.summary.total_errors)}</span>
        </div>
        <div className="analytics-card analytics-card--warning">
          <span className="analytics-card-label">Warnings</span>
          <span className="analytics-card-value">{formatNumber(data.summary.total_warnings)}</span>
        </div>
        <div className="analytics-card analytics-card--ok">
          <span className="analytics-card-label">Pass rate</span>
          <span className="analytics-card-value">{passRate}</span>
        </div>
        <div className="analytics-card">
          <span className="analytics-card-label">Avg turnaround</span>
          <span className="analytics-card-value">{formatHours(data.average_turnaround_hours)}</span>
        </div>
      </div>

      {data.client_pass_rates && data.client_pass_rates.length > 0 && (
        <section className="analytics-section">
          <h4>Per-client pass rates</h4>
          <table className="analytics-table">
            <thead>
              <tr>
                <th>Client</th>
                <th className="analytics-table-num">Runs</th>
                <th className="analytics-table-num">Errors</th>
                <th className="analytics-table-num">Pass rate</th>
              </tr>
            </thead>
            <tbody>
              {data.client_pass_rates.map((c) => (
                <tr key={c.client_name}>
                  <td>{c.client_name}</td>
                  <td className="analytics-table-num">{formatNumber(c.runs)}</td>
                  <td className="analytics-table-num">{formatNumber(c.errors)}</td>
                  <td className="analytics-table-num">{formatPercentFrac(c.pass_rate)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      {data.common_error_categories && data.common_error_categories.length > 0 && (
        <section className="analytics-section">
          <h4>Top check errors</h4>
          <table className="analytics-table">
            <thead>
              <tr>
                <th>Check</th>
                <th className="analytics-table-num">Count</th>
              </tr>
            </thead>
            <tbody>
              {data.common_error_categories.map(([check, count]) => (
                <tr key={check}>
                  <td>{check}</td>
                  <td className="analytics-table-num">{formatNumber(count)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      {data.summary.jobs_by_day && data.summary.jobs_by_day.length > 0 && (
        <section className="analytics-section">
          <h4>Jobs by day (last 30)</h4>
          <table className="analytics-table">
            <thead>
              <tr>
                <th>Day</th>
                <th className="analytics-table-num">Count</th>
              </tr>
            </thead>
            <tbody>
              {data.summary.jobs_by_day.map(([day, count]) => (
                <tr key={day}>
                  <td>{day}</td>
                  <td className="analytics-table-num">{formatNumber(count)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  )
}
