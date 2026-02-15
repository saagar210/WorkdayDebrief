import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SummaryResponse } from '../types';

export default function TomorrowPriorities() {
  const [priorities, setPriorities] = useState<string>('');
  const [jiraBaseUrl, setJiraBaseUrl] = useState<string>('');
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    loadYesterdayPriorities();
    loadJiraUrl();

    // Check if dismissed in this session
    const isDismissed = sessionStorage.getItem('priorities-dismissed');
    if (isDismissed) {
      setDismissed(true);
    }
  }, []);

  const loadYesterdayPriorities = async () => {
    try {
      // Calculate yesterday's date
      const yesterday = new Date();
      yesterday.setDate(yesterday.getDate() - 1);
      const year = yesterday.getFullYear();
      const month = String(yesterday.getMonth() + 1).padStart(2, '0');
      const day = String(yesterday.getDate()).padStart(2, '0');
      const dateStr = `${year}-${month}-${day}`;

      const summary = await invoke<SummaryResponse | null>('get_summary_by_date', {
        date: dateStr,
      });

      if (summary && summary.tomorrowPriorities) {
        setPriorities(summary.tomorrowPriorities);
      }
    } catch (error) {
      console.error('Failed to load yesterday priorities:', error);
    }
  };

  const loadJiraUrl = async () => {
    try {
      const settings = await invoke<any>('get_settings');
      if (settings.jiraBaseUrl) {
        setJiraBaseUrl(settings.jiraBaseUrl);
      }
    } catch (error) {
      console.error('Failed to load Jira URL from settings:', error);
    }
  };

  const handleDismiss = () => {
    setDismissed(true);
    sessionStorage.setItem('priorities-dismissed', 'true');
  };

  // Extract Jira ticket IDs (e.g., PROJ-123, ABC-456)
  const renderPrioritiesWithLinks = () => {
    const splitPattern = /([A-Z]+-\d+)/g;
    const matchPattern = /^[A-Z]+-\d+$/;
    const parts = priorities.split('\n').map((line, lineIdx) => {
      const parts = line.split(splitPattern);
      return (
        <div key={lineIdx} className="mb-1">
          {parts.map((part, partIdx) => {
            if (matchPattern.test(part)) {
              return (
                <a
                  key={partIdx}
                  href={`${jiraBaseUrl}/browse/${part}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-medium text-blue-600 hover:text-blue-800 hover:underline"
                >
                  {part}
                </a>
              );
            }
            return <span key={partIdx}>{part}</span>;
          })}
        </div>
      );
    });

    return parts;
  };

  if (!priorities || dismissed) {
    return null;
  }

  return (
    <div className="mb-6 rounded-lg border border-blue-200 bg-blue-50 p-4 shadow-sm">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <h3 className="mb-2 text-sm font-medium text-blue-900">
            🎯 Yesterday's Priorities (Today's To-Do)
          </h3>
          <div className="text-sm text-blue-800">{renderPrioritiesWithLinks()}</div>
        </div>
        <button
          onClick={handleDismiss}
          className="ml-4 text-blue-600 hover:text-blue-800 focus:outline-none"
          aria-label="Dismiss"
        >
          <svg
            className="h-5 w-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
