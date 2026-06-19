import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { SheetData } from '../types'
import './Welcome.css'

interface WelcomeProps {
  onImportComplete: () => void
}

export default function Welcome({ onImportComplete }: WelcomeProps) {
  const [isLoading, setIsLoading] = useState(false)

  const handleFileImport = async () => {
    const extensions = [
      { name: 'CSV', extensions: ['csv'] },
      { name: 'Excel', extensions: ['xlsx', 'xls'] }
    ]
    const filePath = await open({ filters: extensions, multiple: false })
    if (!filePath) return

    setIsLoading(true)
    try {
      const wb = await invoke('create_workbook', { name: 'Imported Data' })
      const cmd = filePath.endsWith('.csv') ? 'import_csv_file' : 'import_excel_file'
      await invoke<SheetData>(cmd, { workbookId: wb.id, filePath })
      onImportComplete()
    } catch (e) {
      alert(`Import failed: ${e}`)
    } finally {
      setIsLoading(false)
    }
  }

  const handleGoogleSignIn = () => {
    alert('Google sign-in integration coming soon')
  }

  const handleNotionSignIn = () => {
    alert('Notion sign-in integration coming soon')
  }

  return (
    <div className="welcome-container">
      <div className="welcome-content">
        <h1>Welcome to Frappe</h1>
        <p>Your print shop management system</p>

        <div className="import-section">
          <h2>Get Started</h2>
          <p>Choose how you'd like to begin:</p>

          <div className="import-options">
            <button
              className="import-btn google-btn"
              onClick={handleGoogleSignIn}
              disabled={isLoading}
            >
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <path d="M10 0C4.48 0 0 4.48 0 10s4.48 10 10 10 10-4.48 10-10S15.52 0 10 0z" fill="currentColor"/>
              </svg>
              Sign in with Google
            </button>

            <button
              className="import-btn notion-btn"
              onClick={handleNotionSignIn}
              disabled={isLoading}
            >
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <path d="M10 0C4.48 0 0 4.48 0 10s4.48 10 10 10 10-4.48 10-10S15.52 0 10 0z" fill="currentColor"/>
              </svg>
              Sign in with Notion
            </button>

            <button
              className="import-btn file-btn"
              onClick={handleFileImport}
              disabled={isLoading}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                <polyline points="17 8 12 3 7 8"></polyline>
                <line x1="12" y1="3" x2="12" y2="15"></line>
              </svg>
              {isLoading ? 'Importing...' : 'Import from File'}
            </button>
          </div>

          <div className="divider">or</div>

          <button
            className="btn btn-primary"
            onClick={() => invoke('create_workbook', { name: `Workbook 1` }).then(onImportComplete)}
            disabled={isLoading}
          >
            Start with Empty Workbook
          </button>
        </div>
      </div>
    </div>
  )
}
