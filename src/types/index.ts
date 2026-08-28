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

