import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';
import { 
  SearchState, 
  FilterBuilderState, 
  SearchQuery, 
  AssetSearchResult, 
  MetadataFilter, 
  FilterPreset, 
  SearchSuggestion,
  FilterableField,
  SearchAnalytics,
  SortField
} from '../types/search';

interface SearchStore extends SearchState, FilterBuilderState {
  // Search actions
  performSearch: (query: SearchQuery) => Promise<void>;
  performTextSearch: (text: string) => Promise<void>;
  updateQuery: (updates: Partial<SearchQuery>) => void;
  clearSearch: () => void;
  
  // Filter actions
  addFilter: (filter: MetadataFilter) => void;
  removeFilter: (index: number) => void;
  updateFilter: (index: number, filter: MetadataFilter) => void;
  clearFilters: () => void;
  
  // Preset actions
  loadPresets: () => Promise<void>;
  savePreset: (preset: Omit<FilterPreset, 'id' | 'created_by' | 'usage_count' | 'created_at'>) => Promise<void>;
  deletePreset: (presetId: number) => Promise<void>;
  applyPreset: (preset: FilterPreset) => void;
  
  // Suggestion actions
  loadSuggestions: (partialQuery: string) => Promise<void>;
  clearSuggestions: () => void;
  
  // Field actions
  loadAvailableFields: () => Promise<void>;
  
  // Advanced search features
  findSimilarAssets: (assetId: number, threshold?: number) => Promise<AssetSearchResult[]>;
  searchInHierarchy: (parentId: number, query: SearchQuery) => Promise<void>;
  
  // Analytics
  getSearchAnalytics: (startDate: string, endDate: string) => Promise<SearchAnalytics>;
  
  // UI state
  toggleAdvancedMode: () => void;
  setSearchError: (error: string | undefined) => void;
  setLoading: (loading: boolean) => void;
  
  // Pagination and sorting
  setCurrentPage: (page: number) => void;
  setSortBy: (sortBy: SortField) => void;
  setPageSize: (size: number) => void;
}

const DEFAULT_SEARCH_QUERY: SearchQuery = {
  text_query: undefined,
  filters: [],
  hierarchy_scope: undefined,
  sort_by: 'Relevance',
  limit: 50,
  offset: 0,
};

const useSearchStore = create<SearchStore>()(
  persist(
    (set, get) => ({
      // Search state
      query: DEFAULT_SEARCH_QUERY,
      results: [],
      loading: false,
      error: undefined,
      suggestions: [],
      totalResults: 0,
      searchTime: 0,
      
      // Filter state
      filters: [],
      availableFields: [],
      presets: [],
      isAdvancedMode: false,
      
      // Search actions
      performSearch: async (query: SearchQuery) => {
        const startTime = Date.now();
        set({ loading: true, error: undefined });
        
        try {
          const results = await invoke<AssetSearchResult[]>('search_assets_by_metadata', { query });
          const endTime = Date.now();
          
          set({
            query,
            results,
            totalResults: results.length,
            searchTime: endTime - startTime,
            loading: false,
          });
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : 'Search failed';
          set({
            loading: false,
            error: errorMessage,
            results: [],
            totalResults: 0,
          });
          throw error;
        }
      },
      
      performTextSearch: async (text: string) => {
        const currentQuery = get().query;
        const newQuery: SearchQuery = {
          ...currentQuery,
          text_query: text,
          offset: 0, // Reset pagination for new search
        };
        
        await get().performSearch(newQuery);
      },
      
      updateQuery: (updates: Partial<SearchQuery>) => {
        const currentQuery = get().query;
        const newQuery = { ...currentQuery, ...updates };
        set({ query: newQuery });
      },
      
      clearSearch: () => {
        set({
          query: DEFAULT_SEARCH_QUERY,
          results: [],
          totalResults: 0,
          searchTime: 0,
          error: undefined,
          suggestions: [],
        });
      },
      
      // Filter actions
      addFilter: (filter: MetadataFilter) => {
        const currentFilters = get().query.filters;
        const newFilters = [...currentFilters, filter];
        get().updateQuery({ filters: newFilters });
      },
      
      removeFilter: (index: number) => {
        const currentFilters = get().query.filters;
        const newFilters = currentFilters.filter((_, i) => i !== index);
        get().updateQuery({ filters: newFilters });
      },
      
      updateFilter: (index: number, filter: MetadataFilter) => {
        const currentFilters = get().query.filters;
        const newFilters = [...currentFilters];
        newFilters[index] = filter;
        get().updateQuery({ filters: newFilters });
      },
      
      clearFilters: () => {
        get().updateQuery({ filters: [] });
      },
      
      // Preset actions
      loadPresets: async () => {
        try {
          const presets = await invoke<FilterPreset[]>('get_filter_presets', {});
          set({ presets });
        } catch (error) {
          console.error('Failed to load filter presets:', error);
        }
      },
      
      savePreset: async (presetData: Omit<FilterPreset, 'id' | 'created_by' | 'usage_count' | 'created_at'>) => {
        try {
          const preset = await invoke<FilterPreset>('create_metadata_filter_preset', { 
            preset: {
              ...presetData,
              filters: get().query.filters,
            }
          });
          
          const currentPresets = get().presets;
          set({ presets: [...currentPresets, preset] });
        } catch (error) {
          console.error('Failed to save filter preset:', error);
          throw error;
        }
      },
      
      deletePreset: async (presetId: number) => {
        try {
          await invoke('delete_filter_preset', { presetId });
          const currentPresets = get().presets;
          set({ presets: currentPresets.filter(p => p.id !== presetId) });
        } catch (error) {
          console.error('Failed to delete filter preset:', error);
          throw error;
        }
      },
      
      applyPreset: (preset: FilterPreset) => {
        get().updateQuery({ filters: preset.filters });
      },
      
      // Suggestion actions
      loadSuggestions: async (partialQuery: string) => {
        if (partialQuery.length < 2) {
          set({ suggestions: [] });
          return;
        }
        
        try {
          const suggestions = await invoke<SearchSuggestion[]>('get_metadata_search_suggestions', {
            partialQuery,
            limit: 10,
          });
          set({ suggestions });
        } catch (error) {
          console.error('Failed to load search suggestions:', error);
          set({ suggestions: [] });
        }
      },
      
      clearSuggestions: () => {
        set({ suggestions: [] });
      },
      
      // Field actions
      loadAvailableFields: async () => {
        try {
          const fields = await invoke<FilterableField[]>('get_filterable_metadata_fields');
          set({ availableFields: fields });
        } catch (error) {
          console.error('Failed to load available fields:', error);
        }
      },
      
      // Advanced search features
      findSimilarAssets: async (assetId: number, threshold: number = 0.7) => {
        try {
          const similarAssets = await invoke<AssetSearchResult[]>('find_similar_assets', {
            assetId,
            similarityThreshold: threshold,
          });
          return similarAssets;
        } catch (error) {
          console.error('Failed to find similar assets:', error);
          throw error;
        }
      },
      
      searchInHierarchy: async (parentId: number, query: SearchQuery) => {
        const hierarchyQuery = {
          ...query,
          hierarchy_scope: parentId,
        };
        await get().performSearch(hierarchyQuery);
      },
      
      // Analytics
      getSearchAnalytics: async (startDate: string, endDate: string) => {
        try {
          const analytics = await invoke<SearchAnalytics>('get_search_analytics', {
            startDate,
            endDate,
          });
          return analytics;
        } catch (error) {
          console.error('Failed to get search analytics:', error);
          throw error;
        }
      },
      
      // UI state
      toggleAdvancedMode: () => {
        set({ isAdvancedMode: !get().isAdvancedMode });
      },
      
      setSearchError: (error: string | undefined) => {
        set({ error });
      },
      
      setLoading: (loading: boolean) => {
        set({ loading });
      },
      
      // Pagination and sorting
      setCurrentPage: (page: number) => {
        const currentQuery = get().query;
        const limit = currentQuery.limit || 50;
        const offset = (page - 1) * limit;
        get().updateQuery({ offset });
      },
      
      setSortBy: (sortBy: SortField) => {
        get().updateQuery({ sort_by: sortBy });
      },
      
      setPageSize: (size: number) => {
        get().updateQuery({ 
          limit: size,
          offset: 0, // Reset to first page when changing page size
        });
      },
    }),
    {
      name: 'search-store',
      // Only persist certain parts of the state
      partialize: (state) => ({
        isAdvancedMode: state.isAdvancedMode,
        query: {
          sort_by: state.query.sort_by,
          limit: state.query.limit,
        },
      }),
    }
  )
);

// Additional utility functions for search operations
export const searchUtils = {
  // Create a default filter for a given field
  createDefaultFilter: (fieldName: string, fieldType: string): MetadataFilter => ({
    field_name: fieldName,
    field_type: fieldType,
    operator: fieldType === 'number' ? 'Equals' : 'Contains',
    value: '',
    logic_operator: 'And',
  }),
  
  // Validate a search query
  validateQuery: (query: SearchQuery): string[] => {
    const errors: string[] = [];
    
    if (!query.text_query && query.filters.length === 0) {
      errors.push('Search query or filters must be provided');
    }
    
    // Validate filters
    query.filters.forEach((filter, index) => {
      if (!filter.field_name) {
        errors.push(`Filter ${index + 1}: Field name is required`);
      }
      if (filter.value === undefined || filter.value === '') {
        if (!['IsNull', 'IsNotNull'].includes(filter.operator)) {
          errors.push(`Filter ${index + 1}: Value is required for ${filter.operator} operator`);
        }
      }
    });
    
    return errors;
  },
  
  // Build a human-readable query description
  buildQueryDescription: (query: SearchQuery): string => {
    const parts: string[] = [];
    
    if (query.text_query) {
      parts.push(`Text: "${query.text_query}"`);
    }
    
    if (query.filters.length > 0) {
      const filterDescriptions = query.filters.map((filter, index) => {
        const logicPrefix = index === 0 ? '' : ` ${filter.logic_operator.toUpperCase()} `;
        return `${logicPrefix}${filter.field_name} ${filter.operator} ${filter.value}`;
      });
      parts.push(`Filters: ${filterDescriptions.join('')}`);
    }
    
    if (query.hierarchy_scope) {
      parts.push(`Scope: Asset ${query.hierarchy_scope}`);
    }
    
    return parts.join(', ') || 'Empty search';
  },
  
  // Export search results
  exportResults: (results: AssetSearchResult[], format: 'csv' | 'json' = 'csv'): string => {
    if (format === 'json') {
      return JSON.stringify(results, null, 2);
    }
    
    // CSV export
    const headers = ['Asset ID', 'Asset Name', 'Asset Type', 'Hierarchy Path', 'Relevance Score', 'Last Updated'];
    const rows = results.map(result => [
      result.asset_id.toString(),
      result.asset_name,
      result.asset_type,
      result.hierarchy_path.join(' > '),
      result.relevance_score.toFixed(2),
      result.last_updated,
    ]);
    
    const csvContent = [headers, ...rows]
      .map(row => row.map(cell => `"${cell}"`).join(','))
      .join('\n');
    
    return csvContent;
  },
};

export default useSearchStore;