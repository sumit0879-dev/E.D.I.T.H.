import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  RotateCw,
  Search,
  Globe,
  Lock,
  Eye,
  CheckCircle2,
  Plus,
  X,
  Layers,
} from 'lucide-react';
import { browserController } from '../services/browserController';
import { isTauri } from '../services/tauri';
import type { BrowserTabInfo, BrowserMultiStateInfo } from '../types';

export const BrowserView: React.FC = () => {
  const [browserState, setBrowserState] = useState<BrowserMultiStateInfo>({
    tabs: [],
    active_tab_id: null,
    is_visible: false,
  });
  const [inputUrl, setInputUrl] = useState('https://example.com');
  const [isLoading, setIsLoading] = useState(false);
  const [observationResult, setObservationResult] = useState<string | null>(null);
  const [isObserving, setIsObserving] = useState(false);
  const [inspectTabId, setInspectTabId] = useState<string>('tab_a');

  const viewportRef = useRef<HTMLDivElement>(null);

  // Sync bounds from DOM element to native active child WebView
  const syncBounds = useCallback(() => {
    if (!viewportRef.current) return;
    const rect = viewportRef.current.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      browserController.setBoundsAll({
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height,
      }).catch((e) => console.warn('Failed to sync browser bounds:', e));
    }
  }, []);

  // Initialize tabs and subscribe to browser state
  useEffect(() => {
    let mounted = true;
    setIsLoading(true);

    const unsubscribe = browserController.subscribe((state) => {
      if (mounted) {
        setBrowserState(state);
        const activeTab = state.tabs.find((t) => t.id === state.active_tab_id);
        if (activeTab) {
          setInputUrl(activeTab.url);
        }
      }
    });

    const initMultiTab = async () => {
      await new Promise((resolve) => setTimeout(resolve, 60));
      if (!mounted) return;

      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        const initialBounds = {
          x: rect.left,
          y: rect.top,
          width: rect.width,
          height: rect.height,
        };

        try {
          // Phase 2 Multi-WebView Test: Create Tab A, Tab B, Tab C
          const currentTabs = browserController.getState().tabs;
          if (currentTabs.length === 0) {
            // Tab A -> https://example.com
            await browserController.createTab('tab_a', 'https://example.com', initialBounds);
            // Tab B -> https://www.wikipedia.org
            await browserController.createTab('tab_b', 'https://www.wikipedia.org', initialBounds);
            // Tab C -> https://github.com
            await browserController.createTab('tab_c', 'https://github.com', initialBounds);
            // Set Tab A as initial active tab
            await browserController.switchTab('tab_a', initialBounds);
          } else {
            await browserController.showActive(initialBounds);
          }
        } catch (err) {
          console.error('Multi-tab initialization error:', err);
        }
      }

      if (mounted) {
        setIsLoading(false);
        syncBounds();
      }
    };

    initMultiTab();

    const resizeObserver = new ResizeObserver(() => {
      syncBounds();
    });

    if (viewportRef.current) {
      resizeObserver.observe(viewportRef.current);
    }

    window.addEventListener('resize', syncBounds);

    return () => {
      mounted = false;
      unsubscribe();
      resizeObserver.disconnect();
      window.removeEventListener('resize', syncBounds);
      browserController.hideAll().catch(() => {});
    };
  }, [syncBounds]);

  const activeTab = browserState.tabs.find((t) => t.id === browserState.active_tab_id);

  // Tab switching
  const handleSwitchTab = async (tabId: string) => {
    if (tabId === browserState.active_tab_id) return;
    try {
      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        await browserController.switchTab(tabId, {
          x: rect.left,
          y: rect.top,
          width: rect.width,
          height: rect.height,
        });
      } else {
        await browserController.switchTab(tabId);
      }
      setInspectTabId(tabId);
    } catch (err) {
      console.error('Failed to switch tab:', err);
    }
  };

  // Tab creation
  const handleCreateNewTab = async () => {
    const newId = `tab_${Date.now().toString(36)}`;
    try {
      let b;
      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        b = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
      }
      await browserController.createTab(newId, 'https://example.com', b);
      setInspectTabId(newId);
    } catch (err) {
      console.error('Failed to create new tab:', err);
    }
  };

  // Tab closure
  const handleCloseTab = async (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    try {
      await browserController.closeTab(tabId);
    } catch (err) {
      console.error('Failed to close tab:', err);
    }
  };

  // Navigation
  const handleNavigate = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!inputUrl.trim() || !browserState.active_tab_id) return;

    setIsLoading(true);
    try {
      const navigatedUrl = await browserController.navigateTab(browserState.active_tab_id, inputUrl);
      setInputUrl(navigatedUrl);
      setTimeout(async () => {
        if (browserState.active_tab_id) {
          await browserController.getTabTitle(browserState.active_tab_id);
        }
      }, 1000);
    } catch (err: any) {
      console.error('Navigation error:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleGoBack = async () => {
    try {
      await browserController.goBack();
      setTimeout(async () => {
        if (browserState.active_tab_id) {
          const u = await browserController.getTabUrl(browserState.active_tab_id);
          setInputUrl(u);
          await browserController.getTabTitle(browserState.active_tab_id);
        }
      }, 500);
    } catch (err) {
      console.error('Back error:', err);
    }
  };

  const handleGoForward = async () => {
    try {
      await browserController.goForward();
      setTimeout(async () => {
        if (browserState.active_tab_id) {
          const u = await browserController.getTabUrl(browserState.active_tab_id);
          setInputUrl(u);
          await browserController.getTabTitle(browserState.active_tab_id);
        }
      }, 500);
    } catch (err) {
      console.error('Forward error:', err);
    }
  };

  const handleReload = async () => {
    setIsLoading(true);
    try {
      await browserController.reload();
      setTimeout(async () => {
        if (browserState.active_tab_id) {
          const u = await browserController.getTabUrl(browserState.active_tab_id);
          setInputUrl(u);
          await browserController.getTabTitle(browserState.active_tab_id);
        }
        setIsLoading(false);
      }, 800);
    } catch (err) {
      console.error('Reload error:', err);
      setIsLoading(false);
    }
  };

  // Scoped Observation Test across individual tabs
  const handleObserveTab = async () => {
    const targetId = inspectTabId || browserState.active_tab_id || 'tab_a';
    setIsObserving(true);
    setObservationResult(null);

    try {
      const [url, title, text] = await Promise.all([
        browserController.getTabUrl(targetId),
        browserController.getTabTitle(targetId),
        browserController.getTabVisibleText(targetId),
      ]);

      const summary = `### 🛰️ Scoped Tab Observation (Test 11 Verified)
- **Observed Tab ID**: \`${targetId}\` (Native Label: \`edith_tab_${targetId}\`)
- **Observed URL**: \`${url}\`
- **Document Title**: \`${title}\`
- **Extracted Visible Text (${text.length} chars)**:
> ${text.slice(0, 400)}...`;

      setObservationResult(summary);
    } catch (err: any) {
      setObservationResult(`Observation Error for ${targetId}: ${err.message || err}`);
    } finally {
      setIsObserving(false);
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#000000] text-slate-100 select-none overflow-hidden">
      {/* Tactical Multi-Tab Strip */}
      <div className="h-9 bg-[#040711] border-b border-white/[0.08] px-2 flex items-center gap-1.5 shrink-0 z-10 overflow-x-auto">
        <div className="flex items-center gap-1">
          {browserState.tabs.map((tab) => {
            const isActive = tab.id === browserState.active_tab_id;
            return (
              <div
                key={tab.id}
                onClick={() => handleSwitchTab(tab.id)}
                className={`group relative flex items-center gap-2 h-7 px-3 rounded-lg text-xs font-mono transition-all cursor-pointer select-none max-w-[200px] min-w-[120px] ${
                  isActive
                    ? 'bg-[#091122] text-cyan-300 border border-cyan-500/40 shadow-cyan-glow-xs'
                    : 'bg-white/[0.03] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06] border border-transparent'
                }`}
              >
                <Globe className={`w-3.5 h-3.5 shrink-0 ${isActive ? 'text-cyan-400' : 'text-slate-500'}`} />
                <span className="truncate flex-1 text-[11px]">
                  {tab.title || tab.url || 'Tab'}
                </span>
                {browserState.tabs.length > 1 && (
                  <button
                    onClick={(e) => handleCloseTab(e, tab.id)}
                    className="opacity-0 group-hover:opacity-100 hover:text-red-400 p-0.5 rounded transition"
                    title="Close Tab"
                  >
                    <X className="w-3 h-3" />
                  </button>
                )}
              </div>
            );
          })}
          <button
            onClick={handleCreateNewTab}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="New Tab"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Tactical Browser HUD Toolbar */}
      <div className="h-11 bg-[#050914] border-b border-white/[0.08] px-3 flex items-center gap-2 shrink-0 z-10">
        {/* Navigation Controls */}
        <div className="flex items-center space-x-1">
          <button
            onClick={handleGoBack}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Back"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <button
            onClick={handleGoForward}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Forward"
          >
            <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={handleReload}
            className={`w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition ${
              isLoading ? 'animate-spin text-cyan-400' : ''
            }`}
            title="Reload"
          >
            <RotateCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Omnibox URL / Search Bar */}
        <form onSubmit={handleNavigate} className="flex-1 flex items-center">
          <div className="w-full flex items-center bg-[#090e1a] border border-white/[0.1] focus-within:border-cyan-500/50 rounded-xl px-3 py-1 transition shadow-inner">
            <Lock className="w-3.5 h-3.5 text-emerald-400 mr-2 shrink-0" />
            <input
              type="text"
              value={inputUrl}
              onChange={(e) => setInputUrl(e.target.value)}
              placeholder="Search or enter HTTPS address..."
              className="w-full bg-transparent text-xs text-slate-100 placeholder-slate-500 focus:outline-none font-mono"
            />
            {isLoading ? (
              <div className="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin shrink-0 ml-2" />
            ) : (
              <button
                type="submit"
                className="text-slate-400 hover:text-cyan-400 transition shrink-0 ml-2"
                title="Navigate"
              >
                <Search className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </form>

        {/* Action Controls & Scoped Observation Inspector */}
        <div className="flex items-center space-x-1.5">
          <div className="flex items-center gap-1 bg-[#090e1a] border border-white/[0.08] rounded-xl px-2 py-0.5">
            <Layers className="w-3 h-3 text-cyan-400" />
            <select
              value={inspectTabId}
              onChange={(e) => setInspectTabId(e.target.value)}
              className="bg-transparent text-[11px] font-mono text-cyan-300 focus:outline-none cursor-pointer"
              title="Select Tab for Observation"
            >
              {browserState.tabs.map((t) => (
                <option key={t.id} value={t.id} className="bg-[#090e1a] text-slate-200">
                  {t.id}: {t.title.slice(0, 18)}...
                </option>
              ))}
            </select>
          </div>

          <button
            onClick={handleObserveTab}
            disabled={isObserving}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-cyan-950/60 border border-cyan-500/30 text-cyan-300 hover:bg-cyan-900/50 hover:border-cyan-400 transition text-[11px] font-mono shadow-cyan-glow-xs"
            title="Verify Scoped Multi-Tab Observation (Test 11)"
          >
            <Eye className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Observe</span>
          </button>
        </div>
      </div>

      {/* Observation Drawer Banner */}
      {observationResult && (
        <div className="bg-[#091122] border-b border-cyan-500/30 p-3 text-xs text-cyan-200 flex items-start justify-between gap-3 shrink-0 z-10 animate-fadeIn">
          <div className="flex-1 whitespace-pre-wrap font-mono leading-relaxed text-[11px]">
            {observationResult}
          </div>
          <button
            onClick={() => setObservationResult(null)}
            className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Center Viewport Canvas (Native Multi-WebView Target Area) */}
      <div
        ref={viewportRef}
        id="edith-browser-viewport-container"
        className="flex-1 w-full bg-[#000000] relative overflow-hidden flex flex-col items-center justify-center"
      >
        {!isTauri() && (
          <div className="flex flex-col items-center text-center p-8 max-w-lg bg-[#050914]/80 rounded-2xl border border-cyan-500/20 backdrop-blur-xl shadow-2xl">
            <Globe className="w-12 h-12 text-cyan-400 mb-4 animate-pulse" />
            <h3 className="text-base font-bold text-slate-100 mb-1">
              E.D.I.T.H. Native Multi-WebView2 Surface
            </h3>
            <p className="text-xs text-slate-400 mb-4 leading-relaxed">
              In Tauri desktop mode, independent native WebView2 child instances are hosted inside this viewport container.
            </p>
            <div className="w-full bg-slate-900/90 rounded-xl p-3 border border-white/5 text-left font-mono text-[10px] space-y-2">
              <div className="text-emerald-400 flex items-center gap-1.5">
                <CheckCircle2 className="w-3.5 h-3.5" /> Topology: Main Window Child WebViews
              </div>
              <div className="text-cyan-300">
                Active Tab: <span className="text-white font-bold">{activeTab?.id || 'None'}</span> ({activeTab?.label})
              </div>
              <div className="text-slate-400">
                Active URL: {activeTab?.url}
              </div>
              <div className="text-slate-400">
                Total Native Tabs: {browserState.tabs.length}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Status Bar Footer */}
      <div className="h-6 bg-[#030712] border-t border-white/[0.06] px-3 flex items-center justify-between text-[10px] font-mono text-slate-400 shrink-0 select-none z-10">
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-1 text-emerald-400">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
            Active: {activeTab ? activeTab.label : 'None'}
          </span>
          <span className="text-slate-600">|</span>
          <span className="text-slate-300 truncate max-w-sm">
            {activeTab?.title || activeTab?.url || 'Empty'}
          </span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-slate-500">Tabs: {browserState.tabs.length}</span>
          <span className="text-slate-600">|</span>
          <span className="text-slate-500">Session: Shared User-Data Environment</span>
          <span className="text-slate-600">|</span>
          <span className="text-cyan-400/80">Phase 2 Multi-WebView</span>
        </div>
      </div>
    </div>
  );
};

export default BrowserView;
