import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import Toast from './toast';
import Card from './ui/card';
import Button from './ui/button';
import Input from './ui/input';
import Toggle from './ui/toggle';
import Badge from './ui/badge';
import { InfoTooltip } from './ui/tooltip';

type TabId = 'llm' | 'data-sources' | 'delivery' | 'schedule';

interface Settings {
  scheduledTime: string;
  defaultTone: string;
  enableLlm: boolean;
  llmModel: string;
  llmTemperature: number;
  llmTimeoutSecs: number;
  calendarSource: string;
  retentionDays: number;
  jiraBaseUrl: string | null;
  jiraProjectKey: string | null;
  togglWorkspaceId: string | null;
}

export default function SettingsPanelV2() {
  const [activeTab, setActiveTab] = useState<TabId>('llm');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<{ type: 'success' | 'error' | 'warning'; message: string } | null>(null);

  // Form state
  const [scheduledTime, setScheduledTime] = useState('17:00');
  const [defaultTone, setDefaultTone] = useState('professional');
  const [enableLlm, setEnableLlm] = useState(true);
  const [llmModel, setLlmModel] = useState('qwen3:14b');
  const [llmTemperature, setLlmTemperature] = useState(0.7);
  const [llmTimeout, setLlmTimeout] = useState(15);
  const [jiraBaseUrl, setJiraBaseUrl] = useState('');
  const [jiraProjectKey, setJiraProjectKey] = useState('');
  const [jiraEmail, setJiraEmail] = useState('');
  const [jiraApiToken, setJiraApiToken] = useState('');
  const [togglWorkspaceId, setTogglWorkspaceId] = useState('');
  const [togglApiToken, setTogglApiToken] = useState('');
  const [retentionDays, setRetentionDays] = useState(90);

  // Delivery config state
  const [smtpHost, setSmtpHost] = useState('');
  const [smtpPort, setSmtpPort] = useState('587');
  const [smtpFromAddress, setSmtpFromAddress] = useState('');
  const [smtpToAddress, setSmtpToAddress] = useState('');
  const [smtpUsername, setSmtpUsername] = useState('');
  const [smtpPassword, setSmtpPassword] = useState('');
  const [smtpUseTls, setSmtpUseTls] = useState(true);
  const [slackWebhookUrl, setSlackWebhookUrl] = useState('');
  const [fileDirectory, setFileDirectory] = useState('');
  const [testingDelivery, setTestingDelivery] = useState<string | null>(null);

  // Google Calendar OAuth state
  const [googleConnected, setGoogleConnected] = useState(false);
  const [connectingGoogle, setConnectingGoogle] = useState(false);

  // Connection testing state
  const [testingJira, setTestingJira] = useState(false);
  const [testingToggl, setTestingToggl] = useState(false);

  // Validation errors
  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    loadSettings();
  }, []);

  // Validation helpers
  const validateEmail = (email: string): boolean => {
    const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return regex.test(email);
  };

  const validateUrl = (url: string): boolean => {
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const validateTime = (time: string): boolean => {
    const parts = time.split(':');
    if (parts.length !== 2) return false;
    const hour = parseInt(parts[0]);
    const minute = parseInt(parts[1]);
    return !isNaN(hour) && !isNaN(minute) && hour >= 0 && hour <= 23 && minute >= 0 && minute <= 59;
  };

  const validateSettings = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!validateTime(scheduledTime)) {
      newErrors.scheduledTime = 'Invalid time format (use HH:MM)';
    }

    if (llmTemperature < 0 || llmTemperature > 1) {
      newErrors.llmTemperature = 'Temperature must be between 0 and 1';
    }

    if (llmTimeout < 5 || llmTimeout > 30) {
      newErrors.llmTimeout = 'Timeout must be between 5 and 30 seconds';
    }

    if (retentionDays < 7 || retentionDays > 365) {
      newErrors.retentionDays = 'Retention must be between 7 and 365 days';
    }

    if (jiraBaseUrl && !validateUrl(jiraBaseUrl)) {
      newErrors.jiraBaseUrl = 'Invalid URL format';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleConnectGoogle = async () => {
    try {
      setConnectingGoogle(true);

      // Set up event listeners for OAuth completion
      const { listen } = await import('@tauri-apps/api/event');

      const unlisten_success = await listen<string>('oauth-completed', (event) => {
        console.log('OAuth completed:', event.payload);
        setGoogleConnected(true);
        setConnectingGoogle(false);
        setToast({ type: 'success', message: event.payload });
        unlisten_success();
      });

      const unlisten_error = await listen<string>('oauth-error', (event) => {
        console.error('OAuth error:', event.payload);
        setConnectingGoogle(false);
        setToast({ type: 'error', message: `OAuth failed: ${event.payload}` });
        unlisten_error();
      });

      // Start OAuth flow - this will open browser and start callback server
      await invoke<string>('start_google_oauth');

      setToast({
        type: 'success',
        message: 'Authorization opened in browser. Complete the flow to connect.'
      });
    } catch (error: any) {
      console.error('Google OAuth failed:', error);
      setToast({ type: 'error', message: error.toString() || 'Failed to connect Google Calendar' });
      setConnectingGoogle(false);
    }
  };

  const handleDisconnectGoogle = async () => {
    try {
      // Delete refresh token from stronghold
      await invoke('delete_secret', { key: 'google_refresh_token' });
      setGoogleConnected(false);
      setToast({ type: 'success', message: 'Google Calendar disconnected' });
    } catch (error: any) {
      console.error('Failed to disconnect:', error);
      setToast({ type: 'error', message: 'Failed to disconnect' });
    }
  };

  const handleTestJira = async () => {
    try {
      setTestingJira(true);
      const actualToken = jiraApiToken === '••••••'
        ? (await invoke<string | null>('get_secret', { key: 'jira_api_token' })) || ''
        : jiraApiToken;

      const result = await invoke<string>('test_jira_connection', {
        baseUrl: jiraBaseUrl,
        email: jiraEmail,
        apiToken: actualToken,
        projectKey: jiraProjectKey,
      });

      setToast({ type: 'success', message: result });
    } catch (error: any) {
      console.error('Jira test failed:', error);
      const message = error.toString().replace('Error: ', '');
      setToast({ type: 'error', message: `Jira test failed: ${message}` });
    } finally {
      setTestingJira(false);
    }
  };

  const handleTestToggl = async () => {
    try {
      setTestingToggl(true);
      const actualToken = togglApiToken === '••••••'
        ? (await invoke<string | null>('get_secret', { key: 'toggl_api_token' })) || ''
        : togglApiToken;

      const result = await invoke<string>('test_toggl_connection', {
        apiToken: actualToken,
        workspaceId: togglWorkspaceId,
      });

      setToast({ type: 'success', message: result });
    } catch (error: any) {
      console.error('Toggl test failed:', error);
      const message = error.toString().replace('Error: ', '');
      setToast({ type: 'error', message: `Toggl test failed: ${message}` });
    } finally {
      setTestingToggl(false);
    }
  };

  const validateDeliveryConfig = (deliveryType: string): boolean => {
    const newErrors: Record<string, string> = {};

    if (deliveryType === 'email') {
      if (!smtpHost) newErrors.smtpHost = 'Required';
      if (!smtpPort || isNaN(parseInt(smtpPort))) newErrors.smtpPort = 'Invalid port';
      if (!smtpFromAddress) {
        newErrors.smtpFromAddress = 'Required';
      } else if (!validateEmail(smtpFromAddress)) {
        newErrors.smtpFromAddress = 'Invalid email';
      }
      if (!smtpToAddress) {
        newErrors.smtpToAddress = 'Required';
      } else if (!validateEmail(smtpToAddress)) {
        newErrors.smtpToAddress = 'Invalid email';
      }
      if (!smtpUsername) newErrors.smtpUsername = 'Required';
      if (!smtpPassword) newErrors.smtpPassword = 'Required';
    } else if (deliveryType === 'slack') {
      if (!slackWebhookUrl) {
        newErrors.slackWebhookUrl = 'Required';
      } else if (!validateUrl(slackWebhookUrl) || !slackWebhookUrl.includes('hooks.slack.com')) {
        newErrors.slackWebhookUrl = 'Invalid Slack webhook URL';
      }
    } else if (deliveryType === 'file') {
      if (!fileDirectory) newErrors.fileDirectory = 'Required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const loadSettings = async () => {
    try {
      setLoading(true);
      const data = await invoke<Settings>('get_settings');

      // Populate form
      setScheduledTime(data.scheduledTime);
      setDefaultTone(data.defaultTone);
      setEnableLlm(data.enableLlm);
      setLlmModel(data.llmModel);
      setLlmTemperature(data.llmTemperature);
      setLlmTimeout(data.llmTimeoutSecs);
      setJiraBaseUrl(data.jiraBaseUrl || '');
      setJiraProjectKey(data.jiraProjectKey || '');
      setTogglWorkspaceId(data.togglWorkspaceId || '');
      setRetentionDays(data.retentionDays);

      // Load delivery configs
      await loadDeliveryConfigs();

      // Check Google Calendar connection
      const googleToken = await invoke<string | null>('get_secret', {
        key: 'google_refresh_token',
      });
      setGoogleConnected(googleToken !== null);

      // Load API tokens from encrypted storage
      const jiraEmailSecret = await invoke<string | null>('get_secret', {
        key: 'jira_email',
      });
      const jiraTokenSecret = await invoke<string | null>('get_secret', {
        key: 'jira_api_token',
      });
      const togglTokenSecret = await invoke<string | null>('get_secret', {
        key: 'toggl_api_token',
      });

      setJiraEmail(jiraEmailSecret || '');
      setJiraApiToken(jiraTokenSecret ? '••••••' : '');
      setTogglApiToken(togglTokenSecret ? '••••••' : '');
    } catch (error) {
      console.error('Failed to load settings:', error);
      setToast({ type: 'error', message: 'Failed to load settings' });
    } finally {
      setLoading(false);
    }
  };

  const loadDeliveryConfigs = async () => {
    try {
      const configs = await invoke<any[]>('get_delivery_configs');

      configs.forEach((config: any) => {
        if (config.deliveryType === 'email' && config.config) {
          setSmtpHost(config.config.host || '');
          setSmtpPort(config.config.port?.toString() || '587');
          setSmtpFromAddress(config.config.fromAddress || '');
          setSmtpToAddress(config.config.toAddress || '');
          setSmtpUsername(config.config.username || '');
          setSmtpPassword(config.config.password || '');
          setSmtpUseTls(config.config.useTls !== false);
        } else if (config.deliveryType === 'slack' && config.config) {
          setSlackWebhookUrl(config.config.webhookUrl || '');
        } else if (config.deliveryType === 'file' && config.config) {
          setFileDirectory(
            config.config.directoryPath ||
            config.config.directory ||
            config.config.directory_path ||
            ''
          );
        }
      });
    } catch (error) {
      console.error('Failed to load delivery configs:', error);
    }
  };

  const handleSave = async () => {
    if (!validateSettings()) {
      setToast({ type: 'error', message: 'Please fix validation errors before saving' });
      return;
    }

    try {
      setSaving(true);
      setErrors({});

      const updatedSettings: Settings = {
        scheduledTime,
        defaultTone,
        enableLlm,
        llmModel,
        llmTemperature,
        llmTimeoutSecs: llmTimeout,
        calendarSource: 'none',
        retentionDays,
        jiraBaseUrl: jiraBaseUrl || null,
        jiraProjectKey: jiraProjectKey || null,
        togglWorkspaceId: togglWorkspaceId || null,
      };

      await invoke('save_settings', { settings: updatedSettings });

      // Save API tokens to encrypted storage (if not masked)
      if (jiraEmail) {
        await invoke('store_secret', { key: 'jira_email', value: jiraEmail });
      }
      if (jiraApiToken && jiraApiToken !== '••••••') {
        await invoke('store_secret', { key: 'jira_api_token', value: jiraApiToken });
      }
      if (togglApiToken && togglApiToken !== '••••••') {
        await invoke('store_secret', { key: 'toggl_api_token', value: togglApiToken });
      }

      setToast({ type: 'success', message: 'Settings saved successfully!' });
      await loadSettings();
    } catch (error: any) {
      console.error('Failed to save settings:', error);
      const message = error.toString().replace('Error: ', '');
      setToast({ type: 'error', message });
    } finally {
      setSaving(false);
    }
  };

  const handleSaveDeliveryConfig = async (deliveryType: string) => {
    if (!validateDeliveryConfig(deliveryType)) {
      setToast({ type: 'error', message: 'Please fix validation errors before saving' });
      return;
    }

    try {
      setSaving(true);
      setErrors({});
      let config: any = {};

      if (deliveryType === 'email') {
        config = {
          host: smtpHost,
          port: parseInt(smtpPort),
          fromAddress: smtpFromAddress,
          toAddress: smtpToAddress,
          username: smtpUsername,
          password: smtpPassword,
          useTls: smtpUseTls,
        };
      } else if (deliveryType === 'slack') {
        config = { webhookUrl: slackWebhookUrl };
      } else if (deliveryType === 'file') {
        config = { directoryPath: fileDirectory };
      }

      await invoke('save_delivery_config', {
        input: {
          deliveryType,
          config,
          isEnabled: true,
        },
      });

      setToast({ type: 'success', message: `${deliveryType} configuration saved!` });
      await loadDeliveryConfigs();
    } catch (error: any) {
      console.error('Failed to save delivery config:', error);
      const message = error.toString().replace('Error: ', '');
      setToast({ type: 'error', message });
    } finally {
      setSaving(false);
    }
  };

  const handleTestDelivery = async (deliveryType: string) => {
    try {
      setTestingDelivery(deliveryType);
      let config: any = {};

      if (deliveryType === 'email') {
        config = {
          host: smtpHost,
          port: parseInt(smtpPort),
          fromAddress: smtpFromAddress,
          toAddress: smtpToAddress,
          username: smtpUsername,
          password: smtpPassword,
          useTls: smtpUseTls,
        };
      } else if (deliveryType === 'slack') {
        config = { webhookUrl: slackWebhookUrl };
      } else if (deliveryType === 'file') {
        config = { directoryPath: fileDirectory };
      }

      const testConfig = deliveryType === 'email'
        ? { type: 'email', ...config }
        : deliveryType === 'slack'
          ? { type: 'slack', ...config }
          : { type: 'file', ...config };

      const result = await invoke<string>('test_delivery', {
        deliveryType,
        config: testConfig,
      });

      setToast({ type: 'success', message: result });
    } catch (error: any) {
      console.error('Test delivery failed:', error);
      const message = error.toString().replace('Error: ', '');
      setToast({ type: 'error', message });
    } finally {
      setTestingDelivery(null);
    }
  };

  const tabs = [
    { id: 'llm' as TabId, label: 'LLM' },
    { id: 'data-sources' as TabId, label: 'Data Sources' },
    { id: 'delivery' as TabId, label: 'Delivery' },
    { id: 'schedule' as TabId, label: 'Schedule' },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-gray-600">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-2xl font-bold text-gray-900">Settings</h2>
        <p className="mt-1 text-sm text-gray-500">
          Configure LLM, data sources, delivery methods, and scheduling
        </p>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`
                whitespace-nowrap border-b-2 py-4 px-1 text-sm font-medium transition-colors
                ${
                  activeTab === tab.id
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'
                }
              `}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab Content */}
      <div className="min-h-[400px]">
        {activeTab === 'llm' && (
          <Card title="LLM Configuration">
            <div className="space-y-6">
              <Toggle
                enabled={enableLlm}
                onChange={setEnableLlm}
                label="Enable LLM Narrative Generation"
                description="Auto-generate narrative summaries using local Ollama"
              />

              <Input
                label="Model Name"
                value={llmModel}
                onChange={setLlmModel}
                placeholder="qwen3:14b"
                helpText="Recommended: qwen3:14b (M4 Pro 48GB). Install with: ollama pull qwen3:14b"
                disabled={!enableLlm}
              />

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Temperature: {llmTemperature.toFixed(1)}
                  <InfoTooltip content="Lower = more factual and consistent. Higher = more creative and varied." />
                </label>
                <input
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

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Timeout: {llmTimeout}s
                  <InfoTooltip content="How long to wait for LLM response before showing bullet-point fallback." />
                </label>
                <input
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
            </div>
          </Card>
        )}

        {activeTab === 'data-sources' && (
          <div className="space-y-6">
            {/* Jira Section */}
            <Card title="Jira">
              <div className="space-y-4">
                <Input
                  label="Base URL"
                  value={jiraBaseUrl}
                  onChange={setJiraBaseUrl}
                  type="url"
                  placeholder="https://company.atlassian.net"
                  helpText="Your Jira Cloud instance URL"
                  error={errors.jiraBaseUrl}
                />

                <Input
                  label="Project Key"
                  value={jiraProjectKey}
                  onChange={setJiraProjectKey}
                  placeholder="PROJ"
                  helpText="The project key for your tickets (e.g., PROJ in PROJ-123)"
                />

                <Input
                  label="Email"
                  value={jiraEmail}
                  onChange={setJiraEmail}
                  type="email"
                  placeholder="you@company.com"
                  helpText="Your Jira account email"
                />

                <Input
                  label="API Token"
                  value={jiraApiToken}
                  onChange={setJiraApiToken}
                  type="password"
                  placeholder="Enter API token"
                  helpText="Generate at: https://id.atlassian.com/manage-profile/security/api-tokens"
                />

                <div className="pt-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleTestJira}
                    loading={testingJira}
                    disabled={!jiraBaseUrl || !jiraEmail || !jiraApiToken || testingJira}
                  >
                    {testingJira ? 'Testing...' : 'Test Jira Connection'}
                  </Button>
                </div>
              </div>
            </Card>

            {/* Google Calendar Section */}
            <Card title="Google Calendar">
              <div className="space-y-4">
                <p className="text-sm text-gray-600">
                  Connect your Google Calendar to automatically include meeting data in summaries.
                </p>
                {googleConnected ? (
                  <>
                    <Badge variant="success">Connected</Badge>
                    <div className="pt-2">
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={handleDisconnectGoogle}
                      >
                        Disconnect
                      </Button>
                    </div>
                  </>
                ) : (
                  <>
                    <Badge variant="neutral">Not Connected</Badge>
                    <div className="pt-2">
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={handleConnectGoogle}
                        loading={connectingGoogle}
                        disabled={connectingGoogle}
                      >
                        {connectingGoogle ? 'Connecting...' : 'Connect Google Account'}
                      </Button>
                    </div>
                    <p className="text-xs text-gray-500">
                      Opens Google authorization in your browser
                    </p>
                  </>
                )}
              </div>
            </Card>

            {/* Toggl Section */}
            <Card title="Toggl Track">
              <div className="space-y-4">
                <Input
                  label="API Token"
                  value={togglApiToken}
                  onChange={setTogglApiToken}
                  type="password"
                  placeholder="Enter API token"
                  helpText="Find at: https://track.toggl.com/profile (bottom of page)"
                />

                <Input
                  label="Workspace ID"
                  value={togglWorkspaceId}
                  onChange={setTogglWorkspaceId}
                  placeholder="1234567"
                  helpText="Find this in Toggl Track settings under workspace"
                />

                <div className="pt-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleTestToggl}
                    loading={testingToggl}
                    disabled={!togglApiToken || !togglWorkspaceId || testingToggl}
                  >
                    {testingToggl ? 'Testing...' : 'Test Toggl Connection'}
                  </Button>
                </div>
              </div>
            </Card>
          </div>
        )}

        {activeTab === 'delivery' && (
          <div className="space-y-6">
            {/* Email (SMTP) Section */}
            <Card title="Email (SMTP)">
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <Input
                    label="SMTP Host"
                    value={smtpHost}
                    onChange={setSmtpHost}
                    type="url"
                    placeholder="smtp.gmail.com"
                    helpText="Your email provider's SMTP server"
                    error={errors.smtpHost}
                    required
                  />
                  <Input
                    label="Port"
                    value={smtpPort}
                    onChange={setSmtpPort}
                    type="number"
                    placeholder="587"
                    helpText="Usually 587 (TLS) or 465 (SSL)"
                    error={errors.smtpPort}
                    required
                  />
                </div>

                <Input
                  label="From Address"
                  value={smtpFromAddress}
                  onChange={setSmtpFromAddress}
                  type="email"
                  placeholder="workday-debrief@example.com"
                  helpText="Email address to send from"
                  error={errors.smtpFromAddress}
                  required
                />

                <Input
                  label="To Address"
                  value={smtpToAddress}
                  onChange={setSmtpToAddress}
                  type="email"
                  placeholder="you@example.com"
                  helpText="Where to send your summaries"
                  error={errors.smtpToAddress}
                  required
                />

                <Input
                  label="Username"
                  value={smtpUsername}
                  onChange={setSmtpUsername}
                  placeholder="Usually your email address"
                  error={errors.smtpUsername}
                  required
                />

                <Input
                  label="Password"
                  value={smtpPassword}
                  onChange={setSmtpPassword}
                  type="password"
                  placeholder="App password or SMTP password"
                  helpText="Use an app-specific password for Gmail/Outlook"
                  error={errors.smtpPassword}
                  required
                />

                <Toggle
                  enabled={smtpUseTls}
                  onChange={setSmtpUseTls}
                  label="Use TLS"
                  description="Enable TLS encryption (recommended)"
                />

                <div className="flex gap-2 pt-2">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handleSaveDeliveryConfig('email')}
                    loading={saving}
                    disabled={saving || !smtpHost || !smtpFromAddress || !smtpToAddress}
                  >
                    Save Email Config
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleTestDelivery('email')}
                    loading={testingDelivery === 'email'}
                    disabled={testingDelivery !== null || !smtpHost || !smtpFromAddress || !smtpToAddress}
                  >
                    Test Email
                  </Button>
                </div>
              </div>
            </Card>

            {/* Slack Section */}
            <Card title="Slack">
              <div className="space-y-4">
                <p className="text-sm text-gray-600">
                  Send summaries to a Slack channel via webhook
                </p>
                <Input
                  label="Webhook URL"
                  value={slackWebhookUrl}
                  onChange={setSlackWebhookUrl}
                  type="url"
                  placeholder="https://hooks.slack.com/services/..."
                  helpText="Create an Incoming Webhook in your Slack workspace"
                  error={errors.slackWebhookUrl}
                  required
                />

                <div className="flex gap-2 pt-2">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handleSaveDeliveryConfig('slack')}
                    loading={saving}
                    disabled={saving || !slackWebhookUrl}
                  >
                    Save Slack Config
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleTestDelivery('slack')}
                    loading={testingDelivery === 'slack'}
                    disabled={testingDelivery !== null || !slackWebhookUrl}
                  >
                    Test Slack
                  </Button>
                </div>
              </div>
            </Card>

            {/* File Export Section */}
            <Card title="File Export">
              <div className="space-y-4">
                <p className="text-sm text-gray-600">
                  Export summaries as markdown files to a local directory
                </p>
                <Input
                  label="Export Directory"
                  value={fileDirectory}
                  onChange={setFileDirectory}
                  placeholder="/Users/you/Documents/WorkdayDebriefs"
                  helpText="Directory where markdown files will be saved"
                  error={errors.fileDirectory}
                  required
                />

                <div className="flex gap-2 pt-2">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handleSaveDeliveryConfig('file')}
                    loading={saving}
                    disabled={saving || !fileDirectory}
                  >
                    Save File Config
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleTestDelivery('file')}
                    loading={testingDelivery === 'file'}
                    disabled={testingDelivery !== null || !fileDirectory}
                  >
                    Test File Export
                  </Button>
                </div>
              </div>
            </Card>
          </div>
        )}

        {activeTab === 'schedule' && (
          <Card title="Schedule & Preferences">
            <div className="space-y-6">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Daily Generation Time
                </label>
                <input
                  type="time"
                  value={scheduledTime}
                  onChange={(e) => setScheduledTime(e.target.value)}
                  className={`rounded-md border px-3 py-2 text-sm focus:outline-none focus:ring-1 ${
                    errors.scheduledTime
                      ? 'border-red-500 focus:border-red-500 focus:ring-red-500'
                      : 'border-gray-300 focus:border-blue-500 focus:ring-blue-500'
                  }`}
                />
                {errors.scheduledTime ? (
                  <p className="mt-1 text-sm text-red-600">{errors.scheduledTime}</p>
                ) : (
                  <p className="mt-1 text-sm text-gray-500">
                    Summary will generate daily at this time (24-hour format)
                  </p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Default Tone
                </label>
                <select
                  value={defaultTone}
                  onChange={(e) => setDefaultTone(e.target.value)}
                  className="rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                >
                  <option value="professional">Professional</option>
                  <option value="casual">Casual</option>
                  <option value="detailed">Detailed</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Retention Days: {retentionDays}
                </label>
                <input
                  type="range"
                  min="7"
                  max="365"
                  step="1"
                  value={retentionDays}
                  onChange={(e) => setRetentionDays(parseInt(e.target.value))}
                  className="w-full"
                />
                <div className="flex justify-between text-xs text-gray-500 mt-1">
                  <span>7 days</span>
                  <span>365 days (1 year)</span>
                </div>
                <p className="mt-1 text-sm text-gray-500">
                  Summaries older than {retentionDays} days will be automatically archived
                </p>
              </div>
            </div>
          </Card>
        )}
      </div>

      {/* Save Button */}
      <div className="flex justify-end">
        <Button
          onClick={handleSave}
          loading={saving}
          disabled={saving}
          variant="primary"
          size="md"
        >
          {saving ? 'Saving...' : 'Save Settings'}
        </Button>
      </div>

      {/* Toast */}
      {toast && (
        <Toast type={toast.type} message={toast.message} onClose={() => setToast(null)} />
      )}
    </div>
  );
}
