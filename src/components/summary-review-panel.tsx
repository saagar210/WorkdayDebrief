import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SummaryResponse, SummaryInput } from '../types';
import Toast from './toast';
import SendModal from './send-modal';
import TomorrowPriorities from './tomorrow-priorities';

export default function SummaryReviewPanel() {
  const [summary, setSummary] = useState<SummaryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [regeneratingNarrative, setRegeneratingNarrative] = useState(false);
  const [toast, setToast] = useState<{ type: 'success' | 'error' | 'warning'; message: string } | null>(null);

  // Form fields
  const [blockers, setBlockers] = useState('');
  const [tomorrowPriorities, setTomorrowPriorities] = useState('');
  const [manualNotes, setManualNotes] = useState('');
  const [narrative, setNarrative] = useState('');
  const [tone, setTone] = useState<'professional' | 'casual' | 'detailed'>('professional');
  const [narrativeEditable, setNarrativeEditable] = useState(false);
  const [llmSlowWarningShown, setLlmSlowWarningShown] = useState(false);
  const [showSendModal, setShowSendModal] = useState(false);

  // Load today's summary on mount
  useEffect(() => {
    loadTodaySummary();
  }, []);

  const loadTodaySummary = async () => {
    try {
      setLoading(true);
      const data = await invoke<SummaryResponse | null>('get_today_summary');
      if (data) {
        setSummary(data);
        setBlockers(data.blockers || '');
        setTomorrowPriorities(data.tomorrowPriorities || '');
        setManualNotes(data.manualNotes || '');
        setNarrative(data.narrative || '');
      }
    } catch (error) {
      console.error('Failed to load summary:', error);
      setToast({ type: 'error', message: 'Failed to load summary' });
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    try {
      setSaving(true);
      const input: SummaryInput = {
        blockers: blockers || undefined,
        tomorrowPriorities: tomorrowPriorities || undefined,
        manualNotes: manualNotes || undefined,
        narrative: narrative || undefined,
      };

      const savedSummary = await invoke<SummaryResponse>('save_summary', { input });
      setSummary(savedSummary);
      setToast({ type: 'success', message: 'Summary saved successfully!' });
    } catch (error) {
      console.error('Failed to save summary:', error);
      setToast({ type: 'error', message: 'Failed to save summary' });
    } finally {
      setSaving(false);
    }
  };

  const regenerateNarrative = async (summaryId: number, selectedTone: string) => {
    try {
      setRegeneratingNarrative(true);
      setLlmSlowWarningShown(false);

      // Show "Still working..." message after 8 seconds
      const slowWarningTimeout = setTimeout(() => {
        setLlmSlowWarningShown(true);
      }, 8000);

      const newNarrative = await invoke<string>('regenerate_narrative', {
        summaryId,
        tone: selectedTone,
      });

      clearTimeout(slowWarningTimeout);
      setNarrative(newNarrative);
      setNarrativeEditable(false);
      setToast({ type: 'success', message: 'Narrative generated successfully!' });
    } catch (error) {
      console.error('Failed to regenerate narrative:', error);
      setToast({
        type: 'warning',
        message: 'LLM unavailable - using bullet list fallback. Click Retry to try again.',
      });
    } finally {
      setRegeneratingNarrative(false);
      setLlmSlowWarningShown(false);
    }
  };

  const handleGenerate = async () => {
    try {
      setGenerating(true);
      const generatedSummary = await invoke<SummaryResponse>('generate_summary');
      setSummary(generatedSummary);
      setBlockers(generatedSummary.blockers || '');
      setTomorrowPriorities(generatedSummary.tomorrowPriorities || '');
      setManualNotes(generatedSummary.manualNotes || '');
      setNarrative(generatedSummary.narrative || '');

      // Show data source status warnings
      const status = generatedSummary.sourcesStatus;
      if (status) {
        const warnings: string[] = [];
        if (status.jira?.status === 'Failed') warnings.push('Jira');
        if (status.calendar?.status === 'Failed') warnings.push('Calendar');
        if (status.toggl?.status === 'Failed') warnings.push('Toggl');

        if (warnings.length > 0) {
          setToast({
            type: 'warning',
            message: `Generated with partial data. ${warnings.join(', ')} unavailable.`
          });
        } else {
          setToast({ type: 'success', message: 'Summary data generated successfully!' });
        }
      }

      // Auto-generate narrative after aggregation completes
      if (generatedSummary.id) {
        await regenerateNarrative(generatedSummary.id, tone);
      }
    } catch (error) {
      console.error('Failed to generate summary:', error);
      setToast({ type: 'error', message: 'Failed to generate summary' });
    } finally {
      setGenerating(false);
    }
  };

  const handleToneChange = async (newTone: 'professional' | 'casual' | 'detailed') => {
    setTone(newTone);
    if (summary?.id) {
      await regenerateNarrative(summary.id, newTone);
    }
  };

  const handleClearAndRegenerate = async () => {
    if (!summary?.id) return;

    const confirmed = confirm('Discard manual edits and regenerate narrative from LLM?');
    if (confirmed) {
      await regenerateNarrative(summary.id, tone);
    }
  };

  const handleExportMarkdown = () => {
    const markdown = generateMarkdown();
    navigator.clipboard.writeText(markdown);
    setToast({ type: 'success', message: 'Markdown copied to clipboard!' });
  };

  const generateMarkdown = (): string => {
    const date = summary?.summaryDate || new Date().toISOString().split('T')[0];
    const inProgressLines = summary?.ticketsInProgress?.length
      ? summary.ticketsInProgress.map((ticket) => `- ${ticket.id}: ${ticket.title}`).join('\n')
      : '(None)';
    const sections = [
      `# Work Summary — ${date}`,
      '',
      '## Narrative',
      narrative || '(No narrative)',
      '',
      '## In Progress',
      inProgressLines,
      '',
      '## Blockers',
      blockers || '(None)',
      '',
      '## Tomorrow\'s Priorities',
      tomorrowPriorities || '(None)',
      '',
      '## Notes',
      manualNotes || '(None)',
    ];
    return sections.join('\n');
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-gray-600">Loading summary...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Yesterday's Priorities Widget */}
      <TomorrowPriorities />

      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">
            Work Summary for {summary?.summaryDate || 'Today'}
          </h2>
          {summary && (
            <p className="mt-1 text-sm text-gray-500">
              Last updated: {new Date(summary.updatedAt).toLocaleString()}
            </p>
          )}
        </div>
      </div>

      {/* Narrative with tone control and LLM generation */}
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex items-center justify-between mb-4">
          <label className="block text-sm font-medium text-gray-700">
            Narrative Summary
          </label>
          <div className="flex items-center gap-2">
            {/* Tone selector */}
            <select
              value={tone}
              onChange={(e) => handleToneChange(e.target.value as 'professional' | 'casual' | 'detailed')}
              disabled={regeneratingNarrative}
              className="rounded-md border border-gray-300 px-3 py-1 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:opacity-50"
            >
              <option value="professional">Professional</option>
              <option value="casual">Casual</option>
              <option value="detailed">Detailed</option>
            </select>

            {/* Edit/Save toggle */}
            <button
              onClick={() => setNarrativeEditable(!narrativeEditable)}
              className="rounded-md border border-gray-300 bg-white px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
            >
              {narrativeEditable ? 'Lock' : 'Edit'}
            </button>

            {/* Clear & Regenerate */}
            {narrativeEditable && (
              <button
                onClick={handleClearAndRegenerate}
                disabled={regeneratingNarrative || !summary?.id}
                className="rounded-md border border-orange-300 bg-white px-3 py-1 text-sm font-medium text-orange-700 hover:bg-orange-50 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2 disabled:opacity-50"
              >
                Clear & Regenerate
              </button>
            )}
          </div>
        </div>

        {/* Generation status */}
        {regeneratingNarrative && (
          <div className="mb-2 text-sm text-gray-600">
            {llmSlowWarningShown ? (
              <span className="text-orange-600">Still working... (large model)</span>
            ) : (
              <span>Generating narrative...</span>
            )}
          </div>
        )}

        <textarea
          value={narrative}
          onChange={(e) => setNarrative(e.target.value)}
          readOnly={!narrativeEditable}
          disabled={regeneratingNarrative}
          className={`mt-2 w-full rounded-md border px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 ${
            narrativeEditable ? 'border-gray-300 bg-white' : 'border-gray-200 bg-gray-50 cursor-default'
          } disabled:opacity-50`}
          rows={6}
          placeholder="Click 'Generate Summary' to auto-generate narrative, or click 'Edit' to write manually."
        />

        {/* Retry button on error (shown via toast, but user can also retry manually) */}
        {summary?.id && !regeneratingNarrative && (
          <div className="mt-2 flex justify-end">
            <button
              onClick={() => regenerateNarrative(summary.id, tone)}
              className="text-sm text-blue-600 hover:text-blue-800 focus:outline-none"
            >
              Retry Generation
            </button>
          </div>
        )}
      </div>

      {/* User-editable fields */}
      <div className="space-y-4 rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        {/* Blockers */}
        <div>
          <label htmlFor="blockers" className="block text-sm font-medium text-gray-700">
            Blockers
          </label>
          <textarea
            id="blockers"
            value={blockers}
            onChange={(e) => setBlockers(e.target.value)}
            className="mt-2 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            rows={3}
            placeholder="What's blocking you? e.g., Waiting on Network team for VPN logs"
          />
        </div>

        {/* Tomorrow's Priorities */}
        <div>
          <label htmlFor="priorities" className="block text-sm font-medium text-gray-700">
            Tomorrow's Priorities
          </label>
          <textarea
            id="priorities"
            value={tomorrowPriorities}
            onChange={(e) => setTomorrowPriorities(e.target.value)}
            className="mt-2 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            rows={3}
            placeholder="What will you work on tomorrow? e.g., Follow up on JIRA-1234, complete Okta testing"
          />
        </div>

        {/* Manual Notes */}
        <div>
          <label htmlFor="notes" className="block text-sm font-medium text-gray-700">
            Additional Notes
          </label>
          <textarea
            id="notes"
            value={manualNotes}
            onChange={(e) => setManualNotes(e.target.value)}
            className="mt-2 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            rows={3}
            placeholder="Any additional context or notes"
          />
        </div>
      </div>

      {/* Data display section - show aggregated data if available */}
      {summary && (summary.ticketsClosed.length > 0 || summary.ticketsInProgress.length > 0 || summary.meetings.length > 0 || summary.focusHours > 0) && (
        <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
          <h3 className="mb-4 text-lg font-medium text-gray-900">Aggregated Data</h3>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="font-medium text-gray-700">Tickets Closed:</span>{' '}
              <span className="text-gray-900">{summary.ticketsClosed.length}</span>
            </div>
            <div>
              <span className="font-medium text-gray-700">Tickets In Progress:</span>{' '}
              <span className="text-gray-900">{summary.ticketsInProgress.length}</span>
            </div>
            <div>
              <span className="font-medium text-gray-700">Meetings:</span>{' '}
              <span className="text-gray-900">{summary.meetings.length}</span>
            </div>
            <div>
              <span className="font-medium text-gray-700">Focus Hours:</span>{' '}
              <span className="text-gray-900">{summary.focusHours.toFixed(1)}h</span>
            </div>
          </div>

          {/* Source status badges */}
          <div className="mt-4 flex gap-2">
            {summary.sourcesStatus?.jira && (
              <span className={`rounded px-2 py-1 text-xs ${
                summary.sourcesStatus.jira.status === 'Ok' ? 'bg-green-100 text-green-800' :
                summary.sourcesStatus.jira.status === 'Failed' ? 'bg-red-100 text-red-800' :
                'bg-gray-100 text-gray-600'
              }`}>
                Jira: {summary.sourcesStatus.jira.status === 'NotConfigured' ? 'Not configured' : summary.sourcesStatus.jira.status}
              </span>
            )}
            {summary.sourcesStatus?.calendar && (
              <span className={`rounded px-2 py-1 text-xs ${
                summary.sourcesStatus.calendar.status === 'Ok' ? 'bg-green-100 text-green-800' :
                summary.sourcesStatus.calendar.status === 'Failed' ? 'bg-red-100 text-red-800' :
                'bg-gray-100 text-gray-600'
              }`}>
                Calendar: {summary.sourcesStatus.calendar.status === 'NotConfigured' ? 'Not configured' : summary.sourcesStatus.calendar.status}
              </span>
            )}
            {summary.sourcesStatus?.toggl && (
              <span className={`rounded px-2 py-1 text-xs ${
                summary.sourcesStatus.toggl.status === 'Ok' ? 'bg-green-100 text-green-800' :
                summary.sourcesStatus.toggl.status === 'Failed' ? 'bg-red-100 text-red-800' :
                'bg-gray-100 text-gray-600'
              }`}>
                Toggl: {summary.sourcesStatus.toggl.status === 'NotConfigured' ? 'Not configured' : summary.sourcesStatus.toggl.status}
              </span>
            )}
          </div>
        </div>
      )}

      {/* Action Buttons */}
      <div className="flex space-x-4">
        <button
          onClick={handleGenerate}
          disabled={generating}
          className="rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 disabled:opacity-50"
        >
          {generating ? 'Generating...' : 'Generate Summary'}
        </button>
        <button
          onClick={handleSave}
          disabled={saving}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save Draft'}
        </button>
        <button
          onClick={() => setShowSendModal(true)}
          disabled={!summary?.id}
          className="rounded-md bg-purple-600 px-4 py-2 text-sm font-medium text-white hover:bg-purple-700 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 disabled:opacity-50"
        >
          Send
        </button>
        <button
          onClick={handleExportMarkdown}
          className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
        >
          Export as Markdown
        </button>
      </div>

      {/* Toast notifications */}
      {toast && (
        <Toast
          type={toast.type}
          message={toast.message}
          onClose={() => setToast(null)}
        />
      )}

      {/* Send Modal */}
      {showSendModal && summary?.id && (
        <SendModal summaryId={summary.id} onClose={() => setShowSendModal(false)} />
      )}
    </div>
  );
}
