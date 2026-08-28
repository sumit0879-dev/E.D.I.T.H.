import React, { useState, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import type { CustomApp, Note, CustomProvider, ProviderModel, ProviderDef } from '../types';
import {
  Sliders,
  Key,
  Cpu,
  Volume2,
  User,
  AppWindow,
  FileText,
  Save,
  Trash2,
  Play,
  Square,
  Plus,
  Loader2,
  Eye,
  EyeOff,
  Zap,
  ExternalLink,
  ShieldCheck,
  Check,
  Edit2,
  X,
  RefreshCw,
  Globe,
  Sparkles,
  Copy,
  Search,
  CheckSquare,
} from 'lucide-react';

const cloudVoices = [
  { id: 'hi-IN-SwaraNeural', label: 'Hindi - Swara (Female, Natural Hindi)' },
  { id: 'hi-IN-MadhurNeural', label: 'Hindi - Madhur (Male, Natural Hindi)' },
  { id: 'en-IN-NeerjaNeural', label: 'Indian English - Neerja (Female, Accent IN)' },
  { id: 'en-IN-PrabhatNeural', label: 'Indian English - Prabhat (Male, Accent IN)' },
  { id: 'en-US-JennyNeural', label: 'US English - Jenny (Female, Studio Quality)' },
  { id: 'en-US-GuyNeural', label: 'US English - Guy (Male, Studio Quality)' },
];

const tempPresets = [
  { label: 'Code & Math', value: '0.2', desc: 'Deterministic & precise' },
  { label: 'Balanced', value: '0.7', desc: 'Standard conversational' },
  { label: 'Creative', value: '1.0', desc: 'Brainstorming & writing' },
  { label: 'Exploratory', value: '1.3', desc: 'Maximum novelty' },
];

export const SettingsView: React.FC = () => {
  const {
    settings,
    updateSetting,
    providers,
    updateProviderModels,
    customProviders,
    addCustomProvider,
    updateCustomProvider,
    deleteCustomProvider,
    showToast,
    speakText,
  } = useApp();

  const [activeSubTab, setActiveSubTab] = useState<'ai' | 'keys' | 'local' | 'tts' | 'profile' | 'apps' | 'notes'>('ai');

  // Auto-Fetch & Dynamic Model Selection Modal state
  const [showFetchModal, setShowFetchModal] = useState(false);
  const [fetchingProvider, setFetchingProvider] = useState<ProviderDef | null>(null);
  const [fetchedModelsList, setFetchedModelsList] = useState<ProviderModel[]>([]);
  const [selectedModelIdsToImport, setSelectedModelIdsToImport] = useState<string[]>([]);
  const [fetchSearchFilter, setFetchSearchFilter] = useState('');
  const [isFetchingActiveModels, setIsFetchingActiveModels] = useState(false);

  // Custom Provider Modal state
  const [showProviderModal, setShowProviderModal] = useState(false);
  const [editingProviderId, setEditingProviderId] = useState<string | null>(null);
  const [formName, setFormName] = useState('');
  const [formBaseUrl, setFormBaseUrl] = useState('');
  const [formApiKey, setFormApiKey] = useState('');
  const [formModels, setFormModels] = useState<ProviderModel[]>([]);
  const [newModelId, setNewModelId] = useState('');
  const [newModelLabel, setNewModelLabel] = useState('');
  const [isFetchingModels, setIsFetchingModels] = useState(false);

  // Inline Quick Add Model for active provider (Groq, Gemini, Custom)
  const [quickModelId, setQuickModelId] = useState('');
  const [quickModelLabel, setQuickModelLabel] = useState('');
  const [showQuickAddModal, setShowQuickAddModal] = useState(false);

  // Custom apps
  const [customApps, setCustomApps] = useState<CustomApp[]>([]);
  const [newAppName, setNewAppName] = useState('');
  const [newAppPath, setNewAppPath] = useState('');
  const [newAppKeywords, setNewAppKeywords] = useState('');

  // Personal notes
  const [personalNotes, setPersonalNotes] = useState<Note[]>([]);
  const [noteContent, setNoteContent] = useState('');

  // Local model loading
  const [localModelPath, setLocalModelPath] = useState('Models/llama-3.1-8b-instruct.Q4_K_M.gguf');
  const [localLoadMode, setLocalLoadMode] = useState('ram');
  const [localLoadingStatus, setLocalLoadingStatus] = useState<string | null>(null);
  const [isStartingServer, setIsStartingServer] = useState(false);

  // Kokoro local models
  const [kokoroModels, setKokoroModels] = useState<string[]>([]);
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  // Debounced Profile Fields state (DEF-03)
  const [profileNickname, setProfileNickname] = useState(settings.nickname || '');
  const [profileOccupation, setProfileOccupation] = useState(settings.occupation || '');
  const [profileAbout, setProfileAbout] = useState(settings.moreAboutYou || '');
  const [profileInstructions, setProfileInstructions] = useState(settings.customInstructions || '');

  useEffect(() => {
    setProfileNickname(settings.nickname || '');
  }, [settings.nickname]);

  useEffect(() => {
    setProfileOccupation(settings.occupation || '');
  }, [settings.occupation]);

  useEffect(() => {
    setProfileAbout(settings.moreAboutYou || '');
  }, [settings.moreAboutYou]);

  useEffect(() => {
    setProfileInstructions(settings.customInstructions || '');
  }, [settings.customInstructions]);

  const debouncedProfileTimerRef = React.useRef<Record<string, any>>({});

  const handleProfileFieldChange = (key: string, value: string, setter: (v: string) => void) => {
    setter(value);
    if (debouncedProfileTimerRef.current[key]) {
      clearTimeout(debouncedProfileTimerRef.current[key]);
    }
    debouncedProfileTimerRef.current[key] = setTimeout(() => {
      updateSetting(key, value);
      delete debouncedProfileTimerRef.current[key];
    }, 300);
  };

  const handleProfileFieldBlur = (key: string, value: string) => {
    if (debouncedProfileTimerRef.current[key]) {
      clearTimeout(debouncedProfileTimerRef.current[key]);
      delete debouncedProfileTimerRef.current[key];
    }
    updateSetting(key, value);
  };

  useEffect(() => {
    loadCustomApps();
    loadPersonalNotes();
    tauriService.getKokoroModels().then(setKokoroModels).catch(() => {});

    const unlistenProgress = tauriService.onModelProgress((msg) => {
      setLocalLoadingStatus(msg);
    });

    return () => {
      unlistenProgress.then((unlisten) => unlisten());
    };
  }, []);

  const loadCustomApps = async () => {
    try {
      const apps = await tauriService.getCustomApps();
      setCustomApps(apps);
    } catch (e) {
      console.error(e);
    }
  };

  const loadPersonalNotes = async () => {
    try {
      const savedLocal = localStorage.getItem('miko_quick_notes');
      if (savedLocal) {
        setNoteContent(savedLocal);
      }
      const notes = await tauriService.getPersonalNotes();
      setPersonalNotes(notes);
      if (notes.length > 0 && !savedLocal) {
        setNoteContent(notes[0].content);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleNoteChange = (val: string) => {
    setNoteContent(val);
    try {
      localStorage.setItem('miko_quick_notes', val);
    } catch {}
  };

  const handleCopyApiKey = async (keyName: string, value: string) => {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      setCopiedKey(keyName);
      setTimeout(() => setCopiedKey(null), 2000);
      showToast('API Key copied to clipboard', 'info');
    } catch (e) {
      console.error('Failed to copy key:', e);
    }
  };

  const handleAddCustomApp = async () => {
    if (!newAppName.trim() || !newAppPath.trim()) return;
    try {
      await tauriService.addCustomApp(
        newAppName.trim(),
        newAppPath.trim(),
        newAppKeywords.trim() || newAppName.trim()
      );
      setNewAppName('');
      setNewAppPath('');
      setNewAppKeywords('');
      showToast('Custom app registered!', 'success');
      loadCustomApps();
    } catch (e: any) {
      showToast('Add app error: ' + (e.message || e), 'error');
    }
  };

  const handleDeleteCustomApp = async (id: number) => {
    try {
      await tauriService.deleteCustomApp(id);
      showToast('App deleted from registry', 'info');
      loadCustomApps();
    } catch (e: any) {
      showToast('Delete app error: ' + (e.message || e), 'error');
    }
  };

  const handleSaveNote = async () => {
    try {
      await tauriService.savePersonalNote(noteContent);
      showToast('Notes saved!', 'success');
      loadPersonalNotes();
    } catch (e: any) {
      showToast('Note save error: ' + (e.message || e), 'error');
    }
  };

  const handleStartLocalServer = async () => {
    try {
      setIsStartingServer(true);
      setLocalLoadingStatus('Starting llama-server background process...');
      await tauriService.loadLocalLlm(localModelPath, localLoadMode);
      setLocalLoadingStatus('Server is online and ready on http://127.0.0.1:11434!');
      showToast('Local LLM server started successfully!', 'success');
    } catch (e: any) {
      setLocalLoadingStatus('Error: ' + (e.message || e));
      showToast('Local LLM error: ' + (e.message || e), 'error');
    } finally {
      setIsStartingServer(false);
    }
  };

  const handleStopLocalServer = async () => {
    try {
      await tauriService.stopLocalLlm();
      setLocalLoadingStatus('Server stopped.');
      showToast('Local llama server stopped', 'info');
    } catch (e: any) {
      showToast('Stop error: ' + (e.message || e), 'error');
    }
  };

  const activeProviderId = settings.selectedProvider || 'groq';
  const effectiveProviders = providers.length > 0 ? providers : tauriService.DEFAULT_PROVIDERS;
  const currentProvider = effectiveProviders.find((p) => p.id === activeProviderId) || effectiveProviders[0];

  const hasApiKey = (provId: string) => {
    const cp = customProviders.find((p) => p.id === provId);
    if (cp && cp.apiKey && cp.apiKey.trim().length > 0) return true;
    const key = settings['apiKey_' + provId] || settings['apiKey'] || '';
    return key.trim().length > 0;
  };

  // Auto-Fetch Models for any provider (Groq, Gemini, Custom)
  const openAutoFetchModalForProvider = async (prov?: ProviderDef) => {
    const targetProv = prov || currentProvider;
    let baseUrl = targetProv.baseUrl || '';
    let apiKey = '';

    if (targetProv.id === 'groq') {
      baseUrl = 'https://api.groq.com/openai/v1';
      apiKey = settings.apiKey_groq || settings.apiKey || '';
    } else if (targetProv.id === 'gemini') {
      baseUrl = 'gemini';
      apiKey = settings.apiKey_gemini || settings.apiKey || '';
    } else if (targetProv.isCustom) {
      baseUrl = targetProv.baseUrl || '';
      apiKey = targetProv.apiKey || settings['apiKey_' + targetProv.id] || settings.apiKey || '';
    }

    if (!apiKey && targetProv.id !== 'local') {
      showToast(`Please enter your ${targetProv.name} API Key first in API Keys Suite.`, 'warning');
      setActiveSubTab('keys');
      return;
    }

    try {
      setIsFetchingActiveModels(true);
      const fetched = await tauriService.fetchCustomModels(baseUrl, apiKey || undefined);
      if (!fetched || fetched.length === 0) {
        showToast('No models returned from server.', 'warning');
        return;
      }

      setFetchingProvider(targetProv);
      setFetchedModelsList(fetched);

      // Pre-select models that are not already in current provider's model list, or select all if all exist
      const existingIds = new Set((targetProv.models || []).map((m: ProviderModel) => m.id));
      const notAdded = fetched.filter((m: ProviderModel) => !existingIds.has(m.id)).map((m: ProviderModel) => m.id);
      setSelectedModelIdsToImport(notAdded.length > 0 ? notAdded : fetched.map((m: ProviderModel) => m.id));

      setFetchSearchFilter('');
      setShowFetchModal(true);
      showToast(`Fetched ${fetched.length} available models from ${targetProv.name}`, 'info');
    } catch (err: any) {
      showToast('Failed to fetch models: ' + (err.message || err), 'error');
    } finally {
      setIsFetchingActiveModels(false);
    }
  };

  const handleToggleModelSelection = (modelId: string) => {
    setSelectedModelIdsToImport((prev) =>
      prev.includes(modelId) ? prev.filter((id) => id !== modelId) : [...prev, modelId]
    );
  };

  const handleSelectAllFetchedModels = (filteredIds: string[]) => {
    const allSelected = filteredIds.every((id) => selectedModelIdsToImport.includes(id));
    if (allSelected) {
      setSelectedModelIdsToImport((prev) => prev.filter((id) => !filteredIds.includes(id)));
    } else {
      setSelectedModelIdsToImport((prev) => Array.from(new Set([...prev, ...filteredIds])));
    }
  };

  const handleImportSelectedModels = async () => {
    if (!fetchingProvider) return;
    const selectedModels = fetchedModelsList.filter((m: ProviderModel) =>
      selectedModelIdsToImport.includes(m.id)
    );

    if (selectedModels.length === 0) {
      showToast('Please select at least one model to import', 'warning');
      return;
    }

    const existingModels = fetchingProvider.models || [];
    const existingIds = new Set(existingModels.map((m: ProviderModel) => m.id));
    const merged = [...existingModels];

    selectedModels.forEach((m: ProviderModel) => {
      if (!existingIds.has(m.id)) {
        merged.push(m);
        existingIds.add(m.id);
      }
    });

    await updateProviderModels(fetchingProvider.id, merged);

    if (!settings.selectedModel || !merged.some((m: ProviderModel) => m.id === settings.selectedModel)) {
      await updateSetting('selectedModel', merged[0].id);
    }

    showToast(`Added ${selectedModels.length} models to ${fetchingProvider.name}!`, 'success');
    setShowFetchModal(false);
  };

  // Custom Provider Modal handlers
  const openAddProviderModal = () => {
    setEditingProviderId(null);
    setFormName('');
    setFormBaseUrl('');
    setFormApiKey('');
    setFormModels([]);
    setNewModelId('');
    setNewModelLabel('');
    setShowProviderModal(true);
  };

  const openEditProviderModal = (cp: CustomProvider) => {
    setEditingProviderId(cp.id);
    setFormName(cp.name);
    setFormBaseUrl(cp.baseUrl);
    setFormApiKey(cp.apiKey || '');
    setFormModels([...cp.models]);
    setNewModelId('');
    setNewModelLabel('');
    setShowProviderModal(true);
  };

  const handleAutoFetchModels = async () => {
    if (!formBaseUrl.trim()) {
      showToast('Please enter the Base URL first', 'warning');
      return;
    }
    try {
      setIsFetchingModels(true);
      const fetched = await tauriService.fetchCustomModels(formBaseUrl.trim(), formApiKey.trim() || undefined);
      if (!fetched || fetched.length === 0) {
        showToast('No models returned from server.', 'warning');
        return;
      }
      // Merge unique
      const existingIds = new Set(formModels.map((m) => m.id));
      const combined = [...formModels];
      fetched.forEach((m) => {
        if (!existingIds.has(m.id)) {
          combined.push(m);
          existingIds.add(m.id);
        }
      });
      setFormModels(combined);
      showToast(`Fetched ${fetched.length} models successfully!`, 'success');
    } catch (err: any) {
      showToast('Failed to fetch models: ' + (err.message || err), 'error');
    } finally {
      setIsFetchingModels(false);
    }
  };

  const handleAddManualModel = () => {
    if (!newModelId.trim()) return;
    const id = newModelId.trim();
    const label = newModelLabel.trim() || id;
    if (formModels.some((m) => m.id === id)) {
      showToast('Model ID already exists', 'warning');
      return;
    }
    setFormModels((prev) => [...prev, { id, label }]);
    setNewModelId('');
    setNewModelLabel('');
  };

  const handleRemoveModelFromForm = (modelId: string) => {
    setFormModels((prev) => prev.filter((m) => m.id !== modelId));
  };

  const handleSaveCustomProvider = async () => {
    if (!formName.trim() || !formBaseUrl.trim()) {
      showToast('Provider name and Base URL are required', 'warning');
      return;
    }

    let modelsToSave = [...formModels];
    if (modelsToSave.length === 0) {
      modelsToSave = [{ id: 'default-model', label: 'Default Model' }];
    }

    const id = editingProviderId || ('custom_' + Date.now());
    const providerObj: CustomProvider = {
      id,
      name: formName.trim(),
      baseUrl: formBaseUrl.trim(),
      apiKey: formApiKey.trim() || undefined,
      models: modelsToSave,
    };

    if (editingProviderId) {
      await updateCustomProvider(providerObj);
    } else {
      await addCustomProvider(providerObj);
      await updateSetting('selectedProvider', id);
      if (modelsToSave.length > 0) {
        await updateSetting('selectedModel', modelsToSave[0].id);
      }
    }

    if (formApiKey.trim()) {
      await updateSetting('apiKey_' + id, formApiKey.trim());
    }

    setShowProviderModal(false);
  };

  // Quick inline add custom model to active provider (works for Groq, Gemini, or Custom)
  const handleQuickAddModel = async () => {
    if (!quickModelId.trim()) return;
    const id = quickModelId.trim();
    const label = quickModelLabel.trim() || id;
    const currentModels = currentProvider.models || [];
    if (currentModels.some((m) => m.id === id)) {
      showToast('Model ID already exists in this provider', 'warning');
      return;
    }
    const updatedModels = [...currentModels, { id, label }];
    await updateProviderModels(currentProvider.id, updatedModels);
    await updateSetting('selectedModel', id);
    setQuickModelId('');
    setQuickModelLabel('');
    setShowQuickAddModal(false);
    showToast(`Model "${label}" added to ${currentProvider.name}`, 'success');
  };

  // Delete model from active provider (works for Groq, Gemini, or Custom)
  const handleDeleteModelFromActiveProvider = async (modelId: string) => {
    const currentModels = currentProvider.models || [];
    if (currentModels.length <= 1) {
      showToast('Provider must have at least one model checkpoint', 'warning');
      return;
    }
    const updatedModels = currentModels.filter((m) => m.id !== modelId);
    await updateProviderModels(currentProvider.id, updatedModels);
    if (settings.selectedModel === modelId && updatedModels.length > 0) {
      await updateSetting('selectedModel', updatedModels[0].id);
    }
    showToast('Model removed from provider list', 'info');
  };

  return (
    <div className="flex-1 flex h-full bg-[#000000] overflow-hidden">
      {/* Settings Navigation Sidebar */}
      <div className="w-64 bg-[#0a0f1c] border-r border-white/10 p-3 space-y-1 overflow-y-auto custom-scrollbar shrink-0 select-none">
        <div className="text-xs font-bold uppercase tracking-wider text-slate-500 px-3 py-2">
          E.D.I.T.H. Config Suite
        </div>

        <button
          onClick={() => setActiveSubTab('ai')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'ai'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <Cpu className="w-4 h-4 text-cyan-400" />
          <span>AI Models & Providers</span>
        </button>

        <button
          onClick={() => setActiveSubTab('keys')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'keys'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <Key className="w-4 h-4 text-cyan-400" />
          <span>API Keys Suite</span>
        </button>

        <button
          onClick={() => setActiveSubTab('local')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'local'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <Sliders className="w-4 h-4 text-cyan-400" />
          <span>Local GGUF (Llama Server)</span>
        </button>

        <button
          onClick={() => setActiveSubTab('tts')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'tts'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <Volume2 className="w-4 h-4 text-cyan-400" />
          <span>Voice & Speech Synthesis</span>
        </button>

        <button
          onClick={() => setActiveSubTab('profile')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'profile'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <User className="w-4 h-4 text-cyan-400" />
          <span>Profile & Instructions</span>
        </button>

        <button
          onClick={() => setActiveSubTab('apps')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'apps'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <AppWindow className="w-4 h-4 text-cyan-400" />
          <span>Custom Apps Registry</span>
        </button>

        <button
          onClick={() => setActiveSubTab('notes')}
          className={
            'w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl text-xs font-medium transition ' +
            (activeSubTab === 'notes'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 font-bold'
              : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
          }
        >
          <FileText className="w-4 h-4 text-cyan-400" />
          <span>Personal Notes</span>
        </button>
      </div>

      {/* Main Settings Content Area */}
      <div className="flex-1 overflow-y-auto custom-scrollbar p-8 pb-32 max-w-5xl space-y-6">
        {/* SUBTAB 1: AI Models & Providers */}
        {activeSubTab === 'ai' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between border-b border-white/10 pb-4">
              <div>
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <Cpu className="w-5 h-5 text-cyan-400" />
                  <span>AI Engine & Provider Selection</span>
                </h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  Select between Groq, Google Gemini, Custom OpenAI-compatible endpoints, or local GGUF
                </p>
              </div>

              {/* Mode toggle */}
              <div className="flex items-center p-1 rounded-xl bg-black/40 border border-white/10">
                <button
                  onClick={() => updateSetting('aiMode', 'api')}
                  className={
                    'px-4 py-1.5 rounded-lg text-xs font-bold transition ' +
                    (settings.aiMode !== 'local'
                      ? 'bg-cyan-500 text-slate-950 shadow-md shadow-cyan-500/20'
                      : 'text-slate-400 hover:text-slate-200')
                  }
                >
                  Cloud / Custom API
                </button>
                <button
                  onClick={() => updateSetting('aiMode', 'local')}
                  className={
                    'px-4 py-1.5 rounded-lg text-xs font-bold transition ' +
                    (settings.aiMode === 'local'
                      ? 'bg-cyan-500 text-slate-950 shadow-md shadow-cyan-500/20'
                      : 'text-slate-400 hover:text-slate-200')
                  }
                >
                  Local GGUF
                </button>
              </div>
            </div>

            {settings.aiMode !== 'local' ? (
              <div className="space-y-6">
                {/* Visual Provider Cards Grid */}
                <div>
                  <div className="flex items-center justify-between mb-3">
                    <label className="text-xs font-bold uppercase tracking-wider text-slate-400">
                      Active Providers ({effectiveProviders.length})
                    </label>

                    <button
                      onClick={openAddProviderModal}
                      className="px-3 py-1.5 rounded-xl bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 text-white text-xs font-bold flex items-center gap-1.5 shadow-md shadow-cyan-500/20 transition"
                    >
                      <Plus className="w-3.5 h-3.5" />
                      <span>Add Custom Provider</span>
                    </button>
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3">
                    {effectiveProviders.map((prov) => {
                      const isSelected = activeProviderId === prov.id;
                      const hasKey = hasApiKey(prov.id);
                      const isCustom = !!prov.isCustom;
                      const customObj = customProviders.find((c) => c.id === prov.id);

                      return (
                        <div
                          key={prov.id}
                          onClick={() => {
                            updateSetting('selectedProvider', prov.id);
                            if (prov.models && prov.models.length > 0) {
                              updateSetting('selectedModel', prov.models[0].id);
                            }
                          }}
                          className={
                            'p-4 rounded-2xl text-left transition border flex flex-col justify-between cursor-pointer group relative ' +
                            (isSelected
                              ? 'bg-gradient-to-br from-cyan-950/70 via-[#0a1526] to-blue-950/60 border-cyan-500 shadow-xl shadow-cyan-500/10'
                              : 'bg-[#0f172a]/60 border-white/5 hover:border-white/20 hover:bg-white/[0.04]')
                          }
                        >
                          <div>
                            <div className="flex items-center justify-between mb-2">
                              <span className={'text-xs font-bold truncate mr-2 ' + (isSelected ? 'text-cyan-300' : 'text-white')}>
                                {prov.name}
                              </span>

                              <div className="flex items-center gap-1.5 shrink-0">
                                {isCustom ? (
                                  <span className="text-[9px] font-bold text-violet-300 bg-violet-950/60 px-1.5 py-0.5 rounded border border-violet-500/30">
                                    CUSTOM
                                  </span>
                                ) : (
                                  <span className="text-[9px] font-bold text-sky-400 bg-sky-950/60 px-1.5 py-0.5 rounded border border-sky-500/30">
                                    BUILT-IN
                                  </span>
                                )}

                                {hasKey ? (
                                  <span className="flex items-center gap-0.5 text-[9px] font-bold text-emerald-400 bg-emerald-950/60 px-1.5 py-0.5 rounded border border-emerald-500/30" title="API Key Set">
                                    <ShieldCheck className="w-2.5 h-2.5" />
                                  </span>
                                ) : null}
                              </div>
                            </div>

                            <p className="text-[11px] text-slate-400 leading-snug line-clamp-1 font-mono">
                              {prov.baseUrl || (prov.id === 'groq' ? 'https://api.groq.com/openai/v1' : 'Google Gemini API')}
                            </p>
                          </div>

                          <div className="mt-3 pt-2 border-t border-white/5 flex items-center justify-between text-[10px] text-slate-400">
                            <span className="font-mono">{(prov.models || []).length} models</span>

                            <div className="flex items-center gap-1">
                              {isCustom && customObj && (
                                <>
                                  <button
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      openEditProviderModal(customObj);
                                    }}
                                    className="p-1 rounded hover:bg-white/10 text-slate-400 hover:text-cyan-300 transition"
                                    title="Edit Provider"
                                  >
                                    <Edit2 className="w-3 h-3" />
                                  </button>

                                  <button
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      deleteCustomProvider(prov.id);
                                    }}
                                    className="p-1 rounded hover:bg-rose-500/20 text-slate-400 hover:text-rose-300 transition"
                                    title="Delete Provider"
                                  >
                                    <Trash2 className="w-3 h-3" />
                                  </button>
                                </>
                              )}

                              {isSelected && (
                                <div className="w-4 h-4 rounded-full bg-cyan-500 flex items-center justify-center ml-1">
                                  <Check className="w-2.5 h-2.5 text-slate-950 font-bold" />
                                </div>
                              )}
                            </div>
                          </div>
                        </div>
                      );
                    })}

                    {/* Add Custom Provider Card */}
                    <button
                      onClick={openAddProviderModal}
                      className="p-4 rounded-2xl border border-dashed border-white/20 hover:border-cyan-500/60 bg-white/[0.01] hover:bg-cyan-500/5 transition flex flex-col items-center justify-center text-center group min-h-[105px]"
                    >
                      <Plus className="w-5 h-5 text-slate-400 group-hover:text-cyan-400 group-hover:scale-110 transition" />
                      <span className="text-xs font-bold text-slate-300 group-hover:text-cyan-300 mt-1">
                        Add Custom Provider
                      </span>
                      <span className="text-[10px] text-slate-500 mt-0.5">
                        OpenAI, DeepSeek, Ollama, LM Studio...
                      </span>
                    </button>
                  </div>
                </div>

                {/* Model Selector Cards for Active Provider */}
                <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-4">
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                    <div>
                      <h4 className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-2">
                        <span>{currentProvider.name} Models</span>
                        {currentProvider.isCustom ? (
                          <span className="text-[9px] px-1.5 py-0.2 rounded bg-violet-500/20 text-violet-300 border border-violet-500/30">
                            CUSTOM
                          </span>
                        ) : (
                          <span className="text-[9px] px-1.5 py-0.2 rounded bg-cyan-500/20 text-cyan-300 border border-cyan-500/30">
                            BUILT-IN
                          </span>
                        )}
                      </h4>
                      <p className="text-[11px] text-slate-400">Select active model checkpoint for chat</p>
                    </div>

                    <div className="flex items-center flex-wrap gap-2">
                      <button
                        onClick={() => openAutoFetchModalForProvider(currentProvider)}
                        disabled={isFetchingActiveModels}
                        className="px-3 py-1.5 rounded-xl bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 text-xs font-bold hover:bg-cyan-500/30 transition flex items-center gap-1.5 shadow-sm disabled:opacity-50"
                        title="Auto-fetch available models from provider API"
                      >
                        {isFetchingActiveModels ? (
                          <Loader2 className="w-3.5 h-3.5 animate-spin" />
                        ) : (
                          <RefreshCw className="w-3.5 h-3.5" />
                        )}
                        <span>Auto-Fetch Models</span>
                      </button>

                      <button
                        onClick={() => setShowQuickAddModal(!showQuickAddModal)}
                        className="px-3 py-1.5 rounded-xl bg-white/[0.04] text-slate-200 border border-white/10 text-xs font-semibold hover:bg-white/[0.08] hover:text-white transition flex items-center gap-1.5"
                        title="Add custom model manually"
                      >
                        <Plus className="w-3.5 h-3.5 text-cyan-400" />
                        <span>Add Custom Model</span>
                      </button>

                      {!hasApiKey(currentProvider.id) && (
                        <button
                          onClick={() => setActiveSubTab('keys')}
                          className="px-3 py-1.5 rounded-xl bg-amber-500/20 text-amber-300 border border-amber-500/30 text-xs font-medium hover:bg-amber-500/30 transition flex items-center gap-1.5"
                        >
                          <Key className="w-3.5 h-3.5" />
                          <span>Enter Key</span>
                        </button>
                      )}
                    </div>
                  </div>

                  {/* Inline Quick Model Add Input */}
                  {showQuickAddModal && (
                    <div className="p-3.5 rounded-2xl bg-black/50 border border-cyan-500/30 space-y-2.5 animate-slide-in">
                      <div className="text-[11px] font-bold text-cyan-300 flex items-center gap-1.5">
                        <Plus className="w-3.5 h-3.5" />
                        <span>Add New Model Checkpoint to {currentProvider.name}</span>
                      </div>
                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                        <input
                          type="text"
                          placeholder="Model ID (e.g. llama-3.3-70b-specdec, gemini-exp-1206)"
                          value={quickModelId}
                          onChange={(e) => setQuickModelId(e.target.value)}
                          className="px-3 py-1.5 rounded-xl glass-input text-xs font-mono text-slate-200"
                        />
                        <input
                          type="text"
                          placeholder="Friendly Label (optional)"
                          value={quickModelLabel}
                          onChange={(e) => setQuickModelLabel(e.target.value)}
                          className="px-3 py-1.5 rounded-xl glass-input text-xs text-slate-200"
                        />
                      </div>
                      <div className="flex justify-end gap-2 pt-1">
                        <button
                          onClick={() => setShowQuickAddModal(false)}
                          className="px-3 py-1 text-xs text-slate-400 hover:text-slate-200"
                        >
                          Cancel
                        </button>
                        <button
                          onClick={handleQuickAddModel}
                          disabled={!quickModelId.trim()}
                          className="px-3.5 py-1.5 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-bold transition disabled:opacity-50"
                        >
                          Add & Select
                        </button>
                      </div>
                    </div>
                  )}

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
                    {(currentProvider.models || []).length === 0 ? (
                      <div className="col-span-2 p-6 text-center text-xs text-slate-500 border border-dashed border-white/10 rounded-xl">
                        No models added for this provider. Click "Auto-Fetch Models" or "Add Custom Model" above.
                      </div>
                    ) : (
                      currentProvider.models.map((model) => {
                        const isModelSelected = settings.selectedModel === model.id;

                        return (
                          <div
                            key={model.id}
                            onClick={() => updateSetting('selectedModel', model.id)}
                            className={
                              'p-3 rounded-xl text-left transition border flex items-center justify-between cursor-pointer group ' +
                              (isModelSelected
                                ? 'bg-cyan-500/20 border-cyan-500/60 text-white shadow-md'
                                : 'bg-white/[0.02] border-white/5 text-slate-300 hover:bg-white/[0.05]')
                            }
                          >
                            <div className="truncate mr-2 flex-1">
                              <div className="text-xs font-bold truncate text-slate-100">{model.label}</div>
                              <div className="text-[10px] text-slate-500 font-mono truncate">{model.id}</div>
                            </div>

                            <div className="flex items-center gap-1.5 shrink-0">
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleDeleteModelFromActiveProvider(model.id);
                                }}
                                className="p-1 rounded text-slate-500 hover:text-rose-400 opacity-0 group-hover:opacity-100 transition"
                                title="Remove model from list"
                              >
                                <Trash2 className="w-3.5 h-3.5" />
                              </button>

                              {isModelSelected && (
                                <div className="w-5 h-5 rounded-full bg-cyan-500 flex items-center justify-center shrink-0">
                                  <Check className="w-3 h-3 text-slate-950 font-bold" />
                                </div>
                              )}
                            </div>
                          </div>
                        );
                      })
                    )}
                  </div>
                </div>

                {/* Creativity / Temperature Presets */}
                <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-4">
                  <div className="flex justify-between items-center">
                    <div>
                      <h4 className="text-xs font-bold uppercase tracking-wider text-slate-300">
                        Model Temperature ({settings.temperature || '0.7'})
                      </h4>
                      <p className="text-[11px] text-slate-400">Controls randomness and creativity of responses</p>
                    </div>

                    <div className="flex gap-1.5">
                      {tempPresets.map((tp) => (
                        <button
                          key={tp.value}
                          onClick={() => updateSetting('temperature', tp.value)}
                          className={
                            'px-3 py-1 rounded-lg text-xs font-medium transition border ' +
                            ((settings.temperature || '0.7') === tp.value
                              ? 'bg-cyan-500/20 border-cyan-500 text-cyan-300 font-bold'
                              : 'bg-white/[0.02] border-white/5 text-slate-400 hover:text-slate-200')
                          }
                        >
                          {tp.label}
                        </button>
                      ))}
                    </div>
                  </div>

                  <input
                    type="range"
                    min="0.1"
                    max="1.5"
                    step="0.05"
                    value={settings.temperature || '0.7'}
                    onChange={(e) => updateSetting('temperature', e.target.value)}
                    className="w-full accent-cyan-500 h-1.5 bg-slate-800 rounded-lg cursor-pointer"
                  />
                </div>
              </div>
            ) : (
              /* Local GGUF Card */
              <div className="p-6 rounded-2xl glass-card border border-white/10 space-y-4">
                <div className="flex items-center gap-3">
                  <div className="p-3 rounded-xl bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                    <Sliders className="w-6 h-6" />
                  </div>
                  <div>
                    <h4 className="text-sm font-bold text-white">Local Llama-Server Offline Inference</h4>
                    <p className="text-xs text-slate-400">Run local .gguf weights without sending queries to the cloud</p>
                  </div>
                </div>
                <button
                  onClick={() => setActiveSubTab('local')}
                  className="px-4 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-bold transition"
                >
                  Configure Local Model & RAM Allocator →
                </button>
              </div>
            )}
          </div>
        )}

        {/* SUBTAB 2: API Keys */}
        {activeSubTab === 'keys' && (
          <div className="space-y-6">
            <div className="border-b border-white/10 pb-4">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <Key className="w-5 h-5 text-cyan-400" />
                <span>Provider API Keys Suite</span>
              </h3>
              <p className="text-xs text-slate-400 mt-0.5">
                Keys are stored securely in local SQLite database (`edith_memory.db`) and never transmitted elsewhere.
              </p>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5">
              {[
                { key: 'apiKey_groq', label: 'Groq Cloud Key', placeholder: 'gsk_...', link: 'https://console.groq.com' },
                { key: 'apiKey_gemini', label: 'Google Gemini API Key', placeholder: 'AIzaSy...', link: 'https://aistudio.google.com' },
                { key: 'tavilyApiKey', label: 'Tavily Deep Web Search Key', placeholder: 'tvly-...', link: 'https://tavily.com' },
                { key: 'huggingfaceApiKey', label: 'Hugging Face (SDXL) Key', placeholder: 'hf_...', link: 'https://huggingface.co/settings/tokens' },
                ...customProviders.map((cp) => ({
                  key: 'apiKey_' + cp.id,
                  label: `${cp.name} API Key`,
                  placeholder: 'Optional API Key...',
                  link: cp.baseUrl,
                })),
              ].map((item) => (
                <form
                  key={item.key}
                  onSubmit={(e) => e.preventDefault()}
                  aria-label={item.label}
                  className="p-4 rounded-2xl glass-card border border-white/10 space-y-2"
                >
                  <div className="flex justify-between items-center text-xs font-semibold text-slate-200">
                    <label htmlFor={item.key} className="flex items-center gap-1.5 cursor-pointer">
                      <Key className="w-3.5 h-3.5 text-cyan-400" />
                      <span>{item.label}</span>
                    </label>
                    <div className="flex items-center gap-2">
                      {item.link && item.link.startsWith('http') && (
                        <a
                          href={item.link}
                          target="_blank"
                          rel="noreferrer"
                          className="text-[10px] text-cyan-400 hover:underline flex items-center gap-1"
                        >
                          <span>Endpoint</span>
                          <ExternalLink className="w-2.5 h-2.5" />
                        </a>
                      )}
                      <button
                        type="button"
                        onClick={() => handleCopyApiKey(item.key, settings[item.key] || '')}
                        disabled={!(settings[item.key] || '').trim()}
                        className="text-slate-400 hover:text-cyan-400 p-0.5 disabled:opacity-30 disabled:cursor-not-allowed transition"
                        title="Copy Key"
                        aria-label={`Copy ${item.label}`}
                      >
                        {copiedKey === item.key ? (
                          <Check className="w-3.5 h-3.5 text-emerald-400" />
                        ) : (
                          <Copy className="w-3.5 h-3.5" />
                        )}
                      </button>
                      <button
                        type="button"
                        onClick={() => setShowKeys((prev) => ({ ...prev, [item.key]: !prev[item.key] }))}
                        className="text-slate-400 hover:text-slate-200 p-0.5 transition"
                        title={showKeys[item.key] ? 'Hide Key' : 'Show Key'}
                        aria-label={showKeys[item.key] ? `Hide ${item.label}` : `Show ${item.label}`}
                      >
                        {showKeys[item.key] ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
                      </button>
                    </div>
                  </div>

                  <input
                    id={item.key}
                    type={showKeys[item.key] ? 'text' : 'password'}
                    value={settings[item.key] || ''}
                    onChange={(e) => updateSetting(item.key, e.target.value)}
                    placeholder={item.placeholder}
                    aria-label={item.label}
                    autoComplete="off"
                    className="w-full px-3 py-2 rounded-xl glass-input text-xs font-mono text-cyan-300 placeholder-slate-600 focus:outline-none"
                  />
                </form>
              ))}
            </div>
          </div>
        )}

        {/* SUBTAB 3: Local GGUF Server */}
        {activeSubTab === 'local' && (
          <div className="space-y-6">
            <div className="border-b border-white/10 pb-4">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <Sliders className="w-5 h-5 text-cyan-400" />
                <span>Local llama-server Process Controller</span>
              </h3>
              <p className="text-xs text-slate-400 mt-0.5">
                Spawns the bundled `llama-server.exe` directly on `http://127.0.0.1:11434`
              </p>
            </div>

            <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-4">
              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1.5">
                  GGUF Model File Path (e.g. Models/llama-3.1-8b-instruct.Q4_K_M.gguf)
                </label>
                <input
                  type="text"
                  value={localModelPath}
                  onChange={(e) => setLocalModelPath(e.target.value)}
                  placeholder="e.g. Models/llama-3.1-8b-instruct.Q4_K_M.gguf"
                  className="w-full px-3 py-2 rounded-xl glass-input text-xs font-mono text-slate-200"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1.5">
                  RAM Allocation Mode
                </label>
                <div className="grid grid-cols-2 gap-3">
                  <button
                    onClick={() => setLocalLoadMode('ram')}
                    className={
                      'p-3 rounded-xl border text-left transition ' +
                      (localLoadMode === 'ram'
                        ? 'bg-cyan-500/20 border-cyan-500 text-white'
                        : 'bg-white/[0.02] border-white/10 text-slate-400')
                    }
                  >
                    <div className="text-xs font-bold text-cyan-300">RAM Lock (--mlock)</div>
                    <div className="text-[11px] text-slate-400 mt-0.5">Locks model weights in RAM for fastest inference</div>
                  </button>

                  <button
                    onClick={() => setLocalLoadMode('standard')}
                    className={
                      'p-3 rounded-xl border text-left transition ' +
                      (localLoadMode === 'standard'
                        ? 'bg-cyan-500/20 border-cyan-500 text-white'
                        : 'bg-white/[0.02] border-white/10 text-slate-400')
                    }
                  >
                    <div className="text-xs font-bold text-cyan-300">Standard Paging</div>
                    <div className="text-[11px] text-slate-400 mt-0.5">Allows virtual memory paging for lower RAM PCs</div>
                  </button>
                </div>
              </div>

              <div className="flex gap-3 pt-2">
                <button
                  onClick={handleStartLocalServer}
                  disabled={isStartingServer}
                  className="flex-1 py-3 px-4 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-white text-xs font-bold shadow-lg shadow-cyan-500/30 flex items-center justify-center gap-2 transition disabled:opacity-50"
                >
                  {isStartingServer ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4 fill-current" />}
                  <span>Start Local Server</span>
                </button>

                <button
                  onClick={handleStopLocalServer}
                  className="py-3 px-4 rounded-xl bg-rose-600/20 hover:bg-rose-600/30 text-rose-300 border border-rose-500/30 text-xs font-bold flex items-center gap-2 transition"
                >
                  <Square className="w-4 h-4 fill-current" />
                  <span>Stop Server</span>
                </button>
              </div>

              {localLoadingStatus && (
                <div className="p-3.5 rounded-xl bg-black/80 border border-white/10 font-mono text-xs text-cyan-300 flex items-center gap-2">
                  <Zap className="w-4 h-4 text-cyan-400 animate-pulse" />
                  <span>{localLoadingStatus}</span>
                </div>
              )}
            </div>
          </div>
        )}

        {/* SUBTAB 4: Voice & Speech Synthesis */}
        {activeSubTab === 'tts' && (
          <div className="space-y-6">
            <div className="border-b border-white/10 pb-4">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <Volume2 className="w-5 h-5 text-cyan-400" />
                <span>Text-to-Speech & Speech Synthesis Engines</span>
              </h3>
              <p className="text-xs text-slate-400 mt-0.5">
                Synthesizes spoken audio for assistant responses in natural accents
              </p>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <button
                onClick={() => updateSetting('ttsEngine', 'cloud')}
                className={
                  'p-4 rounded-2xl border text-left transition ' +
                  (settings.ttsEngine !== 'local'
                    ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                    : 'bg-white/[0.03] border-white/10 text-slate-400')
                }
              >
                <div className="font-bold text-sm text-cyan-300">EdgeTTS Cloud Neural</div>
                <div className="text-xs text-slate-400 mt-1">High-quality Indian English & Hindi neural voices</div>
              </button>

              <button
                onClick={() => updateSetting('ttsEngine', 'local')}
                className={
                  'p-4 rounded-2xl border text-left transition ' +
                  (settings.ttsEngine === 'local'
                    ? 'bg-cyan-500/20 border-cyan-500/50 text-white'
                    : 'bg-white/[0.03] border-white/10 text-slate-400')
                }
              >
                <div className="font-bold text-sm text-cyan-300">Kokoro ONNX Local</div>
                <div className="text-xs text-slate-400 mt-1">Offline synthesized ONNX voice engine</div>
              </button>
            </div>

            {settings.ttsEngine !== 'local' ? (
              <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-3">
                <label className="text-xs font-bold uppercase tracking-wider text-slate-300 block">
                  Cloud Voice Models
                </label>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
                  {cloudVoices.map((v) => {
                    const isVoiceSelected = (settings.ttsVoice || 'hi-IN-SwaraNeural') === v.id;
                    return (
                      <button
                        key={v.id}
                        onClick={() => updateSetting('ttsVoice', v.id)}
                        className={
                          'p-3 rounded-xl text-left transition border flex items-center justify-between ' +
                          (isVoiceSelected
                            ? 'bg-cyan-500/20 border-cyan-500/60 text-white'
                            : 'bg-white/[0.02] border-white/5 text-slate-300 hover:bg-white/[0.05]')
                        }
                      >
                        <div className="truncate mr-2">
                          <div className="text-xs font-bold text-slate-100">{v.label}</div>
                          <div className="text-[10px] text-slate-500 font-mono">{v.id}</div>
                        </div>
                        {isVoiceSelected && <Check className="w-4 h-4 text-cyan-400 shrink-0" />}
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : (
              <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-3">
                <label className="text-xs font-bold uppercase tracking-wider text-slate-300 block">
                  Kokoro ONNX Models
                </label>
                <div className="space-y-2">
                  {kokoroModels.map((m) => {
                    const isKokoroSelected = settings.kokoroModel === m;
                    return (
                      <button
                        key={m}
                        onClick={() => updateSetting('kokoroModel', m)}
                        className={
                          'w-full p-3 rounded-xl text-left transition border flex items-center justify-between ' +
                          (isKokoroSelected
                            ? 'bg-cyan-500/20 border-cyan-500/60 text-white'
                            : 'bg-white/[0.02] border-white/5 text-slate-300')
                        }
                      >
                        <span className="font-mono text-xs">{m}</span>
                        {isKokoroSelected && <Check className="w-4 h-4 text-cyan-400" />}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}

            <div className="p-5 rounded-2xl glass-card border border-white/10 flex items-center justify-between">
              <div>
                <div className="text-sm font-bold text-white">Auto-speak Assistant Replies</div>
                <div className="text-xs text-slate-400 mt-0.5">Automatically plays voice audio when E.D.I.T.H. responds</div>
              </div>
              <button
                onClick={() =>
                  updateSetting('autoSpeak', settings.autoSpeak === 'true' ? 'false' : 'true')
                }
                className={
                  'px-4 py-2 rounded-xl text-xs font-bold tracking-wider transition ' +
                  (settings.autoSpeak === 'true'
                    ? 'bg-cyan-500 text-slate-950 shadow-md shadow-cyan-500/20'
                    : 'bg-slate-800 text-slate-400')
                }
              >
                {settings.autoSpeak === 'true' ? 'ACTIVE' : 'OFF'}
              </button>
            </div>

            <button
              onClick={() => speakText('Hello! E.D.I.T.H. voice synthesis is fully operational.')}
              className="px-4 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold border border-white/10 flex items-center gap-2 transition"
            >
              <Volume2 className="w-4 h-4 text-cyan-400" />
              <span>Test Voice Playback</span>
            </button>
          </div>
        )}

        {/* SUBTAB 5: Personal Profile */}
        {activeSubTab === 'profile' && (
          <div className="space-y-6">
            <div className="border-b border-white/10 pb-4">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <User className="w-5 h-5 text-cyan-400" />
                <span>Personalization & Custom Instructions</span>
              </h3>
              <p className="text-xs text-slate-400 mt-0.5">
                E.D.I.T.H. personalizes system prompts using your name, profession, and instructions.
              </p>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1">
                  Your Name / Nickname
                </label>
                <input
                  type="text"
                  value={profileNickname}
                  onChange={(e) => handleProfileFieldChange('nickname', e.target.value, setProfileNickname)}
                  onBlur={(e) => handleProfileFieldBlur('nickname', e.target.value)}
                  placeholder="Sumit"
                  className="w-full px-3 py-2 rounded-xl glass-input text-xs text-slate-200"
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1">
                  Occupation / Role
                </label>
                <input
                  type="text"
                  value={profileOccupation}
                  onChange={(e) => handleProfileFieldChange('occupation', e.target.value, setProfileOccupation)}
                  onBlur={(e) => handleProfileFieldBlur('occupation', e.target.value)}
                  placeholder="Software Engineer"
                  className="w-full px-3 py-2 rounded-xl glass-input text-xs text-slate-200"
                />
              </div>
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-300 block mb-1">
                More About You (Preferences, Habits, Context)
              </label>
              <textarea
                rows={3}
                value={profileAbout}
                onChange={(e) => handleProfileFieldChange('moreAboutYou', e.target.value, setProfileAbout)}
                onBlur={(e) => handleProfileFieldBlur('moreAboutYou', e.target.value)}
                placeholder="I love clean TypeScript code, dark themes, and high performance architecture..."
                className="w-full p-3 rounded-xl glass-input text-xs text-slate-200 resize-none"
              />
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-300 block mb-1">
                System Prompt / Custom Instructions
              </label>
              <textarea
                rows={4}
                value={profileInstructions}
                onChange={(e) => handleProfileFieldChange('customInstructions', e.target.value, setProfileInstructions)}
                onBlur={(e) => handleProfileFieldBlur('customInstructions', e.target.value)}
                placeholder="You are E.D.I.T.H., an advanced AI assistant..."
                className="w-full p-3 rounded-xl glass-input text-xs text-slate-200 resize-none font-mono"
              />
            </div>
          </div>
        )}

        {/* SUBTAB 6: Custom Apps Registry */}
        {activeSubTab === 'apps' && (
          <div className="space-y-6">
            <div className="border-b border-white/10 pb-4">
              <h3 className="text-base font-bold text-white flex items-center gap-2">
                <AppWindow className="w-5 h-5 text-cyan-400" />
                <span>Custom Apps Voice/Text Launcher Registry</span>
              </h3>
              <p className="text-xs text-slate-400 mt-0.5">
                Register local apps or executable paths to launch via 'open [app name]'
              </p>
            </div>

            <div className="p-4 rounded-2xl glass-card border border-white/10 space-y-3">
              <div className="text-xs font-bold uppercase tracking-wider text-slate-300">
                Register New Application
              </div>

              <div className="grid grid-cols-3 gap-3">
                <input
                  type="text"
                  placeholder="App Name (e.g. spotify)"
                  value={newAppName}
                  onChange={(e) => setNewAppName(e.target.value)}
                  className="px-3 py-1.5 rounded-lg glass-input text-xs text-slate-200"
                />
                <input
                  type="text"
                  placeholder="Path / Command (e.g. spotify.exe)"
                  value={newAppPath}
                  onChange={(e) => setNewAppPath(e.target.value)}
                  className="px-3 py-1.5 rounded-lg glass-input text-xs font-mono text-slate-200"
                />
                <input
                  type="text"
                  placeholder="Keywords (e.g. spotify,music,songs)"
                  value={newAppKeywords}
                  onChange={(e) => setNewAppKeywords(e.target.value)}
                  className="px-3 py-1.5 rounded-lg glass-input text-xs text-slate-200"
                />
              </div>

              <button
                onClick={handleAddCustomApp}
                disabled={!newAppName.trim() || !newAppPath.trim()}
                className="px-4 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-bold transition disabled:opacity-50 flex items-center gap-1.5"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Add to Registry</span>
              </button>
            </div>

            <div className="space-y-2">
              {customApps.map((app) => (
                <div
                  key={app.id}
                  className="p-3.5 rounded-xl glass-panel border border-white/5 flex items-center justify-between text-xs"
                >
                  <div>
                    <span className="font-bold text-white capitalize mr-2">{app.name}</span>
                    <span className="text-slate-500 font-mono text-[11px]">{app.path}</span>
                    <div className="text-[10px] text-cyan-400/70 mt-0.5">Triggers: {app.keywords}</div>
                  </div>
                  <button
                    onClick={() => handleDeleteCustomApp(app.id)}
                    className="p-1.5 text-slate-500 hover:text-rose-400"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* SUBTAB 7: Personal Notes */}
        {activeSubTab === 'notes' && (
          <div className="space-y-6">
            <div className="border-b border-white/10 pb-4 flex items-center justify-between">
              <div>
                <h3 className="text-base font-bold text-white flex items-center gap-2">
                  <FileText className="w-5 h-5 text-cyan-400" />
                  <span>Personal Notes & Scratchpad</span>
                </h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  Quick thoughts, code snippets, or instructions automatically persisted in SQLite and local storage.
                </p>
              </div>
              <span className="text-[10px] font-mono text-cyan-400 bg-cyan-950/60 px-2 py-0.5 rounded border border-cyan-500/30">
                AUTO-SAVING
              </span>
            </div>

            <div className="space-y-3">
              <textarea
                rows={12}
                value={noteContent}
                onChange={(e) => handleNoteChange(e.target.value)}
                placeholder="Write reminders, snippets, or temporary notes here... (auto-saves as you type)"
                className="w-full p-4 rounded-2xl glass-input text-xs text-slate-100 min-h-[300px] resize-y font-mono focus:outline-none custom-scrollbar leading-relaxed"
              />

              <div className="flex items-center justify-between">
                <span className="text-[11px] text-slate-500 font-mono">
                  {noteContent.length} characters · Saved to localStorage & SQLite
                </span>
                <button
                  onClick={handleSaveNote}
                  className="px-5 py-2.5 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-bold shadow-lg shadow-cyan-600/30 flex items-center gap-2 transition"
                >
                  <Save className="w-4 h-4" />
                  <span>Save Notes Now</span>
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* CUSTOM PROVIDER MODAL */}
      {showProviderModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md p-4 animate-fade-in">
          <div className="w-full max-w-lg bg-[#0c1322] border border-cyan-500/40 rounded-3xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
            {/* Header */}
            <div className="px-6 py-4 border-b border-white/10 flex items-center justify-between bg-white/[0.02] shrink-0">
              <div className="flex items-center gap-2">
                <Globe className="w-5 h-5 text-cyan-400" />
                <h3 className="text-sm font-bold text-white">
                  {editingProviderId ? 'Edit Custom Provider' : 'Add Custom AI Provider'}
                </h3>
              </div>
              <button
                onClick={() => setShowProviderModal(false)}
                className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Modal Form with Strict max-h containment */}
            <form
              onSubmit={(e) => {
                e.preventDefault();
                handleSaveCustomProvider();
              }}
              aria-label="Custom AI Provider Form"
              className="flex flex-col flex-1 overflow-hidden"
            >
              <div className="p-6 space-y-4 overflow-y-auto custom-scrollbar flex-1 max-h-[calc(85vh-130px)]">
                <div>
                  <label htmlFor="custom-provider-name" className="text-xs font-bold text-slate-300 block mb-1">
                    Provider Name <span className="text-rose-400">*</span>
                  </label>
                  <input
                    id="custom-provider-name"
                    type="text"
                    placeholder="e.g. DeepSeek API, Ollama Local, OpenAI, OpenRouter"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    autoComplete="off"
                    className="w-full px-3 py-2 rounded-xl glass-input text-xs text-white"
                  />
                </div>

                <div>
                  <label htmlFor="custom-provider-url" className="text-xs font-bold text-slate-300 block mb-1">
                    Base URL (OpenAI-compatible) <span className="text-rose-400">*</span>
                  </label>
                  <input
                    id="custom-provider-url"
                    type="text"
                    placeholder="e.g. https://api.deepseek.com/v1 or http://localhost:11434/v1"
                    value={formBaseUrl}
                    onChange={(e) => setFormBaseUrl(e.target.value)}
                    autoComplete="off"
                    className="w-full px-3 py-2 rounded-xl glass-input text-xs font-mono text-cyan-300"
                  />
                  <p className="text-[10px] text-slate-500 mt-1">
                    Must support standard <code>/chat/completions</code> and <code>/models</code> endpoints.
                  </p>
                </div>

                <div>
                  <label htmlFor="custom-provider-key" className="text-xs font-bold text-slate-300 block mb-1">
                    API Key (optional for local endpoints)
                  </label>
                  <input
                    id="custom-provider-key"
                    type="password"
                    placeholder="sk-... or leave blank for local servers"
                    value={formApiKey}
                    onChange={(e) => setFormApiKey(e.target.value)}
                    autoComplete="off"
                    className="w-full px-3 py-2 rounded-xl glass-input text-xs font-mono text-slate-200"
                  />
                </div>

                {/* Model Fetching & Management */}
                <div className="pt-2 border-t border-white/10 space-y-3">
                  <div className="flex items-center justify-between">
                    <label className="text-xs font-bold text-slate-300">
                      Models ({formModels.length})
                    </label>

                    <button
                      type="button"
                      onClick={handleAutoFetchModels}
                      disabled={isFetchingModels || !formBaseUrl.trim()}
                      className="px-3 py-1 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-500/40 text-xs font-bold flex items-center gap-1.5 transition disabled:opacity-50"
                    >
                      {isFetchingModels ? (
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      ) : (
                        <RefreshCw className="w-3.5 h-3.5" />
                      )}
                      <span>Auto-Fetch Models</span>
                    </button>
                  </div>

                  {/* Manual Model Add */}
                  <div className="p-3 rounded-2xl bg-black/40 border border-white/5 space-y-2">
                    <div className="text-[11px] font-semibold text-slate-400">Add Model Manually</div>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                      <input
                        type="text"
                        placeholder="Model ID (e.g. deepseek-chat)"
                        value={newModelId}
                        onChange={(e) => setNewModelId(e.target.value)}
                        autoComplete="off"
                        aria-label="Model ID"
                        className="px-3 py-1.5 rounded-lg glass-input text-xs font-mono text-slate-200"
                      />
                      <input
                        type="text"
                        placeholder="Label (e.g. DeepSeek V3)"
                        value={newModelLabel}
                        onChange={(e) => setNewModelLabel(e.target.value)}
                        autoComplete="off"
                        aria-label="Model Friendly Label"
                        className="px-3 py-1.5 rounded-lg glass-input text-xs text-slate-200"
                      />
                    </div>
                    <button
                      type="button"
                      onClick={handleAddManualModel}
                      disabled={!newModelId.trim()}
                      className="w-full py-1.5 rounded-lg bg-white/10 hover:bg-white/15 text-slate-200 text-xs font-semibold flex items-center justify-center gap-1.5 transition disabled:opacity-40"
                    >
                      <Plus className="w-3.5 h-3.5" />
                      <span>Add to Model List</span>
                    </button>
                  </div>

                  {/* Models List */}
                  <div className="space-y-1.5 max-h-40 overflow-y-auto custom-scrollbar p-1">
                    {formModels.length === 0 ? (
                      <div className="text-center py-4 text-xs text-slate-500">
                        No models yet. Click "Auto-Fetch Models" or add one manually above.
                      </div>
                    ) : (
                      formModels.map((m) => (
                        <div
                          key={m.id}
                          className="px-3 py-2 rounded-xl bg-white/[0.03] border border-white/5 flex items-center justify-between text-xs"
                        >
                          <div className="truncate mr-2">
                            <div className="font-bold text-slate-200 truncate">{m.label}</div>
                            <div className="text-[10px] text-cyan-400/70 font-mono truncate">{m.id}</div>
                          </div>
                          <button
                            type="button"
                            onClick={() => handleRemoveModelFromForm(m.id)}
                            className="p-1 text-slate-500 hover:text-rose-400 transition"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </div>

              {/* Footer */}
              <div className="px-6 py-4 border-t border-white/10 flex items-center justify-end gap-3 bg-white/[0.02]">
                <button
                  type="button"
                  onClick={() => setShowProviderModal(false)}
                  className="px-4 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-slate-200 hover:bg-white/5 transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!formName.trim() || !formBaseUrl.trim()}
                  className="px-5 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-white text-xs font-bold shadow-lg shadow-cyan-500/20 transition disabled:opacity-50"
                >
                  {editingProviderId ? 'Update Provider' : 'Save Provider'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* AUTO-FETCH DYNAMIC MODELS MODAL */}
      {showFetchModal && fetchingProvider && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md p-4 animate-fade-in">
          <div className="w-full max-w-xl bg-[#0c1322] border border-cyan-500/40 rounded-3xl shadow-2xl overflow-hidden flex flex-col max-h-[88vh]">
            {/* Modal Header */}
            <div className="px-6 py-4 border-b border-white/10 flex items-center justify-between bg-white/[0.02] shrink-0">
              <div className="flex items-center gap-2.5">
                <div className="p-2 rounded-xl bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                  <Sparkles className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-white flex items-center gap-2">
                    <span>Sync Models: {fetchingProvider.name}</span>
                  </h3>
                  <p className="text-[11px] text-slate-400">
                    Fetched {fetchedModelsList.length} available models from provider API
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowFetchModal(false)}
                className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-white/10 transition"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Search and Select All Bar */}
            <div className="p-4 border-b border-white/5 bg-black/30 space-y-3 shrink-0">
              {/* Search Bar */}
              <div className="relative">
                <Search className="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
                <input
                  type="text"
                  placeholder="Filter models by ID or name (e.g. llama, flash, vision, 70b, deepseek)..."
                  value={fetchSearchFilter}
                  onChange={(e) => setFetchSearchFilter(e.target.value)}
                  className="w-full pl-9 pr-8 py-2 rounded-xl glass-input text-xs text-white placeholder-slate-500"
                />
                {fetchSearchFilter && (
                  <button
                    onClick={() => setFetchSearchFilter('')}
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white text-xs"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>

              {/* Select All Controls */}
              {(() => {
                const query = fetchSearchFilter.toLowerCase().trim();
                const filtered = fetchedModelsList.filter(
                  (m) =>
                    m.id.toLowerCase().includes(query) ||
                    m.label.toLowerCase().includes(query)
                );
                const filteredIds = filtered.map((m) => m.id);
                const allFilteredSelected =
                  filteredIds.length > 0 &&
                  filteredIds.every((id) => selectedModelIdsToImport.includes(id));

                return (
                  <div className="flex items-center justify-between text-xs px-1 select-none">
                    <button
                      type="button"
                      onClick={() => handleSelectAllFetchedModels(filteredIds)}
                      className="flex items-center gap-2 text-cyan-300 hover:text-cyan-200 font-bold transition cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={allFilteredSelected}
                        onChange={() => {}}
                        className="rounded accent-cyan-500 cursor-pointer pointer-events-none"
                      />
                      <span>
                        {allFilteredSelected ? 'Deselect All Filtered' : 'Select All Filtered'} ({filtered.length})
                      </span>
                    </button>

                    <span className="text-[11px] text-slate-400 font-mono">
                      {selectedModelIdsToImport.length} of {fetchedModelsList.length} selected
                    </span>
                  </div>
                );
              })()}
            </div>

            {/* Scrollable Model Checkbox List */}
            <div className="p-4 space-y-2 overflow-y-auto custom-scrollbar flex-1 max-h-[48vh]">
              {(() => {
                const query = fetchSearchFilter.toLowerCase().trim();
                const filtered = fetchedModelsList.filter(
                  (m: ProviderModel) =>
                    m.id.toLowerCase().includes(query) ||
                    m.label.toLowerCase().includes(query)
                );
                const currentModelIds = new Set((fetchingProvider.models || []).map((m: ProviderModel) => m.id));

                if (filtered.length === 0) {
                  return (
                    <div className="text-center py-10 text-xs text-slate-500 border border-dashed border-white/10 rounded-2xl">
                      No models matching "{fetchSearchFilter}"
                    </div>
                  );
                }

                return filtered.map((m: ProviderModel) => {
                  const isChecked = selectedModelIdsToImport.includes(m.id);
                  const isAlreadyAdded = currentModelIds.has(m.id);

                  return (
                    <div
                      key={m.id}
                      onClick={() => handleToggleModelSelection(m.id)}
                      className={
                        'p-3 rounded-2xl border transition flex items-center justify-between cursor-pointer select-none ' +
                        (isChecked
                          ? 'bg-cyan-500/15 border-cyan-500/50 text-white shadow-sm'
                          : 'bg-white/[0.02] border-white/5 text-slate-300 hover:bg-white/[0.05]')
                      }
                    >
                      <div className="flex items-center gap-3 min-w-0 flex-1 mr-2">
                        <input
                          type="checkbox"
                          checked={isChecked}
                          onChange={() => {}}
                          className="w-4 h-4 rounded accent-cyan-500 cursor-pointer shrink-0 pointer-events-none"
                        />
                        <div className="truncate flex-1">
                          <div className="text-xs font-bold text-slate-100 truncate flex items-center gap-2">
                            <span>{m.label}</span>
                            {isAlreadyAdded && (
                              <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                                In Active List
                              </span>
                            )}
                          </div>
                          <div className="text-[10px] text-slate-400 font-mono truncate">{m.id}</div>
                        </div>
                      </div>

                      {isChecked && (
                        <div className="w-5 h-5 rounded-full bg-cyan-500 flex items-center justify-center shrink-0">
                          <Check className="w-3 h-3 text-slate-950 font-bold" />
                        </div>
                      )}
                    </div>
                  );
                });
              })()}
            </div>

            {/* Modal Footer */}
            <div className="px-6 py-4 border-t border-white/10 flex items-center justify-between bg-white/[0.02] shrink-0">
              <span className="text-xs text-slate-400 font-mono">
                {selectedModelIdsToImport.length} models ready to import
              </span>

              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setShowFetchModal(false)}
                  className="px-4 py-2 rounded-xl text-xs font-semibold text-slate-400 hover:text-slate-200 hover:bg-white/5 transition"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleImportSelectedModels}
                  disabled={selectedModelIdsToImport.length === 0}
                  className="px-5 py-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-white text-xs font-bold shadow-lg shadow-cyan-500/20 transition disabled:opacity-40 flex items-center gap-2"
                >
                  <Plus className="w-4 h-4" />
                  <span>Add Selected Models ({selectedModelIdsToImport.length})</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
