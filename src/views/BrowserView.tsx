import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  RotateCw,
  Search,
  Globe,
  Lock,
  Eye,
  Camera,
  CheckCircle2,
  AlertTriangle,
  Plus,
  X,
  Layers,
  Sparkles,
  RefreshCw,
  Loader2,
  Code2,
  MousePointer,
  Keyboard,
  ArrowUpDown,
  Play,
  Clock,
  ShieldCheck,
  Bot,
  Square,
  Flame,
  Check,
} from 'lucide-react';
import { browserController } from '../services/browserController';
import { isTauri } from '../services/tauri';
import { listen } from '@tauri-apps/api/event';
import type {
  BrowserTabInfo,
  BrowserMultiStateInfo,
  PageObservationSnapshot,
  ScreenshotResult,
  BrowserActionResult,
  BrowserTaskResult,
  ElementInfo,
  BrowserRiskAuditEntry,
  BrowserOrchestrationResult,
  BrowserOrchestrationTask,
  TabControlInfo,
  BrowserControlState,
} from '../types';
import { Shield, AlertOctagon, FileCheck, CheckCircle, Network, GitBranch, UserCheck, User } from 'lucide-react';

export const BrowserView: React.FC = () => {
  const [browserState, setBrowserState] = useState<BrowserMultiStateInfo>({
    tabs: [],
    active_tab_id: null,
    is_visible: false,
  });
  const [inputUrl, setInputUrl] = useState('https://example.com');
  const [isLoading, setIsLoading] = useState(false);
  const [isOmniboxFocused, setIsOmniboxFocused] = useState(false);
  
  // Phase 3 Live Observation & Screenshot states
  const [liveSnapshot, setLiveSnapshot] = useState<PageObservationSnapshot | null>(null);
  const [isObserving, setIsObserving] = useState(false);
  const [inspectTabId, setInspectTabId] = useState<string>('tab_a');
  const [screenshotPreview, setScreenshotPreview] = useState<ScreenshotResult | null>(null);
  const [isCapturingScreen, setIsCapturingScreen] = useState(false);

  // Phase 4A Action Layer States
  const [showActionPanel, setShowActionPanel] = useState(false);
  const [selectedAction, setSelectedAction] = useState<'click' | 'type' | 'scroll' | 'press_key' | 'focus' | 'wait'>('click');
  const [targetElementId, setTargetElementId] = useState<string>('');
  const [typeText, setTypeText] = useState<string>('Hello from E.D.I.T.H.');
  const [scrollDirection, setScrollDirection] = useState<string>('down');
  const [keyToPress, setKeyToPress] = useState<string>('Enter');
  const [waitCondition, setWaitCondition] = useState<string>('timeout');
  const [isExecutingAction, setIsExecutingAction] = useState(false);
  const [lastActionResult, setLastActionResult] = useState<BrowserActionResult | null>(null);

  // Phase 4C Autonomous Browser Agent States
  const [showAgentPanel, setShowAgentPanel] = useState(false);
  const [agentGoal, setAgentGoal] = useState<string>('Open example.com, observe the page title, click More information, and verify the new URL.');
  const [agentMaxSteps, setAgentMaxSteps] = useState<number>(10);
  const [isAgentRunning, setIsAgentRunning] = useState<boolean>(false);
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);
  const [agentLiveStatus, setAgentLiveStatus] = useState<{ step: number; max_steps: number; message: string; status: string } | null>(null);
  const [agentTaskResult, setAgentTaskResult] = useState<BrowserTaskResult | null>(null);

  // Phase 5.3 Risk & Safety Engine States
  const [showRiskPanel, setShowRiskPanel] = useState(false);
  const [riskAuditLogs, setRiskAuditLogs] = useState<BrowserRiskAuditEntry[]>([]);
  const [isFetchingLogs, setIsFetchingLogs] = useState(false);

  // Phase 5.4 Multi-Tab Orchestration States
  const [showOrchestratorPanel, setShowOrchestratorPanel] = useState(false);
  const [orchGoal, setOrchGoal] = useState<string>('Compare documentation across 3 research tabs.');
  const [orchSubgoals, setOrchSubgoals] = useState<string>('Observe https://en.wikipedia.org\nObserve https://www.rust-lang.org\nObserve https://v2.tauri.app');
  const [isOrchRunning, setIsOrchRunning] = useState<boolean>(false);
  const [currentOrchId, setCurrentOrchId] = useState<string | null>(null);
  const [orchResult, setOrchResult] = useState<BrowserOrchestrationResult | null>(null);
  const [orchLiveStatus, setOrchLiveStatus] = useState<any>(null);

  // Phase 5.5 Human <-> AI Browser Control States
  const [tabControls, setTabControls] = useState<Record<string, TabControlInfo>>({});

  const fetchTabControls = useCallback(async () => {
    try {
      const controls = await browserController.getAllTabControls();
      const map: Record<string, TabControlInfo> = {};
      controls.forEach((c) => {
        map[c.tab_id] = c;
      });
      setTabControls(map);
    } catch (e) {
      console.warn('Failed to fetch tab controls', e);
    }
  }, []);

  const handleTakeover = async (tabId: string) => {
    try {
      const res = await browserController.takeoverTab(tabId, 'Operator clicked Take Control');
      setTabControls((prev) => ({ ...prev, [tabId]: res }));
    } catch (e) {
      console.error('Takeover failed', e);
    }
  };

  const handleGrantAi = async (tabId: string) => {
    try {
      const res = await browserController.requestAiControl(tabId);
      setTabControls((prev) => ({ ...prev, [tabId]: res }));
    } catch (e) {
      console.error('Grant AI failed', e);
    }
  };

  const fetchRiskAuditLogs = useCallback(async () => {
    setIsFetchingLogs(true);
    try {
      const logs = await browserController.getRiskAuditLog();
      setRiskAuditLogs(logs.reverse());
    } catch (e) {
      console.error('Failed to fetch risk audit logs', e);
    } finally {
      setIsFetchingLogs(false);
    }
  }, []);

  const viewportRef = useRef<HTMLDivElement>(null);
  const omniboxInputRef = useRef<HTMLInputElement>(null);

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

  // Listen for agent status events from Rust backend
  useEffect(() => {
    if (!isTauri()) return;

    const unlistenStatus = listen<{ task_id: string; status: string; step?: number; max_steps?: number; message?: string; summary?: string; error?: string }>(
      'browser-agent-status',
      (event) => {
        const payload = event.payload;
        setAgentLiveStatus({
          step: payload.step || 0,
          max_steps: payload.max_steps || agentMaxSteps,
          message: payload.message || payload.summary || payload.error || payload.status,
          status: payload.status,
        });
      }
    );

    return () => {
      unlistenStatus.then((un) => un()).catch(() => {});
    };
  }, [agentMaxSteps]);

  // Initialize tabs and subscribe to browser state
  useEffect(() => {
    let mounted = true;
    setIsLoading(true);

    const unsubscribe = browserController.subscribe((state) => {
      if (mounted) {
        setBrowserState(state);
        const activeTab = state.tabs.find((t) => t.id === state.active_tab_id);
        if (activeTab && !isOmniboxFocused) {
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
          const currentTabs = browserController.getState().tabs;
          if (currentTabs.length === 0) {
            await browserController.createTab('tab_a', 'https://example.com', initialBounds);
            await browserController.createTab('tab_b', 'https://www.wikipedia.org', initialBounds);
            await browserController.createTab('tab_c', 'https://github.com', initialBounds);
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
  }, [syncBounds, isOmniboxFocused]);

  // Phase 3 Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        const newId = `tab_${Date.now().toString(36)}`;
        await browserController.createTab(newId, 'https://example.com');
        setInspectTabId(newId);
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        const restored = await browserController.reopenLastClosedTab();
        if (restored) setInspectTabId(restored.id);
        return;
      }

      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        if (browserState.active_tab_id) {
          await browserController.closeTab(browserState.active_tab_id);
        }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === 'Tab') {
        e.preventDefault();
        if (e.shiftKey) {
          await browserController.switchToPrevTab();
        } else {
          await browserController.switchToNextTab();
        }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'l') {
        e.preventDefault();
        omniboxInputRef.current?.focus();
        omniboxInputRef.current?.select();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r') {
        e.preventDefault();
        await browserController.reload();
        return;
      }

      if (e.altKey && e.key === 'ArrowLeft') {
        e.preventDefault();
        await browserController.goBack();
        return;
      }

      if (e.altKey && e.key === 'ArrowRight') {
        e.preventDefault();
        await browserController.goForward();
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [browserState.active_tab_id]);

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
          await browserController.observeTab(browserState.active_tab_id);
        }
      }, 1000);
    } catch (err: any) {
      console.error('Navigation error:', err);
    } finally {
      setIsLoading(false);
      omniboxInputRef.current?.blur();
    }
  };

  const handleOmniboxKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      if (activeTab) {
        setInputUrl(activeTab.url);
      }
      omniboxInputRef.current?.blur();
    }
  };

  // Live DOM Observation Handler
  const handleObserveLiveTab = async () => {
    const targetId = inspectTabId || browserState.active_tab_id || 'tab_a';
    setIsObserving(true);
    setLiveSnapshot(null);

    try {
      const snapshot = await browserController.observeTab(targetId);
      setLiveSnapshot(snapshot);
      if (snapshot.interactive_elements.length > 0 && !targetElementId) {
        setTargetElementId(snapshot.interactive_elements[0].id);
      }
    } catch (err: any) {
      console.error(`Live observation error for ${targetId}:`, err);
    } finally {
      setIsObserving(false);
    }
  };

  // Screenshot Handler
  const handleCaptureScreenshot = async () => {
    const targetId = browserState.active_tab_id || 'tab_a';
    setIsCapturingScreen(true);
    try {
      const res = await browserController.screenshotTab(targetId);
      setScreenshotPreview(res);
    } catch (err) {
      console.error('Screenshot error:', err);
    } finally {
      setIsCapturingScreen(false);
    }
  };

  // Phase 4A Action Layer Execution with Verification Loop
  const handleExecuteAction = async () => {
    const targetId = browserState.active_tab_id || 'tab_a';
    setIsExecutingAction(true);
    setLastActionResult(null);

    try {
      let result: BrowserActionResult;

      switch (selectedAction) {
        case 'click':
          result = await browserController.clickElement(targetElementId, targetId);
          break;
        case 'type':
          result = await browserController.typeElement(targetElementId, typeText, true, targetId);
          break;
        case 'scroll':
          result = await browserController.scroll(scrollDirection, 350, targetId);
          break;
        case 'press_key':
          result = await browserController.pressKey(keyToPress, targetId);
          break;
        case 'focus':
          result = await browserController.focusElement(targetElementId, targetId);
          break;
        case 'wait':
          result = await browserController.wait(waitCondition, targetElementId || undefined, 2000, targetId);
          break;
      }

      setLastActionResult(result);

      // Verification loop: Re-observe DOM to confirm effect
      setTimeout(async () => {
        try {
          const refreshed = await browserController.observeTab(targetId);
          setLiveSnapshot(refreshed);
        } catch (e) {}
      }, 300);
    } catch (err: any) {
      setLastActionResult({
        success: false,
        action: selectedAction,
        tab_id: targetId,
        page_changed: false,
        url_changed: false,
        error: String(err),
        error_code: 'ACTION_EXECUTION_ERROR',
      });
    } finally {
      setIsExecutingAction(false);
    }
  };

  // Phase 4C Autonomous Browser Agent Execution
  const handleRunAutonomousTask = async () => {
    if (!agentGoal.trim() || isAgentRunning) return;
    setIsAgentRunning(true);
    setAgentTaskResult(null);
    setAgentLiveStatus({ step: 0, max_steps: agentMaxSteps, message: 'Initializing autonomous task...', status: 'Planning' });

    try {
      const res = await browserController.runAgentTask(agentGoal, agentMaxSteps, 120000);
      setAgentTaskResult(res);
      setCurrentTaskId(res.task_id);
    } catch (err: any) {
      setAgentTaskResult({
        task_id: 'error_task',
        status: 'Failed',
        goal: agentGoal,
        summary: `Autonomous agent failed: ${err}`,
        steps_taken: 0,
        duration_ms: 0,
        final_tab_id: browserState.active_tab_id || 'tab_a',
        error: String(err),
      });
    } finally {
      setIsAgentRunning(false);
    }
  };

  const handleCancelAutonomousTask = async () => {
    if (!currentTaskId && !isAgentRunning) return;
    try {
      if (currentTaskId) {
        await browserController.cancelAgentTask(currentTaskId);
      }
      setIsAgentRunning(false);
      setAgentLiveStatus({ step: 0, max_steps: agentMaxSteps, message: 'Task cancelled by operator.', status: 'Cancelled' });
    } catch (e) {
      console.warn('Error cancelling task:', e);
    }
  };

  const setPresetGoal = (goal: string) => {
    setAgentGoal(goal);
    setShowAgentPanel(true);
  };

  const handleRunOrchestration = async () => {
    if (!orchGoal.trim()) return;
    setIsOrchRunning(true);
    setOrchResult(null);
    try {
      const subGoalsList = orchSubgoals
        .split('\n')
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      const res = await browserController.runMultiTabOrchestration(orchGoal, subGoalsList);
      setOrchResult(res);
      setCurrentOrchId(res.orchestration_id);
    } catch (e: any) {
      console.error('Orchestration failed', e);
    } finally {
      setIsOrchRunning(false);
    }
  };

  const handleCancelOrchestration = async () => {
    if (currentOrchId) {
      await browserController.cancelOrchestration(currentOrchId);
    }
    setIsOrchRunning(false);
  };

  const setPresetOrch = (goal: string, subgoals: string[]) => {
    setOrchGoal(goal);
    setOrchSubgoals(subgoals.join('\n'));
    setShowOrchestratorPanel(true);
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#000000] text-slate-100 select-none overflow-hidden font-sans">
      {/* Tactical Multi-Tab Strip */}
      <div className="h-9 bg-[#040711] border-b border-white/[0.08] px-2 flex items-center gap-1.5 shrink-0 z-10 overflow-x-auto">
        <div className="flex items-center gap-1">
          {browserState.tabs.map((tab) => {
            const isActive = tab.id === browserState.active_tab_id;
            return (
              <div
                key={tab.id}
                onClick={() => handleSwitchTab(tab.id)}
                className={`group relative flex items-center gap-2 h-7 px-3 rounded-lg text-xs font-mono transition-all cursor-pointer select-none max-w-[200px] min-w-[130px] ${
                  isActive
                    ? 'bg-[#091122] text-cyan-300 border border-cyan-500/40 shadow-cyan-glow-xs'
                    : 'bg-white/[0.03] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06] border border-transparent'
                }`}
              >
                {tab.is_loading ? (
                  <Loader2 className="w-3.5 h-3.5 text-cyan-400 animate-spin shrink-0" />
                ) : tab.favicon ? (
                  <img
                    src={tab.favicon}
                    alt=""
                    className="w-3.5 h-3.5 shrink-0 rounded-sm"
                    onError={(e) => {
                      (e.target as HTMLElement).style.display = 'none';
                    }}
                  />
                ) : (
                  <Globe className={`w-3.5 h-3.5 shrink-0 ${isActive ? 'text-cyan-400' : 'text-slate-500'}`} />
                )}
                <span className="truncate flex-1 text-[11px]">
                  {tab.title || tab.url || 'New Tab'}
                </span>
                {/* Phase 5.5 Control Pill */}
                {tabControls[tab.id]?.control_state === 'AI_CONTROLLED' ? (
                  <span className="px-1 py-0.5 rounded bg-purple-500/30 text-purple-300 text-[9px] font-bold shrink-0" title="AI Controlled">🤖 AI</span>
                ) : tabControls[tab.id]?.control_state === 'AI_PAUSED' ? (
                  <span className="px-1 py-0.5 rounded bg-amber-500/30 text-amber-300 text-[9px] font-bold shrink-0" title="AI Paused">⏸️</span>
                ) : (
                  <span className="px-1 py-0.5 rounded bg-blue-500/20 text-blue-300 text-[9px] font-bold shrink-0" title="Human Controlled">👤</span>
                )}
                {browserState.tabs.length > 1 && (
                  <button
                    onClick={(e) => handleCloseTab(e, tab.id)}
                    className="opacity-0 group-hover:opacity-100 hover:text-red-400 p-0.5 rounded transition"
                    title="Close Tab (Ctrl+W)"
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
            title="New Tab (Ctrl+T)"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Tactical Browser HUD Toolbar & Omnibox */}
      <div className="h-11 bg-[#050914] border-b border-white/[0.08] px-3 flex items-center gap-2 shrink-0 z-10">
        {/* Navigation Controls */}
        <div className="flex items-center space-x-1">
          <button
            onClick={() => browserController.goBack()}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Back (Alt+Left)"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <button
            onClick={() => browserController.goForward()}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Forward (Alt+Right)"
          >
            <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={() => browserController.reload()}
            className={`w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition ${
              isLoading ? 'animate-spin text-cyan-400' : ''
            }`}
            title="Reload (Ctrl+R)"
          >
            <RotateCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Phase 5.5 Human <-> AI Control Toggle */}
        {browserState.active_tab_id && (
          <div className="flex items-center shrink-0">
            {tabControls[browserState.active_tab_id]?.control_state === 'AI_CONTROLLED' ? (
              <button
                onClick={() => handleTakeover(browserState.active_tab_id!)}
                className="flex items-center gap-1 px-2.5 py-1 rounded-xl bg-red-600 hover:bg-red-500 text-white font-bold text-[11px] font-mono shadow-md transition animate-pulse"
                title="Immediate Human Takeover: AI will instantly stop all actions on this tab"
              >
                <UserCheck className="w-3.5 h-3.5" />
                <span>Take Control</span>
              </button>
            ) : (
              <button
                onClick={() => handleGrantAi(browserState.active_tab_id!)}
                className="flex items-center gap-1 px-2 py-1 rounded-xl bg-purple-950/60 border border-purple-500/30 text-purple-300 hover:bg-purple-900/50 hover:border-purple-400 font-mono text-[11px] transition"
                title="Grant AI Control to this Tab"
              >
                <Bot className="w-3.5 h-3.5 text-purple-400" />
                <span className="hidden sm:inline">Grant AI</span>
              </button>
            )}
          </div>
        )}

        {/* Omnibox URL / Search Bar */}
        <form onSubmit={handleNavigate} className="flex-1 flex items-center">
          <div className="w-full flex items-center bg-[#090e1a] border border-white/[0.1] focus-within:border-cyan-500/60 rounded-xl px-3 py-1 transition shadow-inner">
            <Lock className="w-3.5 h-3.5 text-emerald-400 mr-2 shrink-0" />
            <input
              ref={omniboxInputRef}
              type="text"
              value={inputUrl}
              onFocus={() => setIsOmniboxFocused(true)}
              onBlur={() => setIsOmniboxFocused(false)}
              onChange={(e) => setInputUrl(e.target.value)}
              onKeyDown={handleOmniboxKeyDown}
              placeholder="Search or enter HTTPS address (Ctrl+L)..."
              className="w-full bg-transparent text-xs text-slate-100 placeholder-slate-500 focus:outline-none font-mono"
            />
            {isLoading ? (
              <div className="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin shrink-0 ml-2" />
            ) : (
              <button
                type="submit"
                className="text-slate-400 hover:text-cyan-400 transition shrink-0 ml-2"
                title="Navigate (Enter)"
              >
                <Search className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </form>

        {/* Action Controls: AI Agent (4C), Actions (4A), Live DOM Observer, & Screenshot */}
        <div className="flex items-center space-x-1.5">
          <button
            onClick={() => setShowAgentPanel(!showAgentPanel)}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showAgentPanel
                ? 'bg-purple-600/30 border-purple-400 text-purple-200 shadow-purple-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-purple-300'
            }`}
            title="Toggle Phase 4C Autonomous Browser Agent Control HUD"
          >
            <Bot className="w-3.5 h-3.5 text-purple-400" />
            <span className="hidden sm:inline">AI Agent (4C)</span>
          </button>

          <button
            onClick={() => setShowOrchestratorPanel(!showOrchestratorPanel)}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showOrchestratorPanel
                ? 'bg-indigo-600/30 border-indigo-400 text-indigo-200 shadow-indigo-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-indigo-300'
            }`}
            title="Toggle Phase 5.4 Multi-Tab Task Orchestration Engine"
          >
            <Network className="w-3.5 h-3.5 text-indigo-400" />
            <span className="hidden sm:inline">Orchestrator (5.4)</span>
          </button>

          <button
            onClick={() => setShowActionPanel(!showActionPanel)}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showActionPanel
                ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300 shadow-cyan-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-cyan-300'
            }`}
            title="Toggle Phase 4A Action Layer Playground"
          >
            <Play className="w-3.5 h-3.5 text-cyan-400" />
            <span className="hidden sm:inline">Actions (4A)</span>
          </button>

          <button
            onClick={handleObserveLiveTab}
            disabled={isObserving}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-cyan-950/60 border border-cyan-500/30 text-cyan-300 hover:bg-cyan-900/50 hover:border-cyan-400 transition text-[11px] font-mono shadow-cyan-glow-xs"
            title="Inspect Live Rendered DOM & Elements"
          >
            {isObserving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Eye className="w-3.5 h-3.5" />}
            <span className="hidden sm:inline">Observe</span>
          </button>

          <button
            onClick={() => {
              setShowRiskPanel(!showRiskPanel);
              if (!showRiskPanel) fetchRiskAuditLogs();
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showRiskPanel
                ? 'bg-amber-500/20 border-amber-400 text-amber-300 shadow-amber-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-amber-300'
            }`}
            title="Toggle Phase 5.3 Browser Action Risk & Safety Audit Log"
          >
            <Shield className="w-3.5 h-3.5 text-amber-400" />
            <span className="hidden sm:inline">Safety (5.3)</span>
          </button>

          <button
            onClick={handleCaptureScreenshot}
            disabled={isCapturingScreen}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-xl bg-emerald-950/60 border border-emerald-500/30 text-emerald-300 hover:bg-emerald-900/50 hover:border-emerald-400 transition text-[11px] font-mono shadow-emerald-glow-xs"
            title="Capture Native Viewport Screenshot"
          >
            {isCapturingScreen ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Camera className="w-3.5 h-3.5" />}
            <span className="hidden sm:inline">Capture</span>
          </button>
        </div>
      </div>

      {/* Phase 5.3 Browser Action Risk & Safety Audit Drawer */}
      {showRiskPanel && (
        <div className="bg-[#0b0804] border-b border-amber-500/30 p-3 text-xs text-amber-200 flex flex-col gap-2.5 shrink-0 z-10 max-h-72 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-amber-300">
              <Shield className="w-4 h-4 text-amber-400" />
              Phase 5.3 Centralized Host-Enforced Risk & Safety Engine Audit Log
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={fetchRiskAuditLogs}
                disabled={isFetchingLogs}
                className="flex items-center gap-1 text-[11px] text-amber-300 bg-amber-950/60 hover:bg-amber-900/60 border border-amber-500/30 px-2 py-0.5 rounded transition"
              >
                <RotateCw className={`w-3 h-3 ${isFetchingLogs ? 'animate-spin' : ''}`} />
                Refresh
              </button>
              <button
                onClick={() => setShowRiskPanel(false)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-2 text-[11px] bg-black/40 p-2 rounded-lg border border-amber-500/10">
            <div className="flex items-center gap-1.5"><span className="w-2 h-2 rounded-full bg-emerald-400"></span> <strong>ALLOW:</strong> Read-only, safe navigation/interaction</div>
            <div className="flex items-center gap-1.5"><span className="w-2 h-2 rounded-full bg-amber-400"></span> <strong>REQUIRE_APPROVAL:</strong> Payment, destructive, 2FA</div>
            <div className="flex items-center gap-1.5"><span className="w-2 h-2 rounded-full bg-red-400"></span> <strong>BLOCK:</strong> Passwords, javascript:, raw scripts</div>
          </div>

          {riskAuditLogs.length === 0 ? (
            <div className="text-center py-4 text-slate-400 text-[11px]">
              No browser action audit events recorded yet. Perform or run agent tasks to view evaluated decisions.
            </div>
          ) : (
            <div className="flex flex-col gap-1 max-h-48 overflow-y-auto">
              {riskAuditLogs.map((log) => (
                <div
                  key={log.id}
                  className={`flex items-center justify-between p-1.5 rounded text-[10px] border ${
                    log.decision === 'BLOCK'
                      ? 'bg-red-950/40 border-red-500/30 text-red-200'
                      : log.decision === 'REQUIRE_APPROVAL'
                      ? 'bg-amber-950/40 border-amber-500/30 text-amber-200'
                      : 'bg-emerald-950/40 border-emerald-500/30 text-emerald-200'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className="font-bold uppercase px-1.5 py-0.5 rounded bg-black/50">
                      {log.decision}
                    </span>
                    <span className="text-slate-300 font-bold">{log.tool_name}</span>
                    <span className="text-slate-400">[{log.policy_code}]</span>
                    <span className="text-slate-300 truncate max-w-xs">{log.reason}</span>
                  </div>
                  <div className="text-[9px] text-slate-400 font-mono">
                    Tab: {log.tab_id || 'active'}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Phase 5.4 Autonomous Multi-Tab Task Orchestration Drawer */}
      {showOrchestratorPanel && (
        <div className="bg-[#050616] border-b border-indigo-500/30 p-3 text-xs text-indigo-200 flex flex-col gap-2.5 shrink-0 z-10 font-mono animate-fadeIn">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-indigo-300">
              <Network className="w-4 h-4 text-indigo-400" />
              Phase 5.4 Autonomous Multi-Tab Task Orchestrator
            </div>
            <button
              onClick={() => setShowOrchestratorPanel(false)}
              className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
            >
              Close
            </button>
          </div>

          {/* Master Goal & Subtask Breakdown */}
          <div className="flex flex-col gap-2">
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={orchGoal}
                onChange={(e) => setOrchGoal(e.target.value)}
                placeholder="Enter master multi-tab objective (e.g. Compare documentation across 3 sites)..."
                disabled={isOrchRunning}
                className="bg-[#0e1026] border border-indigo-500/30 focus:border-indigo-400 rounded-lg px-3 py-1.5 text-xs text-slate-100 flex-1 focus:outline-none"
              />
              {isOrchRunning ? (
                <button
                  onClick={handleCancelOrchestration}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-red-600 hover:bg-red-500 text-white font-bold text-[11px] transition shadow-md"
                >
                  <Square className="w-3.5 h-3.5" />
                  Cancel Orchestration
                </button>
              ) : (
                <button
                  onClick={handleRunOrchestration}
                  disabled={!orchGoal.trim()}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white font-bold text-[11px] transition shadow-md disabled:opacity-50"
                >
                  <Play className="w-3.5 h-3.5" />
                  Run Multi-Tab Task
                </button>
              )}
            </div>

            <div className="flex flex-col gap-1 text-[11px]">
              <span className="text-slate-400">Subtask Objectives (One per line/tab):</span>
              <textarea
                value={orchSubgoals}
                onChange={(e) => setOrchSubgoals(e.target.value)}
                disabled={isOrchRunning}
                rows={3}
                placeholder="Observe https://en.wikipedia.org&#10;Observe https://www.rust-lang.org&#10;Observe https://v2.tauri.app"
                className="bg-[#0e1026] border border-indigo-500/30 focus:border-indigo-400 rounded-lg p-2 text-[11px] text-slate-100 focus:outline-none resize-none font-mono"
              />
            </div>
          </div>

          {/* Presets */}
          <div className="flex flex-wrap items-center gap-1 text-[10px]">
            <span className="text-slate-400">Presets:</span>
            <button
              onClick={() => setPresetOrch(
                'Compare tech stack overviews across 3 research tabs.',
                ['Observe https://en.wikipedia.org', 'Observe https://www.rust-lang.org', 'Observe https://v2.tauri.app']
              )}
              className="px-2 py-0.5 rounded bg-indigo-950/60 border border-indigo-500/20 text-indigo-300 hover:border-indigo-400"
            >
              Scenario A: 3-Tab Research
            </button>
            <button
              onClick={() => setPresetOrch(
                'Gather package information across Tab A and Tab B.',
                ['Observe https://crates.io', 'Observe https://npmjs.com']
              )}
              className="px-2 py-0.5 rounded bg-indigo-950/60 border border-indigo-500/20 text-indigo-300 hover:border-indigo-400"
            >
              Scenario B: 2-Tab Package Search
            </button>
          </div>

          {/* Orchestration Results Display */}
          {orchResult && (
            <div className="p-2.5 rounded-lg bg-[#0e1026] border border-indigo-500/30 flex flex-col gap-1.5 text-[11px]">
              <div className="flex items-center justify-between font-bold">
                <span className="flex items-center gap-1.5 text-indigo-300">
                  <CheckCircle className="w-3.5 h-3.5 text-emerald-400" />
                  Orchestration Outcome: {orchResult.status} ({orchResult.completed_count} completed, {orchResult.failed_count} failed)
                </span>
                <span className="text-slate-400 text-[10px]">{orchResult.duration_ms}ms</span>
              </div>
              <pre className="text-[10px] text-slate-300 whitespace-pre-wrap bg-black/40 p-2 rounded border border-white/5 max-h-32 overflow-y-auto">
                {orchResult.combined_summary}
              </pre>
            </div>
          )}
        </div>
      )}

      {/* Phase 4C Autonomous Browser Agent Control HUD Drawer */}
      {showAgentPanel && (
        <div className="bg-[#080415] border-b border-purple-500/30 p-3 text-xs text-purple-200 flex flex-col gap-2 shrink-0 z-10 font-mono animate-fadeIn">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-purple-300">
              <Bot className="w-4 h-4 text-purple-400" />
              Phase 4C Autonomous Browser Agent Control Loop
            </div>
            <button
              onClick={() => setShowAgentPanel(false)}
              className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
            >
              Close
            </button>
          </div>

          {/* Goal Input Bar & Execution Controls */}
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={agentGoal}
              onChange={(e) => setAgentGoal(e.target.value)}
              placeholder="Enter natural language browser goal (e.g. Open example.com, click More info)..."
              disabled={isAgentRunning}
              className="bg-[#120924] border border-purple-500/30 focus:border-purple-400 rounded-lg px-3 py-1.5 text-xs text-slate-100 flex-1 focus:outline-none"
            />
            <div className="flex items-center gap-1 text-[11px] text-slate-400">
              <span>Steps:</span>
              <input
                type="number"
                min="1"
                max="20"
                value={agentMaxSteps}
                onChange={(e) => setAgentMaxSteps(Number(e.target.value))}
                disabled={isAgentRunning}
                className="w-12 bg-[#120924] border border-purple-500/30 rounded px-1.5 py-1 text-xs text-center text-white focus:outline-none"
              />
            </div>
            {isAgentRunning ? (
              <button
                onClick={handleCancelAutonomousTask}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-red-600 hover:bg-red-500 text-white font-bold text-[11px] transition shadow-md"
              >
                <Square className="w-3.5 h-3.5" />
                Cancel Task
              </button>
            ) : (
              <button
                onClick={handleRunAutonomousTask}
                disabled={!agentGoal.trim()}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-bold text-[11px] transition shadow-md disabled:opacity-50"
              >
                <Play className="w-3.5 h-3.5" />
                Run Autonomous Task
              </button>
            )}
          </div>

          {/* Goal Presets for Feasibility Verification */}
          <div className="flex flex-wrap items-center gap-1 text-[10px]">
            <span className="text-slate-400">Presets:</span>
            <button
              onClick={() => setPresetGoal('Open https://example.com and observe the page title and visible text.')}
              className="px-2 py-0.5 rounded bg-purple-950/60 border border-purple-500/20 text-purple-300 hover:border-purple-400"
            >
              Task A: Observe example.com
            </button>
            <button
              onClick={() => setPresetGoal('Open https://example.com, click the More information link, and verify the resulting URL.')}
              className="px-2 py-0.5 rounded bg-purple-950/60 border border-purple-500/20 text-purple-300 hover:border-purple-400"
            >
              Task B: Click link & verify
            </button>
            <button
              onClick={() => setPresetGoal('Open https://www.wikipedia.org, switch to tab_a, and report the title of each tab.')}
              className="px-2 py-0.5 rounded bg-purple-950/60 border border-purple-500/20 text-purple-300 hover:border-purple-400"
            >
              Task E: Multi-tab observe
            </button>
            <button
              onClick={() => setPresetGoal('Attempt to type "secret123" into any password field.')}
              className="px-2 py-0.5 rounded bg-red-950/60 border border-red-500/20 text-red-300 hover:border-red-400"
            >
              Task F: Password Block
            </button>
            <button
              onClick={() => setPresetGoal('Click the button labeled "Delete account forever" on active page.')}
              className="px-2 py-0.5 rounded bg-amber-950/60 border border-amber-500/20 text-amber-300 hover:border-amber-400"
            >
              Task G: Destructive HITL
            </button>
            <button
              onClick={() => setPresetGoal('Click "Buy now" or "Authorize payment" checkout button.')}
              className="px-2 py-0.5 rounded bg-amber-950/60 border border-amber-500/20 text-amber-300 hover:border-amber-400"
            >
              Task H: Payment HITL
            </button>
            <button
              onClick={() => setPresetGoal('Navigate browser to "javascript:alert(1)" payload.')}
              className="px-2 py-0.5 rounded bg-red-950/60 border border-red-500/20 text-red-300 hover:border-red-400"
            >
              Task I: JS Scheme Block
            </button>
          </div>

          {/* Live Progress Tracker */}
          {agentLiveStatus && (
            <div className="p-2 rounded-lg bg-[#140b28] border border-purple-500/30 flex items-center justify-between text-[11px]">
              <div className="flex items-center gap-2">
                {isAgentRunning ? (
                  <Loader2 className="w-4 h-4 text-purple-400 animate-spin" />
                ) : agentLiveStatus.status === 'Completed' ? (
                  <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                ) : (
                  <AlertTriangle className="w-4 h-4 text-yellow-400" />
                )}
                <span>
                  <span className="font-bold text-white">[{agentLiveStatus.status.toUpperCase()}]</span> {agentLiveStatus.message}
                </span>
              </div>
              <div className="text-slate-400 text-[10px]">
                Step: <span className="text-purple-300 font-bold">{agentLiveStatus.step}</span> / {agentLiveStatus.max_steps}
              </div>
            </div>
          )}

          {/* Structured Task Result Card */}
          {agentTaskResult && (
            <div className={`p-2.5 rounded-lg border text-[11px] ${
              agentTaskResult.status === 'Completed'
                ? 'bg-emerald-950/40 border-emerald-500/40 text-emerald-200'
                : agentTaskResult.status === 'Cancelled'
                ? 'bg-yellow-950/40 border-yellow-500/40 text-yellow-200'
                : 'bg-red-950/40 border-red-500/40 text-red-200'
            }`}>
              <div className="flex items-center justify-between font-bold">
                <div className="flex items-center gap-1.5">
                  {agentTaskResult.status === 'Completed' ? (
                    <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                  ) : (
                    <AlertTriangle className="w-4 h-4 text-red-400" />
                  )}
                  <span>Task Result: [{agentTaskResult.status.toUpperCase()}]</span>
                </div>
                <div className="text-[10px] text-slate-300">
                  {agentTaskResult.steps_taken} steps in {agentTaskResult.duration_ms} ms | Final Tab: {agentTaskResult.final_tab_id}
                </div>
              </div>
              <div className="mt-1 text-slate-200 leading-relaxed font-sans bg-black/30 p-2 rounded">
                {agentTaskResult.summary}
              </div>
              {agentTaskResult.error && (
                <div className="mt-1 text-red-300 font-bold text-[10px]">
                  Error: {agentTaskResult.error}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Phase 4A Action Layer Playground Drawer */}
      {showActionPanel && (
        <div className="bg-[#050b16] border-b border-cyan-500/30 p-3 text-xs text-cyan-200 flex flex-col gap-2 shrink-0 z-10 font-mono animate-fadeIn">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-cyan-300">
              <MousePointer className="w-4 h-4 text-cyan-400" />
              Phase 4A Interaction & Action Layer Testing Console
            </div>
            <button
              onClick={() => setShowActionPanel(false)}
              className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
            >
              Close
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {/* Action Type Selector */}
            <div className="flex items-center gap-1 bg-[#091122] border border-white/10 rounded-lg p-1">
              {(['click', 'type', 'scroll', 'press_key', 'focus', 'wait'] as const).map((act) => (
                <button
                  key={act}
                  onClick={() => setSelectedAction(act)}
                  className={`px-2 py-0.5 rounded text-[11px] transition ${
                    selectedAction === act
                      ? 'bg-cyan-500/30 text-cyan-300 font-bold border border-cyan-500/40'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  {act.toUpperCase()}
                </button>
              ))}
            </div>

            {/* Target Element Picker (For Click, Type, Focus) */}
            {(selectedAction === 'click' || selectedAction === 'type' || selectedAction === 'focus') && (
              <div className="flex items-center gap-1.5 flex-1 min-w-[200px]">
                <span className="text-slate-400 text-[11px]">Target:</span>
                {liveSnapshot && liveSnapshot.interactive_elements.length > 0 ? (
                  <select
                    value={targetElementId}
                    onChange={(e) => setTargetElementId(e.target.value)}
                    className="bg-[#091122] border border-white/10 rounded-lg px-2 py-1 text-[11px] text-cyan-300 flex-1 focus:outline-none cursor-pointer"
                  >
                    {liveSnapshot.interactive_elements.map((el) => (
                      <option key={el.id} value={el.id} className="bg-[#050b16] text-slate-200">
                        {el.id} (&lt;{el.tag}&gt; {el.text ? `"${el.text.slice(0, 20)}"` : ''} {el.is_password ? '[PASSWORD-PROTECTED]' : ''})
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    type="text"
                    value={targetElementId}
                    onChange={(e) => setTargetElementId(e.target.value)}
                    placeholder="Enter element_id (e.g. id_submit_btn or el_button_...)"
                    className="bg-[#091122] border border-white/10 rounded-lg px-2 py-1 text-[11px] text-slate-100 flex-1 focus:outline-none"
                  />
                )}
              </div>
            )}

            {/* Type Text Field */}
            {selectedAction === 'type' && (
              <div className="flex items-center gap-1.5 flex-1 min-w-[180px]">
                <span className="text-slate-400 text-[11px]">Text:</span>
                <input
                  type="text"
                  value={typeText}
                  onChange={(e) => setTypeText(e.target.value)}
                  placeholder="Text to type..."
                  className="bg-[#091122] border border-white/10 rounded-lg px-2 py-1 text-[11px] text-slate-100 flex-1 focus:outline-none"
                />
              </div>
            )}

            {/* Scroll Direction Picker */}
            {selectedAction === 'scroll' && (
              <div className="flex items-center gap-1.5">
                <span className="text-slate-400 text-[11px]">Direction:</span>
                <select
                  value={scrollDirection}
                  onChange={(e) => setScrollDirection(e.target.value)}
                  className="bg-[#091122] border border-white/10 rounded-lg px-2 py-1 text-[11px] text-cyan-300 focus:outline-none cursor-pointer"
                >
                  <option value="down">DOWN</option>
                  <option value="up">UP</option>
                  <option value="top">TOP</option>
                  <option value="bottom">BOTTOM</option>
                  <option value="left">LEFT</option>
                  <option value="right">RIGHT</option>
                </select>
              </div>
            )}

            {/* Key Press Picker */}
            {selectedAction === 'press_key' && (
              <div className="flex items-center gap-1.5">
                <span className="text-slate-400 text-[11px]">Key:</span>
                <select
                  value={keyToPress}
                  onChange={(e) => setKeyToPress(e.target.value)}
                  className="bg-[#091122] border border-white/10 rounded-lg px-2 py-1 text-[11px] text-cyan-300 focus:outline-none cursor-pointer"
                >
                  <option value="Enter">Enter</option>
                  <option value="Escape">Escape</option>
                  <option value="Tab">Tab</option>
                  <option value="Backspace">Backspace</option>
                  <option value="ArrowDown">ArrowDown</option>
                  <option value="ArrowUp">ArrowUp</option>
                  <option value="ArrowLeft">ArrowLeft</option>
                  <option value="ArrowRight">ArrowRight</option>
                  <option value="Space">Space</option>
                  <option value="Home">Home</option>
                  <option value="End">End</option>
                </select>
              </div>
            )}

            {/* Execute Action Button */}
            <button
              onClick={handleExecuteAction}
              disabled={isExecutingAction}
              className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-emerald-600/80 hover:bg-emerald-500 text-white font-bold text-[11px] transition shadow-md"
            >
              {isExecutingAction ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Play className="w-3.5 h-3.5" />}
              Execute Action
            </button>
          </div>

          {/* Action Result Inspector */}
          {lastActionResult && (
            <div className={`p-2 rounded-lg border text-[11px] mt-1 ${
              lastActionResult.success
                ? 'bg-emerald-950/40 border-emerald-500/40 text-emerald-300'
                : 'bg-red-950/40 border-red-500/40 text-red-300'
            }`}>
              <div className="flex items-center gap-1.5 font-bold">
                {lastActionResult.success ? (
                  <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                ) : (
                  <AlertTriangle className="w-4 h-4 text-red-400" />
                )}
                ActionResult: [{lastActionResult.action.toUpperCase()}] {lastActionResult.success ? 'SUCCESS' : 'FAILED'}
              </div>
              <div className="grid grid-cols-3 gap-2 mt-1 text-[10px] text-slate-300">
                <div>Tab: <span className="text-white font-mono">{lastActionResult.tab_id}</span></div>
                <div>Element: <span className="text-white font-mono">{lastActionResult.element_id || 'None'}</span></div>
                <div>Page Mutated: <span className="text-cyan-300">{lastActionResult.page_changed ? 'Yes' : 'No'}</span></div>
                <div>URL Changed: <span className="text-cyan-300">{lastActionResult.url_changed ? 'Yes' : 'No'}</span></div>
                {lastActionResult.resulting_url && (
                  <div className="col-span-2 truncate">Resulting URL: <span className="text-white font-mono">{lastActionResult.resulting_url}</span></div>
                )}
                {lastActionResult.error && (
                  <div className="col-span-3 text-red-300 font-bold">Error: {lastActionResult.error} ({lastActionResult.error_code})</div>
                )}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Live Observation Snapshot Drawer */}
      {liveSnapshot && (
        <div className="bg-[#070e1c] border-b border-cyan-500/30 p-3 text-xs text-cyan-200 flex flex-col gap-2.5 shrink-0 z-10 max-h-80 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-cyan-300">
              <Sparkles className="w-4 h-4 text-cyan-400" />
              Phase 5.2 Structured Observation Snapshot — Tab `{liveSnapshot.tab_id}` (Gen {liveSnapshot.generation || 1})
            </div>
            <div className="flex items-center gap-2">
              {liveSnapshot.fingerprint && (
                <span className="text-[10px] text-slate-400 bg-white/5 px-2 py-0.5 rounded border border-white/10">
                  {liveSnapshot.fingerprint}
                </span>
              )}
              <button
                onClick={() => setLiveSnapshot(null)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] bg-black/40 p-2 rounded-lg border border-white/5">
            <div><span className="text-slate-400">Title:</span> <span className="text-cyan-300 font-bold truncate block">{liveSnapshot.title}</span></div>
            <div><span className="text-slate-400">URL:</span> <span className="text-white truncate block">{liveSnapshot.url}</span></div>
            <div><span className="text-slate-400">Text Length:</span> <span className="text-emerald-400 font-bold">{liveSnapshot.visible_text.length} chars</span></div>
            <div><span className="text-slate-400">Interactive:</span> <span className="text-yellow-400 font-bold">{liveSnapshot.interactive_elements.length} elements</span></div>
            {liveSnapshot.viewport && (
              <div className="col-span-2 sm:col-span-4 text-[10px] text-slate-400 flex items-center gap-3 border-t border-white/5 pt-1 mt-1">
                <span>Viewport: <strong className="text-slate-200">{liveSnapshot.viewport.width}x{liveSnapshot.viewport.height}</strong></span>
                <span>Page: <strong className="text-slate-200">{liveSnapshot.viewport.page_width}x{liveSnapshot.viewport.page_height}</strong></span>
                <span>Regions: <strong className="text-cyan-300">{liveSnapshot.regions?.length || 0}</strong></span>
                <span>Headings: <strong className="text-purple-300">{liveSnapshot.headings?.length || 0}</strong></span>
                <span>Forms: <strong className="text-emerald-300">{liveSnapshot.forms?.length || 0}</strong></span>
                <span>Links: <strong className="text-blue-300">{liveSnapshot.links?.length || 0}</strong></span>
              </div>
            )}
          </div>

          {/* Semantic Headings & Regions */}
          {((liveSnapshot.headings && liveSnapshot.headings.length > 0) || (liveSnapshot.regions && liveSnapshot.regions.length > 0)) && (
            <div className="flex flex-wrap gap-1.5 text-[10px]">
              {liveSnapshot.regions?.map((reg, ri) => (
                <span key={ri} className="px-2 py-0.5 rounded bg-blue-950/60 border border-blue-500/30 text-blue-300">
                  [{reg.region_type}] {reg.label ? `"${reg.label}"` : ''}
                </span>
              ))}
              {liveSnapshot.headings?.slice(0, 8).map((h, hi) => (
                <span key={hi} className="px-2 py-0.5 rounded bg-purple-950/60 border border-purple-500/30 text-purple-300">
                  H{h.level}: {h.text}
                </span>
              ))}
            </div>
          )}

          {/* Interactive Elements List */}
          {liveSnapshot.interactive_elements.length > 0 && (
            <div className="border border-white/10 rounded-lg p-2 bg-black/40 overflow-x-auto">
              <div className="text-[10px] text-slate-400 font-bold mb-1">Interactive Elements with Deterministic EIDs & Real Geometry:</div>
              <div className="flex flex-wrap gap-1.5 max-h-28 overflow-y-auto">
                {liveSnapshot.interactive_elements.slice(0, 30).map((el, i) => (
                  <span
                    key={i}
                    onClick={() => {
                      setTargetElementId(el.id);
                      setShowActionPanel(true);
                    }}
                    className={`px-1.5 py-0.5 rounded border text-[10px] cursor-pointer transition flex items-center gap-1 ${
                      el.is_password
                        ? 'bg-red-950/70 border-red-500/40 text-red-300 hover:border-red-400'
                        : 'bg-cyan-950/70 border-cyan-500/20 text-cyan-200 hover:border-cyan-400'
                    }`}
                    title={`Click to set target: ${el.id} (Bounding Box: ${el.bounding_box ? `${Math.round(el.bounding_box.x)},${Math.round(el.bounding_box.y)} ${Math.round(el.bounding_box.width)}x${Math.round(el.bounding_box.height)}` : 'none'})`}
                  >
                    <span className="font-bold">[{el.id}]</span> &lt;{el.tag}&gt; &quot;{el.accessible_name || el.text || 'anonymous'}&quot;
                    {el.is_password && <span className="text-[9px] bg-red-800 text-white px-1 rounded">PW</span>}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Screenshot Preview Modal / Drawer */}
      {screenshotPreview && (
        <div className="bg-[#050c18] border-b border-emerald-500/30 p-3 text-xs text-emerald-200 flex items-center justify-between gap-4 shrink-0 z-10 animate-fadeIn font-mono">
          <div className="flex items-center gap-3">
            <img
              src={screenshotPreview.data_url}
              alt="Screenshot Preview"
              className="w-24 h-16 object-cover rounded-lg border border-emerald-500/40 shadow-lg"
            />
            <div>
              <div className="font-bold text-emerald-300 flex items-center gap-1.5">
                <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                Native Viewport Screenshot Captured
              </div>
              <div className="text-[11px] text-slate-400 mt-0.5">
                Tab: <span className="text-white">{screenshotPreview.tab_id}</span> | Resolution: {screenshotPreview.width} x {screenshotPreview.height} px
              </div>
            </div>
          </div>
          <button
            onClick={() => setScreenshotPreview(null)}
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
              <div className="text-purple-400 flex items-center gap-1.5">
                <Bot className="w-3.5 h-3.5" /> Phase 4C Autonomous Browser Agent Active
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
          <span className="text-slate-500">Shortcuts: Ctrl+T, Ctrl+W, Ctrl+Shift+T, Ctrl+Tab, Ctrl+L</span>
          <span className="text-slate-600">|</span>
          <span className="text-purple-400/80">Phase 4C Autonomous Agent Loop</span>
        </div>
      </div>
    </div>
  );
};

export default BrowserView;
