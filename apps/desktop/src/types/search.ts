// Search-related types for metadata search and filtering functionality

export interface AssetSearchResult {
  asset_id: number;
  asset_name: string;
  asset_type: string;
  hierarchy_path: string[];
  metadata_matches: MetadataMatch[];
  relevance_score: number;
  last_updated: string;
}

export interface MetadataMatch {
  field_name: string;
  field_value: string;
  highlighted_value: string;
  schema_name: string;
  match_type: MatchType;
}

export type MatchType = 'ExactMatch' | 'PartialMatch' | 'FuzzyMatch' | 'FieldName';

export type FilterOperator = 
  | 'Equals'
  | 'NotEquals'
  | 'Contains'
  | 'StartsWith'
  | 'EndsWith'
  | 'GreaterThan'
  | 'LessThan'
  | 'GreaterThanOrEqual'
  | 'LessThanOrEqual'
  | 'IsNull'
  | 'IsNotNull'
  | 'InRange'
  | 'Regex';

export type LogicOperator = 'And' | 'Or' | 'Not';

export interface MetadataFilter {
  field_name: string;
  field_type: string; // FieldType from metadata types
  operator: FilterOperator;
  value: any;
  logic_operator: LogicOperator;
}

export type SortField = 'Relevance' | 'AssetName' | 'LastUpdated' | 'CreatedDate' | 'AssetType';

export interface SearchQuery {
  text_query?: string;
  filters: MetadataFilter[];
  hierarchy_scope?: number;
  sort_by?: SortField;
  limit?: number;
  offset?: number;
}

export type SuggestionType = 'FieldName' | 'FieldValue' | 'AssetName' | 'SchemaName' | 'RecentSearch';

export interface SearchSuggestion {
  text: string;
  suggestion_type: SuggestionType;
  field_name?: string;
  description: string;
  usage_count: number;
}

export interface FilterPreset {
  id?: number;
  name: string;
  description: string;
  filters: MetadataFilter[];
  created_by: number;
  usage_count: number;
  created_at: string;
}

export interface SearchAnalytics {
  total_searches: number;
  average_response_time_ms: number;
  most_common_searches: string[];
  performance_metrics: { [key: string]: number };
  period_start: string;
  period_end: string;
}

export interface FilterableField {
  field_name: string;
  field_type: string;
  schema_name: string;
  usage_count: number;
  sample_values: string[];
}

// Search state interface for component state management
export interface SearchState {
  query: SearchQuery;
  results: AssetSearchResult[];
  loading: boolean;
  error?: string;
  suggestions: SearchSuggestion[];
  totalResults: number;
  searchTime: number;
}

// Filter builder state for advanced filtering UI
export interface FilterBuilderState {
  filters: MetadataFilter[];
  availableFields: FilterableField[];
  presets: FilterPreset[];
  isAdvancedMode: boolean;
}

// Search context for sharing search state across components  
export interface SearchContextType {
  searchState: SearchState;
  filterState: FilterBuilderState;
  performSearch: (query: SearchQuery) => Promise<void>;
  updateQuery: (updates: Partial<SearchQuery>) => void;
  addFilter: (filter: MetadataFilter) => void;
  removeFilter: (index: number) => void;
  updateFilter: (index: number, filter: MetadataFilter) => void;
  clearFilters: () => void;
  loadPresets: () => Promise<void>;
  savePreset: (preset: Omit<FilterPreset, 'id' | 'created_by' | 'usage_count' | 'created_at'>) => Promise<void>;
  loadSuggestions: (partialQuery: string) => Promise<void>;
  findSimilarAssets: (assetId: number, threshold?: number) => Promise<AssetSearchResult[]>;
}

// UI Component props interfaces
export interface SearchBarProps {
  placeholder?: string;
  onSearch: (query: string) => void;
  onSuggestionSelect?: (suggestion: SearchSuggestion) => void;
  loading?: boolean;
  showAdvancedButton?: boolean;
  onAdvancedClick?: () => void;
  className?: string;
}

export interface SearchResultsProps {
  results: AssetSearchResult[];
  loading?: boolean;
  error?: string;
  totalResults?: number;
  searchTime?: number;
  onAssetClick?: (assetId: number) => void;
  onFindSimilarAssets?: (assetId: number) => void;
  showMetadataDetails?: boolean;
  className?: string;
}

export interface FilterBuilderProps {
  filters: MetadataFilter[];
  availableFields: FilterableField[];
  onFiltersChange: (filters: MetadataFilter[]) => void;
  presets?: FilterPreset[];
  onPresetSave?: (preset: Omit<FilterPreset, 'id' | 'created_by' | 'usage_count' | 'created_at'>) => void;
  onPresetLoad?: (preset: FilterPreset) => void;
  className?: string;
}

export interface SearchAnalyticsProps {
  analytics: SearchAnalytics;
  loading?: boolean;
  error?: string;
  className?: string;
}

// Utility types for search operations
export interface SearchOperation {
  type: 'text' | 'filter' | 'hierarchy' | 'similar';
  parameters: any;
  timestamp: string;
}

export interface SearchHistory {
  id: string;
  query: SearchQuery;
  results_count: number;
  executed_at: string;
  execution_time_ms: number;
}

// Integration types for asset management workflows
export interface AssetSearchIntegration {
  searchInCreation: boolean;
  searchInEditing: boolean;
  suggestSimilarAssets: boolean;
  showMetadataContext: boolean;
  enableBulkOperations: boolean;
}

export interface BulkSearchOperation {
  operation_type: 'update' | 'delete' | 'move' | 'tag';
  target_assets: number[];
  parameters: any;
  confirmation_required: boolean;
}