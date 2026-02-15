import { useState } from 'react';
import SummaryReviewPanel from './components/summary-review-panel';
import HistoricalBrowser from './components/historical-browser';
import SettingsPanelV2 from './components/settings-panel-v2';

type Tab = 'summary' | 'history' | 'settings';

export default function App() {
  const [activeTab, setActiveTab] = useState<Tab>('summary');

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Tab Navigation */}
      <div className="border-b border-gray-200 bg-white">
        <nav className="flex space-x-8 px-6" aria-label="Tabs">
          <button
            onClick={() => setActiveTab('summary')}
            className={`border-b-2 px-1 py-4 text-sm font-medium ${
              activeTab === 'summary'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'
            }`}
          >
            Summary
          </button>
          <button
            onClick={() => setActiveTab('history')}
            className={`border-b-2 px-1 py-4 text-sm font-medium ${
              activeTab === 'history'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'
            }`}
          >
            History
          </button>
          <button
            onClick={() => setActiveTab('settings')}
            className={`border-b-2 px-1 py-4 text-sm font-medium ${
              activeTab === 'settings'
                ? 'border-blue-500 text-blue-600'
                : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'
            }`}
          >
            Settings
          </button>
        </nav>
      </div>

      {/* Tab Content */}
      <main className="mx-auto max-w-7xl p-6">
        {activeTab === 'summary' && <SummaryReviewPanel />}
        {activeTab === 'history' && <HistoricalBrowser />}
        {activeTab === 'settings' && <SettingsPanelV2 />}
      </main>
    </div>
  );
}
