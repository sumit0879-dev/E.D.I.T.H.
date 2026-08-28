import React from 'react';
import { useApp } from '../context/AppContext';
import type { ViewTab } from '../types';
import {
  MessageSquare,
  Terminal,
  Database,
  Cpu,
  Sliders,
  Shield,
} from 'lucide-react';

interface NavItem {
  id: ViewTab;
  label: string;
  sublabel: string;
  icon: React.ComponentType<{ className?: string }>;
  badge?: string;
  shortcut: string;
}

const navItems: NavItem[] = [
  { id: 'chat', label: 'Tactical AI Chat', sublabel: 'E.D.I.T.H. Core', icon: MessageSquare, shortcut: 'Alt+1' },
  { id: 'dev_agent', label: 'E.D.I.T.H. Dev Agent', sublabel: 'Autonomous Coder', icon: Terminal, badge: 'AGENT', shortcut: 'Alt+2' },
  { id: 'memory_bank', label: 'Vector Memory', sublabel: 'LanceDB RAG Bank', icon: Database, shortcut: 'Alt+3' },
  { id: 'plugins', label: 'Cyber Tools', sublabel: 'System & Web Tools', icon: Cpu, shortcut: 'Alt+4' },
  { id: 'settings', label: 'Config Suite', sublabel: 'Models & Endpoints', icon: Sliders, shortcut: 'Alt+5' },
];

export const TacticalNavRail: React.FC = () => {
  const { activeTab, setActiveTab } = useApp();

  return (
    <aside className="w-16 bg-[#030712]/95 backdrop-blur-2xl border-r border-white/[0.08] flex flex-col items-center py-3 select-none z-20 shrink-0 justify-between">
      {/* Top Section: Navigation Icons */}
      <div className="w-full flex flex-col items-center space-y-2">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = activeTab === item.id;

          return (
            <div key={item.id} className="relative group w-full flex justify-center px-2">
              {/* Active Left Neon Indicator Bar */}
              {isActive && (
                <div className="absolute left-0 top-1.5 bottom-1.5 w-1 bg-cyan-400 rounded-r-full shadow-cyan-glow-sm" />
              )}

              <button
                onClick={() => setActiveTab(item.id)}
                aria-label={item.label}
                className={`w-11 h-11 rounded-xl flex items-center justify-center transition-all duration-200 relative ${
                  isActive
                    ? 'bg-gradient-to-tr from-cyan-950/80 to-blue-900/50 border border-cyan-500/50 text-cyan-300 shadow-cyan-glow-sm'
                    : 'text-slate-400 hover:text-slate-100 hover:bg-white/[0.05] border border-transparent'
                }`}
              >
                <Icon className={`w-5 h-5 transition-transform duration-200 ${isActive ? 'scale-110 text-cyan-400' : 'group-hover:scale-105'}`} />

                {/* Micro Badge */}
                {item.badge && (
                  <span className="absolute -top-1 -right-1 text-[8px] font-mono font-bold bg-cyan-500 text-slate-950 px-1 py-0.2 rounded-full leading-none scale-90">
                    {item.badge}
                  </span>
                )}
              </button>

              {/* Hover Tooltip (Flyout) */}
              <div className="absolute left-16 top-1/2 -translate-y-1/2 ml-2 px-3 py-2 rounded-xl bg-[#090d16]/95 border border-cyan-500/30 text-white shadow-2xl backdrop-blur-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-150 z-50 pointer-events-none whitespace-nowrap min-w-[130px]">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-xs font-bold text-slate-100">{item.label}</span>
                  <span className="text-[9px] font-mono text-cyan-400 bg-cyan-950/60 px-1 py-0.2 rounded border border-cyan-500/30">
                    {item.shortcut}
                  </span>
                </div>
                <div className="text-[10px] text-slate-400 mt-0.5">{item.sublabel}</div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Bottom Section: Security Status Pill */}
      <div className="w-full flex flex-col items-center pt-3 border-t border-white/5 px-2">
        <div
          className="w-10 h-10 rounded-xl bg-slate-900/60 border border-white/5 flex items-center justify-center text-slate-400 hover:text-cyan-300 hover:border-cyan-500/30 transition cursor-pointer group"
          title="Stark Security Protocol: Online"
        >
          <Shield className="w-4 h-4 text-emerald-400 group-hover:animate-pulse" />
        </div>
      </div>
    </aside>
  );
};
