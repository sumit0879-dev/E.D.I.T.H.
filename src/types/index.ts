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

export interface ElementInfo {
  id: string;
  tag: string;
  role?: string;
  text: string;
  aria_label?: string;
  href?: string;
  input_type?: string;
  disabled: boolean;
  visible: boolean;
  is_password?: boolean;
  is_in_iframe?: boolean;
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
  visible_text: string;
  selected_text?: string;
  interactive_elements: ElementInfo[];
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
