// TypeScript interfaces matching Rust structs (camelCase for Tauri serde)

interface Ticket {
  id: string;
  title: string;
  status: string;
  url: string;
  resolvedAt?: string;
}

interface Meeting {
  title: string;
  start: string;
  end: string;
  durationMinutes: number;
}

type SourceStatus = 'Ok' | 'Failed' | 'NotConfigured';

interface DataSourcesStatus {
  jira: SourceStatusDetail;
  calendar: SourceStatusDetail;
  toggl: SourceStatusDetail;
}

interface SourceStatusDetail {
  status: SourceStatus;
  fetchedAt?: string;
  error?: string;
}

export interface SummaryResponse {
  id: number;
  summaryDate: string;
  ticketsClosed: Ticket[];
  ticketsInProgress: Ticket[];
  meetings: Meeting[];
  focusHours: number;
  blockers: string;
  tomorrowPriorities: string;
  manualNotes: string;
  narrative: string;
  tone: string;
  deliveredTo: string[];
  createdAt: string;
  updatedAt: string;
  sourcesStatus: DataSourcesStatus;
}

export interface SummaryInput {
  blockers?: string;
  tomorrowPriorities?: string;
  manualNotes?: string;
  narrative?: string;
  tone?: string;
}

export interface SummaryMeta {
  id: number;
  summaryDate: string;
  narrativeSnippet: string;
  deliveredTo: string[];
}

export interface DeliveryConfirmation {
  deliveryType: string;
  success: boolean;
  message: string;
  timestamp: string;
}

export interface Settings {
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
