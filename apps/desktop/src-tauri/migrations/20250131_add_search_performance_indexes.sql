-- Migration: Add performance indexes for metadata search
-- Version: 20250131_add_search_performance_indexes
-- Description: Add specialized indexes to improve metadata search performance

-- Index for metadata field names for faster field-based queries
CREATE INDEX IF NOT EXISTS idx_asset_metadata_field_names ON asset_metadata (
    asset_id,
    (json_extract(metadata_values_json, '$.')) 
) WHERE json_valid(metadata_values_json) = 1;

-- Index for metadata update times for cache invalidation
CREATE INDEX IF NOT EXISTS idx_asset_metadata_updated_at_asset ON asset_metadata (updated_at, asset_id);

-- Composite index for search analytics queries
CREATE INDEX IF NOT EXISTS idx_search_analytics_user_date ON search_analytics (user_id, executed_at DESC);

-- Index for search analytics query text for most common searches
CREATE INDEX IF NOT EXISTS idx_search_analytics_query_text ON search_analytics (query_text, executed_at DESC) 
WHERE query_text IS NOT NULL AND query_text != '';

-- Index for filter presets usage tracking
CREATE INDEX IF NOT EXISTS idx_filter_presets_usage_updated ON metadata_filter_presets (usage_count DESC, created_at DESC);

-- Partial index for active metadata (non-empty values)
CREATE INDEX IF NOT EXISTS idx_asset_metadata_non_empty ON asset_metadata (asset_id, schema_id, updated_at) 
WHERE length(metadata_values_json) > 2;

-- Index for asset hierarchy searches
CREATE INDEX IF NOT EXISTS idx_assets_hierarchy_search ON assets (parent_id, asset_type, name);

-- Index for metadata schema filtering
CREATE INDEX IF NOT EXISTS idx_metadata_schemas_active ON asset_metadata_schemas (asset_type_filter, is_system_template, created_at DESC);

-- Analyze tables for query planner optimization
ANALYZE asset_metadata;
ANALYZE asset_metadata_fts;
ANALYZE search_analytics;
ANALYZE metadata_filter_presets;
ANALYZE assets;
ANALYZE asset_metadata_schemas;