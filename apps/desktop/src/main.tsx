import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';
import { initializeCSPReporting } from './utils/cspReporting';

// Initialize CSP violation reporting
initializeCSPReporting();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <App />
);