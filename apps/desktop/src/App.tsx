import { useEffect, useState } from 'react';
import { ConfigProvider, Alert, Button } from 'antd';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';

import useAuthStore from './store/auth';
import useAppStore from './store/app';
import LoadingScreen from './components/LoadingScreen';
import AdminSetup from './components/AdminSetup';
import LoginScreen from './components/LoginScreen';
import Dashboard from './components/Dashboard';
import ProtectedRoute from './components/ProtectedRoute';

function App() {
  const [appState, setAppState] = useState<'initializing' | 'ready' | 'error'>('initializing');
  const [error, setError] = useState<string>('');
  
  const { isAuthenticated, checkSession } = useAuthStore();
  const { isFirstLaunch, initializeDatabase, checkFirstLaunch } = useAppStore();

  const initializeApp = async () => {
    try {
      setAppState('initializing');
      setError('');
      
      // Initialize database
      await initializeDatabase();
      
      // Check if this is first launch
      await checkFirstLaunch();
      
      // Check existing session if user was previously authenticated
      if (isAuthenticated) {
        await checkSession();
      }
      
      setAppState('ready');
    } catch (err) {
      setAppState('error');
      setError(err as string);
    }
  };

  useEffect(() => {
    initializeApp();
  }, []);

  if (appState === 'initializing') {
    return <LoadingScreen message="Initializing Ferrocodex..." />;
  }

  if (appState === 'error') {
    return (
      <div style={{ 
        height: '100vh', 
        display: 'flex', 
        justifyContent: 'center', 
        alignItems: 'center',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        padding: '20px'
      }}>
        <div style={{ textAlign: 'center', maxWidth: '400px' }}>
          <Alert
            message="Initialization Error"
            description={error}
            type="error"
            showIcon
            style={{ marginBottom: '16px', backgroundColor: 'rgba(255,255,255,0.9)' }}
          />
          <Button type="primary" onClick={initializeApp}>
            Retry
          </Button>
        </div>
      </div>
    );
  }

  return (
    <ConfigProvider
      theme={{
        token: {
          colorPrimary: '#667eea',
        },
      }}
    >
      <Router>
        <Routes>
          {/* First launch - admin setup */}
          <Route 
            path="/setup" 
            element={
              isFirstLaunch ? <AdminSetup /> : <Navigate to="/login" replace />
            } 
          />
          
          {/* Login screen */}
          <Route 
            path="/login" 
            element={
              !isAuthenticated ? <LoginScreen /> : <Navigate to="/dashboard" replace />
            } 
          />
          
          {/* Protected dashboard */}
          <Route 
            path="/dashboard" 
            element={
              <ProtectedRoute>
                <Dashboard />
              </ProtectedRoute>
            } 
          />
          
          {/* Default route - redirect based on state */}
          <Route 
            path="/" 
            element={
              isFirstLaunch ? 
                <Navigate to="/setup" replace /> : 
                isAuthenticated ? 
                  <Navigate to="/dashboard" replace /> : 
                  <Navigate to="/login" replace />
            } 
          />
          
          {/* Catch all - redirect to appropriate page */}
          <Route 
            path="*" 
            element={<Navigate to="/" replace />} 
          />
        </Routes>
      </Router>
    </ConfigProvider>
  );
}

export default App;