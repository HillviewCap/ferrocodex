import React from 'react';
import { List } from 'antd';
import { ConfigurationVersionInfo, sortVersions } from '../types/assets';
import VersionCard from './VersionCard';
import useAuthStore from '../store/auth';
import { canChangeConfigurationStatus } from '../utils/roleUtils';

interface VersionHistoryListProps {
  versions: ConfigurationVersionInfo[];
  onCreateBranch?: (version: ConfigurationVersionInfo) => void;
  showCreateBranch?: boolean;
  onStatusChange?: () => void;
}

const VersionHistoryList: React.FC<VersionHistoryListProps> = ({ 
  versions, 
  onCreateBranch, 
  showCreateBranch = false,
  onStatusChange 
}) => {
  const { token, user } = useAuthStore();
  
  // Sort versions in reverse chronological order (latest first)
  const sortedVersions = sortVersions(versions);
  
  const canUserChangeStatus = canChangeConfigurationStatus(user);

  return (
    <List
      dataSource={sortedVersions}
      pagination={sortedVersions.length > 10 ? {
        pageSize: 10,
        showSizeChanger: true,
        showQuickJumper: true,
        showTotal: (total, range) => `${range[0]}-${range[1]} of ${total} versions`,
        style: { textAlign: 'center', marginTop: '24px' }
      } : false}
      renderItem={(version) => (
        <List.Item style={{ padding: '12px 0' }}>
          <VersionCard 
            version={version} 
            onCreateBranch={onCreateBranch}
            showCreateBranch={showCreateBranch}
            onStatusChange={onStatusChange}
            token={token || undefined}
            canChangeStatus={canUserChangeStatus}
          />
        </List.Item>
      )}
      style={{ width: '100%' }}
    />
  );
};

export default VersionHistoryList;