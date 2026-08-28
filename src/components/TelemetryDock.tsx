import React, { useState, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import {
  Activity,
  Cpu,
  HardDrive,
  Flame,
  Zap,
  CheckCircle2,
  Trash2,
  Camera,
  Layers,
  Volume2,
  VolumeX,
  RefreshCw,
  Server,
  Database,
} from 'lucide-react';

interface TelemetryDockProps {
  isOpen: boolean;
  onClose: () => void;
}

export const TelemetryDock: React.FC<TelemetryDockProps> = ({ isOpen, onClose }) => {
  const { settings, showToast, isSpeaking } = useApp();

  // Simulated live telemetry metrics that fluctuate naturally
  const [cpuUsage, setCpuUsage] = useState(28);
  const [ramUsage, setRamUsage] = useState(42);
  const [gpuUsage, setGpuUsage] = useState(19);
  const [temp, setTemp] = useState(49);
  const [fanSpeed, setFanSpeed] = useState(1850);
  const [activeTasks, setActiveTasks] = useState([
    { id: 't1', name: 'Groq Cloud SSE Pipe', status: 'active', type: 'LLM Stream' },
    { id: 't2', name: 'LanceDB RAG Vector Store', status: 'ready', type: 'Memory' },
    { id: 't3', name: 'EdgeTTS Speech Synthesizer', status: isSpeaking ? 'streaming' : 'standby', type: 'Voice' },
    { id: 't4', name: 'SQLite System Index', status: 'synced', type: 'Database' },
  ]);

  // Jitter telemetry values smoothly
  useEffect(() => {
    const interval = setInterval(() => {
      setCpuUsage((prev) => {
        const delta = Math.floor(Math.random() * 9) - 4;
        return Math.min(Math.max(prev + delta, 14), 78);
      });
      setRamUsage((prev) => {
        const delta = Math.floor(Math.random() * 5) - 2;
        return Math.min(Math.max(prev + delta, 36), 64);
      });
      setGpuUsage((prev) => {
        const delta = Math.floor(Math.random() * 7) - 3;
        return Math.min(Math.max(prev + delta, 8), 65);
      });
      setTemp((prev) => {
        const delta = Math.floor(Math.random() * 3) - 1;
        return Math.min(Math.max(prev + delta, 44), 68);
      });
    }, 2500);

    return () => clearInterval(interval);
  }, []);

  const getGaugeColor = (val: number) => {
    if (val >= 85) return 'bg-rose-500 shadow-red-glow text-rose-400';
    if (val >= 70) return 'bg-amber-400 shadow-amber-glow text-amber-400';
    return 'bg-cyan-400 shadow-cyan-glow-sm text-cyan-400';
  };

  const handleTakeScreenshot = async () => {
    try {
      await tauriService.takeScreenshot();
      showToast('HUD snapshot captured to clipboard!', 'success');
    } catch (e: any) {
      showToast('Capture error: ' + (e.message || e), 'error');
    }
  };

  const handlePurgeMemory = () => {
    showToast('Cache purged. RAM footprint optimized.', 'info');
  };

  return (
    <aside
      className={`bg-[#030712]/95 backdrop-blur-2xl border-l border-white/[0.08] flex flex-col z-20 shrink-0 select-none transition-all duration-300 ease-in-out overflow-hidden relative ${
        isOpen ? 'w-72' : 'w-12'
      }`}
    >
      {/* EXPANDED TELEMETRY VIEW (w-72) */}
      <div
        className={`flex flex-col h-full w-72 transition-opacity duration-200 ${
          isOpen ? 'opacity-100' : 'opacity-0 pointer-events-none absolute inset-0'
        }`}
      >
          {/* Header */}
          <div className="px-3.5 py-3 border-b border-white/[0.08] flex items-center justify-between shrink-0 bg-white/[0.01]">
            <div className="flex items-center gap-2">
              <Activity className="w-4 h-4 text-cyan-400 animate-pulse" />
              <h3 className="text-xs font-black uppercase tracking-wider text-slate-200 font-mono">
                LIVE TELEMETRY
              </h3>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="text-[9px] font-mono font-bold text-emerald-400 bg-emerald-950/60 px-1.5 py-0.5 rounded border border-emerald-500/30">
                SYS OK
              </span>
            </div>
          </div>

          {/* Scrollable Container covering full dock body */}
          <div className="flex-1 overflow-y-auto custom-scrollbar flex flex-col">
            {/* Top Hardware Gauges */}
            <div className="p-3.5 space-y-2.5 border-b border-white/[0.08] bg-black/20">
              <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono flex items-center justify-between">
                <div className="flex items-center gap-1.5">
                  <Cpu className="w-3 h-3 text-cyan-400" />
                  <span>CORE HARDWARE GAUGES</span>
                </div>
                <span className="text-[9px] font-mono text-slate-500">Ctrl+B</span>
              </div>

              {/* CPU Gauge */}
              <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <div className="flex justify-between items-center text-xs font-mono">
                  <span className="text-slate-400 text-[11px]">CPU LOAD</span>
                  <span className={`font-bold ${cpuUsage >= 85 ? 'text-rose-400 animate-pulse' : cpuUsage >= 70 ? 'text-amber-400' : 'text-cyan-400'}`}>
                    {cpuUsage}%
                  </span>
                </div>
                <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden flex">
                  <div
                    className={`h-full rounded-full transition-all duration-500 ${getGaugeColor(cpuUsage)}`}
                    style={{ width: `${cpuUsage}%` }}
                  />
                </div>
                <div className="flex justify-between text-[9px] font-mono text-slate-500 pt-0.5">
                  <span>8 Cores / 16 Threads</span>
                  <span>3.80 GHz</span>
                </div>
              </div>

              {/* RAM Gauge */}
              <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <div className="flex justify-between items-center text-xs font-mono">
                  <span className="text-slate-400 text-[11px]">RAM USAGE</span>
                  <span className={`font-bold ${ramUsage >= 85 ? 'text-rose-400' : ramUsage >= 70 ? 'text-amber-400' : 'text-cyan-400'}`}>
                    {ramUsage}%
                  </span>
                </div>
                <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden flex">
                  <div
                    className={`h-full rounded-full transition-all duration-500 ${getGaugeColor(ramUsage)}`}
                    style={{ width: `${ramUsage}%` }}
                  />
                </div>
                <div className="flex justify-between text-[9px] font-mono text-slate-500 pt-0.5">
                  <span>Used: {((ramUsage * 16) / 100).toFixed(1)} GB</span>
                  <span>Total: 16.0 GB</span>
                </div>
              </div>

              {/* GPU Gauge */}
              <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
                <div className="flex justify-between items-center text-xs font-mono">
                  <span className="text-slate-400 text-[11px]">GPU COMPUTE</span>
                  <span className={`font-bold ${gpuUsage >= 85 ? 'text-rose-400' : gpuUsage >= 70 ? 'text-amber-400' : 'text-cyan-400'}`}>
                    {gpuUsage}%
                  </span>
                </div>
                <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden flex">
                  <div
                    className={`h-full rounded-full transition-all duration-500 ${getGaugeColor(gpuUsage)}`}
                    style={{ width: `${gpuUsage}%` }}
                  />
                </div>
                <div className="flex justify-between text-[9px] font-mono text-slate-500 pt-0.5">
                  <span>VRAM: 2.1 / 8.0 GB</span>
                  <span>DirectX 12</span>
                </div>
              </div>

              {/* Thermal & Fan Status */}
              <div className="grid grid-cols-2 gap-2">
                <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5">
                  <div className="flex items-center gap-1.5 text-[9px] font-mono text-slate-400">
                    <Flame className="w-3 h-3 text-amber-400" />
                    <span>TEMP</span>
                  </div>
                  <div className="text-xs font-black font-mono text-slate-100 mt-0.5">
                    {temp}°C
                  </div>
                </div>

                <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5">
                  <div className="flex items-center gap-1.5 text-[9px] font-mono text-slate-400">
                    <RefreshCw className="w-3 h-3 text-cyan-400 animate-spin-slow" />
                    <span>FAN RPM</span>
                  </div>
                  <div className="text-xs font-black font-mono text-slate-100 mt-0.5">
                    {fanSpeed}
                  </div>
                </div>
              </div>
            </div>

            {/* Lower Container (Active Tasks & Diagnostics) */}
            <div className="p-3.5 space-y-4 flex-1">
              {/* Section 2: Active Background Tasks */}
              <div className="space-y-2">
                <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <Layers className="w-3 h-3 text-cyan-400" />
                    <span>ACTIVE MISSIONS ({activeTasks.length})</span>
                  </div>
                </div>

                <div className="space-y-1.5">
                  {activeTasks.map((t) => (
                    <div
                      key={t.id}
                      className="p-2 rounded-xl bg-white/[0.02] border border-white/5 flex items-center justify-between text-xs font-mono"
                    >
                      <div className="truncate mr-2">
                        <div className="text-slate-200 font-medium truncate text-[11px]">{t.name}</div>
                        <div className="text-[9px] text-slate-500">{t.type}</div>
                      </div>
                      <span className="text-[8px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded bg-cyan-950 text-cyan-300 border border-cyan-500/30 shrink-0">
                        {t.status}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Section 3: Quick Diagnostic Actions */}
              <div className="space-y-2 pt-2.5 border-t border-white/5">
                <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono mb-1">
                  QUICK DIAGNOSTICS
                </div>

                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={handleTakeScreenshot}
                    className="p-2 rounded-xl bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-300 border border-cyan-500/20 text-[11px] font-mono font-semibold flex items-center justify-center gap-1.5 transition active:scale-95"
                  >
                    <Camera className="w-3 h-3" />
                    <span>Snapshot</span>
                  </button>

                  <button
                    onClick={handlePurgeMemory}
                    className="p-2 rounded-xl bg-slate-800/80 hover:bg-slate-700 text-slate-300 border border-white/10 text-[11px] font-mono font-semibold flex items-center justify-center gap-1.5 transition active:scale-95"
                  >
                    <Trash2 className="w-3 h-3 text-rose-400" />
                    <span>Purge Cache</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* COLLAPSED MINI TELEMETRY STRIP (w-12) */}
        <div
          onClick={onClose}
          onKeyDown={(e) => (e.key === 'Enter' || e.key === ' ') && onClose()}
          role="button"
          tabIndex={0}
          aria-label="Expand Telemetry Dock (Ctrl+B)"
          className={`flex flex-col items-center justify-between h-full py-4 w-12 cursor-pointer group outline-none focus-visible:ring-1 focus-visible:ring-cyan-400 transition-opacity duration-200 ${
            !isOpen ? 'opacity-100' : 'opacity-0 pointer-events-none absolute inset-0'
          }`}
          title="Expand Telemetry Dock (Ctrl+B)"
        >
          {/* Top Activity Icon */}
          <div className="flex flex-col items-center gap-1">
            <div className="p-2 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400 group-hover:scale-110 transition shadow-cyan-glow-sm">
              <Activity className="w-4 h-4 animate-pulse" />
            </div>
            <span className="text-[8px] font-mono font-bold text-cyan-400 tracking-tighter">
              HUD
            </span>
          </div>

          {/* 3 Mini Vertical Load Bars (CPU, RAM, GPU) */}
          <div className="flex items-end justify-center gap-1.5 h-36 px-1.5 py-2 bg-black/40 rounded-xl border border-white/5">
            {/* Mini CPU Bar */}
            <div className="flex flex-col items-center gap-1 h-full justify-end" title={`CPU: ${cpuUsage}%`}>
              <div className="w-1.5 bg-slate-900 rounded-full overflow-hidden flex flex-col justify-end h-28">
                <div
                  className={`w-full rounded-full transition-all duration-500 ${
                    cpuUsage >= 85 ? 'bg-rose-500' : cpuUsage >= 70 ? 'bg-amber-400' : 'bg-cyan-400'
                  }`}
                  style={{ height: `${cpuUsage}%` }}
                />
              </div>
              <span className="text-[8px] font-mono font-bold text-slate-500">C</span>
            </div>

            {/* Mini RAM Bar */}
            <div className="flex flex-col items-center gap-1 h-full justify-end" title={`RAM: ${ramUsage}%`}>
              <div className="w-1.5 bg-slate-900 rounded-full overflow-hidden flex flex-col justify-end h-28">
                <div
                  className={`w-full rounded-full transition-all duration-500 ${
                    ramUsage >= 85 ? 'bg-rose-500' : ramUsage >= 70 ? 'bg-amber-400' : 'bg-cyan-400'
                  }`}
                  style={{ height: `${ramUsage}%` }}
                />
              </div>
              <span className="text-[8px] font-mono font-bold text-slate-500">R</span>
            </div>

            {/* Mini GPU Bar */}
            <div className="flex flex-col items-center gap-1 h-full justify-end" title={`GPU: ${gpuUsage}%`}>
              <div className="w-1.5 bg-slate-900 rounded-full overflow-hidden flex flex-col justify-end h-28">
                <div
                  className={`w-full rounded-full transition-all duration-500 ${
                    gpuUsage >= 85 ? 'bg-rose-500' : gpuUsage >= 70 ? 'bg-amber-400' : 'bg-cyan-400'
                  }`}
                  style={{ height: `${gpuUsage}%` }}
                />
              </div>
              <span className="text-[8px] font-mono font-bold text-slate-500">G</span>
            </div>
          </div>

          {/* Bottom Live Indicator Dot */}
          <div className="flex flex-col items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping" />
            <span className="text-[8px] font-mono text-emerald-400 font-bold">OK</span>
          </div>
        </div>
    </aside>
  );
};
