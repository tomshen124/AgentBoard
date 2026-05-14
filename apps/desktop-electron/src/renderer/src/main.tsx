import './assets/main.css'
import './stores/quota-store'
import { createRoot } from 'react-dom/client'
import App from './App'
import { NotifyWindow } from './components/notify/NotifyWindow'
import { installStreamingPerfMonitor } from './lib/streaming-perf'

const isNotifyWindow = window.location.hash.startsWith('#notify')

installStreamingPerfMonitor()

const rootEl = document.getElementById('root')!
try {
  createRoot(rootEl).render(isNotifyWindow ? <NotifyWindow /> : <App />)
} catch (err) {
  rootEl.innerHTML = `<div style="color:red;padding:20px;font-family:monospace;"><h1>React render error</h1><pre>${String(err)}</pre></div>`
  console.error('React render error:', err)
}
