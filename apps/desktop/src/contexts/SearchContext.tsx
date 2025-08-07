import React, { createContext, useContext, useReducer, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { message } from 'antd';
import {
  SearchContextType,
  SearchState,
  FilterBuilderState,
  SearchQuery,
  MetadataFilter,
  AssetSearchResult,
  SearchSuggestion,
  FilterPreset,
  FilterableField
} from '../types/search';
import useAuthStore from '../store/auth';

interface SearchContextProviderProps {
  children: React.ReactNode;
}

// Search reducer actions
type SearchAction =
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_QUERY'; payload: Partial<SearchQuery> }
  | { type: 'SET_RESULTS'; payload: { results: AssetSearchResult[]; searchTime: number } }
  | { type: 'SET_ERROR'; payload: string }
  | { type: 'CLEAR_ERROR' }
  | { type: 'SET_SUGGESTIONS'; payload: SearchSuggestion[] }
  | { type: 'ADD_FILTER'; payload: MetadataFilter }
  | { type: 'REMOVE_FILTER'; payload: number }
  | { type: 'UPDATE_FILTER'; payload: { index: number; filter: MetadataFilter } }
  | { type: 'CLEAR_FILTERS' }
  | { type: 'SET_AVAILABLE_FIELDS'; payload: FilterableField[] }
  | { type: 'SET_PRESETS'; payload: FilterPreset[] }
  | { type: 'TOGGLE_ADVANCED_MODE'; payload: boolean };

// Initial search state
const initialSearchState: SearchState = {
  query: {
    text_query: '',
    filters: [],
    hierarchy_scope: undefined,
    sort_by: 'Relevance',
    limit: 50,
    offset: 0,
  },
  results: [],
  loading: false,
  error: undefined,
  suggestions: [],
  totalResults: 0,
  searchTime: 0,
};

// Initial filter builder state
const initialFilterState: FilterBuilderState = {
  filters: [],
  availableFields: [],
  presets: [],
  isAdvancedMode: false,
};

// Combined initial state
const initialState = {
  searchState: initialSearchState,
  filterState: initialFilterState,
};

// Search reducer
function searchReducer(state: typeof initialState, action: SearchAction): typeof initialState {
  switch (action.type) {
    case 'SET_LOADING':
      return {
        ...state,
        searchState: {
          ...state.searchState,
          loading: action.payload,
        },
      };

    case 'SET_QUERY':
      return {
        ...state,
        searchState: {
          ...state.searchState,
          query: {
            ...state.searchState.query,
            ...action.payload,
          },
        },
      };

    case 'SET_RESULTS':
      return {
        ...state,
        searchState: {
          ...state.searchState,
          results: action.payload.results,
          totalResults: action.payload.results.length,
          searchTime: action.payload.searchTime,
          loading: false,
          error: undefined,
        },
      };

    case 'SET_ERROR':
      return {
        ...state,
        searchState: {
          ...state.searchState,
          error: action.payload,
          loading: false,
        },
      };

    case 'CLEAR_ERROR':
      return {
        ...state,
        searchState: {
          ...state.searchState,
          error: undefined,
        },
      };

    case 'SET_SUGGESTIONS':
      return {
        ...state,
        searchState: {
          ...state.searchState,
          suggestions: action.payload,
        },
      };

    case 'ADD_FILTER':
      const newFilters = [...state.filterState.filters, action.payload];
      return {
        ...state,
        filterState: {
          ...state.filterState,
          filters: newFilters,
        },
        searchState: {
          ...state.searchState,
          query: {
            ...state.searchState.query,
            filters: newFilters,
          },
        },
      };

    case 'REMOVE_FILTER':
      const filteredFilters = state.filterState.filters.filter((_, index) => index !== action.payload);
      return {
        ...state,
        filterState: {
          ...state.filterState,
          filters: filteredFilters,
        },
        searchState: {
          ...state.searchState,
          query: {
            ...state.searchState.query,
            filters: filteredFilters,
          },
        },
      };

    case 'UPDATE_FILTER':
      const updatedFilters = state.filterState.filters.map((filter, index) =>
        index === action.payload.index ? action.payload.filter : filter
      );
      return {
        ...state,
        filterState: {
          ...state.filterState,
          filters: updatedFilters,
        },
        searchState: {
          ...state.searchState,
          query: {
            ...state.searchState.query,
            filters: updatedFilters,
          },
        },
      };

    case 'CLEAR_FILTERS':
      return {
        ...state,
        filterState: {
          ...state.filterState,
          filters: [],
        },
        searchState: {
          ...state.searchState,
          query: {
            ...state.searchState.query,
            filters: [],
          },
        },
      };

    case 'SET_AVAILABLE_FIELDS':
      return {
        ...state,
        filterState: {
          ...state.filterState,
          availableFields: action.payload,
        },
      };

    case 'SET_PRESETS':
      return {
        ...state,
        filterState: {
          ...state.filterState,
          presets: action.payload,
        },
      };

    case 'TOGGLE_ADVANCED_MODE':
      return {
        ...state,
        filterState: {
          ...state.filterState,
          isAdvancedMode: action.payload,
        },
      };

    default:
      return state;
  }
}

// Create the context with default value
const SearchContext = createContext<SearchContextType | undefined>(undefined);

// Search context provider component
export function SearchContextProvider({ children }: SearchContextProviderProps) {
  const { token, user } = useAuthStore();
  const [state, dispatch] = useReducer(searchReducer, initialState);

  // Perform search with current query
  const performSearch = useCallback(async (query: SearchQuery) => {
    if (!token) {
      message.error('Authentication required for search');
      return;
    }

    dispatch({ type: 'SET_LOADING', payload: true });
    dispatch({ type: 'CLEAR_ERROR' });

    try {
      const startTime = performance.now();
      
      const results = await invoke<AssetSearchResult[]>('search_assets_by_metadata', {
        token,
        query,
      });

      const searchTime = performance.now() - startTime;

      dispatch({ 
        type: 'SET_RESULTS', 
        payload: { results, searchTime } 
      });

      console.log(`Search completed in ${searchTime.toFixed(2)}ms, found ${results.length} results`);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Search failed';
      dispatch({ type: 'SET_ERROR', payload: errorMessage });
      message.error(`Search failed: ${errorMessage}`);
    }
  }, [token]);

  // Update query parameters
  const updateQuery = useCallback((updates: Partial<SearchQuery>) => {
    dispatch({ type: 'SET_QUERY', payload: updates });
  }, []);

  // Add a new filter
  const addFilter = useCallback((filter: MetadataFilter) => {
    dispatch({ type: 'ADD_FILTER', payload: filter });
  }, []);

  // Remove a filter by index
  const removeFilter = useCallback((index: number) => {
    dispatch({ type: 'REMOVE_FILTER', payload: index });
  }, []);

  // Update a filter at specific index
  const updateFilter = useCallback((index: number, filter: MetadataFilter) => {
    dispatch({ type: 'UPDATE_FILTER', payload: { index, filter } });
  }, []);

  // Clear all filters
  const clearFilters = useCallback(() => {
    dispatch({ type: 'CLEAR_FILTERS' });
  }, []);

  // Load filter presets for current user
  const loadPresets = useCallback(async () => {
    if (!token) return;

    try {
      const presets = await invoke<FilterPreset[]>('get_metadata_filter_presets', {
        token,
      });

      dispatch({ type: 'SET_PRESETS', payload: presets });
    } catch (error) {
      console.error('Failed to load filter presets:', error);
    }
  }, [token]);

  // Save a new filter preset
  const savePreset = useCallback(async (preset: Omit<FilterPreset, 'id' | 'created_by' | 'usage_count' | 'created_at'>) => {
    if (!token) {
      message.error('Authentication required to save presets');
      return;
    }

    try {
      const newPreset: FilterPreset = {
        ...preset,
        created_by: user?.id || 0,
        usage_count: 0,
        created_at: new Date().toISOString(),
      };

      const createdPreset = await invoke<FilterPreset>('create_metadata_filter_preset', {
        token,
        preset: newPreset,
      });

      // Reload presets to include the new one
      await loadPresets();
      
      message.success(`Filter preset "${createdPreset.name}" saved successfully`);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to save preset';
      message.error(`Failed to save preset: ${errorMessage}`);
    }
  }, [token, user?.id, loadPresets]);

  // Load search suggestions for auto-complete
  const loadSuggestions = useCallback(async (partialQuery: string) => {
    if (!token || !partialQuery.trim()) {
      dispatch({ type: 'SET_SUGGESTIONS', payload: [] });
      return;
    }

    try {
      const suggestions = await invoke<SearchSuggestion[]>('get_metadata_search_suggestions', {
        token,
        partialQuery,
        limit: 10,
      });

      dispatch({ type: 'SET_SUGGESTIONS', payload: suggestions });
    } catch (error) {
      console.error('Failed to load search suggestions:', error);
      dispatch({ type: 'SET_SUGGESTIONS', payload: [] });
    }
  }, [token]);

  // Find similar assets to a given asset
  const findSimilarAssets = useCallback(async (assetId: number, threshold: number = 0.5): Promise<AssetSearchResult[]> => {
    if (!token) {
      message.error('Authentication required to find similar assets');
      return [];
    }

    try {
      const similarAssets = await invoke<AssetSearchResult[]>('find_similar_assets', {
        token,
        assetId,
        similarityThreshold: threshold,
      });

      message.success(`Found ${similarAssets.length} similar assets`);
      return similarAssets;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to find similar assets';
      message.error(`Failed to find similar assets: ${errorMessage}`);
      return [];
    }
  }, [token]);

  // Load available filterable fields on mount
  React.useEffect(() => {
    if (token) {
      const loadFilterableFields = async () => {
        try {
          const fields = await invoke<FilterableField[]>('get_filterable_metadata_fields', {
            token,
          });

          dispatch({ type: 'SET_AVAILABLE_FIELDS', payload: fields });
        } catch (error) {
          console.error('Failed to load filterable fields:', error);
        }
      };

      loadFilterableFields();
      loadPresets();
    }
  }, [token, loadPresets]);

  const contextValue: SearchContextType = {
    searchState: state.searchState,
    filterState: state.filterState,
    performSearch,
    updateQuery,
    addFilter,
    removeFilter,
    updateFilter,
    clearFilters,
    loadPresets,
    savePreset,
    loadSuggestions,
    findSimilarAssets,
  };

  return (
    <SearchContext.Provider value={contextValue}>
      {children}
    </SearchContext.Provider>
  );
}

// Hook to use search context
export function useSearch(): SearchContextType {
  const context = useContext(SearchContext);
  if (context === undefined) {
    throw new Error('useSearch must be used within a SearchContextProvider');
  }
  return context;
}

export default SearchContext;