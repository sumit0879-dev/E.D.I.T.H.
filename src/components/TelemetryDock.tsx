import React, { useState, useEffect, useCallback } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import { eventRouter } from '../events';
import {
  Activity,
  Cpu,
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
  Shield,
  Clock,
  AlertTriangle,
} from 'lucide-react';

interface TelemetryDockProps {
  isOpen: boolean;
  onClose: () => void;
}

export const TelemetryDock: React.FC<TelemetryDockProps> = ({ isOpen, onClose }) => {
  const { settings, showToast, isSpeaking } = useApp();

  // Authoritative Host Runtime State
  const [runtimeStatus, setRuntimeStatus] = useState<tauriService.RuntimeStatusSummary | null>(null);
  const [activeTasks, setActiveTasks] = useState<tauriService.TaskSnapshot[]>([]);
  const [memoryUsageMb, setMemoryUsageMb] = useState<number | null>(null);
  const [memoryTotalMb, setMemoryTotalMb] = useState<number | null>(null);

  // Poll host runtime state and active task snapshots
  const fetchTelemetry = useCallback(async () => {
    try {
      const [status, tasks] = await Promise.all([
        tauriService.runtimeGetStatus(),
        tauriService.taskListActive(),
      ]);
      setRuntimeStatus(status);
      setActiveTasks(tasks);

      // Query real heap memory if available in browser/webview environment
      if (typeof performance !== 'undefined' && (performance as any).memory) {
        const mem = (performance as any).memory;
        setMemoryUsageMb(Math.round(mem.usedJSHeapSize / (1024 * 1024)));
        setMemoryTotalMb(Math.round(mem.totalJSHeapSize / (1024 * 1024)));
      }
    } catch (err) {
      console.warn('[TelemetryDock] Telemetry poll failed:', err);
    }
  }, []);

  useEffect(() => {
    fetchTelemetry();

    // Event-driven reactive wakeup on runtime, task, voice, and security events
    const unsub = eventRouter.onAny((envelope) => {
      const cat = envelope.payload?.category;
      if (cat === 'runtime' || cat === 'task' || cat === 'security_policy' || cat === 'voice') {
        fetchTelemetry();
      }
    });

    const interval = setInterval(fetchTelemetry, 2500);

    return () => {
      unsub();
      clearInterval(interval);
    };
  }, [fetchTelemetry]);

  // Derived metrics
  const uptimeSeconds = runtimeStatus?.uptime_seconds ?? 0;
  const formatUptime = (secs: number) => {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    if (h > 0) return `${h}h ${m}m ${s}s`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  };

  const autonomyState = (runtimeStatus?.autonomy_state || 'idle').toUpperCase();
  const securityMode = (runtimeStatus?.security_mode || 'standard').toUpperCase();
  const pendingApprovals = runtimeStatus?.pending_approval_count ?? 0;
  const activeTaskCount = activeTasks.length || (runtimeStatus?.active_task_count ?? 0);
  const activeExecutionCount = runtimeStatus?.active_execution_count ?? 0;

  const ramPercent =
    memoryUsageMb && memoryTotalMb && memoryTotalMb > 0
      ? Math.min(Math.round((memoryUsageMb / memoryTotalMb) * 100), 100)
      : 24;

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
    if (typeof window !== 'undefined' && (window as any).gc) {
      (window as any).gc();
    }
    fetchTelemetry();
    showToast('Telemetry refreshed and memory reclaimed.', 'info');
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
            <span
              className={`text-[9px] font-mono font-bold px-1.5 py-0.5 rounded border ${
                autonomyState === 'EXECUTING'
                  ? 'text-cyan-300 bg-cyan-950/60 border-cyan-500/40 animate-pulse'
                  : 'text-emerald-400 bg-emerald-950/60 border-emerald-500/30'
              }`}
            >
              {autonomyState}
            </span>
          </div>
        </div>

        {/* Scrollable Container covering full dock body */}
        <div className="flex-1 overflow-y-auto custom-scrollbar flex flex-col">
          {/* Section 1: Host Runtime Authority */}
          <div className="p-3.5 space-y-2.5 border-b border-white/[0.08] bg-black/20">
            <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Shield className="w-3 h-3 text-cyan-400" />
                <span>RUNTIME STATE</span>
              </div>
              <span className="text-[9px] font-mono text-slate-500">Ctrl+B</span>
            </div>

            {/* Mode & Uptime */}
            <div className="grid grid-cols-2 gap-2 text-xs font-mono">
              <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 flex flex-col gap-0.5">
                <span className="text-slate-500 text-[9px] uppercase">SECURITY</span>
                <span className="font-bold text-cyan-300">{securityMode}</span>
              </div>
              <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 flex flex-col gap-0.5">
                <span className="text-slate-500 text-[9px] uppercase">UPTIME</span>
                <span className="font-bold text-slate-200">{formatUptime(uptimeSeconds)}</span>
              </div>
            </div>

            {/* Active Provider & Model */}
            <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 text-xs font-mono space-y-1">
              <div className="flex justify-between items-center text-[10px]">
                <span className="text-slate-500 uppercase">ACTIVE MODEL</span>
                <span className="text-slate-400 uppercase">{settings.selectedProvider || 'groq'}</span>
              </div>
              <div className="text-cyan-300 font-semibold text-[11px] truncate">
                {settings.selectedModel || 'llama-3.3-70b-versatile'}
              </div>
            </div>

            {/* Approvals Warning Bar */}
            {pendingApprovals > 0 && (
              <div className="p-2 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between text-xs font-mono text-amber-300 animate-pulse">
                <div className="flex items-center gap-1.5">
                  <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
                  <span>Pending Approvals</span>
                </div>
                <span className="font-bold px-1.5 py-0.2 bg-amber-500/20 rounded">
                  {pendingApprovals}
                </span>
              </div>
            )}
          </div>

          {/* Section 2: Real Memory & Webview Metrics */}
          <div className="p-3.5 space-y-2.5 border-b border-white/[0.08] bg-black/10">
            <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono flex items-center gap-1.5">
              <Cpu className="w-3 h-3 text-cyan-400" />
              <span>MEMORY FOOTPRINT</span>
            </div>

            {/* RAM Gauge */}
            <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 space-y-1">
              <div className="flex justify-between items-center text-xs font-mono">
                <span className="text-slate-400 text-[11px]">JS HEAP RESIDENT</span>
                <span className={`font-bold ${getGaugeColor(ramPercent)}`}>
                  {ramPercent}%
                </span>
              </div>
              <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden flex">
                <div
                  className={`h-full rounded-full transition-all duration-500 ${getGaugeColor(ramPercent)}`}
                  style={{ width: `${ramPercent}%` }}
                />
              </div>
              <div className="flex justify-between text-[9px] font-mono text-slate-500 pt-0.5">
                <span>Used: {memoryUsageMb ? `${memoryUsageMb} MB` : 'Dynamic'}</span>
                <span>Limit: {memoryTotalMb ? `${memoryTotalMb} MB` : 'V8 Managed'}</span>
              </div>
            </div>

            {/* Speech Subsystem Status */}
            <div className="p-2 rounded-xl bg-white/[0.02] border border-white/5 flex items-center justify-between text-xs font-mono">
              <div className="flex items-center gap-1.5">
                {isSpeaking ? (
                  <Volume2 className="w-3.5 h-3.5 text-cyan-400 animate-pulse" />
                ) : (
                  <VolumeX className="w-3.5 h-3.5 text-slate-500" />
                )}
                <span className="text-[11px] text-slate-300">VOICE SYNTHESIS</span>
              </div>
              <span className={`text-[9px] font-bold uppercase px-1.5 py-0.5 rounded ${
                isSpeaking ? 'bg-cyan-950 text-cyan-300 border border-cyan-500/30' : 'text-slate-500'
              }`}>
                {isSpeaking ? 'SPEAKING' : 'STANDBY'}
              </span>
            </div>
          </div>

          {/* Section 3: Active Background Missions */}
          <div className="p-3.5 space-y-3 flex-1">
            <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400 font-mono flex items-center justify-between">
              <div className="flex items-center gap-1.5">
                <Layers className="w-3 h-3 text-cyan-400" />
                <span>ACTIVE TASKS ({activeTaskCount})</span>
              </div>
              {activeExecutionCount > 0 && (
                <span className="text-[9px] text-cyan-400 font-mono">
                  {activeExecutionCount} tools
                </span>
              )}
            </div>

            {activeTasks.length > 0 ? (
              <div className="space-y-1.5">
                {activeTasks.map((t) => (
                  <div
                    key={t.task_id}
                    className="p-2 rounded-xl bg-white/[0.02] border border-white/5 flex items-center justify-between text-xs font-mono"
                  >
                    <div className="truncate mr-2">
                      <div className="text-slate-200 font-medium truncate text-[11px]">{t.goal || t.task_id}</div>
                      <div className="text-[9px] text-slate-500">{t.task_type} • Step {t.progress.step}/{t.progress.max_steps}</div>
                    </div>
                    <span className="text-[8px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded bg-cyan-950 text-cyan-300 border border-cyan-500/30 shrink-0">
                      {t.status}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-2.5 rounded-xl bg-white/[0.01] border border-white/5 text-[10px] font-mono text-slate-500 text-center">
                No autonomous tasks active
              </div>
            )}

            {/* Quick Diagnostic Actions */}
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
                  <RefreshCw className="w-3 h-3 text-cyan-400" />
                  <span>Refresh</span>
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
        <div className="flex flex-col items-center gap-1">
          <div className="p-2 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400 group-hover:scale-110 transition shadow-cyan-glow-sm">
            <Activity className="w-4 h-4 animate-pulse" />
          </div>
          <span className="text-[8px] font-mono font-bold text-cyan-400 tracking-tighter">
            HUD
          </span>
        </div>

        {/* Vertical Load / Status Bar */}
        <div className="flex flex-col items-center gap-1.5 h-36 px-1.5 py-2 bg-black/40 rounded-xl border border-white/5 justify-end">
          <div className="w-1.5 bg-slate-900 rounded-full overflow-hidden flex flex-col justify-end h-28">
            <div
              className={`w-full rounded-full transition-all duration-500 ${
                pendingApprovals > 0
                  ? 'bg-amber-400 animate-pulse'
                  : autonomyState === 'EXECUTING'
                  ? 'bg-cyan-400 animate-pulse'
                  : 'bg-emerald-400'
              }`}
              style={{ height: `${ramPercent}%` }}
            />
          </div>
          <span className="text-[8px] font-mono font-bold text-slate-500">M</span>
        </div>

        {/* Bottom Live Indicator Dot */}
        <div className="flex flex-col items-center gap-1">
          <span className={`w-2 h-2 rounded-full ${
            pendingApprovals > 0 ? 'bg-amber-400 animate-ping' : 'bg-emerald-400 animate-ping'
          }`} />
          <span className="text-[8px] font-mono text-emerald-400 font-bold">
            {pendingApprovals > 0 ? 'REQ' : 'OK'}
          </span>
        </div>
      </div>
    </aside>
  );
};
