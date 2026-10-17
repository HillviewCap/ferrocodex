// antd 5 needs this to render message/notification/Modal static methods on React 19
import '@ant-design/v5-patch-for-react-19';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';
import { initializeCSPReporting } from './utils/cspReporting';

// Initialize CSP violation reporting
initializeCSPReporting();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <App />
);