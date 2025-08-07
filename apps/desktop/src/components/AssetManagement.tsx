import React, { useState } from 'react';
import { message } from 'antd';
import { AssetInfo } from '../types/assets';
import useAssetStore from '../store/assets';
import ImportConfigurationModal from './ImportConfigurationModal';
import ConfigurationHistoryView from './ConfigurationHistoryView';
import { AssetHierarchyView } from './hierarchy/AssetHierarchyView';

const AssetManagement: React.FC = () => {
  const { 
    currentView,
    selectedAsset,
    navigateToHistory,
    navigateToDashboard
  } = useAssetStore();
  const [importModalVisible, setImportModalVisible] = useState(false);

  const handleImportSuccess = (_asset: AssetInfo) => {
    setImportModalVisible(false);
    message.success('Configuration imported successfully!');
  };


  // Render history view if selected
  if (currentView === 'history' && selectedAsset) {
    return (
      <ConfigurationHistoryView
        asset={selectedAsset}
        onBack={navigateToDashboard}
      />
    );
  }

  // Render new asset hierarchy view
  return (
    <div style={{ height: '100vh', overflow: 'hidden' }}>
      <AssetHierarchyView
        onAssetSelect={(asset) => {
          // You can add any additional logic here when an asset is selected
          console.log('Asset selected:', asset);
        }}
        onNavigateToHistory={(asset) => {
          // Convert AssetInfo to match expected format and navigate to history
          navigateToHistory(asset);
        }}
      />
      
      <ImportConfigurationModal
        visible={importModalVisible}
        onCancel={() => setImportModalVisible(false)}
        onSuccess={handleImportSuccess}
      />
    </div>
  );
};

export default AssetManagement;