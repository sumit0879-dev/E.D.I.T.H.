import React, { useState, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import type { BuiltinApp, CustomApp, WeatherResult, PluginWithState } from '../types';
import {
  Puzzle,
  Volume2,
  VolumeX,
  Volume1,
  Terminal,
  Send,
  Camera,
  CloudSun,
  Play,
  AppWindow,
  MessageCircle,
  Mail,
  Minimize2,
  Maximize2,
  Copy,
  Trash2,
  Layers,
  MapPin,
} from 'lucide-react';

const cityPresets = [
  { name: 'New Delhi', lat: 28.6139, lon: 77.2090 },
  { name: 'Mumbai', lat: 19.0760, lon: 72.8777 },
  { name: 'Bengaluru', lat: 12.9716, lon: 77.5946 },
  { name: 'London', lat: 51.5074, lon: -0.1278 },
  { name: 'New York', lat: 40.7128, lon: -74.0060 },
  { name: 'Tokyo', lat: 35.6762, lon: 139.6503 },
];

const terminalQuickCommands = [
  'dir',
  'ipconfig',
  'whoami',
  'systeminfo',
  'ping 8.8.8.8',
  'cargo --version',
  'node -v',
];

export const PluginsView: React.FC = () => {
  const { plugins, togglePlugin, showToast } = useApp();

  const [categoryFilter, setCategoryFilter] = useState<string>('all');
  const [builtinApps, setBuiltinApps] = useState<BuiltinApp[]>([]);
  const [customApps, setCustomApps] = useState<CustomApp[]>([]);
  const [terminalCmd, setTerminalCmd] = useState('');
  const [terminalOutput, setTerminalOutput] = useState('');
  const [isExecutingCmd, setIsExecutingCmd] = useState(false);

  // Weather state
  const [selectedCity, setSelectedCity] = useState(cityPresets[0]);
  const [weatherData, setWeatherData] = useState<WeatherResult | null>(null);
  const [isFetchingWeather, setIsFetchingWeather] = useState(false);

  // WhatsApp & Gmail state
  const [waNumber, setWaNumber] = useState('');
  const [waMessage, setWaMessage] = useState('');
  const [gmailAddress, setGmailAddress] = useState('');
  const [gmailMessage, setGmailMessage] = useState('');

  // Screenshot state
  const [screenshotB64, setScreenshotB64] = useState<string | null>(null);

  const effectivePlugins = plugins.length > 0 ? plugins : tauriService.DEFAULT_PLUGINS;

  useEffect(() => {
    tauriService.getBuiltinApps().then(setBuiltinApps);
    tauriService.getCustomApps().then(setCustomApps);
    fetchCityWeather(cityPresets[0]);
  }, []);

  const fetchCityWeather = async (city: typeof cityPresets[0]) => {
    setSelectedCity(city);
    try {
      setIsFetchingWeather(true);
      const res = await tauriService.getWeather(city.lat, city.lon);
      setWeatherData(res);
    } catch (e: any) {
      showToast('Weather error: ' + (e.message || e), 'error');
    } finally {
      setIsFetchingWeather(false);
    }
  };

  const handleRunTerminal = async (cmdToRun?: string) => {
    const cmd = (cmdToRun || terminalCmd).trim();
    if (!cmd) return;
    try {
      setIsExecutingCmd(true);
      const res = await tauriService.pluginSystemTerminal(cmd);
      setTerminalOutput(res);
    } catch (e: any) {
      setTerminalOutput('Error: ' + (e.message || e));
    } finally {
      setIsExecutingCmd(false);
    }
  };

  const handleSystemControl = async (action: string) => {
    try {
      const res = await tauriService.pluginSystemControl(action);
      showToast(res, 'info');
    } catch (e: any) {
      showToast('System control error: ' + (e.message || e), 'error');
    }
  };

  const handleTakeScreenshot = async () => {
    try {
      const b64 = await tauriService.takeScreenshot();
      setScreenshotB64(b64);
      showToast('Screen captured!', 'success');
    } catch (e: any) {
      showToast('Screenshot error: ' + (e.message || e), 'error');
    }
  };

  const handleLaunch = async (path: string) => {
    try {
      await tauriService.launchApp(path);
      showToast('Launched ' + path, 'success');
    } catch (e: any) {
      showToast('Launch failed: ' + (e.message || e), 'error');
    }
  };

  const handleSendWhatsapp = async () => {
    if (!waNumber.trim() || !waMessage.trim()) return;
    try {
      const res = await tauriService.pluginWhatsapp(waNumber.trim(), waMessage.trim());
      showToast(res, 'success');
    } catch (e: any) {
      showToast('WhatsApp error: ' + (e.message || e), 'error');
    }
  };

  const handleSendGmail = async () => {
    if (!gmailAddress.trim() || !gmailMessage.trim()) return;
    try {
      const res = await tauriService.pluginGmail(gmailAddress.trim(), gmailMessage.trim());
      showToast(res, 'success');
    } catch (e: any) {
      showToast('Email error: ' + (e.message || e), 'error');
    }
  };

  const filteredPlugins = effectivePlugins.filter((p) => {
    if (categoryFilter === 'all') return true;
    return p.category === categoryFilter;
  });

  return (
    <div className="flex-1 flex flex-col h-full bg-[#000000] overflow-y-auto custom-scrollbar p-6 pb-32 space-y-6">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between pb-4 border-b border-white/10 gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-2xl bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
            <Puzzle className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-white">E.D.I.T.H. Tactical Plugins & Automation Hub</h2>
            <p className="text-xs text-slate-400">
              Manage installed plugins, execute desktop automations, and trigger system controls
            </p>
          </div>
        </div>

        {/* Category Filters */}
        <div className="flex items-center gap-1.5 p-1 rounded-xl bg-black/40 border border-white/10">
          {['all', 'system', 'utility', 'media', 'developer', 'social'].map((cat) => (
            <button
              key={cat}
              onClick={() => setCategoryFilter(cat)}
              className={
                'px-3 py-1 rounded-lg text-xs font-semibold uppercase tracking-wider transition ' +
                (categoryFilter === cat
                  ? 'bg-cyan-500 text-slate-950 font-bold shadow-md shadow-cyan-500/20'
                  : 'text-slate-400 hover:text-slate-200')
              }
            >
              {cat}
            </button>
          ))}
        </div>
      </div>

      {/* Installed Plugins Grid */}
      <div>
        <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3 flex items-center gap-2">
          <Layers className="w-4 h-4 text-cyan-400" />
          <span>Active Plugins ({effectivePlugins.filter((p) => p.enabled).length}/{effectivePlugins.length})</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3.5">
          {filteredPlugins.map((plugin) => (
            <div
              key={plugin.id}
              className={
                'p-4 rounded-2xl glass-panel transition border flex flex-col justify-between ' +
                (plugin.enabled
                  ? 'border-cyan-500/40 shadow-xl shadow-cyan-500/5'
                  : 'border-white/5 opacity-60')
              }
            >
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs font-bold text-white">{plugin.name}</span>
                  <button
                    onClick={() => togglePlugin(plugin.id)}
                    className={
                      'px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wider transition ' +
                      (plugin.enabled
                        ? 'bg-cyan-500 text-slate-950 font-bold shadow-md shadow-cyan-500/20'
                        : 'bg-slate-800 text-slate-400')
                    }
                  >
                    {plugin.enabled ? 'ENABLED' : 'DISABLED'}
                  </button>
                </div>
                <p className="text-[11px] text-slate-400 leading-relaxed mb-3">
                  {plugin.description}
                </p>
              </div>

              <div className="flex items-center justify-between pt-2 border-t border-white/5 text-[10px]">
                <span className="text-cyan-400/80 font-mono uppercase">
                  {plugin.category}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Quick Tools & Execution Widgets */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 pt-4 border-t border-white/10">
        {/* Widget 1: System Audio Control */}
        <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-4">
          <h4 className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-2">
            <Volume2 className="w-4 h-4 text-cyan-400" />
            <span>System Audio & Controls</span>
          </h4>

          <div className="grid grid-cols-3 gap-2">
            <button
              onClick={() => handleSystemControl('volume_up')}
              className="p-3 rounded-xl bg-white/[0.04] hover:bg-cyan-500/20 hover:text-cyan-300 text-xs font-semibold text-slate-300 transition flex flex-col items-center gap-1.5 border border-white/5 shadow-sm"
            >
              <Volume2 className="w-4 h-4 text-cyan-400" />
              <span>Vol Up</span>
            </button>
            <button
              onClick={() => handleSystemControl('volume_down')}
              className="p-3 rounded-xl bg-white/[0.04] hover:bg-cyan-500/20 hover:text-cyan-300 text-xs font-semibold text-slate-300 transition flex flex-col items-center gap-1.5 border border-white/5 shadow-sm"
            >
              <Volume1 className="w-4 h-4 text-cyan-400" />
              <span>Vol Down</span>
            </button>
            <button
              onClick={() => handleSystemControl('mute')}
              className="p-3 rounded-xl bg-white/[0.04] hover:bg-rose-500/20 hover:text-rose-300 text-xs font-semibold text-slate-300 transition flex flex-col items-center gap-1.5 border border-white/5 shadow-sm"
            >
              <VolumeX className="w-4 h-4 text-rose-400" />
              <span>Mute</span>
            </button>
          </div>

          {/* Screenshot tool */}
          <div className="pt-2 border-t border-white/5 space-y-2">
            <button
              onClick={handleTakeScreenshot}
              className="w-full py-2.5 px-3 rounded-xl bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 border border-cyan-500/30 text-xs font-bold flex items-center justify-center gap-2 transition shadow-md"
            >
              <Camera className="w-4 h-4" />
              <span>Capture Screen Now</span>
            </button>
            {screenshotB64 && (
              <div className="rounded-xl overflow-hidden border border-white/10 max-h-36">
                <img src={screenshotB64} alt="Screen Preview" className="w-full object-cover" />
              </div>
            )}
          </div>
        </div>

        {/* Widget 2: Live Weather & Deep Links */}
        <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-4">
          <h4 className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-2">
            <CloudSun className="w-4 h-4 text-amber-400" />
            <span>Live Weather & Messaging</span>
          </h4>

          {/* City Weather Selector */}
          <div className="p-3.5 rounded-xl bg-[#090d16] border border-white/10 space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-white flex items-center gap-1.5">
                <MapPin className="w-3.5 h-3.5 text-amber-400" />
                <span>{selectedCity.name}</span>
              </span>
              <div className="flex gap-1">
                {cityPresets.slice(0, 4).map((city) => (
                  <button
                    key={city.name}
                    onClick={() => fetchCityWeather(city)}
                    className={
                      'px-1.5 py-0.5 rounded text-[10px] transition ' +
                      (selectedCity.name === city.name
                        ? 'bg-amber-500/30 text-amber-300 font-bold'
                        : 'text-slate-500 hover:text-slate-300')
                    }
                  >
                    {city.name.split(' ')[0]}
                  </button>
                ))}
              </div>
            </div>

            {weatherData ? (
              <div className="flex items-center justify-between pt-1">
                <div>
                  <div className="text-2xl font-black text-white">{weatherData.temperature}°C</div>
                  <div className="text-xs text-amber-300 font-medium">{weatherData.condition}</div>
                </div>
                <CloudSun className="w-10 h-10 text-amber-400" />
              </div>
            ) : (
              <div className="text-xs text-slate-500">
                {isFetchingWeather ? 'Updating weather...' : 'No weather data'}
              </div>
            )}
          </div>

          {/* WhatsApp Direct Sender */}
          <div className="space-y-1.5 pt-2 border-t border-white/5">
            <label className="text-[11px] font-semibold text-emerald-400 flex items-center gap-1.5">
              <MessageCircle className="w-3.5 h-3.5" />
              <span>WhatsApp Direct Message</span>
            </label>
            <input
              type="text"
              placeholder="Phone (e.g. 9876543210)"
              value={waNumber}
              onChange={(e) => setWaNumber(e.target.value)}
              className="w-full px-2.5 py-1.5 rounded-lg glass-input text-xs font-mono text-slate-200"
            />
            <div className="flex gap-2">
              <input
                type="text"
                placeholder="Message text..."
                value={waMessage}
                onChange={(e) => setWaMessage(e.target.value)}
                className="flex-1 px-2.5 py-1.5 rounded-lg glass-input text-xs text-slate-200"
              />
              <button
                onClick={handleSendWhatsapp}
                className="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-bold transition shadow-md shadow-emerald-600/30"
              >
                Send
              </button>
            </div>
          </div>
        </div>

        {/* Widget 3: Terminal Command Runner */}
        <div className="p-5 rounded-2xl glass-card border border-white/10 space-y-3 flex flex-col justify-between">
          <div>
            <div className="flex items-center justify-between mb-2">
              <h4 className="text-xs font-bold uppercase tracking-wider text-slate-300 flex items-center gap-2">
                <Terminal className="w-4 h-4 text-violet-400" />
                <span>Terminal Runner</span>
              </h4>
              {terminalOutput && (
                <button
                  onClick={() => setTerminalOutput('')}
                  className="text-[10px] text-slate-400 hover:text-slate-200 flex items-center gap-1"
                >
                  <Trash2 className="w-3 h-3" />
                  <span>Clear</span>
                </button>
              )}
            </div>

            {/* Quick command pills */}
            <div className="flex flex-wrap gap-1 mb-2.5">
              {terminalQuickCommands.map((qc) => (
                <button
                  key={qc}
                  onClick={() => {
                    setTerminalCmd(qc);
                    handleRunTerminal(qc);
                  }}
                  className="px-2 py-0.5 rounded-md bg-white/[0.04] hover:bg-violet-500/20 text-[10px] font-mono text-slate-400 hover:text-violet-300 transition border border-white/5"
                >
                  {qc}
                </button>
              ))}
            </div>

            <div className="flex gap-2 mb-2.5">
              <input
                type="text"
                placeholder="dir, ipconfig, ping..."
                value={terminalCmd}
                onChange={(e) => setTerminalCmd(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleRunTerminal()}
                className="flex-1 px-3 py-1.5 rounded-lg glass-input text-xs font-mono text-slate-200 placeholder-slate-500"
              />
              <button
                onClick={() => handleRunTerminal()}
                disabled={isExecutingCmd || !terminalCmd.trim()}
                className="px-3.5 py-1.5 rounded-lg bg-violet-600 hover:bg-violet-500 text-white text-xs font-bold transition shadow-md shadow-violet-600/30 disabled:opacity-50"
              >
                {isExecutingCmd ? '...' : 'Run'}
              </button>
            </div>

            <div className="h-36 overflow-y-auto p-3 rounded-xl bg-black/90 border border-white/10 font-mono text-[11px] text-emerald-400 selection:bg-cyan-500/30 whitespace-pre-wrap">
              {terminalOutput || 'Type a shell command or click a preset above...'}
            </div>
          </div>

          <div className="text-[10px] text-slate-500">
            Executes quietly in background via `plugin_system_terminal`
          </div>
        </div>
      </div>

      {/* Builtin & Custom App Launchers Hub */}
      <div className="pt-4 border-t border-white/10">
        <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3 flex items-center gap-2">
          <AppWindow className="w-4 h-4 text-cyan-400" />
          <span>App Launchers ({builtinApps.length + customApps.length})</span>
        </h3>

        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 lg:grid-cols-7 gap-2.5">
          {builtinApps.map((app) => (
            <button
              key={app.id}
              onClick={() => handleLaunch(app.path)}
              className="p-3 rounded-xl glass-panel hover:border-cyan-500/50 text-left transition group shadow-sm"
            >
              <div className="text-xs font-bold text-white group-hover:text-cyan-300 capitalize truncate">
                {app.name}
              </div>
              <div className="text-[10px] text-slate-500 font-mono truncate">{app.path}</div>
            </button>
          ))}

          {customApps.map((app) => (
            <button
              key={app.id}
              onClick={() => handleLaunch(app.path)}
              className="p-3 rounded-xl glass-panel hover:border-violet-500/50 text-left transition group border-violet-500/20 shadow-sm"
            >
              <div className="text-xs font-bold text-violet-300 capitalize truncate">
                {app.name}
              </div>
              <div className="text-[10px] text-slate-500 font-mono truncate">{app.path}</div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};
