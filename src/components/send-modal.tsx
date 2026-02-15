import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DeliveryConfirmation } from '../types';
import Toast from './toast';

interface SendModalProps {
  summaryId: number;
  onClose: () => void;
}

interface DeliveryConfigRow {
  id: number;
  deliveryType: string;
  config: Record<string, any>;
  isEnabled: boolean;
}

interface AvailableMethod {
  id: string;
  label: string;
  configured: boolean;
  detail: string;
  config?: Record<string, any>;
}

export default function SendModal({ summaryId, onClose }: SendModalProps) {
  const [selectedMethods, setSelectedMethods] = useState<Set<string>>(new Set());
  const [sending, setSending] = useState(false);
  const [results, setResults] = useState<DeliveryConfirmation[]>([]);
  const [toast, setToast] = useState<{ type: 'success' | 'error' | 'warning'; message: string } | null>(null);
  const [availableMethods, setAvailableMethods] = useState<AvailableMethod[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadDeliveryConfigs();
  }, []);

  const loadDeliveryConfigs = async () => {
    try {
      setLoading(true);
      const configs = await invoke<DeliveryConfigRow[]>('get_delivery_configs');

      const methods: AvailableMethod[] = [
        { id: 'email', label: 'Email', configured: false, detail: 'Not configured' },
        { id: 'slack', label: 'Slack', configured: false, detail: 'Not configured' },
        { id: 'file', label: 'File Export', configured: false, detail: 'Not configured' },
      ];

      configs.forEach((config) => {
        const method = methods.find((m) => m.id === config.deliveryType);
        if (method && config.isEnabled) {
          method.configured = true;
          method.config = config.config;

          if (config.deliveryType === 'email') {
            method.detail = `To: ${config.config.toAddress || 'Unknown'}`;
          } else if (config.deliveryType === 'slack') {
            method.detail = 'Webhook configured';
          } else if (config.deliveryType === 'file') {
            method.detail =
              config.config.directoryPath ||
              config.config.directory ||
              config.config.directory_path ||
              '~/Documents';
          }
        }
      });

      setAvailableMethods(methods);
    } catch (error) {
      console.error('Failed to load delivery configs:', error);
      setToast({ type: 'error', message: 'Failed to load delivery methods' });
    } finally {
      setLoading(false);
    }
  };

  const toggleMethod = (methodId: string) => {
    const newSelected = new Set(selectedMethods);
    if (newSelected.has(methodId)) {
      newSelected.delete(methodId);
    } else {
      newSelected.add(methodId);
    }
    setSelectedMethods(newSelected);
  };

  const handleSend = async () => {
    if (selectedMethods.size === 0) {
      setToast({ type: 'warning', message: 'Please select at least one delivery method' });
      return;
    }

    setSending(true);
    setResults([]);

    try {
      // Build delivery configs from the actual configured methods
      const configs = Array.from(selectedMethods)
        .map((methodId) => {
          const method = availableMethods.find((m) => m.id === methodId);
          if (!method || !method.config) return null;

          return {
            deliveryType: methodId,
            config: method.config,
            isEnabled: true,
          };
        })
        .filter((c) => c !== null);

      const confirmations = await invoke<DeliveryConfirmation[]>('send_summary', {
        summaryId,
        deliveryConfigs: configs,
      });

      setResults(confirmations);

      if (confirmations.length === 0) {
        setToast({
          type: 'error',
          message: 'No deliveries were executed. Check your delivery configuration.',
        });
        return;
      }

      const successCount = confirmations.filter((c) => c.success).length;
      const failCount = confirmations.length - successCount;

      if (failCount === 0) {
        setToast({ type: 'success', message: `Sent successfully to ${successCount} destination(s)!` });
      } else if (successCount === 0) {
        setToast({ type: 'error', message: 'All deliveries failed' });
      } else {
        setToast({
          type: 'warning',
          message: `${successCount} succeeded, ${failCount} failed`,
        });
      }
    } catch (error) {
      console.error('Send failed:', error);
      setToast({ type: 'error', message: 'Failed to send summary' });
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
      <div className="w-full max-w-md rounded-lg bg-white p-6 shadow-xl">
        <h2 className="mb-4 text-xl font-bold text-gray-900">Send Summary</h2>

        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="text-gray-600">Loading delivery methods...</div>
          </div>
        ) : results.length === 0 ? (
          <>
            <p className="mb-4 text-sm text-gray-600">
              Select delivery methods for this summary:
            </p>

            <div className="space-y-3">
              {availableMethods.map((method) => (
                <label
                  key={method.id}
                  className={`flex items-center justify-between rounded-lg border p-3 ${
                    method.configured
                      ? 'cursor-pointer border-gray-300 hover:bg-gray-50'
                      : 'border-gray-200 bg-gray-50 opacity-50'
                  }`}
                >
                  <div className="flex items-center">
                    <input
                      type="checkbox"
                      checked={selectedMethods.has(method.id)}
                      onChange={() => toggleMethod(method.id)}
                      disabled={!method.configured}
                      className="mr-3 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 disabled:opacity-50"
                    />
                    <div>
                      <div className="font-medium text-gray-900">{method.label}</div>
                      <div className="text-xs text-gray-500">
                        {method.configured ? method.detail : 'Not configured — Set up in Settings'}
                      </div>
                    </div>
                  </div>
                </label>
              ))}
            </div>

            <div className="mt-6 flex justify-end space-x-3">
              <button
                onClick={onClose}
                className="rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
              >
                Cancel
              </button>
              <button
                onClick={handleSend}
                disabled={sending || selectedMethods.size === 0}
                className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
              >
                {sending ? 'Sending...' : 'Send Now'}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="mb-4 space-y-2">
              {results.map((result, idx) => (
                <div
                  key={idx}
                  className={`rounded-lg border p-3 ${
                    result.success
                      ? 'border-green-200 bg-green-50'
                      : 'border-red-200 bg-red-50'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-medium capitalize">{result.deliveryType}</span>
                    <span className={result.success ? 'text-green-700' : 'text-red-700'}>
                      {result.success ? '✓' : '✗'}
                    </span>
                  </div>
                  <div className="mt-1 text-sm text-gray-600">{result.message}</div>
                </div>
              ))}
            </div>

            <div className="flex justify-end">
              <button
                onClick={onClose}
                className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
              >
                Close
              </button>
            </div>
          </>
        )}

        {toast && (
          <Toast type={toast.type} message={toast.message} onClose={() => setToast(null)} />
        )}
      </div>
    </div>
  );
}
