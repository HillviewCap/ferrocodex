import React from 'react';
import { List } from 'antd';
import { ConfigurationVersionInfo, sortVersions } from '../types/assets';
import VersionCard from './VersionCard';

interface VersionHistoryListProps {
  versions: ConfigurationVersionInfo[];
}

const VersionHistoryList: React.FC<VersionHistoryListProps> = ({ versions }) => {
  // Sort versions in reverse chronological order (latest first)
  const sortedVersions = sortVersions(versions);

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
          <VersionCard version={version} />
        </List.Item>
      )}
      style={{ width: '100%' }}
    />
  );
};

export default VersionHistoryList;