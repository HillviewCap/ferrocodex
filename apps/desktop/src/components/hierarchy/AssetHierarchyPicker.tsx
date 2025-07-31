import React, { useState, useCallback, useMemo } from 'react';
import { TreeSelect, Typography, Space, Button } from 'antd';
import { FolderOutlined, HomeOutlined } from '@ant-design/icons';
import { AssetHierarchy } from '../../types/assets';

const { Text } = Typography;

export interface AssetHierarchyPickerProps {
  hierarchyData: AssetHierarchy[];
  value?: number | null;
  onChange?: (value: number | null) => void;
  placeholder?: string;
  disabled?: boolean;
  allowClear?: boolean;
  excludeAssetId?: number; // Exclude this asset and its descendants from selection
}

interface TreeSelectDataNode {
  title: React.ReactNode;
  value: number;
  key: number;
  children?: TreeSelectDataNode[];
  disabled?: boolean;
}

export const AssetHierarchyPicker: React.FC<AssetHierarchyPickerProps> = ({
  hierarchyData,
  value,
  onChange,
  placeholder = "Select parent folder (leave empty for root level)",
  disabled = false,
  allowClear = true,
  excludeAssetId,
}) => {
  const [searchValue, setSearchValue] = useState('');

  // Convert AssetHierarchy to TreeSelect data, filtering out devices and excluded assets
  const convertToTreeSelectData = useCallback((
    assets: AssetHierarchy[], 
    excludeId?: number
  ): TreeSelectDataNode[] => {
    return assets
      .filter(asset => {
        // Only include folders
        if (asset.asset_type !== 'Folder') return false;
        // Exclude the specified asset
        if (excludeId && asset.id === excludeId) return false;
        return true;
      })
      .map(asset => {
        const hasExcludedDescendant = excludeId ? isDescendantOf(asset, excludeId) : false;
        
        return {
          title: (
            <Space size={8}>
              <FolderOutlined style={{ color: '#1890ff' }} />
              <Text>{asset.name}</Text>
              {asset.children.filter(c => c.asset_type === 'Folder').length > 0 && (
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  ({asset.children.filter(c => c.asset_type === 'Folder').length} folders)
                </Text>
              )}
            </Space>
          ),
          value: asset.id,
          key: asset.id,
          children: asset.children.length > 0 ? 
            convertToTreeSelectData(asset.children, excludeId) : undefined,
          disabled: hasExcludedDescendant,
        };
      });
  }, []);

  // Check if an asset is a descendant of the excluded asset
  const isDescendantOf = useCallback((asset: AssetHierarchy, excludeId: number): boolean => {
    if (asset.id === excludeId) return true;
    return asset.children.some(child => isDescendantOf(child, excludeId));
  }, []);

  const treeData = useMemo(() => 
    convertToTreeSelectData(hierarchyData, excludeAssetId), 
    [hierarchyData, excludeAssetId, convertToTreeSelectData]
  );

  const handleChange = (newValue: number | null) => {
    onChange?.(newValue);
  };

  const handleClear = () => {
    onChange?.(null);
  };

  const filterTreeNode = (input: string, node: any) => {
    const title = typeof node.title === 'string' ? node.title : '';
    return title.toLowerCase().includes(input.toLowerCase());
  };

  return (
    <div>
      <Space direction="vertical" size={8} style={{ width: '100%' }}>
        <TreeSelect
          value={value}
          onChange={handleChange}
          treeData={treeData}
          placeholder={placeholder}
          disabled={disabled}
          allowClear={allowClear}
          showSearch
          filterTreeNode={filterTreeNode}
          searchValue={searchValue}
          onSearch={setSearchValue}
          style={{ width: '100%' }}
          dropdownStyle={{ maxHeight: 400, overflow: 'auto' }}
          treeDefaultExpandAll={false}
          treeIcon
          treeLine
          showCheckedStrategy={TreeSelect.SHOW_PARENT}
        />
        
        <Space>
          <Button 
            type="link" 
            size="small" 
            icon={<HomeOutlined />}
            onClick={handleClear}
            disabled={disabled}
            style={{ padding: 0, height: 'auto' }}
          >
            Place in root level
          </Button>
        </Space>
        
        {value && (
          <div style={{ fontSize: '12px', color: '#666' }}>
            <Text type="secondary">
              Asset will be created inside the selected folder
            </Text>
          </div>
        )}
        
        {!value && (
          <div style={{ fontSize: '12px', color: '#666' }}>
            <Text type="secondary">
              Asset will be created at the root level
            </Text>
          </div>
        )}
      </Space>
    </div>
  );
};

export default AssetHierarchyPicker;