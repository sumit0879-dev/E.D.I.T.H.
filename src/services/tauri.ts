import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  Session,
  Message,
  Note,
  CustomApp,
  BuiltinApp,
  PluginWithState,
  ProviderDef,
  ProviderModel,
  WeatherResult,
  MemoryChunk,
  AgentStatus,
  BrowserViewportBounds,
  BrowserInfo,
  BrowserTabInfo,
  BrowserMultiStateInfo,
  PageObservationSnapshot,
  ScreenshotResult,
  ElementInfo,
  BrowserActionResult,
  BrowserToolDefinition,
  BrowserToolExecutionResult,
  BrowserTaskState,
  BrowserTaskResult,
  BrowserActionContext,
  BrowserRiskAssessment,
  BrowserRiskAuditEntry,
  PendingBrowserActionApproval,
  BrowserTabWork,
  BrowserOrchestrationTask,
  BrowserSubtaskResult,
  BrowserOrchestrationResult,
} from '../types';

export const isTauri = () => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// Full catalog of supported hardcoded AI providers and models (Groq and Gemini only)
export const DEFAULT_PROVIDERS: ProviderDef[] = [
  {
    id: 'groq',
    name: 'Groq Cloud (Ultra-Fast)',
    models: [
      { id: 'llama-3.3-70b-versatile', label: 'Meta LLaMA 3.3 70B Versatile (Recommended)' },
      { id: 'llama-3.1-8b-instant', label: 'Meta LLaMA 3.1 8B Instant (Ultra-Fast Chat)' },
      { id: 'openai/gpt-oss-120b', label: 'OpenAI GPT-OSS 120B (High Performance)' },
      { id: 'openai/gpt-oss-20b', label: 'OpenAI GPT-OSS 20B (Fast & Lightweight)' },
      { id: 'deepseek-r1-distill-llama-70b', label: 'DeepSeek R1 Distill LLaMA 70B (Reasoning)' },
      { id: 'qwen/qwen3.6-27b', label: 'Qwen 3.6 27B (Multilingual Reasoning)' },
      { id: 'qwen-qwq-32b', label: 'Qwen QwQ 32B (Advanced Reasoning)' },
      { id: 'groq/compound', label: 'Groq Compound Agentic System' },
      { id: 'groq/compound-mini', label: 'Groq Compound Mini' },
    ],
  },
  {
    id: 'gemini',
    name: 'Google Gemini API',
    models: [
      { id: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash (Analytical & Fast)' },
      { id: 'gemini-2.5-flash-lite', label: 'Gemini 2.5 Flash Lite (Ultra Lightweight)' },
      { id: 'gemini-2.5-pro', label: 'Gemini 2.5 Pro (Deep Reasoning & Coding)' },
      { id: 'gemini-2.0-flash', label: 'Gemini 2.0 Flash (Next-Gen Multimodal)' },
      { id: 'gemini-2.0-flash-lite', label: 'Gemini 2.0 Flash Lite (Cost-Efficient)' },
      { id: 'gemini-1.5-flash', label: 'Gemini 1.5 Flash (High Speed)' },
      { id: 'gemini-1.5-pro', label: 'Gemini 1.5 Pro (Long Context Window)' },
      { id: 'gemini-3.7-flash', label: 'Gemini 3.7 Flash (Frontier Agentic & Coding)' },
      { id: 'gemini-3.6-flash', label: 'Gemini 3.6 Flash (Agentic Planning)' },
      { id: 'gemini-3.1-pro', label: 'Gemini 3.1 Pro (Frontier Multimodal Intelligence)' },
    ],
  },
];

export const DEFAULT_PLUGINS: PluginWithState[] = [
  {
    id: 'web_search',
    name: 'Deep Web Search (Tavily)',
    description: "DuckDuckGo & Tavily AI web search. Type 'search [query]' to fetch live web sources.",
    category: 'utility',
    enabled: true,
  },
  {
    id: 'media_player',
    name: 'Media Player (YouTube)',
    description: "Searches and plays songs & videos on YouTube. Type 'play [song name]'.",
    category: 'media',
    enabled: true,
  },
  {
    id: 'app_launcher',
    name: 'Desktop App Launcher',
    description: "Launches system and custom registered apps. Type 'open [app name]'.",
    category: 'system',
    enabled: true,
  },
  {
    id: 'system_control',
    name: 'System Control (Audio & Windows)',
    description: "Volume up, volume down, and mute system audio. Say 'volume up' or 'mute'.",
    category: 'system',
    enabled: true,
  },
  {
    id: 'terminal',
    name: 'System Terminal',
    description: "Execute shell commands with output. Type 'cmd [command]' or 'terminal [command]'.",
    category: 'system',
    enabled: true,
  },
  {
    id: 'dev_agent',
    name: 'Dev Agent (E.D.I.T.H.)',
    description: 'AI developer assistant that autonomously reads files and runs project commands.',
    category: 'developer',
    enabled: true,
  },
  {
    id: 'whatsapp',
    name: 'WhatsApp Integration',
    description: "Send WhatsApp messages via deep link. Type 'whatsapp [number] [message]'.",
    category: 'social',
    enabled: true,
  },
  {
    id: 'gmail',
    name: 'Email Integration',
    description: "Compose emails via default mail client. Type 'email [address] [message]'.",
    category: 'social',
    enabled: true,
  },
];

export const DEFAULT_BUILTIN_APPS: BuiltinApp[] = [
  { id: 'bi_notepad', name: 'notepad', path: 'notepad.exe', keywords: 'notepad,text,editor', builtin: true },
  { id: 'bi_calc', name: 'calculator', path: 'calc.exe', keywords: 'calculator,calc,math', builtin: true },
  { id: 'bi_chrome', name: 'chrome', path: 'chrome', keywords: 'chrome,browser,google', builtin: true },
  { id: 'bi_explorer', name: 'file explorer', path: 'explorer.exe', keywords: 'explorer,files,folder', builtin: true },
  { id: 'bi_cmd', name: 'command prompt', path: 'cmd.exe', keywords: 'cmd,command,prompt,terminal', builtin: true },
  { id: 'bi_taskmgr', name: 'task manager', path: 'taskmgr.exe', keywords: 'task,manager,process', builtin: true },
  { id: 'bi_settings', name: 'settings', path: 'ms-settings:', keywords: 'settings,control', builtin: true },
  { id: 'bi_paint', name: 'paint', path: 'mspaint.exe', keywords: 'paint,draw,art', builtin: true },
  { id: 'bi_vscode', name: 'vs code', path: 'code', keywords: 'vscode,code,editor,ide', builtin: true },
];

// LocalStorage helpers for browser/testing persistence
const getStoredJson = <T>(key: string, fallback: T): T => {
  try {
    const val = localStorage.getItem(key);
    return val ? JSON.parse(val) : fallback;
  } catch {
    return fallback;
  }
};

const setStoredJson = (key: string, value: any) => {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
};

// --- Settings ---
export async function getAllSettings(): Promise<Record<string, string>> {
  if (!isTauri()) {
    return getStoredJson<Record<string, string>>('edith_settings', {});
  }
  try {
    const res = await invoke<Record<string, string>>('get_all_settings');
    return res || {};
  } catch (e) {
    console.warn('getAllSettings fallback to localStorage:', e);
    return getStoredJson<Record<string, string>>('edith_settings', {});
  }
}

export async function saveSetting(key: string, value: string): Promise<void> {
  const current = getStoredJson<Record<string, string>>('edith_settings', {});
  current[key] = value;
  setStoredJson('edith_settings', current);

  if (isTauri()) {
    try {
      await invoke('save_setting', { key, value });
    } catch (e) {
      console.warn('save_setting invoke failed:', e);
    }
  }
}

export async function syncSettings(settings: Record<string, string>): Promise<void> {
  setStoredJson('edith_settings', settings);
  if (isTauri()) {
    try {
      await invoke('sync_settings', { settings });
    } catch (e) {
      console.warn('sync_settings invoke failed:', e);
    }
  }
}

// --- Sessions & History ---
export async function getAllSessions(): Promise<Session[]> {
  if (!isTauri()) {
    return getStoredJson<Session[]>('edith_sessions', [
      { id: 'session_default', title: 'General Chat' },
    ]);
  }
  try {
    const res = await invoke<Session[]>('get_all_sessions');
    return res && res.length > 0
      ? res
      : [{ id: 'session_default', title: 'General Chat' }];
  } catch (e) {
    return getStoredJson<Session[]>('edith_sessions', [
      { id: 'session_default', title: 'General Chat' },
    ]);
  }
}

export async function createSession(sessionId: string, title: string): Promise<void> {
  const list = getStoredJson<Session[]>('edith_sessions', []);
  if (!list.find((s) => s.id === sessionId)) {
    setStoredJson('edith_sessions', [{ id: sessionId, title }, ...list]);
  }
  if (isTauri()) {
    try {
      await invoke('create_session', { sessionId, title });
    } catch (e) {
      console.warn('create_session invoke error:', e);
    }
  }
}

export async function renameSession(sessionId: string, newTitle: string): Promise<void> {
  const list = getStoredJson<Session[]>('edith_sessions', []);
  setStoredJson(
    'edith_sessions',
    list.map((s) => (s.id === sessionId ? { ...s, title: newTitle } : s))
  );
  if (isTauri()) {
    try {
      await invoke('rename_session', { sessionId, newTitle });
    } catch (e) {
      console.warn('rename_session invoke error:', e);
    }
  }
}

export async function deleteSession(sessionId: string): Promise<void> {
  const list = getStoredJson<Session[]>('edith_sessions', []);
  setStoredJson(
    'edith_sessions',
    list.filter((s) => s.id !== sessionId)
  );
  if (isTauri()) {
    try {
      await invoke('delete_session', { sessionId });
    } catch (e) {
      console.warn('delete_session invoke error:', e);
    }
  }
}

export async function getSessionMessages(sessionId: string): Promise<Message[]> {
  if (!isTauri()) {
    return getStoredJson<Message[]>('edith_msgs_' + sessionId, []);
  }
  try {
    const res = await invoke<Message[]>('get_session_messages', { sessionId });
    return res || [];
  } catch (e) {
    return getStoredJson<Message[]>('edith_msgs_' + sessionId, []);
  }
}

export async function saveSessionMessage(
  sessionId: string,
  role: string,
  text: string,
  time: string
): Promise<void> {
  const msgs = getStoredJson<Message[]>('edith_msgs_' + sessionId, []);
  msgs.push({ role: role as any, text, content: text, time, session_id: sessionId });
  setStoredJson('edith_msgs_' + sessionId, msgs);

  if (isTauri()) {
    try {
      await invoke('save_session_message', { sessionId, role, text, time });
    } catch (e) {
      console.warn('save_session_message invoke error:', e);
    }
  }
}

// --- Chat Command ---
export async function chatCommand(
  message: string,
  sessionId: string,
  history: Array<{ role: string; text: string }>,
  appSettings: Record<string, any>
): Promise<{ response: string; type: string }> {
  if (!isTauri()) {
    // Helpful simulation when running in pure browser
    const lower = message.toLowerCase();
    if (lower.startsWith('open ')) {
      return { response: 'Launched ' + message.slice(5) + ' (Simulated)', type: 'plugin' };
    }
    if (lower.startsWith('play ')) {
      window.open('https://www.youtube.com/results?search_query=' + encodeURIComponent(message.slice(5)), '_blank');
      return { response: 'Opening YouTube for: ' + message.slice(5), type: 'plugin' };
    }
    if (lower.startsWith('search ')) {
      return {
        response: '### 🌐 Search Results for "' + message.slice(7) + '"\n- **Source 1**: AI advancements demonstrate rapid reasoning speed.\n- **Source 2**: Multi-modal workflows are now integrated into modern desktops.',
        type: 'ai',
      };
    }
    return {
      response: 'Hello! I am **E.D.I.T.H. Mark-85**. Tactical AI core active.\n\n```json\n{\n  "status": "connected",\n  "provider": "' + (appSettings.selectedProvider || 'groq') + '",\n  "model": "' + (appSettings.selectedModel || 'llama-3.3-70b-versatile') + '"\n}\n```\nHow may I assist your mission, Commander?',
      type: 'ai',
    };
  }
  return await invoke('chat_command', {
    message,
    sessionId,
    history,
    appSettings,
  });
}

// --- Providers ---
export async function getProviders(): Promise<{ providers: ProviderDef[] }> {
  if (!isTauri()) {
    return { providers: DEFAULT_PROVIDERS };
  }
  try {
    const res = await invoke<{ providers: ProviderDef[] }>('get_providers');
    if (res && res.providers && res.providers.length > 0) {
      return res;
    }
    return { providers: DEFAULT_PROVIDERS };
  } catch (e) {
    return { providers: DEFAULT_PROVIDERS };
  }
}

// --- Local LLM Server ---
export async function loadLocalLlm(path: string, loadMode: string): Promise<void> {
  if (!isTauri()) return;
  return await invoke('load_local_llm', { path, loadMode });
}

export async function stopLocalLlm(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('stop_local_llm');
}

export async function localChat(prompt: string, emitEvent?: string): Promise<string> {
  if (!isTauri()) return '';
  return await invoke('local_chat', { prompt, emitEvent });
}

// --- Dev Agent (E.D.I.T.H.) ---
export async function agentStatus(): Promise<AgentStatus> {
  if (!isTauri()) return { is_ready: true, project_path: 'E:\\Projects\\E.D.I.T.H' };
  try {
    return await invoke('agent_status');
  } catch {
    return { is_ready: false, project_path: '' };
  }
}

export async function agentSetPath(path: string): Promise<void> {
  if (!isTauri()) return;
  return await invoke('agent_set_path', { path });
}

export async function agentReset(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('agent_reset');
}

export async function agentChat(message: string, sessionId?: string): Promise<string> {
  if (!isTauri()) {
    return 'Dev Agent (E.D.I.T.H.) analyzed the workspace: Code structure is verified and all modules are ready.';
  }
  return await invoke('agent_chat', { message, sessionId });
}

export interface CommandProposalPayload {
  proposal_id: string;
  session_id: string;
  command: string;
  working_dir: string;
  risk_level: string;
  expires_at: number;
}

export async function agentResolveProposal(
  proposalId: string,
  action: 'Approve' | 'Reject',
  sessionId?: string
): Promise<{ success: boolean; output: string; error?: string; execution_time_ms: number }> {
  if (!isTauri()) {
    return { success: true, output: `[Mock Proposal ${action}] Execution completed.`, execution_time_ms: 50 };
  }
  return await invoke('agent_resolve_proposal', {
    proposalId,
    sessionId: sessionId || null,
    action,
  });
}

export async function onToolProposal(callback: (proposal: CommandProposalPayload) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<CommandProposalPayload>('tool-proposal', (event) => {
      callback(event.payload);
    });
  } catch {
    return () => {};
  }
}

// --- Plugins ---
export async function getPlugins(): Promise<PluginWithState[]> {
  if (!isTauri()) {
    return getStoredJson<PluginWithState[]>('edith_plugins', DEFAULT_PLUGINS);
  }
  try {
    const res = await invoke<PluginWithState[]>('get_plugins');
    return res && res.length > 0 ? res : DEFAULT_PLUGINS;
  } catch {
    return DEFAULT_PLUGINS;
  }
}

export async function togglePlugin(pluginId: string): Promise<boolean> {
  const current = getStoredJson<PluginWithState[]>('edith_plugins', DEFAULT_PLUGINS);
  let newState = true;
  const updated = current.map((p) => {
    if (p.id === pluginId) {
      newState = !p.enabled;
      return { ...p, enabled: newState };
    }
    return p;
  });
  setStoredJson('edith_plugins', updated);

  if (isTauri()) {
    try {
      return await invoke('toggle_plugin', { pluginId });
    } catch {
      return newState;
    }
  }
  return newState;
}

export async function getBuiltinApps(): Promise<BuiltinApp[]> {
  if (!isTauri()) return DEFAULT_BUILTIN_APPS;
  try {
    const res = await invoke<BuiltinApp[]>('get_builtin_apps');
    return res && res.length > 0 ? res : DEFAULT_BUILTIN_APPS;
  } catch {
    return DEFAULT_BUILTIN_APPS;
  }
}

export async function pluginSystemTerminal(cmd: string): Promise<string> {
  if (!isTauri()) {
    return 'Windows PowerShell [Simulated Mode]\nCommand: ' + cmd + '\nStatus: Success\nOutput: Ready';
  }
  return await invoke('plugin_system_terminal', { cmd });
}

export async function pluginSystemControl(action: string): Promise<string> {
  if (!isTauri()) return 'System control ' + action + ' executed (Simulated)';
  return await invoke('plugin_system_control', { action });
}

export async function pluginAppLauncher(appPath: string): Promise<string> {
  if (!isTauri()) return 'Launched ' + appPath;
  return await invoke('plugin_app_launcher', { appPath });
}

export async function pluginWebSearch(query: string, apiKey?: string): Promise<string> {
  if (!isTauri()) return 'Search results for: ' + query;
  return await invoke('plugin_web_search', { query, apiKey });
}

export async function pluginMediaPlayer(query: string): Promise<string> {
  if (!isTauri()) {
    window.open('https://www.youtube.com/results?search_query=' + encodeURIComponent(query), '_blank');
    return 'Playing: ' + query;
  }
  return await invoke('plugin_media_player', { query });
}

export async function pluginWhatsapp(number: string, message: string): Promise<string> {
  if (!isTauri()) {
    window.open('https://wa.me/' + number + '?text=' + encodeURIComponent(message), '_blank');
    return 'WhatsApp opened for ' + number;
  }
  return await invoke('plugin_whatsapp', { number, message });
}

export async function pluginGmail(email: string, message: string): Promise<string> {
  if (!isTauri()) {
    window.open('mailto:' + email + '?subject=Message%20from%20E.D.I.T.H.&body=' + encodeURIComponent(message), '_blank');
    return 'Email opened for ' + email;
  }
  return await invoke('plugin_gmail', { email, message });
}

export async function takeScreenshot(): Promise<string> {
  if (!isTauri()) return '';
  return await invoke('take_screenshot');
}

// --- Personal Notes ---
export async function getPersonalNotes(): Promise<Note[]> {
  if (!isTauri()) {
    return getStoredJson<Note[]>('edith_notes', [
      { id: '1', content: '# Welcome to E.D.I.T.H. Notes\n- Keep quick reminders and notes here.\n- Auto-saved to SQLite.' },
    ]);
  }
  try {
    const res = await invoke<Note[]>('get_personal_notes');
    return res && res.length > 0 ? res : [];
  } catch {
    return getStoredJson<Note[]>('edith_notes', []);
  }
}

export async function savePersonalNote(content: string): Promise<void> {
  setStoredJson('edith_notes', [{ id: '1', content }]);
  if (isTauri()) {
    try {
      await invoke('save_personal_note', { content });
    } catch {}
  }
}

export async function deletePersonalNote(noteId: string): Promise<void> {
  setStoredJson('edith_notes', []);
  if (isTauri()) {
    try {
      await invoke('delete_personal_note', { noteId });
    } catch {}
  }
}

// --- Custom Apps ---
export async function getCustomApps(): Promise<CustomApp[]> {
  if (!isTauri()) {
    return getStoredJson<CustomApp[]>('edith_custom_apps', [
      { id: 1, name: 'vscode', path: 'code', keywords: 'vscode,code,editor' },
      { id: 2, name: 'spotify', path: 'spotify.exe', keywords: 'spotify,music,songs' },
    ]);
  }
  try {
    const res = await invoke<CustomApp[]>('get_custom_apps');
    return res || [];
  } catch {
    return getStoredJson<CustomApp[]>('edith_custom_apps', []);
  }
}

export async function addCustomApp(name: string, path: string, keywords: string): Promise<void> {
  const current = getStoredJson<CustomApp[]>('edith_custom_apps', []);
  current.push({ id: Date.now(), name, path, keywords });
  setStoredJson('edith_custom_apps', current);
  if (isTauri()) {
    try {
      await invoke('add_custom_app', { name, path, keywords });
    } catch {}
  }
}

export async function deleteCustomApp(appId: number): Promise<void> {
  const current = getStoredJson<CustomApp[]>('edith_custom_apps', []);
  setStoredJson(
    'edith_custom_apps',
    current.filter((a) => a.id !== appId)
  );
  if (isTauri()) {
    try {
      await invoke('delete_custom_app', { appId });
    } catch {}
  }
}

export async function launchApp(path: string): Promise<void> {
  if (!isTauri()) {
    console.log('Launch app simulated:', path);
    return;
  }
  return await invoke('launch_app', { path });
}

// --- TTS (Text to Speech) ---
export async function ttsSpeak(text: string, voice?: string): Promise<void> {
  if (!isTauri()) {
    if ('speechSynthesis' in window) {
      window.speechSynthesis.cancel();
      const utterance = new SpeechSynthesisUtterance(text.replace(/[*`_~#]/g, ''));
      window.speechSynthesis.speak(utterance);
    }
    return;
  }
  return await invoke('tts_speak', { text, voice });
}

export async function localTtsSpeak(text: string, voice: string, modelName: string): Promise<void> {
  if (!isTauri()) return;
  return await invoke('local_tts_speak', { text, voice, modelName });
}

export async function getKokoroModels(): Promise<string[]> {
  if (!isTauri()) return ['kokoro-v1.0.int8.onnx', 'kokoro-v1.0.fp16.onnx'];
  try {
    const res = await invoke<string[]>('get_kokoro_models');
    return res && res.length > 0 ? res : ['kokoro-v1.0.int8.onnx'];
  } catch {
    return ['kokoro-v1.0.int8.onnx'];
  }
}

export async function ttsStop(): Promise<void> {
  if (!isTauri()) {
    if ('speechSynthesis' in window) {
      window.speechSynthesis.cancel();
    }
    return;
  }
  return await invoke('tts_stop');
}

// --- Vector Memory (LanceDB) ---
export async function saveToMemory(text: string, source: string): Promise<void> {
  const current = getStoredJson<MemoryChunk[]>('edith_memory', []);
  current.push({ id: 'mem_' + Date.now(), text, source, score: 0.12 });
  setStoredJson('edith_memory', current);
  if (isTauri()) {
    try {
      await invoke('save_to_memory_cmd', { text, source });
    } catch {}
  }
}

export async function searchMemory(query: string): Promise<MemoryChunk[]> {
  if (!isTauri()) {
    const current = getStoredJson<MemoryChunk[]>('edith_memory', [
      { id: '1', text: 'E.D.I.T.H. (Even Dead, I\'m The Hero) is an advanced desktop assistant for high-productivity workflows.', source: 'system_core', score: 0.08 },
      { id: '2', text: 'Project built with Tauri 2, Rust backend and React frontend.', source: 'project_docs', score: 0.15 },
    ]);
    return current.filter((c) => c.text.toLowerCase().includes(query.toLowerCase()) || c.source.toLowerCase().includes(query.toLowerCase()));
  }
  try {
    const res = await invoke<MemoryChunk[]>('search_memory_cmd', { query });
    return res || [];
  } catch {
    return [];
  }
}

export async function getMemories(): Promise<MemoryChunk[]> {
  if (!isTauri()) {
    return getStoredJson<MemoryChunk[]>('edith_memory', [
      { id: '1', text: 'E.D.I.T.H. (Even Dead, I\'m The Hero) is an advanced desktop assistant for high-productivity workflows.', source: 'system_core', score: 0.08 },
      { id: '2', text: 'Project built with Tauri 2, Rust backend and React frontend.', source: 'project_docs', score: 0.15 },
      { id: '3', text: 'Stark-grade tactical intelligence with custom AI providers support.', source: 'chat:session_1', score: 0.22 },
    ]);
  }
  try {
    const res = await invoke<MemoryChunk[]>('get_memories_cmd');
    return res || [];
  } catch {
    return [];
  }
}

export async function deleteMemory(source: string): Promise<void> {
  const current = getStoredJson<MemoryChunk[]>('edith_memory', []);
  setStoredJson('edith_memory', current.filter((c) => c.source !== source));
  if (isTauri()) {
    try {
      await invoke('delete_memory_cmd', { source });
    } catch {}
  }
}

// --- Custom Providers & Models Fetching ---
export async function fetchCustomModels(baseUrl: string, apiKey?: string): Promise<ProviderModel[]> {
  if (!isTauri()) {
    try {
      const rawUrl = baseUrl.trim().replace(/\/chat\/completions\/?$/, '').replace(/\/$/, '');
      const trimmedKey = apiKey?.trim() || '';
      const isGemini = rawUrl.includes('generativelanguage.googleapis.com') || rawUrl === 'gemini';

      let url = rawUrl;
      const headers: Record<string, string> = {};

      if (isGemini) {
        url = trimmedKey ? `https://generativelanguage.googleapis.com/v1beta/models?key=${trimmedKey}` : 'https://generativelanguage.googleapis.com/v1beta/models';
      } else {
        if (!url.endsWith('/models')) {
          url = url + '/models';
        }
        if (trimmedKey) {
          headers['Authorization'] = `Bearer ${trimmedKey}`;
        }
      }

      const res = await fetch(url, { headers });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      const data = await res.json();
      const list = data.data || data.models || [];
      return list
        .filter((m: any) => {
          if (m.supportedGenerationMethods) {
            return m.supportedGenerationMethods.includes('generateContent');
          }
          return true;
        })
        .map((m: any) => {
          const rawId = m.id || m.name || '';
          const cleanId = rawId.replace(/^models\//, '');
          return {
            id: cleanId,
            label: m.displayName || m.name || cleanId,
          };
        });
    } catch (err: any) {
      throw new Error(err.message || 'Failed to fetch models from endpoint');
    }
  }
  return await invoke<ProviderModel[]>('fetch_custom_models', { baseUrl, apiKey: apiKey || null });
}

// --- Weather ---
export async function getWeather(lat: number, lon: number): Promise<WeatherResult> {
  if (!isTauri()) {
    return { temperature: 28.5, weather_code: 1, condition: 'Clear Sky / Sunny' };
  }
  try {
    return await invoke('get_weather', { lat, lon });
  } catch {
    return { temperature: 28.5, weather_code: 1, condition: 'Partly Cloudy' };
  }
}

export async function getBaseDir(): Promise<string> {
  if (!isTauri()) return '.';
  return await invoke('get_base_dir');
}

// --- Event Subscriptions ---
export async function onChatChunk(callback: (chunk: string) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<string>('chat-chunk', (event) => {
      callback(event.payload);
    });
  } catch {
    return () => {};
  }
}

export async function onModelProgress(callback: (msg: string) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<string>('model-progress', (event) => {
      callback(event.payload);
    });
  } catch {
    return () => {};
  }
}

// --- Browser Webview2 Multi-Tab Controller Methods ---
export async function browserCreateTab(tabId: string, url?: string, bounds?: BrowserViewportBounds): Promise<BrowserTabInfo> {
  if (!isTauri()) {
    return {
      id: tabId,
      label: `edith_tab_${tabId}`,
      url: url || 'https://example.com',
      title: 'Simulated Tab',
      is_active: true,
      is_loading: false,
      can_go_back: false,
      can_go_forward: false,
      created_at: Date.now(),
    };
  }
  return await invoke<BrowserTabInfo>('browser_create_tab', { tabId, url, bounds });
}

export async function browserSwitchTab(tabId: string, bounds?: BrowserViewportBounds): Promise<BrowserTabInfo> {
  if (!isTauri()) {
    return {
      id: tabId,
      label: `edith_tab_${tabId}`,
      url: 'https://example.com',
      title: 'Simulated Tab',
      is_active: true,
      is_loading: false,
      can_go_back: false,
      can_go_forward: false,
      created_at: Date.now(),
    };
  }
  return await invoke<BrowserTabInfo>('browser_switch_tab', { tabId, bounds });
}

export async function browserCloseTab(tabId: string): Promise<BrowserTabInfo | null> {
  if (!isTauri()) return null;
  return await invoke<BrowserTabInfo | null>('browser_close_tab', { tabId });
}

export async function browserNavigateTab(tabId: string, url: string): Promise<string> {
  if (!isTauri()) return url;
  return await invoke<string>('browser_navigate_tab', { tabId, url });
}

export async function browserGoBackTab(tabId: string): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_go_back_tab', { tabId });
}

export async function browserGoForwardTab(tabId: string): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_go_forward_tab', { tabId });
}

export async function browserReloadTab(tabId: string): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_reload_tab', { tabId });
}

export async function browserGetMultiState(): Promise<BrowserMultiStateInfo> {
  if (!isTauri()) {
    return {
      tabs: [],
      active_tab_id: null,
      is_visible: false,
    };
  }
  return await invoke<BrowserMultiStateInfo>('browser_get_multi_state');
}

export async function browserSetBoundsAll(bounds: BrowserViewportBounds): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_set_bounds_all', { bounds });
}

export async function browserHideAll(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_hide_all');
}

export async function browserShowActive(bounds?: BrowserViewportBounds): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_show_active', { bounds });
}

export async function browserGetTabUrl(tabId: string): Promise<string> {
  if (!isTauri()) return 'https://example.com';
  return await invoke<string>('browser_get_tab_url', { tabId });
}

export async function browserGetTabTitle(tabId: string): Promise<string> {
  if (!isTauri()) return 'Tab Title';
  return await invoke<string>('browser_get_tab_title', { tabId });
}

export async function browserGetTabVisibleText(tabId: string): Promise<string> {
  if (!isTauri()) {
    return 'Sample simulated visible text content.';
  }
  return await invoke<string>('browser_get_tab_visible_text', { tabId });
}

export async function browserObserveTab(
  tabId: string,
  scope?: string
): Promise<PageObservationSnapshot> {
  if (!isTauri()) {
    return {
      tab_id: tabId,
      url: 'https://example.com',
      title: 'Example Domain (Simulated)',
      generation: 1,
      fingerprint: 'fp_simulated_01',
      viewport: { width: 1024, height: 768, scroll_x: 0, scroll_y: 0, page_width: 1024, page_height: 768 },
      visible_text: 'Example Domain. This domain is for use in illustrative examples in documents.',
      selected_text: undefined,
      regions: [
        {
          region_type: 'main',
          label: 'Main Content',
          elements_count: 5,
        },
      ],
      headings: [
        {
          level: 1,
          text: 'Example Domain',
        },
      ],
      interactive_elements: [
        {
          id: 'id_more_info',
          tag: 'a',
          role: 'link',
          accessible_name: 'More information...',
          text: 'More information...',
          href: 'https://www.iana.org/domains/example',
          disabled: false,
          visible: true,
          interactable: true,
          value_available: true,
          bounding_box: { x: 100, y: 200, width: 150, height: 24 },
        },
      ],
      forms: [],
      links: [
        {
          text: 'More information...',
          href: 'https://www.iana.org/domains/example',
          visible: true,
          is_external: true,
        },
      ],
      timestamp: Date.now(),
    };
  }
  return await invoke<PageObservationSnapshot>('browser_observe_tab', { tabId, scope });
}

export async function browserScreenshotTab(tabId: string, bounds?: BrowserViewportBounds): Promise<ScreenshotResult> {
  if (!isTauri()) {
    return {
      tab_id: tabId,
      data_url: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
      width: 800,
      height: 600,
    };
  }
  return await invoke<ScreenshotResult>('browser_screenshot_tab', { tabId, bounds });
}

export async function browserReopenLastClosedTab(bounds?: BrowserViewportBounds): Promise<BrowserTabInfo | null> {
  if (!isTauri()) return null;
  return await invoke<BrowserTabInfo | null>('browser_reopen_last_closed_tab', { bounds });
}

// --- Phase 4A Browser Interaction / Action Layer Methods ---
export async function browserClickElement(tabId: string, elementId: string): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: 'click',
      tab_id: tabId,
      element_id: elementId,
      page_changed: true,
      url_changed: false,
      resulting_url: 'https://example.com',
    };
  }
  return await invoke<BrowserActionResult>('browser_click_element', { tabId, elementId });
}

export async function browserTypeElement(
  tabId: string,
  elementId: string,
  text: string,
  clearFirst?: boolean
): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: 'type',
      tab_id: tabId,
      element_id: elementId,
      page_changed: true,
      url_changed: false,
      resulting_url: 'https://example.com',
    };
  }
  return await invoke<BrowserActionResult>('browser_type_element', {
    tabId,
    elementId,
    text,
    clearFirst: clearFirst ?? true,
  });
}

export async function browserScroll(
  tabId: string,
  direction: string,
  amount?: number
): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: `scroll_${direction}`,
      tab_id: tabId,
      page_changed: true,
      url_changed: false,
    };
  }
  return await invoke<BrowserActionResult>('browser_scroll', { tabId, direction, amount });
}

export async function browserPressKey(tabId: string, key: string): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: `press_key_${key}`,
      tab_id: tabId,
      page_changed: true,
      url_changed: false,
    };
  }
  return await invoke<BrowserActionResult>('browser_press_key', { tabId, key });
}

export async function browserFocusElement(tabId: string, elementId: string): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: 'focus',
      tab_id: tabId,
      element_id: elementId,
      page_changed: false,
      url_changed: false,
    };
  }
  return await invoke<BrowserActionResult>('browser_focus_element', { tabId, elementId });
}

export async function browserWait(
  tabId: string,
  condition: string,
  target?: string,
  timeoutMs?: number
): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: `wait_${condition}`,
      tab_id: tabId,
      element_id: target,
      page_changed: false,
      url_changed: false,
    };
  }
  return await invoke<BrowserActionResult>('browser_wait', { tabId, condition, target, timeoutMs });
}

export async function browserSelectOption(
  tabId: string,
  elementId: string,
  value: string
): Promise<BrowserActionResult> {
  if (!isTauri()) {
    return {
      success: true,
      action: 'select_option',
      tab_id: tabId,
      element_id: elementId,
      page_changed: true,
      url_changed: false,
    };
  }
  return await invoke<BrowserActionResult>('browser_select_option', { tabId, elementId, value });
}

// --- Legacy Phase 1 Delegate Methods ---
export async function browserCreate(url?: string, bounds?: BrowserViewportBounds): Promise<BrowserInfo> {
  if (!isTauri()) {
    return {
      is_created: true,
      is_visible: true,
      current_url: url || 'https://example.com',
      title: 'Example Domain (Simulated)',
      bounds,
    };
  }
  return await invoke<BrowserInfo>('browser_create', { url, bounds });
}

export async function browserDestroy(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_destroy');
}

export async function browserShow(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_show');
}

export async function browserHide(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_hide');
}

export async function browserNavigate(url: string): Promise<string> {
  if (!isTauri()) return url;
  return await invoke<string>('browser_navigate', { url });
}

export async function browserGoBack(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_go_back');
}

export async function browserGoForward(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_go_forward');
}

export async function browserReload(): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_reload');
}

export async function browserSetBounds(bounds: BrowserViewportBounds): Promise<void> {
  if (!isTauri()) return;
  return await invoke('browser_set_bounds', { bounds });
}

export async function browserGetUrl(): Promise<string> {
  if (!isTauri()) return 'https://example.com';
  return await invoke<string>('browser_get_url');
}

export async function browserGetTitle(): Promise<string> {
  if (!isTauri()) return 'Example Domain';
  return await invoke<string>('browser_get_title');
}

export async function browserGetVisibleText(): Promise<string> {
  if (!isTauri()) {
    return 'Example Domain. This domain is for use in illustrative examples in documents.';
  }
  return await invoke<string>('browser_get_visible_text');
}

// --- Phase 4B AI Browser Tool Integration APIs ---
export async function browserGetToolDefinitions(): Promise<BrowserToolDefinition[]> {
  if (!isTauri()) {
    return [];
  }
  return await invoke<BrowserToolDefinition[]>('browser_get_tool_definitions_cmd');
}

export async function browserExecuteTool(
  toolName: string,
  argumentsObj: Record<string, any>
): Promise<BrowserToolExecutionResult> {
  if (!isTauri()) {
    return {
      success: true,
      tool_name: toolName,
      data: { simulated: true },
      duration_ms: 1,
    };
  }
  return await invoke<BrowserToolExecutionResult>('browser_execute_tool_cmd', {
    toolName,
    arguments: argumentsObj,
  });
}

// --- Phase 4C Autonomous Browser Agent APIs ---
export async function browserAgentRunTask(
  goal: string,
  maxSteps?: number,
  timeoutMs?: number
): Promise<BrowserTaskResult> {
  if (!isTauri()) {
    return {
      task_id: 'sim_task',
      status: 'Completed',
      goal,
      summary: 'Simulated task execution in non-Tauri mode.',
      steps_taken: 1,
      duration_ms: 100,
      final_tab_id: 'tab_a',
    };
  }
  return await invoke<BrowserTaskResult>('browser_agent_run_task', {
    goal,
    maxSteps,
    timeoutMs,
  });
}

export async function browserAgentCancelTask(taskId: string): Promise<boolean> {
  if (!isTauri()) return true;
  return await invoke<boolean>('browser_agent_cancel_task', { taskId });
}

export async function browserAgentGetCurrentTask(): Promise<BrowserTaskState | null> {
  if (!isTauri()) return null;
  return await invoke<BrowserTaskState | null>('browser_agent_get_current_task');
}

// --- Phase 5.3 Browser Action Risk & Safety Engine APIs ---
export async function browserAssessActionRisk(
  context: BrowserActionContext
): Promise<BrowserRiskAssessment> {
  if (!isTauri()) {
    return {
      risk_level: 'LOW',
      decision: 'ALLOW',
      policy_code: 'SAFE_INTERACTION',
      reason: 'Simulated assessment in web preview mode.',
      user_explanation: 'Action permitted.',
    };
  }
  return await invoke<BrowserRiskAssessment>('browser_assess_action_risk', { context });
}

export async function browserGetRiskAuditLog(): Promise<BrowserRiskAuditEntry[]> {
  if (!isTauri()) return [];
  return await invoke<BrowserRiskAuditEntry[]>('browser_get_risk_audit_log');
}

export async function browserResolveActionApproval(
  approvalId: string,
  decision: string
): Promise<PendingBrowserActionApproval> {
  if (!isTauri()) {
    return {
      approval_id: approvalId,
      context: { tool_name: 'browser_click', tab_id: 'tab_a' },
      assessment: {
        risk_level: 'HIGH',
        decision: 'REQUIRE_APPROVAL',
        policy_code: 'DESTRUCTIVE_ACTION',
        reason: 'Simulated',
        user_explanation: 'Simulated approval resolution.',
      },
      created_at: Date.now(),
      status: decision,
    };
  }
  return await invoke<PendingBrowserActionApproval>('browser_resolve_action_approval', {
    approvalId,
    decision,
  });
}

// --- Phase 5.4 Autonomous Multi-Tab Task Orchestration APIs ---
export async function browserOrchestratorRunTask(
  goal: string,
  subtaskGoals?: string[],
  globalMaxSteps?: number,
  timeoutMs?: number
): Promise<BrowserOrchestrationResult> {
  if (!isTauri()) {
    return {
      orchestration_id: 'sim_orch_1',
      status: 'COMPLETED',
      goal,
      subtask_results: [
        {
          work_id: 'work_1',
          tab_id: 'tab_a',
          status: 'COMPLETED',
          summary: 'Simulated subtask 1 completed.',
          evidence: ['Title: Simulated'],
          steps_taken: 1,
          duration_ms: 50,
        },
      ],
      combined_summary: 'Master Goal completed in simulated environment.',
      completed_count: 1,
      failed_count: 0,
      duration_ms: 100,
    };
  }
  return await invoke<BrowserOrchestrationResult>('browser_orchestrator_run_task', {
    goal,
    subtaskGoals,
    globalMaxSteps,
    timeoutMs,
  });
}

export async function browserOrchestratorCancelTask(orchestrationId: string): Promise<boolean> {
  if (!isTauri()) return true;
  return await invoke<boolean>('browser_orchestrator_cancel_task', { orchestrationId });
}

export async function browserOrchestratorGetCurrentTask(): Promise<BrowserOrchestrationTask | null> {
  if (!isTauri()) return null;
  return await invoke<BrowserOrchestrationTask | null>('browser_orchestrator_get_current_task');
}

