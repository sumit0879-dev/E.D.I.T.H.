import React, { useState, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import {
  Camera,
  VolumeX,
  CloudSun,
  Cpu,
  ChevronDown,
  Check,
} from 'lucide-react';

export const Header: React.FC = () => {
  const { settings, updateSetting, providers, isSpeaking, stopSpeaking, showToast } = useApp();
  const [weather, setWeather] = useState<{ temp: number; cond: string } | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [showModelPicker, setShowModelPicker] = useState(false);

  useEffect(() => {
    tauriService.getWeather(28.6139, 77.2090)
      .then((res) => {
        if (res) {
          setWeather({ temp: res.temperature, cond: res.condition });
        }
      })
      .catch(() => {});
  }, []);

  const handleScreenshot = async () => {
    try {
      setCapturing(true);
      const b64 = await tauriService.takeScreenshot();
      if (b64) {
        const link = document.createElement('a');
        link.href = b64;
        link.download = 'screenshot_' + Date.now() + '.jpg';
        link.click();
        showToast('Screenshot saved to Downloads!', 'success');
      }
    } catch (e: any) {
      showToast('Screenshot failed: ' + (e.message || e), 'error');
    } finally {
      setCapturing(false);
    }
  };

  const effectiveProviders = providers.length > 0 ? providers : tauriService.DEFAULT_PROVIDERS;
  const currentProvider = effectiveProviders.find((p) => p.id === (settings.selectedProvider || 'groq')) || effectiveProviders[0];

  return (
    <header className="h-14 border-b border-white/10 bg-[#090d16]/90 px-6 flex items-center justify-between z-30 select-none backdrop-blur-md relative">
      {/* Left side: Model Quick-Switcher */}
      <div className="flex items-center gap-3">
        {/* Model Quick Switch Dropdown */}
        <div className="relative">
          <button
            onClick={() => setShowModelPicker(!showModelPicker)}
            className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-cyan-950/50 hover:bg-cyan-950/80 border border-cyan-500/30 text-cyan-300 text-xs font-mono transition shadow-sm"
          >
            <Cpu className="w-3.5 h-3.5 text-cyan-400" />
            <span className="font-bold truncate max-w-[200px]">
              {settings.aiMode === 'local' ? 'Local GGUF' : settings.selectedModel || 'llama-3.3-70b-versatile'}
            </span>
            <ChevronDown className="w-3 h-3 text-cyan-400 opacity-80" />
          </button>

          {/* Quick Model Selector Dropdown Popover */}
          {showModelPicker && (
            <div
              className="absolute left-0 top-full mt-2 w-80 p-2 rounded-2xl glass-panel shadow-2xl border border-white/15 z-50 animate-slide-in backdrop-blur-2xl"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="text-[10px] uppercase font-bold tracking-wider text-slate-400 px-2 py-1 mb-1 border-b border-white/5 flex items-center justify-between">
                <span>Active Provider: {currentProvider?.name || 'Groq'}</span>
                {currentProvider?.isCustom && (
                  <span className="text-[9px] px-1 py-0.2 rounded bg-violet-500/20 text-violet-300 border border-violet-500/30">
                    CUSTOM
                  </span>
                )}
              </div>

              <div className="space-y-1 max-h-60 overflow-y-auto">
                {(currentProvider?.models || []).length === 0 ? (
                  <div className="p-3 text-center text-xs text-slate-500">
                    No models configured. Add models in Settings.
                  </div>
                ) : (
                  currentProvider.models.map((m) => {
                    const isSel = settings.selectedModel === m.id;
                    return (
                      <button
                        key={m.id}
                        onClick={() => {
                          updateSetting('selectedModel', m.id);
                          setShowModelPicker(false);
                        }}
                        className={
                          'w-full p-2 rounded-xl text-left text-xs transition flex items-center justify-between ' +
                          (isSel
                            ? 'bg-cyan-500/20 text-white font-bold border border-cyan-500/40'
                            : 'text-slate-300 hover:bg-white/[0.05]')
                        }
                      >
                        <span className="truncate">{m.label}</span>
                        {isSel && <Check className="w-3.5 h-3.5 text-cyan-400 shrink-0 ml-1" />}
                      </button>
                    );
                  })
                )}
              </div>

              <div className="pt-2 mt-2 border-t border-white/5 flex justify-between items-center px-1">
                <span className="text-[10px] text-slate-500">Manage in Settings</span>
                <button
                  onClick={() => setShowModelPicker(false)}
                  className="text-[10px] text-cyan-400 hover:underline"
                >
                  Close
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Weather widget */}
        {weather && (
          <div className="hidden md:flex items-center gap-1.5 px-3 py-1 rounded-xl bg-slate-900/60 border border-white/5 text-slate-300 text-xs">
            <CloudSun className="w-3.5 h-3.5 text-amber-400" />
            <span>{weather.temp}°C</span>
            <span className="text-slate-500">·</span>
            <span className="text-slate-400">{weather.cond}</span>
          </div>
        )}
      </div>

      {/* Right side: Quick actions */}
      <div className="flex items-center gap-2">
        {isSpeaking && (
          <button
            onClick={stopSpeaking}
            className="flex items-center gap-1.5 px-3 py-1 rounded-xl bg-rose-500/20 text-rose-300 border border-rose-500/30 hover:bg-rose-500/30 transition text-xs font-semibold animate-pulse"
            title="Stop AI voice output"
          >
            <VolumeX className="w-3.5 h-3.5" />
            <span>Stop Speaking</span>
          </button>
        )}

        <button
          onClick={handleScreenshot}
          disabled={capturing}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-white/[0.04] hover:bg-white/[0.08] text-slate-300 hover:text-white transition text-xs font-medium border border-white/10"
          title="Take full screen capture"
        >
          <Camera className="w-3.5 h-3.5 text-cyan-400" />
          <span className="hidden sm:inline">Screenshot</span>
        </button>
      </div>
    </header>
  );
};

