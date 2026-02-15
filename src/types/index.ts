// TypeScript interfaces matching Rust structs (camelCase for Tauri serde)

export interface Ticket {
  id: string;
  title: string;
  status: string;
  url: string;
  resolvedAt?: string;
}

export interface Meeting {
  title: string;
  start: string;
  end: string;
  durationMinutes: number;
}

export enum SourceStatus {
  Ok = 'Ok',
  Failed = 'Failed',
  NotConfigured = 'NotConfigured',
}

export interface DataSourcesStatus {
  jira: SourceStatusDetail;
  calendar: SourceStatusDetail;
  toggl: SourceStatusDetail;
}

export interface SourceStatusDetail {
  status: SourceStatus;
  fetchedAt?: string;
  error?: string;
}

export interface AggregatedData {
  ticketsClosed: Ticket[];
  ticketsInProgress: Ticket[];
  meetings: Meeting[];
  focusHours: number;
  dataSourcesStatus: DataSourcesStatus;
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
  jiraBaseUrl?: string;
  jiraProjectKey?: string;
  togglWorkspaceId?: string;
}

export interface DeliveryConfig {
  id?: number;
  deliveryType: string;
  config: Record<string, unknown>;
  isEnabled: boolean;
}

export interface AppError {
  error: string;
  details?: string;
}
