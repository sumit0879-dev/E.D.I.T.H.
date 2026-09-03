import React, { useState, useEffect, useRef } from 'react';
import { useApp } from '../context/AppContext';
import { browserController } from '../services/browserController';
import {
  Activity,
  Cpu,
  Radio,
  Sparkles,
  Layers,
  PanelRightClose,
  PanelRightOpen,
  Wifi,
  Clock,
  Zap,
  ChevronDown,
  Check,
  Globe,
  Sliders,
} from 'lucide-react';

interface TopHudBarProps {
  isTelemetryOpen: boolean;
  onToggleTelemetry: () => void;
}

export const TopHudBar: React.FC<TopHudBarProps> = ({
  isTelemetryOpen,
  onToggleTelemetry,
}) => {
  const {
    activeTab,
    settings,
    updateSetting,
    providers,
    customProviders,
    showToast,
    isSpeaking,
    isRecording,
  } = useApp();
  const [timeStr, setTimeStr] = useState('');
  const [latency, setLatency] = useState(14);
  const [waveformBars, setWaveformBars] = useState<number[]>([30, 60, 45, 80, 50, 90, 40, 70, 35]);
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const modelMenuRef = useRef<HTMLDivElement>(null);

  // Fix Bug 2: Temporarily hide native child webview while Model Switcher dropdown is open to prevent occlusion/cutting
  useEffect(() => {
    if (activeTab === 'browser') {
      if (isModelMenuOpen) {
        browserController.hideAll().catch(() => {});
      } else {
        browserController.showActive().catch(() => {});
      }
    }
  }, [isModelMenuOpen, activeTab]);

  // BUG #10: Close dropdown immediately when switching application views
  useEffect(() => {
    setIsModelMenuOpen(false);
  }, [activeTab]);

  // BUG #10: Close dropdown when pressing Escape key
  useEffect(() => {
    if (!isModelMenuOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsModelMenuOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isModelMenuOpen]);

  // Close dropdown when clicking outside (mouse or touch)
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent | TouchEvent) => {
      if (modelMenuRef.current && !modelMenuRef.current.contains(event.target as Node)) {
        setIsModelMenuOpen(false);
      }
    };
    if (isModelMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('touchstart', handleClickOutside);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('touchstart', handleClickOutside);
    };
  }, [isModelMenuOpen]);

  // Real-time live clock
  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      const hours = String(now.getHours()).padStart(2, '0');
      const mins = String(now.getMinutes()).padStart(2, '0');
      const secs = String(now.getSeconds()).padStart(2, '0');
      setTimeStr(`${hours}:${mins}:${secs}`);
    };
    updateTime();
    const timer = setInterval(updateTime, 1000);
    return () => clearInterval(timer);
  }, []);

  // Waveform snippet animation when speaking or recording
  useEffect(() => {
    if (!isSpeaking && !isRecording) {
      setWaveformBars([20, 35, 25, 40, 30, 45, 25, 35, 20]);
      return;
    }

    const interval = setInterval(() => {
      setWaveformBars((prev) =>
        prev.map(() => Math.floor(Math.random() * 70) + 30)
      );
    }, 100);

    return () => clearInterval(interval);
  }, [isSpeaking, isRecording]);

  // Simulated latency jitter (12-18ms)
  useEffect(() => {
    const interval = setInterval(() => {
      setLatency(Math.floor(Math.random() * 6) + 12);
    }, 4000);
    return () => clearInterval(interval);
  }, []);

  const handleSelectModel = async (providerId: string, modelId: string, modelLabel: string, isLocal = false) => {
    if (isLocal) {
      await updateSetting('aiMode', 'local');
      showToast('AI Mode switched to Local Llama GGUF', 'info');
    } else {
      await updateSetting('aiMode', 'api');
      await updateSetting('selectedProvider', providerId);
      await updateSetting('selectedModel', modelId);
      showToast(`Model switched to ${modelLabel}`, 'info');
    }
    setIsModelMenuOpen(false);
  };

  const getActiveModelDisplay = () => {
    if (settings.aiMode === 'local') return 'Local GGUF';
    const activeProv =
      providers.find((p) => p.id === settings.selectedProvider) ||
      customProviders.find((p) => p.id === settings.selectedProvider);
    const activeModelObj = activeProv?.models.find((m) => m.id === settings.selectedModel);
    return activeModelObj?.label || settings.selectedModel || 'llama-3.3-70b';
  };

  return (
    <header className="h-12 bg-[#030712]/90 backdrop-blur-xl border-b border-white/[0.08] px-4 flex items-center justify-between z-40 select-none relative">
      {/* Left: Mark Logo + Pulse Status */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          {/* Stark Core Icon */}
          <div className="relative w-7 h-7 rounded-lg bg-cyan-950/60 border border-cyan-500/40 flex items-center justify-center shadow-cyan-glow-sm">
            <Sparkles className="w-4 h-4 text-cyan-400 animate-pulse" />
            <div className="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-cyan-400 animate-ping" />
          </div>

          <div>
            <div className="flex items-center gap-1.5">
              <span className="text-xs font-black tracking-widest text-white font-mono">
                E.D.I.T.H.
              </span>
              <span className="text-[9px] font-mono font-bold text-cyan-400 bg-cyan-950/70 px-1.5 py-0.2 rounded border border-cyan-500/30">
                MK-85
              </span>
            </div>
          </div>
        </div>

        <div className="hidden sm:flex items-center gap-1.5 pl-3 border-l border-white/10 text-[11px] font-mono">
          <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
          <span className="text-emerald-400 font-semibold text-[10px] tracking-wider uppercase">
            {isRecording ? 'LISTENING' : isSpeaking ? 'AUDIO OUTPUT' : 'STANDBY ACTIVE'}
          </span>
        </div>
      </div>

      {/* Center: Realtime Voice Waveform Snippet */}
      <div className="hidden md:flex items-center gap-2 px-4 py-1 rounded-xl bg-black/40 border border-white/5">
        <Radio className={`w-3.5 h-3.5 ${isRecording || isSpeaking ? 'text-cyan-400 animate-pulse' : 'text-slate-500'}`} />
        <div className="flex items-center gap-1 h-4 px-1">
          {waveformBars.map((h, i) => (
            <span
              key={i}
              className={`w-0.5 rounded-full transition-all duration-100 ${
                isRecording
                  ? 'bg-cyan-400'
                  : isSpeaking
                  ? 'bg-sky-400'
                  : 'bg-slate-600'
              }`}
              style={{ height: `${h}%` }}
            />
          ))}
        </div>
        <span className="text-[10px] font-mono text-slate-400 uppercase tracking-wider">
          {isRecording ? 'VOICE IN' : isSpeaking ? 'TTS OUT' : 'AUDIO HUD'}
        </span>
      </div>

      {/* Right: Telemetry Indicators + Clock + Model Selector + Dock Toggle */}
      <div className="flex items-center gap-3">
        {/* Host Info */}
        <div className="hidden lg:flex items-center gap-1.5 text-[11px] font-mono text-slate-400 bg-white/[0.02] px-2.5 py-1 rounded-lg border border-white/5">
          <Wifi className="w-3 h-3 text-cyan-400" />
          <span>localhost:1420</span>
          <span className="text-slate-600">·</span>
          <span className="text-emerald-400">{latency}ms</span>
        </div>

        {/* Interactive Quick Model Selector Pill & Dropdown */}
        <div className="relative hidden sm:block" ref={modelMenuRef}>
          <button
            type="button"
            onClick={() => setIsModelMenuOpen((prev) => !prev)}
            aria-label="Change AI Model"
            className="flex items-center gap-1.5 text-[11px] font-mono text-cyan-300 bg-cyan-950/50 hover:bg-cyan-900/60 px-2.5 py-1 rounded-lg border border-cyan-500/30 hover:border-cyan-400/60 max-w-[200px] xl:max-w-[300px] truncate shadow-sm transition cursor-pointer"
            title="Click to quickly switch AI Model"
          >
            <Cpu className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
            <span className="truncate font-semibold">{getActiveModelDisplay()}</span>
            <ChevronDown className={`w-3.5 h-3.5 text-cyan-400/80 shrink-0 transition-transform duration-200 ${isModelMenuOpen ? 'rotate-180' : ''}`} />
          </button>

          {/* Model Selection Dropdown */}
          {isModelMenuOpen && (
            <div className="absolute right-0 mt-2 w-80 sm:w-96 max-w-[calc(100vw-2rem)] bg-[#090e1a]/98 backdrop-blur-2xl border border-cyan-500/40 rounded-2xl shadow-2xl z-50 p-2.5 space-y-2 animate-fade-in text-xs">
              <div className="px-2 py-1.5 border-b border-white/10 flex items-center justify-between text-slate-300">
                <span className="font-bold text-white flex items-center gap-1.5 font-mono text-[11px]">
                  <Sparkles className="w-3.5 h-3.5 text-cyan-400" />
                  <span>Select AI Model</span>
                </span>
                <span className="text-[10px] text-cyan-400 font-mono">Quick Switch</span>
              </div>

              <div className="max-h-80 overflow-y-auto custom-scrollbar space-y-3 pr-1">
                {/* Built-in Providers (Groq, Gemini) */}
                {providers.map((prov) => {
                  const isCurrentProvider = settings.aiMode === 'api' && settings.selectedProvider === prov.id;

                  return (
                    <div key={prov.id} className="space-y-1">
                      <div className="px-2 py-0.5 text-[10px] font-bold font-mono text-slate-400 uppercase tracking-wider flex items-center justify-between">
                        <span>{prov.name}</span>
                        {isCurrentProvider && (
                          <span className="text-[9px] text-cyan-400 font-mono">Active Provider</span>
                        )}
                      </div>

                      <div className="space-y-0.5">
                        {prov.models.map((m) => {
                          const isSelected =
                            settings.aiMode === 'api' &&
                            settings.selectedProvider === prov.id &&
                            settings.selectedModel === m.id;

                          return (
                            <button
                              key={m.id}
                              type="button"
                              onClick={() => handleSelectModel(prov.id, m.id, m.label)}
                              className={`w-full px-2.5 py-1.5 rounded-xl text-left transition flex items-center justify-between group cursor-pointer ${
                                isSelected
                                  ? 'bg-cyan-500/20 text-white font-bold border border-cyan-500/40 shadow-sm'
                                  : 'hover:bg-white/[0.05] text-slate-300'
                              }`}
                            >
                              <div className="truncate mr-2">
                                <div className="truncate text-xs">{m.label}</div>
                                <div className="text-[10px] font-mono text-cyan-400/70 truncate">{m.id}</div>
                              </div>
                              {isSelected && (
                                <Check className="w-4 h-4 text-cyan-400 shrink-0" />
                              )}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  );
                })}

                {/* Custom Providers */}
                {customProviders.length > 0 && (
                  <div className="space-y-1 pt-1 border-t border-white/5">
                    <div className="px-2 py-0.5 text-[10px] font-bold font-mono text-slate-400 uppercase tracking-wider">
                      Custom OpenAI Endpoints
                    </div>
                    {customProviders.map((cp) => (
                      <div key={cp.id} className="space-y-0.5">
                        <div className="px-2 text-[10px] text-cyan-300 font-semibold">{cp.name}</div>
                        {cp.models.map((m) => {
                          const isSelected =
                            settings.aiMode === 'api' &&
                            settings.selectedProvider === cp.id &&
                            settings.selectedModel === m.id;

                          return (
                            <button
                              key={m.id}
                              type="button"
                              onClick={() => handleSelectModel(cp.id, m.id, m.label)}
                              className={`w-full px-2.5 py-1.5 rounded-xl text-left transition flex items-center justify-between group cursor-pointer ${
                                isSelected
                                  ? 'bg-cyan-500/20 text-white font-bold border border-cyan-500/40 shadow-sm'
                                  : 'hover:bg-white/[0.05] text-slate-300'
                              }`}
                            >
                              <div className="truncate mr-2">
                                <div className="truncate text-xs">{m.label}</div>
                                <div className="text-[10px] font-mono text-cyan-400/70 truncate">{m.id}</div>
                              </div>
                              {isSelected && (
                                <Check className="w-4 h-4 text-cyan-400 shrink-0" />
                              )}
                            </button>
                          );
                        })}
                      </div>
                    ))}
                  </div>
                )}

                {/* Local Llama GGUF Mode Option */}
                <div className="pt-1 border-t border-white/5">
                  <button
                    type="button"
                    onClick={() => handleSelectModel('local', 'local-gguf', 'Local Llama GGUF', true)}
                    className={`w-full px-2.5 py-2 rounded-xl text-left transition flex items-center justify-between group cursor-pointer ${
                      settings.aiMode === 'local'
                        ? 'bg-cyan-500/20 text-white font-bold border border-cyan-500/40 shadow-sm'
                        : 'hover:bg-white/[0.05] text-slate-300'
                    }`}
                  >
                    <div className="flex items-center gap-2 truncate">
                      <Sliders className="w-4 h-4 text-cyan-400 shrink-0" />
                      <div>
                        <div className="text-xs font-bold text-white">Local Llama-Server GGUF</div>
                        <div className="text-[10px] text-slate-400 font-mono">Offline inference</div>
                      </div>
                    </div>
                    {settings.aiMode === 'local' && (
                      <Check className="w-4 h-4 text-cyan-400 shrink-0" />
                    )}
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Digital Monospace Live Clock */}
        <div className="flex items-center gap-1.5 text-xs font-mono font-bold text-white bg-slate-900/80 px-2.5 py-1 rounded-lg border border-white/10">
          <Clock className="w-3.5 h-3.5 text-cyan-400" />
          <span className="tracking-wider">{timeStr || '00:00:00'}</span>
        </div>

        {/* Toggle Telemetry Dock Button */}
        <button
          onClick={onToggleTelemetry}
          aria-label={isTelemetryOpen ? 'Collapse Telemetry Dock (Ctrl+B)' : 'Expand Telemetry Dock (Ctrl+B)'}
          className={`p-1.5 rounded-lg border transition flex items-center gap-1 text-xs font-mono cursor-pointer ${
            isTelemetryOpen
              ? 'bg-cyan-500/20 border-cyan-500/50 text-cyan-300 shadow-cyan-glow-sm'
              : 'bg-white/[0.03] border-white/10 text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]'
          }`}
          title={isTelemetryOpen ? 'Collapse Telemetry Dock (Ctrl+B)' : 'Expand Telemetry Dock (Ctrl+B)'}
        >
          {isTelemetryOpen ? (
            <PanelRightClose className="w-4 h-4 text-cyan-400" />
          ) : (
            <PanelRightOpen className="w-4 h-4" />
          )}
        </button>
      </div>
    </header>
  );
};
