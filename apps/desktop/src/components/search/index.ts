// Export all search components
export { default as AdvancedSearchInterface } from './AdvancedSearchInterface';
export { default as FilterPanel } from './FilterPanel';
export { default as SearchResultsList } from './SearchResultsList';
export { default as SavedSearchManager } from './SavedSearchManager';

// Export search context and hooks
export { SearchContextProvider, useSearch } from '../../contexts/SearchContext';

// Export search store
export { default as useSearchStore, searchUtils } from '../../store/search';

// Export types
export type {
  SearchState,
  FilterBuilderState,
  SearchQuery,
  AssetSearchResult,
  MetadataFilter,
  FilterPreset,
  SearchSuggestion,
  FilterableField,
  SearchContextType,
  SearchBarProps,
  SearchResultsProps,
  FilterBuilderProps,
  SearchAnalyticsProps,
} from '../../types/search';