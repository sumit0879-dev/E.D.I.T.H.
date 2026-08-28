import React from 'react';
import { AppProvider, useApp } from './context/AppContext';
import { TopHudBar } from './components/TopHudBar';
import { TacticalNavRail } from './components/TacticalNavRail';
import { TelemetryDock } from './components/TelemetryDock';
import { ToastContainer } from './components/Toast';

import { ChatView } from './views/ChatView';
import { BrowserView } from './views/BrowserView';
import { DevAgentView } from './views/DevAgentView';
import { MemoryBankView } from './views/MemoryBankView';
import { PluginsView } from './views/PluginsView';
import { SettingsView } from './views/SettingsView';
import { browserController } from './services/browserController';

const MainLayout: React.FC = () => {
  const { activeTab, setActiveTab, isTelemetryOpen, toggleTelemetry } = useApp();

  // Browser WebView Visibility Lifecycle
  React.useEffect(() => {
    if (activeTab === 'browser') {
      browserController.show().catch(() => {});
    } else {
      browserController.hide().catch(() => {});
    }
  }, [activeTab]);

  // Global Keyboard Shortcuts (Ctrl+B for Telemetry Dock, Alt+1..6 for Tab navigation)
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle Telemetry Dock: Ctrl+B / Cmd+B
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        toggleTelemetry();
        return;
      }

      // Quick Tab Switching: Alt+1 through Alt+6
      if (e.altKey && !e.ctrlKey && !e.metaKey) {
        const tabMap: Record<string, typeof activeTab> = {
          '1': 'chat',
          '2': 'browser',
          '3': 'dev_agent',
          '4': 'memory_bank',
          '5': 'plugins',
          '6': 'settings',
        };
        if (tabMap[e.key]) {
          e.preventDefault();
          setActiveTab(tabMap[e.key]);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleTelemetry, setActiveTab]);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-[#000000] text-slate-100 antialiased font-sans select-none">
      {/* Top 48px HUD Header */}
      <TopHudBar
        isTelemetryOpen={isTelemetryOpen}
        onToggleTelemetry={toggleTelemetry}
      />

      {/* 3-Column Tactical HUD Body */}
      <div className="flex-1 flex overflow-hidden min-h-0 relative">
        {/* Column 1: Left 64px Icon Navigation Rail */}
        <TacticalNavRail />

        {/* Column 2: Adaptive Center Stage Viewport */}
        <main className="flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden relative bg-[#000000]">
          {activeTab === 'chat' && <ChatView />}
          {activeTab === 'browser' && <BrowserView />}
          {activeTab === 'dev_agent' && <DevAgentView />}
          {activeTab === 'memory_bank' && <MemoryBankView />}
          {activeTab === 'plugins' && <PluginsView />}
          {activeTab === 'settings' && <SettingsView />}
        </main>

        {/* Column 3: Right Telemetry Dock (Expanded w-72 / Collapsed w-12) */}
        <TelemetryDock
          isOpen={isTelemetryOpen}
          onClose={toggleTelemetry}
        />
      </div>

      {/* Floating System Notifications */}
      <ToastContainer />
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <AppProvider>
      <MainLayout />
    </AppProvider>
  );
};

export default App;
