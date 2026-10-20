import React from 'react';
import { Tag } from 'antd';
import { SecurityClassificationLevel, SECURITY_CLASSIFICATIONS } from '../../types/security';

export interface ClassificationTagProps {
  level?: SecurityClassificationLevel;
  compact?: boolean;
}

// Colored tag for an asset's security classification; renders nothing for an unknown level
export const ClassificationTag: React.FC<ClassificationTagProps> = ({ level, compact = false }) => {
  const classification = level ? SECURITY_CLASSIFICATIONS[level] : undefined;
  if (!classification) return null;

  return (
    <Tag
      color={classification.color}
      style={compact ? { fontSize: '11px', lineHeight: '16px', paddingInline: 4, marginInlineEnd: 0 } : undefined}
    >
      {classification.displayName}
    </Tag>
  );
};

export default ClassificationTag;
