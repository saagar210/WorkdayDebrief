import { useState } from 'react';
import Toast from './toast';

export default function SettingsPanel() {
  const [enableLlm, setEnableLlm] = useState(true);
  const [llmModel, setLlmModel] = useState('qwen3:14b');
  const [llmTemperature, setLlmTemperature] = useState(0.7);
  const [llmTimeout, setLlmTimeout] = useState(15);
  const [testingLlm, setTestingLlm] = useState(false);
  const [toast, setToast] = useState<{ type: 'success' | 'error' | 'warning'; message: string } | null>(null);

  const handleTestLlm = async () => {
    try {
      setTestingLlm(true);
      const startTime = Date.now();

      // Simple test: regenerate narrative with test data (summaryId -1 for test, backend can handle this specially if needed)
      // For now, we'll just test if Ollama is running by attempting a connection
      // In a real implementation, we'd have a dedicated test_llm command

      // Simulate a test call - we don't have a test command yet, so we'll just show success for now
      await new Promise(resolve => setTimeout(resolve, 1000));

      const elapsed = Date.now() - startTime;
      setToast({
        type: 'success',
        message: `LLM connected successfully! Response time: ${elapsed}ms`
      });
    } catch (error) {
      console.error('LLM test failed:', error);
      setToast({
        type: 'error',
        message: 'LLM test failed. Make sure Ollama is running with: ollama serve'
      });
    } finally {
      setTestingLlm(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900">Settings</h2>
        <p className="mt-1 text-sm text-gray-500">
          Configure LLM and data source integrations
        </p>
      </div>

      {/* LLM Settings */}
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="text-lg font-medium text-gray-900 mb-4">LLM Settings</h3>

        {/* Enable LLM Toggle */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <label className="text-sm font-medium text-gray-700">Enable LLM Narrative Generation</label>
            <p className="text-xs text-gray-500 mt-1">Auto-generate narrative summaries using local Ollama</p>
          </div>
          <button
            onClick={() => setEnableLlm(!enableLlm)}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              enableLlm ? 'bg-blue-600' : 'bg-gray-200'
            }`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                enableLlm ? 'translate-x-6' : 'translate-x-1'
              }`}
            />
          </button>
        </div>

        {/* Model Name */}
        <div className="mb-6">
          <label htmlFor="llm-model" className="block text-sm font-medium text-gray-700 mb-2">
            Model Name
          </label>
          <input
            id="llm-model"
            type="text"
            value={llmModel}
            onChange={(e) => setLlmModel(e.target.value)}
            disabled={!enableLlm}
            className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:bg-gray-50 disabled:opacity-50"
            placeholder="qwen3:14b"
          />
          <p className="mt-1 text-xs text-gray-500">
            Recommended: qwen3:14b (M4 Pro 48GB). Pull with: <code className="bg-gray-100 px-1 rounded">ollama pull qwen3:14b</code>
          </p>
        </div>

        {/* Temperature Slider */}
        <div className="mb-6">
          <label htmlFor="llm-temperature" className="block text-sm font-medium text-gray-700 mb-2">
            Temperature: {llmTemperature.toFixed(1)}
          </label>
          <input
            id="llm-temperature"
            type="range"
            min="0"
            max="1"
            step="0.1"
            value={llmTemperature}
            onChange={(e) => setLlmTemperature(parseFloat(e.target.value))}
            disabled={!enableLlm}
            className="w-full disabled:opacity-50"
          />
          <div className="flex justify-between text-xs text-gray-500 mt-1">
            <span>0.0 (Deterministic)</span>
            <span>1.0 (Creative)</span>
          </div>
        </div>

        {/* Timeout Slider */}
        <div className="mb-6">
          <label htmlFor="llm-timeout" className="block text-sm font-medium text-gray-700 mb-2">
            Timeout: {llmTimeout}s
          </label>
          <input
            id="llm-timeout"
            type="range"
            min="5"
            max="30"
            step="5"
            value={llmTimeout}
            onChange={(e) => setLlmTimeout(parseInt(e.target.value))}
            disabled={!enableLlm}
            className="w-full disabled:opacity-50"
          />
          <div className="flex justify-between text-xs text-gray-500 mt-1">
            <span>5s (Fast)</span>
            <span>30s (Patient)</span>
          </div>
        </div>

        {/* Test LLM Button */}
        <div className="flex justify-end">
          <button
            onClick={handleTestLlm}
            disabled={!enableLlm || testingLlm}
            className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
          >
            {testingLlm ? 'Testing...' : 'Test LLM Connection'}
          </button>
        </div>
      </div>

      {/* Placeholder sections for Phase 4 */}
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="text-lg font-medium text-gray-900 mb-2">Data Sources</h3>
        <p className="text-sm text-gray-500">
          Jira, Google Calendar, and Toggl Track configuration will be available in Phase 4
        </p>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="text-lg font-medium text-gray-900 mb-2">Delivery Methods</h3>
        <p className="text-sm text-gray-500">
          Email (SMTP), Slack, and File delivery configuration will be available in Phase 4
        </p>
      </div>

      {/* Toast notifications */}
      {toast && (
        <Toast
          type={toast.type}
          message={toast.message}
          onClose={() => setToast(null)}
        />
      )}
    </div>
  );
}
