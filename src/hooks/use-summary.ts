import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { SummaryResponse } from '../types';

export function useTodaySummary() {
  return useQuery<SummaryResponse | null>({
    queryKey: ['summary', 'today'],
    queryFn: async () => {
      try {
        return await invoke<SummaryResponse | null>('get_today_summary');
      } catch (error) {
        console.error('Failed to fetch today summary:', error);
        throw error;
      }
    },
  });
}
