export interface Session {
  id: string;
  title: string;
}

export interface Message {
  id?: number | string;
  role: 'user' | 'assistant' | 'system';
  text: string;
  content?: string;
  time?: string;
  created_at?: string;
  session_id?: string;
  isStreaming?: boolean;
  type?: 'ai' | 'plugin' | 'error';
}

export interface Note {
  id: string;
  content: string;
}

export interface CustomApp {
  id: number;
  name: string;
  path: string;
  keywords: string;
}

export interface BuiltinApp {
  id: string;
  name: string;
  path: string;
  keywords: string;
  builtin: boolean;
}

export interface PluginWithState {
  id: string;
  name: string;
  description: string;
  category: string;
  enabled: boolean;
}

export interface ProviderModel {
  id: string;
  label: string;
}

export interface CustomProvider {
  id: string;
  name: string;
  baseUrl: string;
  apiKey?: string;
  models: ProviderModel[];
}

export interface ProviderDef {
  id: string;
  name: string;
  models: ProviderModel[];
  isCustom?: boolean;
  baseUrl?: string;
  apiKey?: string;
}

export interface WeatherResult {
  temperature: number;
  weather_code: number;
  condition: string;
}

export interface MemoryChunk {
  id: string;
  text: string;
  source: string;
  score?: number;
}

export interface AgentStatus {
  is_ready: boolean;
  project_path: string;
}

export interface AppSettings {
  aiMode: 'api' | 'local';
  selectedProvider: string;
  selectedModel: string;
  temperature: string;
  customInstructions: string;
  nickname: string;
  occupation: string;
  moreAboutYou: string;
  tavilyApiKey: string;
  customProviders?: string;
  ttsVoice: string;
  ttsEngine: 'cloud' | 'local';
  kokoroModel: string;
  autoSpeak: string;
  [key: string]: string | undefined;
}

export interface BrowserViewportBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface BrowserElementBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RegionInfo {
  region_type: string;
  label?: string;
  element_id?: string;
  bounding_box?: BrowserElementBounds;
  elements_count: number;
}

export interface HeadingInfo {
  level: number;
  text: string;
  id?: string;
}

export interface FormControlInfo {
  element_id: string;
  field_type: string;
  label?: string;
  placeholder?: string;
  required: boolean;
  disabled: boolean;
  is_password: boolean;
}

export interface FormInfo {
  id?: string;
  name?: string;
  action?: string;
  method?: string;
  controls: FormControlInfo[];
}

export interface LinkInfo {
  text: string;
  href: string;
  role?: string;
  visible: boolean;
  is_external: boolean;
}

export interface ViewportInfo {
  width: number;
  height: number;
  scroll_x: number;
  scroll_y: number;
  page_width: number;
  page_height: number;
}

export interface ElementInfo {
  id: string;
  tag: string;
  role?: string;
  accessible_name?: string;
  text: string;
  aria_label?: string;
  href?: string;
  input_type?: string;
  placeholder?: string;
  value_available?: boolean;
  disabled: boolean;
  checked?: boolean;
  selected?: boolean;
  visible: boolean;
  interactable?: boolean;
  is_password?: boolean;
  is_in_iframe?: boolean;
  parent_region?: string;
  bounding_box?: BrowserElementBounds;
}

export interface BrowserActionResult {
  success: boolean;
  action: string;
  tab_id: string;
  element_id?: string;
  page_changed: boolean;
  url_changed: boolean;
  resulting_url?: string;
  error?: string;
  error_code?: string;
}

export interface PageObservationSnapshot {
  tab_id: string;
  url: string;
  title: string;
  generation?: number;
  fingerprint?: string;
  viewport?: ViewportInfo;
  visible_text: string;
  selected_text?: string;
  regions?: RegionInfo[];
  headings?: HeadingInfo[];
  interactive_elements: ElementInfo[];
  forms?: FormInfo[];
  links?: LinkInfo[];
  timestamp: number;
}

export interface DownloadItemInfo {
  id: string;
  tab_id: string;
  url: string;
  suggested_filename: string;
  state: string;
  total_bytes?: number;
  timestamp: number;
}

export interface ScreenshotResult {
  tab_id: string;
  data_url: string;
  width: number;
  height: number;
}

export interface BrowserTabInfo {
  id: string;
  label: string;
  url: string;
  title: string;
  favicon?: string;
  is_active: boolean;
  is_loading: boolean;
  can_go_back: boolean;
  can_go_forward: boolean;
  error?: string;
  created_at: number;
}

export interface BrowserMultiStateInfo {
  tabs: BrowserTabInfo[];
  active_tab_id: string | null;
  is_visible: boolean;
  bounds?: BrowserViewportBounds;
  downloads?: DownloadItemInfo[];
}

export interface BrowserInfo {
  is_created: boolean;
  is_visible: boolean;
  current_url: string;
  title: string;
  bounds?: BrowserViewportBounds;
}

export type ViewTab = 
  | 'chat' 
  | 'browser'
  | 'dev_agent' 
  | 'memory_bank' 
  | 'plugins' 
  | 'settings';

export interface BrowserToolDefinition {
  name: string;
  description: string;
  category: 'observation' | 'navigation' | 'interaction';
  risk_level: 'OBSERVE' | 'LOW_RISK_ACTION' | 'BLOCKED_FOR_AI';
  parameters: Record<string, any>;
}

export interface BrowserToolExecutionResult {
  success: boolean;
  tool_name: string;
  tab_id?: string;
  data?: any;
  error?: string;
  error_code?: string;
  duration_ms: number;
}

export type BrowserTaskStatus =
  | 'Planning'
  | 'Running'
  | 'Waiting'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'TimedOut';

export interface BrowserTaskState {
  task_id: string;
  goal: string;
  status: BrowserTaskStatus;
  current_tab_id: string;
  step_count: number;
  max_steps: number;
  started_at: number;
  timeout_ms: number;
  last_observation?: string;
  last_action?: string;
  last_error?: string;
}

export interface BrowserTaskResult {
  task_id: string;
  status: BrowserTaskStatus;
  goal: string;
  summary: string;
  steps_taken: number;
  duration_ms: number;
  final_tab_id: string;
  error?: string;
}

export type BrowserRiskLevel = 'LOW' | 'MEDIUM' | 'HIGH' | 'BLOCKED';
export type BrowserRiskDecision = 'ALLOW' | 'REQUIRE_APPROVAL' | 'BLOCK';

export interface BrowserActionContext {
  tool_name: string;
  tab_id: string;
  url?: string;
  title?: string;
  element_id?: string;
  element_tag?: string;
  element_role?: string;
  element_text?: string;
  element_aria_label?: string;
  element_href?: string;
  input_type?: string;
  placeholder?: string;
  text_to_type?: string;
  is_password?: boolean;
  form_action?: string;
  form_method?: string;
  parent_region?: string;
}

export interface BrowserRiskAssessment {
  risk_level: BrowserRiskLevel;
  decision: BrowserRiskDecision;
  policy_code: string;
  reason: string;
  user_explanation: string;
}

export interface BrowserRiskAuditEntry {
  id: string;
  timestamp: number;
  task_id?: string;
  tool_name: string;
  tab_id: string;
  risk_level: BrowserRiskLevel;
  decision: BrowserRiskDecision;
  policy_code: string;
  reason: string;
}

export interface PendingBrowserActionApproval {
  approval_id: string;
  task_id?: string;
  context: BrowserActionContext;
  assessment: BrowserRiskAssessment;
  created_at: number;
  status: string;
}

// --- Phase 5.4 Autonomous Multi-Tab Task Orchestration Types ---
export type TabOwnership = 'USER' | 'AGENT_TEMPORARY' | 'AGENT_SHARED';

export type OrchestrationStatus =
  | 'PLANNING'
  | 'RUNNING'
  | 'WAITING_FOR_APPROVAL'
  | 'WAITING_FOR_TABS'
  | 'COMPLETED'
  | 'PARTIALLY_COMPLETED'
  | 'FAILED'
  | 'CANCELLED'
  | 'TIMED_OUT';

export type TabWorkStatus =
  | 'QUEUED'
  | 'RUNNING'
  | 'WAITING'
  | 'COMPLETED'
  | 'FAILED'
  | 'CANCELLED';

export interface BrowserTabWork {
  work_id: string;
  orchestration_id: string;
  tab_id: string;
  ownership: TabOwnership;
  objective: string;
  status: TabWorkStatus;
  step_count: number;
  max_steps: number;
  depends_on?: string;
  last_observation?: string;
  last_action?: string;
  last_error?: string;
  summary?: string;
  evidence: string[];
  started_at: number;
  duration_ms: number;
}

export interface BrowserOrchestrationTask {
  orchestration_id: string;
  goal: string;
  status: OrchestrationStatus;
  started_at: number;
  timeout_ms: number;
  global_step_count: number;
  global_max_steps: number;
  max_concurrent_tabs: number;
  subtasks: BrowserTabWork[];
  completed_count: number;
  failed_count: number;
  final_summary?: string;
  error?: string;
}

export interface BrowserSubtaskResult {
  work_id: string;
  tab_id: string;
  status: TabWorkStatus;
  summary: string;
  evidence: string[];
  steps_taken: number;
  started_at: number;
  completed_at: number;
  duration_ms: number;
  error?: string;
}

export interface BrowserOrchestrationResult {
  orchestration_id: string;
  status: OrchestrationStatus;
  goal: string;
  subtask_results: BrowserSubtaskResult[];
  combined_summary: string;
  completed_count: number;
  failed_count: number;
  duration_ms: number;
  error?: string;
}

// --- Phase 5.5 Human <-> AI Browser Control / Takeover Types ---
export type BrowserControlState =
  | 'USER_CONTROLLED'
  | 'AI_CONTROLLED'
  | 'AI_PAUSED'
  | 'WAITING_FOR_APPROVAL'
  | 'TRANSITIONING';

export interface TabControlInfo {
  tab_id: string;
  control_state: BrowserControlState;
  last_transition: number;
  ai_task_id?: string;
  reason?: string;
}

// --- Phase 5.6A Browser History & Bookmarks Types ---
export interface BrowserHistoryEntry {
  id: string;
  url: string;
  title: string;
  visited_at: number;
  tab_id?: string;
  visit_count: number;
  last_visited_at: number;
}

export interface BrowserBookmarkFolder {
  id: string;
  name: string;
  parent_id?: string;
  created_at: number;
}

export interface BrowserBookmark {
  id: string;
  title: string;
  url: string;
  folder_id?: string;
  favicon?: string;
  created_at: number;
  updated_at: number;
}
