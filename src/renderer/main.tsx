import { createRoot } from 'react-dom/client'
import './styles/global.css'
import { App } from './App'
import { ErrorBoundary } from './components/ErrorBoundary'
import { api } from '@lib/bridge'
import { parseRendererEnv } from '@shared/env/env-schema'

// Install the bridge before React mounts: every provider and hook reads
// `window.api` during its first render.
window.api = api

parseRendererEnv(import.meta.env as Record<string, string | undefined>)

createRoot(document.getElementById('root')!).render(
  <ErrorBoundary>
    <App />
  </ErrorBoundary>,
)

document.getElementById('loading-screen')?.remove()
