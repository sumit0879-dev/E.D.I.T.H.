import React from 'react';
import { useApp } from '../context/AppContext';
import type { ViewTab } from '../types';
import {
  MessageSquare,
  Bot,
  Brain,
  Puzzle,
  Settings,
  Activity,
  ChevronRight,
  Zap,
} from 'lucide-react';

interface TabItem {
  id: ViewTab;
  label: string;
  sublabel: string;
  icon: React.ElementType;
  badge?: string;
  glow?: string;
}

const tabs: TabItem[] = [
  {
    id: 'chat',
    label: 'E.D.I.T.H. Chat',
    sublabel: 'Cloud & Custom LLMs',
    icon: MessageSquare,
    glow: 'from-cyan-500/20 to-blue-500/10',
  },
  {
    id: 'dev_agent',
    label: 'Dev Agent',
    sublabel: 'E.D.I.T.H. File & Terminal AI',
    icon: Bot,
    badge: 'E.D.I.T.H.',
    glow: 'from-violet-500/20 to-purple-500/10',
  },
  {
    id: 'memory_bank',
    label: 'Memory Bank',
    sublabel: 'LanceDB Semantic Vectors',
    icon: Brain,
    glow: 'from-amber-500/20 to-yellow-500/10',
  },
  {
    id: 'plugins',
    label: 'Plugins Hub',
    sublabel: 'System, Media, Terminal',
    icon: Puzzle,
    glow: 'from-blue-500/20 to-cyan-500/10',
  },
  {
    id: 'settings',
    label: 'Settings',
    sublabel: 'Keys, Custom Models, TTS',
    icon: Settings,
    glow: 'from-slate-500/20 to-zinc-500/10',
  },
];

export const Sidebar: React.FC = () => {
  const { activeTab, setActiveTab, settings, isSpeaking } = useApp();

  return (
    <aside className="w-64 bg-[#0c121e]/95 border-r border-white/10 flex flex-col justify-between shrink-0 select-none z-20 backdrop-blur-xl">
      {/* App Branding */}
      <div>
        <div className="p-4 flex items-center gap-3 border-b border-white/5">
          <div className="relative flex items-center justify-center w-10 h-10 rounded-xl bg-gradient-to-tr from-cyan-500 via-indigo-500 to-violet-500 shadow-lg shadow-cyan-500/20">
            <Zap className="w-5 h-5 text-white animate-pulse" />
            {isSpeaking && (
              <span className="absolute -top-1 -right-1 w-3 h-3 bg-emerald-400 rounded-full animate-ping" />
            )}
          </div>
          <div>
            <div className="flex items-center gap-1.5">
              <span className="font-black text-base tracking-wider text-white">E.D.I.T.H.</span>
              <span className="text-[9px] uppercase font-bold tracking-widest px-1.5 py-0.5 rounded bg-cyan-500/20 text-cyan-400 border border-cyan-500/30">
                v2.0
              </span>
            </div>
            <p className="text-[11px] text-slate-400 font-mono tracking-tight">Even Dead, I'm The Hero</p>
          </div>
        </div>

        {/* Navigation Tabs */}
        <nav className="p-2 space-y-1">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;

            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={
                  'w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-left transition-all duration-200 group relative ' +
                  (isActive
                    ? 'bg-gradient-to-r ' + (tab.glow || '') + ' border border-white/15 shadow-md shadow-black/40 text-white'
                    : 'text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
                }
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div
                    className={
                      'p-2 rounded-lg transition ' +
                      (isActive
                        ? 'bg-white/10 text-cyan-400 shadow-inner'
                        : 'bg-white/[0.03] text-slate-400 group-hover:text-slate-200 group-hover:bg-white/[0.06]')
                    }
                  >
                    <Icon className="w-4 h-4" />
                  </div>
                  <div className="truncate">
                    <div className="text-xs font-semibold tracking-wide flex items-center gap-1.5">
                      <span>{tab.label}</span>
                      {tab.badge && (
                        <span className="text-[9px] font-bold px-1.5 py-0.2 bg-violet-500/20 text-violet-300 rounded border border-violet-500/30">
                          {tab.badge}
                        </span>
                      )}
                    </div>
                    <div className="text-[10px] text-slate-500 truncate group-hover:text-slate-400">
                      {tab.sublabel}
                    </div>
                  </div>
                </div>

                {isActive && (
                  <ChevronRight className="w-4 h-4 text-cyan-400 shrink-0 ml-1" />
                )}
              </button>
            );
          })}
        </nav>
      </div>

      {/* Footer System Status Card */}
      <div className="p-3 border-t border-white/5 bg-black/20 m-2 rounded-xl">
        <div className="flex items-center justify-between text-[11px] text-slate-400 mb-1.5">
          <div className="flex items-center gap-1.5">
            <Activity className="w-3.5 h-3.5 text-emerald-400" />
            <span>AI Provider</span>
          </div>
          <span className="font-mono text-cyan-300 uppercase text-[10px] px-1.5 py-0.5 bg-cyan-500/10 rounded border border-cyan-500/20 truncate max-w-[110px]">
            {settings.aiMode === 'local' ? 'Local GGUF' : settings.selectedProvider || 'groq'}
          </span>
        </div>

        <div className="text-[11px] text-slate-400 flex items-center justify-between">
          <span>Active Model:</span>
          <span className="text-[10px] font-mono font-semibold text-cyan-400 truncate max-w-[120px]">
            {settings.aiMode === 'local' ? 'Local GGUF' : settings.selectedModel || 'llama-3.3-70b'}
          </span>
        </div>
      </div>
    </aside>
  );
};
