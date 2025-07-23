import React from 'react';
import { List, message } from 'antd';
import { ConfigurationVersionInfo, sortVersions } from '../types/assets';
import VersionCard from './VersionCard';
import useAuthStore from '../store/auth';
import { canChangeConfigurationStatus, canPromoteToGolden, canExportConfiguration, canArchiveVersion, canRestoreVersion, canLinkFirmware } from '../utils/roleUtils';

interface VersionHistoryListProps {
  versions: ConfigurationVersionInfo[];
  onCreateBranch?: (version: ConfigurationVersionInfo) => void;
  showCreateBranch?: boolean;
  onStatusChange?: () => void;
  onGoldenPromotion?: () => void;
  onExport?: (version: ConfigurationVersionInfo, exportPath: string) => void;
}

const VersionHistoryList: React.FC<VersionHistoryListProps> = ({ 
  versions, 
  onCreateBranch, 
  showCreateBranch = false,
  onStatusChange,
  onGoldenPromotion,
  onExport
}) => {
  const { token, user } = useAuthStore();
  
  // Sort versions in reverse chronological order (latest first)
  const sortedVersions = sortVersions(versions);
  
  const canUserChangeStatus = canChangeConfigurationStatus(user);
  const canUserPromoteToGolden = canPromoteToGolden(user);
  const canUserExport = canExportConfiguration(user);
  const canUserArchive = canArchiveVersion(user);
  const canUserRestore = canRestoreVersion(user);
  const canUserLinkFirmware = canLinkFirmware(user);

  const handleExport = (exportPath: string, version: ConfigurationVersionInfo) => {
    if (onExport) {
      onExport(version, exportPath);
    } else {
      message.success(`Configuration ${version.version_number} exported successfully to ${exportPath}`);
    }
  };

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
            canPromoteToGolden={canUserPromoteToGolden}
            onGoldenPromotion={onGoldenPromotion}
            canExport={canUserExport}
            onExport={(exportPath) => handleExport(exportPath, version)}
            canArchive={canUserArchive}
            canRestore={canUserRestore}
            canLinkFirmware={canUserLinkFirmware}
            onFirmwareLinked={onStatusChange}
            onFirmwareUnlinked={onStatusChange}
          />
        </List.Item>
      )}
      style={{ width: '100%' }}
    />
  );
};

export default VersionHistoryList;