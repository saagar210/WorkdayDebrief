import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SummaryMeta, SummaryResponse } from '../types';

export default function HistoricalBrowser() {
  const [summaries, setSummaries] = useState<SummaryMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedSummary, setSelectedSummary] = useState<SummaryResponse | null>(null);
  const [selectedDate, setSelectedDate] = useState<string | null>(null);

  useEffect(() => {
    loadSummaries();
  }, []);

  const loadSummaries = async () => {
    try {
      setLoading(true);
      const data = await invoke<SummaryMeta[]>('list_summaries', { daysBack: 30 });
      setSummaries(data);
    } catch (error) {
      console.error('Failed to load summaries:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSelectSummary = async (date: string) => {
    try {
      setSelectedDate(date);
      const summary = await invoke<SummaryResponse | null>('get_summary_by_date', { date });
      setSelectedSummary(summary);
    } catch (error) {
      console.error('Failed to load summary:', error);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-gray-600">Loading summaries...</div>
      </div>
    );
  }

  if (summaries.length === 0) {
    return (
      <div className="rounded-lg border border-gray-200 bg-white p-12 text-center shadow-sm">
        <h3 className="text-lg font-medium text-gray-900">No summaries yet</h3>
        <p className="mt-2 text-sm text-gray-500">
          Switch to the Summary tab to create your first summary.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-3 gap-6">
      {/* Left: List of summaries */}
      <div className="col-span-1 space-y-2">
        <h3 className="text-lg font-semibold text-gray-900">Past 30 Days</h3>
        <div className="space-y-2">
          {summaries.map((summary) => (
            <button
              key={summary.id}
              onClick={() => handleSelectSummary(summary.summaryDate)}
              className={`w-full rounded-lg border p-3 text-left transition-colors ${
                selectedDate === summary.summaryDate
                  ? 'border-blue-500 bg-blue-50'
                  : 'border-gray-200 bg-white hover:bg-gray-50'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium text-gray-900">{summary.summaryDate}</span>
                {summary.deliveredTo.length > 0 && (
                  <span className="rounded bg-green-100 px-2 py-1 text-xs text-green-800">
                    Sent
                  </span>
                )}
              </div>
              {summary.narrativeSnippet && (
                <p className="mt-1 line-clamp-2 text-sm text-gray-600">
                  {summary.narrativeSnippet}
                </p>
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Right: Selected summary detail */}
      <div className="col-span-2">
        {selectedSummary ? (
          <div className="space-y-4 rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
            <div>
              <h2 className="text-2xl font-bold text-gray-900">
                Summary for {selectedSummary.summaryDate}
              </h2>
              <p className="mt-1 text-sm text-gray-500">
                Last updated: {new Date(selectedSummary.updatedAt).toLocaleString()}
              </p>
            </div>

            {selectedSummary.narrative && (
              <div>
                <h3 className="font-medium text-gray-900">Narrative</h3>
                <p className="mt-2 whitespace-pre-wrap text-gray-700">
                  {selectedSummary.narrative}
                </p>
              </div>
            )}

            {selectedSummary.blockers && (
              <div>
                <h3 className="font-medium text-gray-900">Blockers</h3>
                <p className="mt-2 whitespace-pre-wrap text-gray-700">
                  {selectedSummary.blockers}
                </p>
              </div>
            )}

            {selectedSummary.tomorrowPriorities && (
              <div>
                <h3 className="font-medium text-gray-900">Tomorrow's Priorities</h3>
                <p className="mt-2 whitespace-pre-wrap text-gray-700">
                  {selectedSummary.tomorrowPriorities}
                </p>
              </div>
            )}

            {selectedSummary.manualNotes && (
              <div>
                <h3 className="font-medium text-gray-900">Notes</h3>
                <p className="mt-2 whitespace-pre-wrap text-gray-700">
                  {selectedSummary.manualNotes}
                </p>
              </div>
            )}

            {selectedSummary.deliveredTo.length > 0 && (
              <div>
                <h3 className="font-medium text-gray-900">Delivered To</h3>
                <div className="mt-2 flex gap-2">
                  {selectedSummary.deliveredTo.map((target) => (
                    <span
                      key={target}
                      className="rounded-full bg-green-100 px-3 py-1 text-sm text-green-800"
                    >
                      {target}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="flex h-full items-center justify-center rounded-lg border border-gray-200 bg-white p-12 shadow-sm">
            <p className="text-gray-500">Select a summary to view details</p>
          </div>
        )}
      </div>
    </div>
  );
}
