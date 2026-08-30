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
import { useApp } from '../context/AppContext';
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
  BrowserHistoryEntry,
  BrowserBookmark,
  BrowserDownload,
  BrowserProfile,
  BrowserProfileType,
  PrivacyStatus,
  PrivacyRule,
  TabPrivacyStats,
  FindResult,
  ReaderDocument,
  BrowserTabGroup,
} from '../types';
import { Shield, ShieldOff, AlertOctagon, FileCheck, CheckCircle, Network, GitBranch, UserCheck, User, Star, Bookmark, History, Trash2, Download, Folder, ExternalLink, FileText, XCircle, Users, Edit2, Pin, PinOff, Copy, Compass, LayoutGrid, Terminal, Cpu, Printer, ZoomIn, ZoomOut, ChevronUp, ChevronDown, Type, BookOpen, FileDown, FolderPlus, Tag, ChevronRight } from 'lucide-react';

export const BrowserView: React.FC = () => {
  const { isTelemetryOpen } = useApp();
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

  // Phase 5.6A History & Bookmarks States
  const [showHistoryPanel, setShowHistoryPanel] = useState(false);
  const [historyList, setHistoryList] = useState<BrowserHistoryEntry[]>([]);
  const [historySearchQuery, setHistorySearchQuery] = useState('');
  const [isFetchingHistory, setIsFetchingHistory] = useState(false);

  const [showBookmarksPanel, setShowBookmarksPanel] = useState(false);
  const [bookmarksList, setBookmarksList] = useState<BrowserBookmark[]>([]);
  const [bookmarkSearchQuery, setBookmarkSearchQuery] = useState('');
  const [isFetchingBookmarks, setIsFetchingBookmarks] = useState(false);
  const [isActiveTabBookmarked, setIsActiveTabBookmarked] = useState(false);

  const fetchHistory = useCallback(async (query?: string) => {
    setIsFetchingHistory(true);
    try {
      if (query && query.trim()) {
        const res = await browserController.searchHistory(query.trim());
        setHistoryList(res);
      } else {
        const res = await browserController.getRecentHistory(50);
        setHistoryList(res);
      }
    } catch (e) {
      console.warn('Failed to fetch history', e);
    } finally {
      setIsFetchingHistory(false);
    }
  }, []);

  const fetchBookmarks = useCallback(async (query?: string) => {
    setIsFetchingBookmarks(true);
    try {
      if (query && query.trim()) {
        const res = await browserController.searchBookmarks(query.trim());
        setBookmarksList(res);
      } else {
        const res = await browserController.getBookmarks();
        setBookmarksList(res);
      }
    } catch (e) {
      console.warn('Failed to fetch bookmarks', e);
    } finally {
      setIsFetchingBookmarks(false);
    }
  }, []);

  const checkActiveTabBookmark = useCallback(async (url: string) => {
    try {
      const bookmarked = await browserController.isBookmarked(url);
      setIsActiveTabBookmarked(bookmarked);
    } catch (e) {
      console.warn('Failed to check bookmark status', e);
    }
  }, []);

  const handleToggleBookmarkActiveTab = async () => {
    const activeTab = browserState.tabs.find((t) => t.id === browserState.active_tab_id);
    if (!activeTab || !activeTab.url) return;

    try {
      if (isActiveTabBookmarked) {
        const all = await browserController.getBookmarks();
        const found = all.find((b) => b.url === activeTab.url);
        if (found) {
          await browserController.deleteBookmark(found.id);
        }
        setIsActiveTabBookmarked(false);
      } else {
        await browserController.addBookmark(activeTab.title || activeTab.url, activeTab.url, undefined, activeTab.favicon);
        setIsActiveTabBookmarked(true);
      }
      fetchBookmarks();
    } catch (e) {
      console.error('Failed to toggle bookmark', e);
    }
  };

  // Phase 5.6B Download Manager States
  const [showDownloadsPanel, setShowDownloadsPanel] = useState(false);
  const [downloadsList, setDownloadsList] = useState<BrowserDownload[]>([]);
  const [isFetchingDownloads, setIsFetchingDownloads] = useState(false);

  const fetchDownloads = useCallback(async () => {
    setIsFetchingDownloads(true);
    try {
      const res = await browserController.getDownloads(50);
      setDownloadsList(res);
    } catch (e) {
      console.warn('Failed to fetch downloads', e);
    } finally {
      setIsFetchingDownloads(false);
    }
  }, []);

  const handleCancelDownload = async (id: string) => {
    try {
      await browserController.cancelDownload(id);
      fetchDownloads();
    } catch (e) {
      console.error('Failed to cancel download', e);
    }
  };

  const handleDeleteDownloadRecord = async (id: string) => {
    try {
      await browserController.deleteDownloadRecord(id);
      fetchDownloads();
    } catch (e) {
      console.error('Failed to delete download record', e);
    }
  };

  const handleClearDownloadRecords = async () => {
    if (confirm('Clear download history records? (Files on disk will not be deleted)')) {
      try {
        await browserController.clearDownloadRecords();
        fetchDownloads();
      } catch (e) {
        console.error('Failed to clear download records', e);
      }
    }
  };

  const handleShowInFolder = async (id: string) => {
    try {
      await browserController.showDownloadInFolder(id);
    } catch (e) {
      console.error('Failed to show in folder', e);
    }
  };

  const handleOpenFile = async (id: string) => {
    try {
      await browserController.openDownloadFile(id);
    } catch (e) {
      console.error('Failed to open file', e);
    }
  };

  // Phase 5.6C Browser Profiles States
  const [showProfilesPanel, setShowProfilesPanel] = useState(false);
  const [profilesList, setProfilesList] = useState<BrowserProfile[]>([]);
  const [isFetchingProfiles, setIsFetchingProfiles] = useState(false);
  const [newProfileName, setNewProfileName] = useState('');
  const [newProfileType, setNewProfileType] = useState<BrowserProfileType>('USER');
  const [isCreatingProfile, setIsCreatingProfile] = useState(false);
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editingProfileName, setEditingProfileName] = useState('');

  const fetchProfiles = useCallback(async () => {
    setIsFetchingProfiles(true);
    try {
      const res = await browserController.getProfiles();
      setProfilesList(res);
    } catch (e) {
      console.warn('Failed to fetch profiles', e);
    } finally {
      setIsFetchingProfiles(false);
    }
  }, []);

  const handleCreateProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newProfileName.trim()) return;
    try {
      await browserController.createProfile(newProfileName.trim(), newProfileType);
      setNewProfileName('');
      setIsCreatingProfile(false);
      fetchProfiles();
    } catch (e) {
      console.error('Failed to create profile', e);
    }
  };

  const handleSwitchProfile = async (profileId: string) => {
    try {
      await browserController.switchProfile(profileId);
      fetchProfiles();
    } catch (e) {
      console.error('Failed to switch profile', e);
    }
  };

  const handleRenameProfile = async (profileId: string) => {
    if (!editingProfileName.trim()) return;
    try {
      await browserController.renameProfile(profileId, editingProfileName.trim());
      setEditingProfileId(null);
      setEditingProfileName('');
      fetchProfiles();
    } catch (e) {
      console.error('Failed to rename profile', e);
    }
  };

  const handleDeleteProfile = async (profileId: string) => {
    if (confirm(`Delete browser profile '${profileId}' and its storage data? This cannot be undone.`)) {
      try {
        await browserController.deleteProfile(profileId);
        fetchProfiles();
      } catch (e: any) {
        alert(String(e));
      }
    }
  };

  // Phase 5.6D Tab Context Menu & New Tab States
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; tabId: string } | null>(null);
  const [newTabSearchQuery, setNewTabSearchQuery] = useState('');

  const handleTabContextMenu = (e: React.MouseEvent, tabId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, tabId });
  };

  useEffect(() => {
    const handleWindowClick = () => setContextMenu(null);
    window.addEventListener('click', handleWindowClick);
    return () => window.removeEventListener('click', handleWindowClick);
  }, []);

  // Phase 5.6E Content Blocking & Web Request Privacy Policy States
  const [showPrivacyPanel, setShowPrivacyPanel] = useState(false);
  const [privacyStatus, setPrivacyStatus] = useState<PrivacyStatus | null>(null);
  const [isFetchingPrivacy, setIsFetchingPrivacy] = useState(false);
  const [customRulePattern, setCustomRulePattern] = useState('');
  const [customRuleCategory, setCustomRuleCategory] = useState<'AD' | 'TRACKER' | 'CUSTOM'>('CUSTOM');
  const [privacyRulesList, setPrivacyRulesList] = useState<PrivacyRule[]>([]);

  const fetchPrivacyStatus = useCallback(async (tabId?: string, profileId?: string) => {
    setIsFetchingPrivacy(true);
    try {
      const status = await browserController.getPrivacyStatus(tabId, profileId);
      setPrivacyStatus(status);
      const rules = await browserController.listPrivacyRules(profileId);
      setPrivacyRulesList(rules);
    } catch (e) {
      console.warn('Failed to fetch privacy status', e);
    } finally {
      setIsFetchingPrivacy(false);
    }
  }, []);

  const handleTogglePrivacyProtection = async () => {
    if (!privacyStatus) return;
    try {
      const next = !privacyStatus.enabled;
      await browserController.togglePrivacyProtection(next);
      setPrivacyStatus((prev: PrivacyStatus | null) => (prev ? { ...prev, enabled: next } : null));
    } catch (e) {
      console.error('Failed to toggle privacy protection', e);
    }
  };

  const handleToggleSiteAllowlist = async (domain: string) => {
    if (!domain) return;
    try {
      const isAllowlisted = privacyStatus?.allowlisted_domains.includes(domain);
      if (isAllowlisted) {
        await browserController.removeAllowlistDomain(domain);
      } else {
        await browserController.allowlistDomain(domain);
      }
      fetchPrivacyStatus(browserState.active_tab_id || undefined);
    } catch (e) {
      console.error('Failed to toggle site allowlist', e);
    }
  };

  const handleAddCustomRule = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!customRulePattern.trim()) return;
    try {
      await browserController.addPrivacyRule(customRulePattern.trim(), 'DOMAIN', customRuleCategory);
      setCustomRulePattern('');
      fetchPrivacyStatus(browserState.active_tab_id || undefined);
    } catch (e) {
      console.error('Failed to add custom rule', e);
    }
  };

  const handleRemoveCustomRule = async (ruleId: string) => {
    try {
      await browserController.removePrivacyRule(ruleId);
      fetchPrivacyStatus(browserState.active_tab_id || undefined);
    } catch (e) {
      console.error('Failed to remove custom rule', e);
    }
  };

  const handleResetTabStats = async (tabId: string) => {
    try {
      await browserController.resetTabPrivacyStats(tabId);
      fetchPrivacyStatus(tabId);
    } catch (e) {
      console.error('Failed to reset tab stats', e);
    }
  };

  // Phase 5.6F-A Advanced Browser Utilities States
  const [showFindHud, setShowFindHud] = useState(false);
  const [findQuery, setFindQuery] = useState('');
  const [findResult, setFindResult] = useState<FindResult | null>(null);
  const [findCaseSensitive, setFindCaseSensitive] = useState(false);
  const [showZoomDropdown, setShowZoomDropdown] = useState(false);
  const findInputRef = useRef<HTMLInputElement>(null);

  const handleOpenFind = () => {
    setShowFindHud(true);
    setTimeout(() => {
      findInputRef.current?.focus();
      findInputRef.current?.select();
    }, 50);
  };

  const handleCloseFind = async () => {
    setShowFindHud(false);
    setFindQuery('');
    setFindResult(null);
    if (browserState.active_tab_id) {
      await browserController.clearFind(browserState.active_tab_id);
    }
  };

  const handleFind = async (forward = true) => {
    if (!browserState.active_tab_id || !findQuery.trim()) return;
    try {
      const res = await browserController.findInPage(
        browserState.active_tab_id,
        findQuery,
        forward,
        findCaseSensitive
      );
      setFindResult(res);
    } catch (e) {
      console.warn('Find execution failed:', e);
    }
  };

  const handleZoomIn = async () => {
    if (!browserState.active_tab_id) return;
    await browserController.zoomIn(browserState.active_tab_id);
  };

  const handleZoomOut = async () => {
    if (!browserState.active_tab_id) return;
    await browserController.zoomOut(browserState.active_tab_id);
  };

  const handleZoomReset = async () => {
    if (!browserState.active_tab_id) return;
    await browserController.zoomReset(browserState.active_tab_id);
  };

  const handlePrint = async () => {
    if (!browserState.active_tab_id) return;
    try {
      await browserController.printTab(browserState.active_tab_id);
    } catch (e) {
      console.error('Print failed:', e);
    }
  };

  const handleOpenLinkInNewTab = async (url: string) => {
    try {
      await browserController.openLinkInNewTab(url, browserState.active_tab_id || undefined);
    } catch (e) {
      console.error('Open link in new tab failed:', e);
    }
  };

  const handleCopyLink = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
    } catch (e) {
      console.error('Copy link failed:', e);
    }
  };

  // Phase 5.6F-B Save Page + Reader Mode States & Handlers
  const [readerDocs, setReaderDocs] = useState<Record<string, ReaderDocument>>({});
  const [isExtractingReader, setIsExtractingReader] = useState(false);
  const [readerFontSize, setReaderFontSize] = useState<number>(18);
  const [readerLineWidth, setReaderLineWidth] = useState<'narrow' | 'normal' | 'wide'>('normal');
  const [readerTheme, setReaderTheme] = useState<'dark' | 'sepia' | 'onyx' | 'light'>('dark');
  const [saveStatusToast, setSaveStatusToast] = useState<string | null>(null);
  const [recoveryNotice, setRecoveryNotice] = useState<string | null>(null);

  const handleToggleReaderMode = async (tabId?: string) => {
    const targetId = tabId || browserState.active_tab_id;
    if (!targetId) return;
    const tab = browserState.tabs.find((t) => t.id === targetId);
    if (!tab) return;

    if (tab.is_reader_mode) {
      await browserController.readerModeExit(targetId);
    } else {
      setIsExtractingReader(true);
      try {
        const doc = await browserController.readerModeEnter(targetId);
        setReaderDocs((prev) => ({ ...prev, [targetId]: doc }));
      } catch (e) {
        console.error('Failed to enter reader mode:', e);
      } finally {
        setIsExtractingReader(false);
      }
    }
  };

  const handleSavePageHtml = async (tabId?: string) => {
    const targetId = tabId || browserState.active_tab_id;
    if (!targetId) return;
    try {
      const path = await browserController.savePageHtml(targetId);
      setSaveStatusToast(`Saved: ${path}`);
      setTimeout(() => setSaveStatusToast(null), 4000);
      fetchDownloads();
    } catch (e) {
      console.error('Failed to save page:', e);
      setSaveStatusToast(`Error saving page: ${e}`);
      setTimeout(() => setSaveStatusToast(null), 4000);
    }
  };

  // Phase 5.6F-C Tab Groups & Tab Search States
  const [tabGroups, setTabGroups] = useState<BrowserTabGroup[]>([]);
  const [isFetchingGroups, setIsFetchingGroups] = useState(false);
  const [showCreateGroupModal, setShowCreateGroupModal] = useState(false);
  const [targetTabForGroup, setTargetTabForGroup] = useState<string | null>(null);
  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupColor, setNewGroupColor] = useState('blue');
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null);
  const [editingGroupName, setEditingGroupName] = useState('');
  const [editingGroupColor, setEditingGroupColor] = useState('blue');
  const [groupContextMenu, setGroupContextMenu] = useState<{ groupId: string; x: number; y: number } | null>(null);
  const [showTabSearchModal, setShowTabSearchModal] = useState(false);
  const [tabSearchQuery, setTabSearchQuery] = useState('');
  const [tabSearchSelectedIndex, setTabSearchSelectedIndex] = useState(0);
  const tabSearchInputRef = useRef<HTMLInputElement>(null);

  const GROUP_COLORS: Record<string, { bg: string; text: string; border: string; badge: string; tabBorder: string; activeTab: string; dot: string }> = {
    blue: {
      bg: 'bg-blue-950/60 hover:bg-blue-900/60',
      text: 'text-blue-300',
      border: 'border-blue-500/40',
      badge: 'bg-blue-500',
      tabBorder: 'border-b-2 border-b-blue-400',
      activeTab: 'bg-blue-950/50 border-blue-500/50 text-blue-200',
      dot: '#3b82f6',
    },
    purple: {
      bg: 'bg-purple-950/60 hover:bg-purple-900/60',
      text: 'text-purple-300',
      border: 'border-purple-500/40',
      badge: 'bg-purple-500',
      tabBorder: 'border-b-2 border-b-purple-400',
      activeTab: 'bg-purple-950/50 border-purple-500/50 text-purple-200',
      dot: '#a855f7',
    },
    green: {
      bg: 'bg-emerald-950/60 hover:bg-emerald-900/60',
      text: 'text-emerald-300',
      border: 'border-emerald-500/40',
      badge: 'bg-emerald-500',
      tabBorder: 'border-b-2 border-b-emerald-400',
      activeTab: 'bg-emerald-950/50 border-emerald-500/50 text-emerald-200',
      dot: '#10b981',
    },
    yellow: {
      bg: 'bg-amber-950/60 hover:bg-amber-900/60',
      text: 'text-amber-300',
      border: 'border-amber-500/40',
      badge: 'bg-amber-500',
      tabBorder: 'border-b-2 border-b-amber-400',
      activeTab: 'bg-amber-950/50 border-amber-500/50 text-amber-200',
      dot: '#f59e0b',
    },
    orange: {
      bg: 'bg-orange-950/60 hover:bg-orange-900/60',
      text: 'text-orange-300',
      border: 'border-orange-500/40',
      badge: 'bg-orange-500',
      tabBorder: 'border-b-2 border-b-orange-400',
      activeTab: 'bg-orange-950/50 border-orange-500/50 text-orange-200',
      dot: '#f97316',
    },
    red: {
      bg: 'bg-red-950/60 hover:bg-red-900/60',
      text: 'text-red-300',
      border: 'border-red-500/40',
      badge: 'bg-red-500',
      tabBorder: 'border-b-2 border-b-red-400',
      activeTab: 'bg-red-950/50 border-red-500/50 text-red-200',
      dot: '#ef4444',
    },
    gray: {
      bg: 'bg-slate-900/80 hover:bg-slate-800/80',
      text: 'text-slate-300',
      border: 'border-slate-500/40',
      badge: 'bg-slate-400',
      tabBorder: 'border-b-2 border-b-slate-400',
      activeTab: 'bg-slate-900/50 border-slate-500/50 text-slate-200',
      dot: '#94a3b8',
    },
  };

  const fetchTabGroups = useCallback(async (profileId?: string) => {
    setIsFetchingGroups(true);
    try {
      const groups = await browserController.listTabGroups(profileId);
      setTabGroups(groups);
    } catch (e) {
      console.warn('Failed to fetch tab groups:', e);
    } finally {
      setIsFetchingGroups(false);
    }
  }, []);

  const handleCreateGroup = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!newGroupName.trim()) return;
    try {
      const activeTab = browserState.tabs.find((t) => t.id === (targetTabForGroup || browserState.active_tab_id));
      const profileId = activeTab?.profile_id || 'profile_default';
      const created = await browserController.createTabGroup(newGroupName.trim(), profileId, newGroupColor);
      if (targetTabForGroup) {
        await browserController.moveTabToGroup(targetTabForGroup, created.id);
      }
      setNewGroupName('');
      setTargetTabForGroup(null);
      setShowCreateGroupModal(false);
      fetchTabGroups(profileId);
    } catch (err) {
      console.error('Failed to create tab group:', err);
    }
  };

  const handleRenameGroup = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!editingGroupId || !editingGroupName.trim()) return;
    try {
      await browserController.renameTabGroup(editingGroupId, editingGroupName.trim(), editingGroupColor);
      setEditingGroupId(null);
      setEditingGroupName('');
      fetchTabGroups();
    } catch (err) {
      console.error('Failed to rename tab group:', err);
    }
  };

  const handleDeleteGroup = async (groupId: string) => {
    try {
      await browserController.deleteTabGroup(groupId);
      setGroupContextMenu(null);
      fetchTabGroups();
    } catch (err) {
      console.error('Failed to delete tab group:', err);
    }
  };

  const handleToggleGroupCollapse = async (groupId: string, currentCollapsed: boolean) => {
    try {
      await browserController.setTabGroupCollapsed(groupId, !currentCollapsed);
      setTabGroups((prev) =>
        prev.map((g) => (g.id === groupId ? { ...g, is_collapsed: !currentCollapsed } : g))
      );
    } catch (err) {
      console.error('Failed to toggle group collapse:', err);
    }
  };

  const handleMoveTabToGroup = async (tabId: string, groupId: string) => {
    try {
      await browserController.moveTabToGroup(tabId, groupId);
      setContextMenu(null);
      fetchTabGroups();
    } catch (err) {
      console.error('Failed to move tab to group:', err);
    }
  };

  const handleRemoveTabFromGroup = async (tabId: string) => {
    try {
      await browserController.removeTabFromGroup(tabId);
      setContextMenu(null);
      fetchTabGroups();
    } catch (err) {
      console.error('Failed to remove tab from group:', err);
    }
  };

  const handleCloseGroupTabs = async (groupId: string) => {
    try {
      await browserController.closeTabGroup(groupId);
      setGroupContextMenu(null);
      setContextMenu(null);
      fetchTabGroups();
    } catch (err) {
      console.error('Failed to close group tabs:', err);
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

  // Sync bounds when Telemetry Dock opens/closes or finishes 300ms transition
  useEffect(() => {
    syncBounds();
    const timer = setTimeout(() => {
      syncBounds();
    }, 320);
    return () => clearTimeout(timer);
  }, [isTelemetryOpen, syncBounds]);

  // Listen for agent status events & download progress events from Rust backend
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

    const unlistenDownload = listen<BrowserDownload>(
      'browser-download-progress',
      (event) => {
        const payload = event.payload;
        setDownloadsList((prev) => {
          const idx = prev.findIndex((d) => d.id === payload.id);
          if (idx >= 0) {
            const next = [...prev];
            next[idx] = payload;
            return next;
          } else {
            return [payload, ...prev];
          }
        });
      }
    );

    return () => {
      unlistenStatus.then((un) => un()).catch(() => {});
      unlistenDownload.then((un) => un()).catch(() => {});
    };
  }, [agentMaxSteps]);

  // Initialize tabs and subscribe to browser state
  useEffect(() => {
    let mounted = true;
    setIsLoading(true);

    fetchBookmarks();
    fetchHistory();
    fetchProfiles();

    const unsubscribe = browserController.subscribe((state) => {
      if (mounted) {
        setBrowserState(state);
        const activeTab = state.tabs.find((t) => t.id === state.active_tab_id);
        if (activeTab && !isOmniboxFocused) {
          setInputUrl(activeTab.url === 'edith://newtab' ? '' : activeTab.url);
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
            const recoveryReport = await browserController.runStartupRecovery();
            if (recoveryReport && recoveryReport.notice) {
              setRecoveryNotice(recoveryReport.notice);
            }
            const restored = await browserController.restoreSession();
            if (restored.length === 0) {
              await browserController.createTab('tab_a', 'edith://newtab', initialBounds);
            }
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

    fetchTabGroups();

    return () => {
      mounted = false;
      unsubscribe();
      resizeObserver.disconnect();
      window.removeEventListener('resize', syncBounds);
      browserController.hideAll().catch(() => {});
    };
  }, [syncBounds, isOmniboxFocused, fetchBookmarks, fetchHistory, fetchProfiles, fetchTabGroups]);

  // Phase 5.6D Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      const isInput = ['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement)?.tagName);

      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        handleCreateNewTab();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        handleReopenTab();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        if (browserState.active_tab_id) {
          handleCloseTab(undefined, browserState.active_tab_id);
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

      if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === 'l' || e.key.toLowerCase() === 'k') || (e.altKey && e.key.toLowerCase() === 'd')) {
        e.preventDefault();
        omniboxInputRef.current?.focus();
        omniboxInputRef.current?.select();
        return;
      }

      if (((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r') || e.key === 'F5') {
        e.preventDefault();
        await browserController.reload();
        return;
      }

      if (e.altKey && e.key === 'ArrowLeft' && !isInput) {
        e.preventDefault();
        await browserController.goBack();
        return;
      }

      if (e.altKey && e.key === 'ArrowRight' && !isInput) {
        e.preventDefault();
        await browserController.goForward();
        return;
      }

      // Phase 5.6F-A Shortcuts
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        handleOpenFind();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handleZoomIn();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === '-') {
        e.preventDefault();
        handleZoomOut();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === '0') {
        e.preventDefault();
        handleZoomReset();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'p') {
        e.preventDefault();
        handlePrint();
        return;
      }

      // Phase 5.6F-B Shortcuts
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'r') {
        e.preventDefault();
        handleToggleReaderMode();
        return;
      }

      // Phase 5.6F-C Shortcuts
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'a') {
        e.preventDefault();
        setShowTabSearchModal((prev) => !prev);
        setTimeout(() => tabSearchInputRef.current?.focus(), 50);
        return;
      }

      if (e.key === 'Escape') {
        if (showTabSearchModal) {
          e.preventDefault();
          setShowTabSearchModal(false);
          return;
        }
        const activeTab = browserState.tabs.find((t) => t.id === browserState.active_tab_id);
        if (activeTab?.is_reader_mode && !showFindHud) {
          e.preventDefault();
          handleToggleReaderMode();
          return;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [browserState.active_tab_id, browserState.tabs, showFindHud, showTabSearchModal]);

  const activeTab = browserState.tabs.find((t) => t.id === browserState.active_tab_id);
  const isNewTab = !activeTab || !activeTab.url || activeTab.url === 'edith://newtab' || activeTab.url === 'about:blank';

  // Phase 5.6F-C Step 9: Auto-expand collapsed group if active tab belongs to it
  useEffect(() => {
    if (activeTab?.group_id) {
      const grp = tabGroups.find((g) => g.id === activeTab.group_id);
      if (grp && grp.is_collapsed) {
        handleToggleGroupCollapse(grp.id, true);
      }
    }
  }, [activeTab?.id, activeTab?.group_id, tabGroups]);

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
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to switch tab:', err);
    }
  };

  // Tab creation
  const handleCreateNewTab = async (url: string = 'edith://newtab') => {
    const newId = `tab_${Date.now().toString(36)}`;
    try {
      let b;
      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        b = { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
      }
      await browserController.createTab(newId, url, b);
      setInspectTabId(newId);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to create new tab:', err);
    }
  };

  // Tab closure
  const handleCloseTab = async (e?: React.MouseEvent, tabId?: string) => {
    if (e) e.stopPropagation();
    const targetId = tabId || browserState.active_tab_id;
    if (!targetId) return;
    try {
      await browserController.closeTab(targetId);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to close tab:', err);
    }
  };

  // Phase 5.6D Tab Operations
  const handleDuplicateTab = async (tabId: string) => {
    try {
      await browserController.duplicateTab(tabId);
      browserController.saveSession();
    } catch (e) {
      console.error('Failed to duplicate tab', e);
    }
  };

  const handleTogglePinTab = async (tabId: string) => {
    try {
      await browserController.togglePinTab(tabId);
      browserController.saveSession();
    } catch (e) {
      console.error('Failed to toggle pin tab', e);
    }
  };

  const handleCloseOtherTabs = async (tabId: string) => {
    try {
      await browserController.closeOtherTabs(tabId);
      browserController.saveSession();
    } catch (e) {
      console.error('Failed to close other tabs', e);
    }
  };

  const handleCloseTabsToRight = async (tabId: string) => {
    try {
      await browserController.closeTabsToRight(tabId);
      browserController.saveSession();
    } catch (e) {
      console.error('Failed to close tabs to right', e);
    }
  };

  const handleReopenTab = async () => {
    try {
      const restored = await browserController.reopenLastClosedTab();
      if (restored) setInspectTabId(restored.id);
      browserController.saveSession();
    } catch (e) {
      console.error('Failed to reopen last closed tab', e);
    }
  };

  // Navigation
  const handleNavigate = async (e?: React.FormEvent, customUrl?: string) => {
    if (e) e.preventDefault();
    const rawTarget = (customUrl || inputUrl).trim();
    if (!rawTarget || !browserState.active_tab_id) return;

    let finalUrl = rawTarget;
    if (finalUrl === 'edith://newtab' || finalUrl === 'about:blank') {
      // Stay on new tab
    } else if (!finalUrl.includes('://') && !finalUrl.includes('.') && !finalUrl.startsWith('localhost')) {
      // Search query via DuckDuckGo
      finalUrl = `https://duckduckgo.com/?q=${encodeURIComponent(finalUrl)}`;
    } else if (!finalUrl.includes('://')) {
      finalUrl = `https://${finalUrl}`;
    }

    setIsLoading(true);
    try {
      const navigatedUrl = await browserController.navigateTab(browserState.active_tab_id, finalUrl);
      setInputUrl(navigatedUrl);
      browserController.saveSession();
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
        setInputUrl(activeTab.url === 'edith://newtab' ? '' : activeTab.url);
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
        <div className="flex items-center gap-1.5 flex-1 min-w-0">
          {/* 1. Global Pinned Tabs (Phase 5.6D & 5.6F-C) */}
          {browserState.tabs.filter((t) => t.is_pinned).map((tab) => {
            const isActive = tab.id === browserState.active_tab_id;
            return (
              <div
                key={tab.id}
                onClick={() => handleSwitchTab(tab.id)}
                onContextMenu={(e) => handleTabContextMenu(e, tab.id)}
                className={`group relative flex items-center justify-center w-8 h-7 rounded-lg text-xs font-mono transition-all cursor-pointer select-none shrink-0 ${
                  isActive
                    ? 'bg-[#091122] text-cyan-300 border border-cyan-500/40 shadow-cyan-glow-xs'
                    : 'bg-white/[0.03] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06] border border-transparent'
                }`}
                title={`Pinned: ${tab.title || tab.url}`}
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
                <span className="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 bg-cyan-400 rounded-full" />
              </div>
            );
          })}

          {/* Divider between pinned and groups/tabs if pinned exist */}
          {browserState.tabs.some((t) => t.is_pinned) && (
            <div className="w-[1px] h-4 bg-white/10 mx-0.5 shrink-0" />
          )}

          {/* 2. Grouped Tabs (Phase 5.6F-C) */}
          {tabGroups
            .filter((g) => !activeTab?.profile_id || g.profile_id === activeTab.profile_id)
            .map((group) => {
              const colorDef = GROUP_COLORS[group.color] || GROUP_COLORS.blue;
              const groupTabs = browserState.tabs.filter((t) => !t.is_pinned && t.group_id === group.id);

              return (
                <div key={group.id} className="flex items-center gap-1 shrink-0">
                  {/* Group Header Pill */}
                  <div
                    onClick={() => handleToggleGroupCollapse(group.id, group.is_collapsed)}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setGroupContextMenu({ groupId: group.id, x: e.clientX, y: e.clientY });
                    }}
                    className={`flex items-center gap-1.5 h-7 px-2.5 rounded-lg border text-xs font-mono font-bold cursor-pointer transition select-none ${colorDef.bg} ${colorDef.border} ${colorDef.text}`}
                    title={`Tab Group: ${group.name} (${groupTabs.length} tabs) — Click to collapse/expand, right-click for options`}
                  >
                    <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: colorDef.dot }} />
                    <span className="text-[11px] tracking-wide uppercase max-w-[100px] truncate">{group.name}</span>
                    <span className="text-[10px] opacity-75">({groupTabs.length})</span>
                    {group.is_collapsed ? (
                      <ChevronRight className="w-3 h-3 shrink-0 opacity-80" />
                    ) : (
                      <ChevronDown className="w-3 h-3 shrink-0 opacity-80" />
                    )}
                  </div>

                  {/* Child Tabs in Group (Hidden when collapsed) */}
                  {!group.is_collapsed &&
                    groupTabs.map((tab) => {
                      const isActive = tab.id === browserState.active_tab_id;
                      return (
                        <div
                          key={tab.id}
                          onClick={() => handleSwitchTab(tab.id)}
                          onContextMenu={(e) => handleTabContextMenu(e, tab.id)}
                          className={`group relative flex items-center gap-2 h-7 px-3 rounded-lg text-xs font-mono transition-all cursor-pointer select-none max-w-[180px] min-w-[120px] shrink-0 border ${
                            isActive
                              ? `${colorDef.activeTab} shadow-sm`
                              : 'bg-white/[0.03] text-slate-400 hover:text-slate-200 hover:bg-white/[0.06] border-transparent'
                          } ${colorDef.tabBorder}`}
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
                            {tab.url === 'edith://newtab' ? 'New Tab' : (tab.title || tab.url || 'New Tab')}
                          </span>
                          {tabControls[tab.id]?.control_state === 'AI_CONTROLLED' ? (
                            <span className="px-1 py-0.5 rounded bg-purple-500/30 text-purple-300 text-[9px] font-bold shrink-0" title="AI Controlled">🤖</span>
                          ) : tabControls[tab.id]?.control_state === 'AI_PAUSED' ? (
                            <span className="px-1 py-0.5 rounded bg-amber-500/30 text-amber-300 text-[9px] font-bold shrink-0" title="AI Paused">⏸️</span>
                          ) : null}
                          {tab.is_pdf && (
                            <span className="px-1 py-0.2 rounded bg-orange-950/80 text-orange-400 border border-orange-500/40 text-[8px] font-bold shrink-0">PDF</span>
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
                </div>
              );
            })}

          {/* 3. Ungrouped Tabs (Phase 5.6F-C) */}
          {browserState.tabs
            .filter((t) => !t.is_pinned && (!t.group_id || !tabGroups.some((g) => g.id === t.group_id)))
            .map((tab) => {
              const isActive = tab.id === browserState.active_tab_id;
              return (
                <div
                  key={tab.id}
                  onClick={() => handleSwitchTab(tab.id)}
                  onContextMenu={(e) => handleTabContextMenu(e, tab.id)}
                  className={`group relative flex items-center gap-2 h-7 px-3 rounded-lg text-xs font-mono transition-all cursor-pointer select-none max-w-[200px] min-w-[130px] shrink-0 ${
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
                    {tab.url === 'edith://newtab' ? 'New Tab' : (tab.title || tab.url || 'New Tab')}
                  </span>
                  {tabControls[tab.id]?.control_state === 'AI_CONTROLLED' ? (
                    <span className="px-1.5 py-0.5 rounded bg-purple-500/30 text-purple-300 text-[9px] font-bold shrink-0 flex items-center gap-1" title="AI Controlled">🤖 AI</span>
                  ) : tabControls[tab.id]?.control_state === 'AI_PAUSED' ? (
                    <span className="px-1.5 py-0.5 rounded bg-amber-500/30 text-amber-300 text-[9px] font-bold shrink-0" title="AI Paused">⏸️</span>
                  ) : null}
                  {tab.profile_id && tab.profile_id !== 'profile_default' && (
                    <span className="px-1 py-0.5 rounded bg-cyan-950/80 border border-cyan-500/30 text-cyan-300 text-[8px] font-mono shrink-0" title={`Profile: ${tab.profile_id}`}>
                      {tab.profile_id.startsWith('agent_') ? 'AI' : tab.profile_id.replace('profile_', '')}
                    </span>
                  )}
                  {tab.is_pdf && (
                    <span className="px-1 py-0.2 rounded bg-orange-950/80 text-orange-400 border border-orange-500/40 text-[8px] font-bold shrink-0">PDF</span>
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

          {/* 4. Action Buttons */}
          <button
            onClick={() => handleCreateNewTab('edith://newtab')}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition shrink-0"
            title="New Tab (Ctrl+T)"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>

          <button
            onClick={() => {
              setTargetTabForGroup(null);
              setShowCreateGroupModal(true);
            }}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition shrink-0"
            title="New Tab Group"
          >
            <FolderPlus className="w-3.5 h-3.5" />
          </button>

          <button
            onClick={() => {
              setShowTabSearchModal(true);
              setTimeout(() => tabSearchInputRef.current?.focus(), 50);
            }}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition shrink-0"
            title="Search Open Tabs (Ctrl+Shift+A)"
          >
            <Compass className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Phase 5.6D Tab Context Menu Popup */}
      {contextMenu && (
        <div
          style={{ top: `${contextMenu.y}px`, left: `${contextMenu.x}px` }}
          className="fixed z-50 bg-[#091122] border border-cyan-500/30 shadow-2xl rounded-xl p-1 text-xs text-slate-200 font-mono flex flex-col gap-0.5 min-w-[170px] backdrop-blur-md animate-fadeIn"
          onClick={(e) => e.stopPropagation()}
        >
          <button
            onClick={() => {
              handleCreateNewTab('edith://newtab');
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <span className="flex items-center gap-2"><Plus className="w-3.5 h-3.5 text-cyan-400" /> New Tab</span>
            <span className="text-[10px] text-slate-500">Ctrl+T</span>
          </button>
          <button
            onClick={() => {
              browserController.reload();
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <span className="flex items-center gap-2"><RotateCw className="w-3.5 h-3.5 text-cyan-400" /> Reload</span>
            <span className="text-[10px] text-slate-500">Ctrl+R</span>
          </button>
          <button
            onClick={() => {
              handleDuplicateTab(contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <Copy className="w-3.5 h-3.5 text-indigo-400" />
            Duplicate Tab
          </button>
          <button
            onClick={() => {
              handleTogglePinTab(contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            {browserState.tabs.find((t) => t.id === contextMenu.tabId)?.is_pinned ? (
              <>
                <PinOff className="w-3.5 h-3.5 text-amber-400" />
                Unpin Tab
              </>
            ) : (
              <>
                <Pin className="w-3.5 h-3.5 text-cyan-400" />
                Pin Tab
              </>
            )}
          </button>
          {/* Phase 5.6F-C Tab Group Options */}
          {(() => {
            const targetTab = browserState.tabs.find((t) => t.id === contextMenu.tabId);
            if (!targetTab || targetTab.is_pinned) return null;
            const availableGroups = tabGroups.filter(
              (g) => (!targetTab.profile_id || g.profile_id === targetTab.profile_id) && g.id !== targetTab.group_id
            );
            return (
              <>
                <div className="h-px bg-white/10 my-0.5" />
                {targetTab.group_id && (
                  <>
                    <button
                      onClick={() => {
                        handleRemoveTabFromGroup(contextMenu.tabId);
                        setContextMenu(null);
                      }}
                      className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-white/10 text-slate-300 text-left transition"
                    >
                      <Tag className="w-3.5 h-3.5 text-slate-400" />
                      Remove from Group
                    </button>
                    <button
                      onClick={() => {
                        if (targetTab.group_id) handleCloseGroupTabs(targetTab.group_id);
                        setContextMenu(null);
                      }}
                      className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-red-500/20 hover:text-red-200 text-left transition"
                    >
                      <X className="w-3.5 h-3.5 text-red-400" />
                      Close Group Tabs
                    </button>
                  </>
                )}
                {availableGroups.map((g) => {
                  const cDef = GROUP_COLORS[g.color] || GROUP_COLORS.blue;
                  return (
                    <button
                      key={g.id}
                      onClick={() => {
                        handleMoveTabToGroup(contextMenu.tabId, g.id);
                        setContextMenu(null);
                      }}
                      className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-white/10 text-slate-200 text-left transition"
                    >
                      <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: cDef.dot }} />
                      <span className="truncate">Move to: {g.name}</span>
                    </button>
                  );
                })}
                <button
                  onClick={() => {
                    setTargetTabForGroup(contextMenu.tabId);
                    setShowCreateGroupModal(true);
                    setContextMenu(null);
                  }}
                  className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
                >
                  <FolderPlus className="w-3.5 h-3.5 text-cyan-400" />
                  Add to New Group...
                </button>
              </>
            );
          })()}
          <div className="h-px bg-white/10 my-0.5" />
          <button
            onClick={() => {
              const targetTab = browserState.tabs.find((t) => t.id === contextMenu.tabId);
              if (targetTab?.url) handleCopyLink(targetTab.url);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <Copy className="w-3.5 h-3.5 text-cyan-400" />
            Copy Tab URL
          </button>
          <button
            onClick={() => {
              handleOpenFind();
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <span className="flex items-center gap-2"><Search className="w-3.5 h-3.5 text-cyan-400" /> Find in Page...</span>
            <span className="text-[10px] text-slate-500">Ctrl+F</span>
          </button>
          <button
            onClick={() => {
              handlePrint();
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <span className="flex items-center gap-2"><Printer className="w-3.5 h-3.5 text-cyan-400" /> Print Tab...</span>
            <span className="text-[10px] text-slate-500">Ctrl+P</span>
          </button>
          <button
            onClick={() => {
              handleToggleReaderMode(contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <span className="flex items-center gap-2"><BookOpen className="w-3.5 h-3.5 text-cyan-400" /> Toggle Reader Mode</span>
            <span className="text-[10px] text-slate-500">Ctrl+Shift+R</span>
          </button>
          <button
            onClick={() => {
              handleSavePageHtml(contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <FileDown className="w-3.5 h-3.5 text-cyan-400" />
            Save Page HTML...
          </button>
          <button
            onClick={() => {
              handlePrint();
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
          >
            <FileText className="w-3.5 h-3.5 text-amber-400" />
            Save Page as PDF...
          </button>
          <div className="h-px bg-white/10 my-0.5" />
          <button
            onClick={() => {
              handleCloseTab(undefined, contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-red-500/20 hover:text-red-200 text-left transition"
          >
            <span className="flex items-center gap-2"><X className="w-3.5 h-3.5 text-red-400" /> Close Tab</span>
            <span className="text-[10px] text-slate-500">Ctrl+W</span>
          </button>
          <button
            onClick={() => {
              handleCloseOtherTabs(contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-white/10 text-slate-300 text-left transition"
          >
            <Trash2 className="w-3.5 h-3.5 text-slate-400" />
            Close Other Tabs
          </button>
          <button
            onClick={() => {
              handleCloseTabsToRight(contextMenu.tabId);
              setContextMenu(null);
            }}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-white/10 text-slate-300 text-left transition"
          >
            <ArrowRight className="w-3.5 h-3.5 text-slate-400" />
            Close Tabs to Right
          </button>
          <button
            onClick={() => {
              handleReopenTab();
              setContextMenu(null);
            }}
            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-blue-500/20 hover:text-blue-200 text-left transition"
          >
            <span className="flex items-center gap-2"><History className="w-3.5 h-3.5 text-blue-400" /> Reopen Closed</span>
            <span className="text-[10px] text-slate-500">Ctrl+Shift+T</span>
          </button>
        </div>
      )}

      {/* Phase 5.6F-C Group Context Menu Popup */}
      {groupContextMenu && (
        <div
          style={{ top: `${groupContextMenu.y}px`, left: `${groupContextMenu.x}px` }}
          className="fixed z-50 bg-[#091122] border border-cyan-500/30 shadow-2xl rounded-xl p-1 text-xs text-slate-200 font-mono flex flex-col gap-0.5 min-w-[180px] backdrop-blur-md animate-fadeIn"
          onClick={(e) => e.stopPropagation()}
        >
          {(() => {
            const group = tabGroups.find((g) => g.id === groupContextMenu.groupId);
            if (!group) return null;
            return (
              <>
                <div className="px-2.5 py-1 text-[10px] font-bold text-slate-400 border-b border-white/10 uppercase tracking-wider">
                  Group: {group.name}
                </div>
                <button
                  onClick={() => {
                    setEditingGroupId(group.id);
                    setEditingGroupName(group.name);
                    setEditingGroupColor(group.color);
                    setGroupContextMenu(null);
                  }}
                  className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-cyan-500/20 hover:text-cyan-200 text-left transition"
                >
                  <Edit2 className="w-3.5 h-3.5 text-cyan-400" />
                  Rename / Recolor Group...
                </button>
                <button
                  onClick={() => {
                    handleToggleGroupCollapse(group.id, group.is_collapsed);
                    setGroupContextMenu(null);
                  }}
                  className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-white/10 text-slate-300 text-left transition"
                >
                  {group.is_collapsed ? (
                    <>
                      <ChevronDown className="w-3.5 h-3.5 text-emerald-400" />
                      Expand Group
                    </>
                  ) : (
                    <>
                      <ChevronRight className="w-3.5 h-3.5 text-amber-400" />
                      Collapse Group
                    </>
                  )}
                </button>
                <div className="h-px bg-white/10 my-0.5" />
                <button
                  onClick={() => handleCloseGroupTabs(group.id)}
                  className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-red-500/20 hover:text-red-200 text-left transition"
                >
                  <X className="w-3.5 h-3.5 text-red-400" />
                  Close Group Tabs
                </button>
                <button
                  onClick={() => handleDeleteGroup(group.id)}
                  className="flex items-center gap-2 px-2.5 py-1.5 rounded-lg hover:bg-red-500/20 hover:text-red-200 text-left transition"
                  title="Ungroups tabs without closing them"
                >
                  <Trash2 className="w-3.5 h-3.5 text-red-400" />
                  Delete Group (Ungroup Tabs)
                </button>
              </>
            );
          })()}
        </div>
      )}

      {/* Phase 5.7C Startup Recovery Notice Banner */}
      {recoveryNotice && (
        <div className="bg-cyan-950/90 border-b border-cyan-500/40 px-3 py-1.5 flex items-center justify-between text-xs font-mono text-cyan-200 z-20 shrink-0 backdrop-blur-sm animate-fadeIn">
          <div className="flex items-center gap-2">
            <CheckCircle className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
            <span>{recoveryNotice}</span>
          </div>
          <button
            onClick={() => setRecoveryNotice(null)}
            className="text-cyan-400 hover:text-cyan-100 px-2 py-0.5 rounded text-[10px] bg-cyan-900/50 hover:bg-cyan-800/60 border border-cyan-500/30 transition"
          >
            Dismiss
          </button>
        </div>
      )}

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
            {(activeTab?.is_pdf || activeTab?.url?.toLowerCase().endsWith('.pdf') || activeTab?.url?.toLowerCase().includes('/pdf/')) && (
              <span className="px-1.5 py-0.5 rounded bg-orange-950/80 text-orange-400 border border-orange-500/40 text-[9px] font-bold shrink-0 mr-1.5 font-mono">
                PDF
              </span>
            )}
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
            {/* Phase 5.6A: Bookmark Star Toggle */}
            <button
              type="button"
              onClick={handleToggleBookmarkActiveTab}
              className={`transition shrink-0 ml-1.5 p-0.5 rounded hover:bg-white/[0.08] ${
                isActiveTabBookmarked ? 'text-amber-400' : 'text-slate-500 hover:text-amber-400'
              }`}
              title={isActiveTabBookmarked ? 'Bookmarked (Click to remove)' : 'Bookmark this tab'}
            >
              <Star className={`w-3.5 h-3.5 ${isActiveTabBookmarked ? 'fill-current' : ''}`} />
            </button>
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

        {/* Action Controls: Bookmarks (5.6A), History (5.6A), AI Agent (4C), Actions (4A), Live DOM Observer, & Screenshot */}
        <div className="flex items-center space-x-1.5">
          {/* Phase 5.6A Bookmarks Button */}
          <button
            onClick={() => {
              setShowBookmarksPanel(!showBookmarksPanel);
              setShowHistoryPanel(false);
              if (!showBookmarksPanel) fetchBookmarks();
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showBookmarksPanel
                ? 'bg-amber-500/20 border-amber-400 text-amber-300 shadow-amber-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-amber-300'
            }`}
            title="Toggle Bookmarks (5.6A)"
          >
            <Bookmark className="w-3.5 h-3.5 text-amber-400" />
            <span className="hidden sm:inline">Bookmarks</span>
          </button>

          {/* Phase 5.6A History Button */}
          <button
            onClick={() => {
              setShowHistoryPanel(!showHistoryPanel);
              setShowBookmarksPanel(false);
              setShowDownloadsPanel(false);
              if (!showHistoryPanel) fetchHistory();
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showHistoryPanel
                ? 'bg-blue-500/20 border-blue-400 text-blue-300 shadow-blue-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-blue-300'
            }`}
            title="Toggle History (5.6A)"
          >
            <History className="w-3.5 h-3.5 text-blue-400" />
            <span className="hidden sm:inline">History</span>
          </button>

          {/* Phase 5.6B Downloads Button */}
          <button
            onClick={() => {
              setShowDownloadsPanel(!showDownloadsPanel);
              setShowHistoryPanel(false);
              setShowBookmarksPanel(false);
              if (!showDownloadsPanel) fetchDownloads();
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showDownloadsPanel
                ? 'bg-emerald-500/20 border-emerald-400 text-emerald-300 shadow-emerald-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-emerald-300'
            }`}
            title="Toggle Downloads Manager (5.6B)"
          >
            <Download className="w-3.5 h-3.5 text-emerald-400" />
            <span className="hidden sm:inline">Downloads</span>
            {downloadsList.filter((d) => d.status === 'DOWNLOADING').length > 0 && (
              <span className="px-1 py-0.2 rounded-full bg-emerald-500 text-black text-[9px] font-bold animate-pulse">
                {downloadsList.filter((d) => d.status === 'DOWNLOADING').length}
              </span>
            )}
          </button>

          {/* Phase 5.6C Profiles Button */}
          <button
            onClick={() => {
              setShowProfilesPanel(!showProfilesPanel);
              setShowPrivacyPanel(false);
              setShowDownloadsPanel(false);
              setShowHistoryPanel(false);
              setShowBookmarksPanel(false);
              if (!showProfilesPanel) fetchProfiles();
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showProfilesPanel
                ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300 shadow-cyan-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-cyan-300'
            }`}
            title="Toggle Browser Profiles & Storage Isolation (5.6C)"
          >
            <Users className="w-3.5 h-3.5 text-cyan-400" />
            <span className="hidden sm:inline">Profiles</span>
            {profilesList.find((p) => p.is_active) && (
              <span className="px-1 py-0.2 rounded bg-cyan-950/80 border border-cyan-500/30 text-cyan-300 text-[9px]">
                {profilesList.find((p) => p.is_active)?.name.slice(0, 8)}
              </span>
            )}
          </button>

          {/* Phase 5.6E Privacy & Content Blocker Shield Button */}
          <button
            onClick={() => {
              setShowPrivacyPanel(!showPrivacyPanel);
              setShowProfilesPanel(false);
              setShowDownloadsPanel(false);
              setShowHistoryPanel(false);
              setShowBookmarksPanel(false);
              if (!showPrivacyPanel) fetchPrivacyStatus(browserState.active_tab_id || undefined);
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-[11px] font-mono transition border ${
              showPrivacyPanel
                ? 'bg-emerald-500/20 border-emerald-400 text-emerald-300 shadow-emerald-glow-xs'
                : privacyStatus?.enabled === false
                ? 'bg-red-950/40 border-red-500/40 text-red-300'
                : 'bg-[#090e1a] border-white/[0.08] text-slate-300 hover:text-emerald-300'
            }`}
            title="Toggle Privacy & Content Blocker (5.6E)"
          >
            {privacyStatus?.enabled === false ? (
              <ShieldOff className="w-3.5 h-3.5 text-red-400" />
            ) : (
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
            )}
            <span className="hidden sm:inline">Shield</span>
            {privacyStatus?.tab_stats && privacyStatus.tab_stats.blocked_total > 0 && (
              <span className="px-1 py-0.2 rounded-full bg-emerald-500 text-black text-[9px] font-bold">
                {privacyStatus.tab_stats.blocked_total}
              </span>
            )}
          </button>

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

          <div className="w-[1px] h-4 bg-white/10 mx-0.5" />

          {/* Phase 5.6F-A: Find in Page Button */}
          <button
            onClick={handleOpenFind}
            className={`p-1.5 rounded-xl border text-slate-300 hover:text-cyan-300 transition ${
              showFindHud ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300' : 'bg-[#090e1a] border-white/[0.08]'
            }`}
            title="Find in Page (Ctrl+F)"
          >
            <Search className="w-3.5 h-3.5" />
          </button>

          {/* Phase 5.6F-A: Zoom Controls */}
          <div className="relative flex items-center bg-[#090e1a] border border-white/[0.08] rounded-xl text-[11px] font-mono">
            <button
              onClick={handleZoomOut}
              className="px-1.5 py-1 text-slate-400 hover:text-white transition"
              title="Zoom Out (Ctrl+-)"
            >
              -
            </button>
            <button
              onClick={handleZoomReset}
              className="px-1.5 py-1 text-slate-200 font-bold hover:text-cyan-300 transition"
              title="Reset Zoom (Ctrl+0)"
            >
              {Math.round((activeTab?.zoom_level || 1.0) * 100)}%
            </button>
            <button
              onClick={handleZoomIn}
              className="px-1.5 py-1 text-slate-400 hover:text-white transition"
              title="Zoom In (Ctrl++)"
            >
              +
            </button>
          </div>

          {/* Phase 5.6F-A: Print Button */}
          <button
            onClick={handlePrint}
            className="p-1.5 rounded-xl bg-[#090e1a] border border-white/[0.08] text-slate-300 hover:text-cyan-300 transition"
            title="Print Page (Ctrl+P)"
          >
            <Printer className="w-3.5 h-3.5" />
          </button>

          {/* Phase 5.6F-B: Reader Mode Toggle Button */}
          <button
            onClick={() => handleToggleReaderMode(activeTab?.id)}
            disabled={!activeTab || isNewTab}
            className={`p-1.5 rounded-xl border text-slate-300 hover:text-cyan-300 transition disabled:opacity-40 ${
              activeTab?.is_reader_mode
                ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300 shadow-cyan-glow-xs'
                : 'bg-[#090e1a] border-white/[0.08]'
            }`}
            title="Toggle Reader Mode (Ctrl+Shift+R)"
          >
            <BookOpen className="w-3.5 h-3.5" />
          </button>

          {/* Phase 5.6F-B: Save Page HTML Button */}
          <button
            onClick={() => handleSavePageHtml(activeTab?.id)}
            disabled={!activeTab || isNewTab}
            className="p-1.5 rounded-xl bg-[#090e1a] border border-white/[0.08] text-slate-300 hover:text-cyan-300 transition disabled:opacity-40"
            title="Save Page HTML (Downloads)"
          >
            <FileDown className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Phase 5.6B Downloads Drawer */}
      {showDownloadsPanel && (
        <div className="bg-[#030d09] border-b border-emerald-500/30 p-3 text-xs text-emerald-200 flex flex-col gap-2.5 shrink-0 z-10 max-h-80 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-emerald-300">
              <Download className="w-4 h-4 text-emerald-400" />
              Phase 5.6B Download Manager ({downloadsList.length} items)
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={handleClearDownloadRecords}
                className="flex items-center gap-1 text-[10px] text-slate-300 hover:text-red-300 bg-white/5 hover:bg-red-950/40 border border-white/10 px-2 py-0.5 rounded transition"
                title="Clear download records (files remain on disk)"
              >
                <Trash2 className="w-3 h-3" />
                Clear Records
              </button>
              <button
                onClick={() => setShowDownloadsPanel(false)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>

          {downloadsList.length === 0 ? (
            <div className="text-slate-400 text-center py-4 text-[11px]">No downloads recorded yet.</div>
          ) : (
            <div className="flex flex-col gap-1.5 max-h-60 overflow-y-auto">
              {downloadsList.map((dl) => (
                <div
                  key={dl.id}
                  className="flex flex-col bg-black/50 border border-white/5 hover:border-emerald-500/30 p-2 rounded-lg text-[11px] gap-1 transition"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 truncate flex-1">
                      <FileText className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                      <span className="text-emerald-200 font-bold truncate">{dl.filename}</span>
                      {dl.tab_id && (
                        <span className="text-[9px] px-1 py-0.2 rounded bg-cyan-950 text-cyan-300 border border-cyan-500/30">
                          {dl.tab_id}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <span className={`text-[9px] px-1.5 py-0.5 rounded font-bold ${
                        dl.status === 'COMPLETED'
                          ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-500/40'
                          : dl.status === 'DOWNLOADING'
                          ? 'bg-cyan-950/80 text-cyan-300 border border-cyan-500/40 animate-pulse'
                          : dl.status === 'CANCELLED'
                          ? 'bg-amber-950/80 text-amber-400 border border-amber-500/40'
                          : 'bg-red-950/80 text-red-400 border border-red-500/40'
                      }`}>
                        {dl.status}
                      </span>
                    </div>
                  </div>

                  {/* Progress bar during downloading */}
                  {dl.status === 'DOWNLOADING' && (
                    <div className="w-full bg-black/60 rounded-full h-1.5 overflow-hidden border border-white/10 my-0.5">
                      <div
                        className="bg-emerald-400 h-full transition-all duration-200"
                        style={{ width: `${Math.round(dl.progress * 100)}%` }}
                      />
                    </div>
                  )}

                  <div className="flex items-center justify-between text-[10px] text-slate-400 mt-0.5">
                    <div className="flex items-center gap-3 truncate">
                      <span>{(dl.received_bytes / 1024).toFixed(1)} KB{dl.total_bytes ? ` / ${(dl.total_bytes / 1024).toFixed(1)} KB (${Math.round(dl.progress * 100)}%)` : ''}</span>
                      <span className="truncate max-w-[240px]" title={dl.destination}>{dl.destination}</span>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      {dl.status === 'DOWNLOADING' && (
                        <button
                          onClick={() => handleCancelDownload(dl.id)}
                          className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-red-950/60 hover:bg-red-900/60 text-red-300 border border-red-500/30 transition"
                          title="Cancel Download"
                        >
                          <XCircle className="w-3 h-3" />
                          Cancel
                        </button>
                      )}
                      {dl.status === 'COMPLETED' && (
                        <>
                          <button
                            onClick={() => handleOpenFile(dl.id)}
                            className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-emerald-950/60 hover:bg-emerald-900/60 text-emerald-300 border border-emerald-500/30 transition"
                            title="Open downloaded file (or folder if executable)"
                          >
                            <ExternalLink className="w-3 h-3" />
                            Open
                          </button>
                          <button
                            onClick={() => handleShowInFolder(dl.id)}
                            className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/5 hover:bg-white/10 text-slate-300 border border-white/10 transition"
                            title="Show in explorer folder"
                          >
                            <Folder className="w-3 h-3" />
                            Folder
                          </button>
                        </>
                      )}
                      <button
                        onClick={() => handleDeleteDownloadRecord(dl.id)}
                        className="text-slate-500 hover:text-red-400 p-0.5 transition"
                        title="Remove download record"
                      >
                        <Trash2 className="w-3 h-3" />
                      </button>
                    </div>
                  </div>
                  {dl.error && (
                    <div className="text-[10px] text-red-400 bg-red-950/40 px-1.5 py-0.5 rounded border border-red-500/20">
                      Error: {dl.error}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Phase 5.6A Bookmarks Drawer */}
      {showBookmarksPanel && (
        <div className="bg-[#0b0903] border-b border-amber-500/30 p-3 text-xs text-amber-200 flex flex-col gap-2.5 shrink-0 z-10 max-h-72 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-amber-300">
              <Bookmark className="w-4 h-4 text-amber-400" />
              Phase 5.6A Saved Bookmarks ({bookmarksList.length})
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={bookmarkSearchQuery}
                onChange={(e) => {
                  setBookmarkSearchQuery(e.target.value);
                  fetchBookmarks(e.target.value);
                }}
                placeholder="Search bookmarks..."
                className="bg-black/60 border border-white/10 rounded px-2 py-0.5 text-[11px] text-amber-200 placeholder-slate-500 focus:outline-none w-44"
              />
              <button
                onClick={() => setShowBookmarksPanel(false)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>
          {bookmarksList.length === 0 ? (
            <div className="text-slate-400 text-center py-4 text-[11px]">No bookmarks saved yet. Click the star icon in the address bar to bookmark any page.</div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-1.5 max-h-48 overflow-y-auto">
              {bookmarksList.map((bm) => (
                <div
                  key={bm.id}
                  className="flex items-center justify-between bg-black/40 border border-white/5 hover:border-amber-500/30 p-1.5 rounded-lg text-[11px] group transition"
                >
                  <div
                    onClick={() => {
                      if (browserState.active_tab_id) {
                        browserController.navigateTab(browserState.active_tab_id, bm.url);
                        setShowBookmarksPanel(false);
                      }
                    }}
                    className="flex items-center gap-2 truncate cursor-pointer flex-1"
                    title={`Open: ${bm.url}`}
                  >
                    <Bookmark className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                    <div className="truncate">
                      <div className="text-amber-200 font-bold truncate">{bm.title}</div>
                      <div className="text-[10px] text-slate-400 truncate">{bm.url}</div>
                    </div>
                  </div>
                  <button
                    onClick={async () => {
                      await browserController.deleteBookmark(bm.id);
                      fetchBookmarks(bookmarkSearchQuery);
                      if (browserState.tabs.find((t) => t.id === browserState.active_tab_id)?.url === bm.url) {
                        setIsActiveTabBookmarked(false);
                      }
                    }}
                    className="opacity-0 group-hover:opacity-100 text-slate-400 hover:text-red-400 p-1 rounded transition"
                    title="Delete bookmark"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Phase 5.6C Profiles Drawer */}
      {showProfilesPanel && (
        <div className="bg-[#030914] border-b border-cyan-500/30 p-3 text-xs text-cyan-200 flex flex-col gap-2.5 shrink-0 z-10 max-h-80 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-cyan-300">
              <Users className="w-4 h-4 text-cyan-400" />
              Phase 5.6C Browser Profiles & Storage Isolation ({profilesList.length})
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setIsCreatingProfile(!isCreatingProfile)}
                className="flex items-center gap-1 text-[10px] text-cyan-300 hover:text-cyan-100 bg-cyan-950/60 hover:bg-cyan-900/60 border border-cyan-500/30 px-2 py-0.5 rounded transition"
              >
                <Plus className="w-3 h-3" />
                New Profile
              </button>
              <button
                onClick={() => setShowProfilesPanel(false)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>

          {/* Create Profile Inline Form */}
          {isCreatingProfile && (
            <form onSubmit={handleCreateProfile} className="flex items-center gap-2 p-2 bg-black/60 border border-cyan-500/30 rounded-lg">
              <input
                type="text"
                value={newProfileName}
                onChange={(e) => setNewProfileName(e.target.value)}
                placeholder="Enter Profile Name (e.g. Work, Research, Personal)..."
                className="bg-[#091122] border border-white/10 rounded px-2.5 py-1 text-xs text-cyan-100 placeholder-slate-500 focus:outline-none flex-1 font-mono"
                autoFocus
              />
              <select
                value={newProfileType}
                onChange={(e) => setNewProfileType(e.target.value as BrowserProfileType)}
                className="bg-[#091122] border border-white/10 rounded px-2 py-1 text-xs text-cyan-300 focus:outline-none cursor-pointer font-mono"
              >
                <option value="USER">USER</option>
                <option value="WORK">WORK</option>
                <option value="RESEARCH">RESEARCH</option>
                <option value="AGENT_TEMPORARY">AGENT_TEMPORARY</option>
              </select>
              <button
                type="submit"
                className="px-3 py-1 rounded bg-cyan-600 hover:bg-cyan-500 text-black font-bold text-xs transition"
              >
                Create
              </button>
              <button
                type="button"
                onClick={() => setIsCreatingProfile(false)}
                className="px-2 py-1 rounded text-slate-400 hover:text-white text-xs"
              >
                Cancel
              </button>
            </form>
          )}

          {/* Profiles Grid */}
          {profilesList.length === 0 ? (
            <div className="text-slate-400 text-center py-4 text-[11px]">Loading browser profiles...</div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 max-h-56 overflow-y-auto">
              {profilesList.map((p) => (
                <div
                  key={p.id}
                  className={`flex items-center justify-between p-2 rounded-lg border text-[11px] transition ${
                    p.is_active
                      ? 'bg-cyan-950/40 border-cyan-500/40 shadow-cyan-glow-xs'
                      : 'bg-black/40 border-white/5 hover:border-cyan-500/20'
                  }`}
                >
                  <div className="flex flex-col gap-0.5 truncate flex-1 mr-2">
                    <div className="flex items-center gap-1.5">
                      <span className="font-bold text-slate-100 truncate">{p.name}</span>
                      <span
                        className={`px-1.5 py-0.2 rounded text-[9px] font-bold ${
                          p.profile_type === 'DEFAULT'
                            ? 'bg-blue-500/20 text-blue-300 border border-blue-500/30'
                            : p.profile_type === 'WORK'
                            ? 'bg-indigo-500/20 text-indigo-300 border border-indigo-500/30'
                            : p.profile_type === 'RESEARCH'
                            ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30'
                            : p.profile_type === 'AGENT_TEMPORARY'
                            ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30'
                            : 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30'
                        }`}
                      >
                        {p.profile_type}
                      </span>
                      {p.is_active && (
                        <span className="px-1 py-0.2 rounded bg-emerald-500 text-black text-[9px] font-bold">
                          ACTIVE
                        </span>
                      )}
                    </div>
                    <div className="text-[10px] text-slate-400 font-mono truncate" title={p.user_data_dir}>
                      Dir: {p.user_data_dir}
                    </div>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0">
                    {!p.is_active && (
                      <button
                        onClick={() => handleSwitchProfile(p.id)}
                        className="px-2 py-0.5 rounded bg-cyan-600/30 hover:bg-cyan-600/50 border border-cyan-500/40 text-cyan-200 text-[10px] font-bold transition"
                      >
                        Switch
                      </button>
                    )}
                    {!p.is_default && (
                      <button
                        onClick={() => handleDeleteProfile(p.id)}
                        className="p-1 text-slate-500 hover:text-red-400 transition"
                        title="Delete Profile & Storage"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Phase 5.6A History Drawer */}
      {showHistoryPanel && (
        <div className="bg-[#030814] border-b border-blue-500/30 p-3 text-xs text-blue-200 flex flex-col gap-2.5 shrink-0 z-10 max-h-80 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-blue-300">
              <History className="w-4 h-4 text-blue-400" />
              Phase 5.6A Browsing History ({historyList.length} items)
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={historySearchQuery}
                onChange={(e) => {
                  setHistorySearchQuery(e.target.value);
                  fetchHistory(e.target.value);
                }}
                placeholder="Search history..."
                className="bg-black/60 border border-white/10 rounded px-2 py-0.5 text-[11px] text-blue-200 placeholder-slate-500 focus:outline-none w-44"
              />
              <button
                onClick={async () => {
                  if (confirm('Clear all browsing history?')) {
                    await browserController.clearHistory();
                    fetchHistory();
                  }
                }}
                className="flex items-center gap-1 text-[10px] text-red-300 hover:text-red-200 bg-red-950/40 hover:bg-red-900/60 border border-red-500/30 px-2 py-0.5 rounded transition"
                title="Clear All History"
              >
                <Trash2 className="w-3 h-3" />
                Clear All
              </button>
              <button
                onClick={() => setShowHistoryPanel(false)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>
          {historyList.length === 0 ? (
            <div className="text-slate-400 text-center py-4 text-[11px]">No history entries found.</div>
          ) : (
            <div className="flex flex-col gap-1 max-h-56 overflow-y-auto">
              {historyList.map((entry) => (
                <div
                  key={entry.id}
                  className="flex items-center justify-between bg-black/40 border border-white/5 hover:border-blue-500/30 p-1.5 rounded-lg text-[11px] group transition"
                >
                  <div
                    onClick={() => {
                      if (browserState.active_tab_id) {
                        browserController.navigateTab(browserState.active_tab_id, entry.url);
                        setShowHistoryPanel(false);
                      }
                    }}
                    className="flex items-center gap-2 truncate cursor-pointer flex-1"
                    title={`Open: ${entry.url}`}
                  >
                    <Globe className="w-3.5 h-3.5 text-blue-400 shrink-0" />
                    <div className="truncate flex-1">
                      <div className="text-blue-200 font-bold truncate">{entry.title}</div>
                      <div className="text-[10px] text-slate-400 truncate">{entry.url}</div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 shrink-0 text-[10px] text-slate-400 font-mono">
                    {entry.visit_count > 1 && (
                      <span className="px-1.5 py-0.5 rounded bg-blue-950/80 border border-blue-500/30 text-blue-300">
                        {entry.visit_count}x
                      </span>
                    )}
                    <span>{new Date(entry.last_visited_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                    <button
                      onClick={async (e) => {
                        e.stopPropagation();
                        await browserController.deleteHistory(entry.id);
                        fetchHistory(historySearchQuery);
                      }}
                      className="opacity-0 group-hover:opacity-100 text-slate-400 hover:text-red-400 p-1 rounded transition"
                      title="Delete history item"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Phase 5.6E Privacy & Content Blocker Drawer */}
      {showPrivacyPanel && (
        <div className="bg-[#030d0a] border-b border-emerald-500/30 p-3.5 text-xs text-emerald-200 flex flex-col gap-3 shrink-0 z-10 max-h-96 overflow-y-auto animate-fadeIn font-mono">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 font-bold text-emerald-300">
              <ShieldCheck className="w-4 h-4 text-emerald-400" />
              Phase 5.6E Host Content Blocking & Privacy Engine
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => fetchPrivacyStatus(browserState.active_tab_id || undefined)}
                disabled={isFetchingPrivacy}
                className="flex items-center gap-1 text-[11px] text-emerald-300 bg-emerald-950/60 hover:bg-emerald-900/60 border border-emerald-500/30 px-2 py-0.5 rounded transition"
              >
                <RotateCw className={`w-3 h-3 ${isFetchingPrivacy ? 'animate-spin' : ''}`} />
                Refresh
              </button>
              <button
                onClick={() => setShowPrivacyPanel(false)}
                className="text-slate-400 hover:text-white text-xs px-2 py-0.5 rounded bg-white/10"
              >
                Close
              </button>
            </div>
          </div>

          {/* Master Toggle & Current Site Card */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {/* Protection Controls */}
            <div className="bg-black/50 p-3 rounded-xl border border-white/5 flex flex-col gap-2.5">
              <div className="flex items-center justify-between">
                <span className="font-bold text-slate-200 text-xs">Host Request Filtering:</span>
                <button
                  onClick={handleTogglePrivacyProtection}
                  className={`px-3 py-1 rounded-lg text-xs font-bold font-mono transition ${
                    privacyStatus?.enabled
                      ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-400'
                      : 'bg-red-950 text-red-300 border border-red-500'
                  }`}
                >
                  {privacyStatus?.enabled ? 'PROTECTION ON' : 'PROTECTION OFF'}
                </button>
              </div>

              <div className="text-[11px] text-slate-400 flex flex-col gap-1 border-t border-white/5 pt-2">
                <div className="flex justify-between">
                  <span>Ad Blocking:</span>
                  <span className={privacyStatus?.block_ads ? 'text-emerald-400' : 'text-slate-500'}>
                    {privacyStatus?.block_ads ? 'Active' : 'Disabled'}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>Tracker & Telemetry Blocking:</span>
                  <span className={privacyStatus?.block_trackers ? 'text-emerald-400' : 'text-slate-500'}>
                    {privacyStatus?.block_trackers ? 'Active' : 'Disabled'}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>Do-Not-Track & GPC:</span>
                  <span className="text-cyan-400">Enforced</span>
                </div>
                <div className="flex justify-between">
                  <span>Compiled Rule Count:</span>
                  <span className="text-slate-200 font-bold">{privacyStatus?.total_rules_loaded || 48} rules</span>
                </div>
              </div>
            </div>

            {/* Current Site Status */}
            <div className="bg-black/50 p-3 rounded-xl border border-white/5 flex flex-col gap-2.5">
              <div className="flex items-center justify-between">
                <span className="font-bold text-slate-200 text-xs">Current Site Policy:</span>
                {(() => {
                  const activeTab = browserState.tabs.find((t) => t.id === browserState.active_tab_id);
                  let domain = '';
                  if (activeTab?.url) {
                    try { domain = new URL(activeTab.url).hostname; } catch {}
                  }
                  const isAllowlisted = domain && privacyStatus?.allowlisted_domains.includes(domain);
                  return domain ? (
                    <button
                      onClick={() => handleToggleSiteAllowlist(domain)}
                      className={`px-2.5 py-1 rounded-lg text-xs font-mono transition border ${
                        isAllowlisted
                          ? 'bg-amber-950/80 border-amber-500 text-amber-300'
                          : 'bg-emerald-950/80 border-emerald-500 text-emerald-300'
                      }`}
                    >
                      {isAllowlisted ? 'Allowlisted (Bypassed)' : 'Protected (Active)'}
                    </button>
                  ) : (
                    <span className="text-slate-500 text-[11px]">No active site</span>
                  );
                })()}
              </div>

              {/* Per-Tab Statistics */}
              <div className="grid grid-cols-3 gap-2 border-t border-white/5 pt-2 text-center">
                <div className="bg-white/5 p-2 rounded-lg">
                  <div className="text-[10px] text-slate-400">Ads Blocked</div>
                  <div className="text-sm font-bold text-emerald-400">{privacyStatus?.tab_stats?.blocked_ads || 0}</div>
                </div>
                <div className="bg-white/5 p-2 rounded-lg">
                  <div className="text-[10px] text-slate-400">Trackers</div>
                  <div className="text-sm font-bold text-cyan-400">{privacyStatus?.tab_stats?.blocked_trackers || 0}</div>
                </div>
                <div className="bg-white/5 p-2 rounded-lg">
                  <div className="text-[10px] text-slate-400">Total Filtered</div>
                  <div className="text-sm font-bold text-slate-100">{privacyStatus?.tab_stats?.blocked_total || 0}</div>
                </div>
              </div>

              {browserState.active_tab_id && (
                <button
                  onClick={() => handleResetTabStats(browserState.active_tab_id!)}
                  className="text-[10px] text-slate-400 hover:text-slate-200 underline text-right"
                >
                  Reset Tab Counters
                </button>
              )}
            </div>
          </div>

          {/* Custom Blocking Rules Form & List */}
          <div className="bg-black/40 p-3 rounded-xl border border-white/5 flex flex-col gap-2">
            <div className="font-bold text-slate-200 text-xs">Custom Domain & Pattern Rules:</div>
            <form onSubmit={handleAddCustomRule} className="flex gap-2">
              <input
                type="text"
                value={customRulePattern}
                onChange={(e) => setCustomRulePattern(e.target.value)}
                placeholder="Domain or pattern (e.g. adserver.net, /track/)..."
                className="bg-black/60 border border-white/10 rounded px-2.5 py-1 text-xs text-slate-200 placeholder-slate-500 focus:outline-none flex-1 font-mono"
              />
              <select
                value={customRuleCategory}
                onChange={(e) => setCustomRuleCategory(e.target.value as any)}
                className="bg-black/60 border border-white/10 rounded px-2 py-1 text-xs text-slate-200 focus:outline-none font-mono"
              >
                <option value="CUSTOM">Custom</option>
                <option value="AD">Ad</option>
                <option value="TRACKER">Tracker</option>
              </select>
              <button
                type="submit"
                className="px-3 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-black font-bold text-xs transition font-mono"
              >
                Add Rule
              </button>
            </form>

            {/* Rules List */}
            {privacyRulesList.length > 0 && (
              <div className="flex flex-wrap gap-1.5 max-h-28 overflow-y-auto mt-1">
                {privacyRulesList.map((r) => (
                  <div
                    key={r.id}
                    className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-white/5 border border-white/10 text-[10px] text-slate-300"
                  >
                    <span className="font-mono text-emerald-300">{r.pattern}</span>
                    <span className="text-slate-500">({r.category})</span>
                    <button
                      onClick={() => handleRemoveCustomRule(r.id)}
                      className="text-slate-500 hover:text-red-400 ml-1"
                      title="Delete Rule"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

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

      {/* Center Viewport Canvas (Native Multi-WebView Target Area or New Tab Page) */}
      <div
        ref={viewportRef}
        id="edith-browser-viewport-container"
        className="flex-1 w-full bg-[#02050e] relative overflow-hidden flex flex-col items-center justify-start"
      >
        {isNewTab ? (
          /* Phase 5.6D Native E.D.I.T.H. New Tab Page */
          <div className="w-full h-full overflow-y-auto px-6 py-8 flex flex-col items-center animate-fadeIn z-20">
            <div className="max-w-4xl w-full flex flex-col items-center gap-6">
              {/* E.D.I.T.H. Branding & Status */}
              <div className="flex flex-col items-center text-center gap-1.5 mt-2">
                <div className="flex items-center gap-2">
                  <Globe className="w-8 h-8 text-cyan-400 animate-pulse" />
                  <h1 className="text-2xl font-black tracking-wider text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 via-blue-400 to-indigo-300 font-mono">
                    E.D.I.T.H. BROWSER
                  </h1>
                </div>
                <div className="flex items-center gap-2 text-xs text-slate-400 font-mono">
                  <span>Native Multi-WebView2 Core</span>
                  <span>•</span>
                  <span className="flex items-center gap-1 text-cyan-300">
                    <Users className="w-3 h-3 text-cyan-400" />
                    Profile: <strong>{activeTab?.profile_id || 'profile_default'}</strong>
                  </span>
                  <span>•</span>
                  <span className="text-emerald-400">Security Enforced</span>
                </div>
              </div>

              {/* Central Omnibox Search Field */}
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  if (newTabSearchQuery.trim()) {
                    handleNavigate(e, newTabSearchQuery.trim());
                  }
                }}
                className="w-full max-w-2xl relative flex items-center shadow-2xl"
              >
                <div className="w-full flex items-center bg-[#070e1c]/90 border border-cyan-500/30 focus-within:border-cyan-400 rounded-2xl px-4 py-2.5 backdrop-blur-xl transition">
                  <Search className="w-5 h-5 text-cyan-400 mr-3 shrink-0" />
                  <input
                    type="text"
                    value={newTabSearchQuery}
                    onChange={(e) => setNewTabSearchQuery(e.target.value)}
                    placeholder="Search with DuckDuckGo or enter web address..."
                    className="bg-transparent text-sm text-slate-100 placeholder-slate-500 focus:outline-none flex-1 font-mono"
                    autoFocus
                  />
                  <button
                    type="submit"
                    className="px-3 py-1 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-black font-bold text-xs transition font-mono shadow-md"
                  >
                    Go
                  </button>
                </div>
              </form>

              {/* Quick Launch Shortcut Tiles */}
              <div className="w-full grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-2.5">
                {[
                  { name: 'Google', url: 'https://www.google.com', icon: Globe, color: 'text-blue-400' },
                  { name: 'GitHub', url: 'https://github.com', icon: Code2, color: 'text-purple-400' },
                  { name: 'Wikipedia', url: 'https://en.wikipedia.org', icon: Globe, color: 'text-slate-300' },
                  { name: 'Rust Docs', url: 'https://doc.rust-lang.org', icon: Terminal, color: 'text-amber-400' },
                  { name: 'Tauri v2', url: 'https://v2.tauri.app', icon: Cpu, color: 'text-cyan-400' },
                  { name: 'DuckDuckGo', url: 'https://duckduckgo.com', icon: Search, color: 'text-emerald-400' },
                ].map((item) => (
                  <button
                    key={item.name}
                    onClick={() => handleNavigate(undefined, item.url)}
                    className="flex flex-col items-center justify-center p-3 rounded-xl bg-black/40 border border-white/5 hover:border-cyan-500/40 hover:bg-cyan-950/20 transition group shadow-sm"
                  >
                    <item.icon className={`w-5 h-5 ${item.color} mb-1.5 group-hover:scale-110 transition-transform`} />
                    <span className="text-xs font-mono text-slate-300 group-hover:text-cyan-200">{item.name}</span>
                  </button>
                ))}
              </div>

              {/* Bookmarks & Recent History Grid */}
              <div className="w-full grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* Bookmarks Section */}
                <div className="p-4 rounded-2xl bg-[#040813]/80 border border-amber-500/20 backdrop-blur-md flex flex-col gap-2.5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 text-xs font-bold text-amber-300 font-mono">
                      <Bookmark className="w-4 h-4 text-amber-400" />
                      Bookmarks ({bookmarksList.length})
                    </div>
                    <button
                      onClick={() => setShowBookmarksPanel(true)}
                      className="text-[10px] text-amber-400/80 hover:text-amber-200 font-mono transition"
                    >
                      View All →
                    </button>
                  </div>
                  {bookmarksList.length === 0 ? (
                    <div className="text-center py-6 text-slate-500 text-xs font-mono">
                      No bookmarks saved yet. Star pages from the top address bar.
                    </div>
                  ) : (
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-1.5">
                      {bookmarksList.slice(0, 6).map((bm) => (
                        <div
                          key={bm.id}
                          onClick={() => handleNavigate(undefined, bm.url)}
                          className="flex items-center gap-2 p-2 rounded-lg bg-black/40 border border-white/5 hover:border-amber-500/30 cursor-pointer group transition"
                        >
                          <Bookmark className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                          <div className="truncate flex-1">
                            <div className="text-xs text-amber-200 font-medium truncate group-hover:text-amber-100">{bm.title}</div>
                            <div className="text-[10px] text-slate-500 truncate">{bm.url}</div>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Recent History Section */}
                <div className="p-4 rounded-2xl bg-[#040813]/80 border border-blue-500/20 backdrop-blur-md flex flex-col gap-2.5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 text-xs font-bold text-blue-300 font-mono">
                      <History className="w-4 h-4 text-blue-400" />
                      Recent History ({historyList.length})
                    </div>
                    <button
                      onClick={() => setShowHistoryPanel(true)}
                      className="text-[10px] text-blue-400/80 hover:text-blue-200 font-mono transition"
                    >
                      View All →
                    </button>
                  </div>
                  {historyList.length === 0 ? (
                    <div className="text-center py-6 text-slate-500 text-xs font-mono">
                      No recent browsing history.
                    </div>
                  ) : (
                    <div className="flex flex-col gap-1">
                      {historyList.slice(0, 5).map((entry) => (
                        <div
                          key={entry.id}
                          onClick={() => handleNavigate(undefined, entry.url)}
                          className="flex items-center justify-between p-1.5 rounded-lg bg-black/40 border border-white/5 hover:border-blue-500/30 cursor-pointer group transition"
                        >
                          <div className="flex items-center gap-2 truncate flex-1">
                            <Globe className="w-3.5 h-3.5 text-blue-400 shrink-0" />
                            <div className="truncate flex-1">
                              <div className="text-xs text-blue-200 font-medium truncate group-hover:text-blue-100">{entry.title}</div>
                              <div className="text-[10px] text-slate-500 truncate">{entry.url}</div>
                            </div>
                          </div>
                          <span className="text-[9px] text-slate-500 font-mono shrink-0 ml-2">
                            {new Date(entry.last_visited_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>

              {/* Quick Feature Drawers Launcher */}
              <div className="flex flex-wrap items-center justify-center gap-2 text-xs font-mono">
                <button
                  onClick={() => setShowHistoryPanel(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-[#060c18] border border-blue-500/30 text-blue-300 hover:bg-blue-950/40 transition"
                >
                  <History className="w-3.5 h-3.5" /> History (5.6A)
                </button>
                <button
                  onClick={() => setShowBookmarksPanel(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-[#0c0903] border border-amber-500/30 text-amber-300 hover:bg-amber-950/40 transition"
                >
                  <Bookmark className="w-3.5 h-3.5" /> Bookmarks (5.6A)
                </button>
                <button
                  onClick={() => setShowDownloadsPanel(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-[#030e09] border border-emerald-500/30 text-emerald-300 hover:bg-emerald-950/40 transition"
                >
                  <Download className="w-3.5 h-3.5" /> Downloads (5.6B)
                </button>
                <button
                  onClick={() => setShowProfilesPanel(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-[#030c14] border border-cyan-500/30 text-cyan-300 hover:bg-cyan-950/40 transition"
                >
                  <Users className="w-3.5 h-3.5" /> Profiles (5.6C)
                </button>
                <button
                  onClick={() => {
                    setShowPrivacyPanel(true);
                    fetchPrivacyStatus(browserState.active_tab_id || undefined);
                  }}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-[#030d0a] border border-emerald-500/30 text-emerald-300 hover:bg-emerald-950/40 transition"
                >
                  <ShieldCheck className="w-3.5 h-3.5" /> Shield / Privacy (5.6E)
                </button>
                <button
                  onClick={() => setShowAgentPanel(true)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-[#0c0414] border border-purple-500/30 text-purple-300 hover:bg-purple-950/40 transition"
                >
                  <Bot className="w-3.5 h-3.5" /> AI Agent HUD (4C)
                </button>
              </div>
            </div>
          </div>
        ) : !isTauri() ? (
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
        ) : null}

        {/* Phase 5.6F-A Floating Find in Page HUD */}
        {showFindHud && (
          <div className="absolute top-2 right-4 z-40 bg-[#080d1a]/95 border border-cyan-500/40 backdrop-blur-xl shadow-2xl rounded-xl p-1.5 flex items-center gap-2 font-mono text-xs animate-fadeIn text-slate-200">
            <div className="flex items-center gap-1.5 px-2 py-1 bg-black/60 border border-white/10 rounded-lg">
              <Search className="w-3.5 h-3.5 text-cyan-400" />
              <input
                ref={findInputRef}
                type="text"
                value={findQuery}
                onChange={(e) => setFindQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    handleFind(!e.shiftKey);
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    handleCloseFind();
                  }
                }}
                placeholder="Find in page (Ctrl+F)..."
                className="bg-transparent border-none text-xs text-white placeholder-slate-500 focus:outline-none w-44"
              />
              {findResult && findQuery && (
                <span className={`text-[10px] px-1.5 py-0.5 rounded font-bold ${
                  findResult.match_found
                    ? 'bg-cyan-950 text-cyan-300 border border-cyan-500/30'
                    : 'bg-red-950 text-red-300 border border-red-500/30'
                }`}>
                  {findResult.match_found
                    ? `${findResult.active_match_ordinal}/${findResult.matches_count}`
                    : '0/0'}
                </span>
              )}
            </div>

            {/* Previous & Next Buttons */}
            <button
              onClick={() => handleFind(false)}
              title="Previous Match (Shift+Enter)"
              disabled={!findQuery}
              className="p-1 rounded hover:bg-white/10 text-slate-300 hover:text-white disabled:opacity-30 transition"
            >
              <ChevronUp className="w-4 h-4" />
            </button>
            <button
              onClick={() => handleFind(true)}
              title="Next Match (Enter)"
              disabled={!findQuery}
              className="p-1 rounded hover:bg-white/10 text-slate-300 hover:text-white disabled:opacity-30 transition"
            >
              <ChevronDown className="w-4 h-4" />
            </button>

            {/* Case Sensitive Toggle */}
            <button
              onClick={() => setFindCaseSensitive(!findCaseSensitive)}
              title="Match Case"
              className={`px-1.5 py-0.5 rounded text-[10px] font-bold border transition ${
                findCaseSensitive
                  ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300'
                  : 'bg-white/5 border-transparent text-slate-400 hover:text-slate-200'
              }`}
            >
              Aa
            </button>

            <div className="w-[1px] h-4 bg-white/10 mx-0.5" />

            {/* Close Find HUD */}
            <button
              onClick={handleCloseFind}
              title="Close (Escape)"
              className="p-1 rounded hover:bg-red-950/60 hover:text-red-300 text-slate-400 transition"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        )}

        {/* Phase 5.6F-B Save Page Toast Notification */}
        {saveStatusToast && (
          <div className="absolute top-3 left-1/2 transform -translate-x-1/2 z-50 bg-[#06111f] border border-cyan-500/50 text-cyan-200 px-4 py-2 rounded-xl shadow-2xl backdrop-blur-md text-xs font-mono flex items-center gap-2 animate-fadeIn">
            <CheckCircle2 className="w-4 h-4 text-cyan-400" />
            <span>{saveStatusToast}</span>
          </div>
        )}

        {/* Phase 5.6F-B Isolated Reader Mode Surface */}
        {activeTab?.is_reader_mode && !isNewTab && (
          <div className={`absolute inset-0 z-30 overflow-y-auto flex flex-col transition-colors duration-200 ${
            readerTheme === 'sepia'
              ? 'bg-[#f4ecd8] text-[#5b4636]'
              : readerTheme === 'onyx'
              ? 'bg-black text-[#e0e0e0]'
              : readerTheme === 'light'
              ? 'bg-[#fafafa] text-[#222222]'
              : 'bg-[#060a14] text-[#d1d5db]'
          }`}>
            {/* Reader Header Toolbar */}
            <div className={`sticky top-0 z-20 px-4 py-2 flex flex-wrap items-center justify-between gap-3 border-b backdrop-blur-md font-mono text-xs ${
              readerTheme === 'sepia'
                ? 'bg-[#f4ecd8]/95 border-[#d3c2a6]'
                : readerTheme === 'onyx'
                ? 'bg-black/95 border-white/10'
                : readerTheme === 'light'
                ? 'bg-white/95 border-slate-200'
                : 'bg-[#080e1c]/95 border-cyan-500/30 text-cyan-300'
            }`}>
              <div className="flex items-center gap-2 truncate max-w-md">
                <BookOpen className="w-4 h-4 text-cyan-400 shrink-0" />
                <span className="font-bold truncate">{readerDocs[activeTab.id]?.title || activeTab.title}</span>
                {readerDocs[activeTab.id]?.reading_time_minutes && (
                  <span className="px-2 py-0.5 rounded-full text-[10px] bg-cyan-500/10 border border-cyan-500/30 text-cyan-400 shrink-0">
                    📖 {readerDocs[activeTab.id].reading_time_minutes} min read ({readerDocs[activeTab.id].word_count.toLocaleString()} words)
                  </span>
                )}
              </div>

              <div className="flex items-center gap-2">
                {/* Font Size Adjusters */}
                <div className="flex items-center gap-1 bg-black/20 rounded-lg p-0.5 border border-white/10 text-xs">
                  <button
                    onClick={() => setReaderFontSize((prev) => Math.max(13, prev - 1))}
                    className="px-1.5 py-0.5 rounded hover:bg-white/10 font-bold"
                    title="Smaller Text"
                  >
                    A-
                  </button>
                  <span className="px-1 text-[11px] font-bold">{readerFontSize}px</span>
                  <button
                    onClick={() => setReaderFontSize((prev) => Math.min(26, prev + 1))}
                    className="px-1.5 py-0.5 rounded hover:bg-white/10 font-bold"
                    title="Larger Text"
                  >
                    A+
                  </button>
                </div>

                {/* Line Width */}
                <div className="flex items-center gap-0.5 bg-black/20 rounded-lg p-0.5 border border-white/10 text-[11px]">
                  {(['narrow', 'normal', 'wide'] as const).map((w) => (
                    <button
                      key={w}
                      onClick={() => setReaderLineWidth(w)}
                      className={`px-1.5 py-0.5 rounded capitalize transition ${
                        readerLineWidth === w ? 'bg-cyan-500/30 text-cyan-300 font-bold border border-cyan-500/40' : 'hover:bg-white/10'
                      }`}
                    >
                      {w}
                    </button>
                  ))}
                </div>

                {/* Theme Switcher */}
                <div className="flex items-center gap-0.5 bg-black/20 rounded-lg p-0.5 border border-white/10 text-[11px]">
                  {[
                    { id: 'dark', label: 'Dark' },
                    { id: 'sepia', label: 'Sepia' },
                    { id: 'onyx', label: 'Onyx' },
                    { id: 'light', label: 'Light' },
                  ].map((t) => (
                    <button
                      key={t.id}
                      onClick={() => setReaderTheme(t.id as any)}
                      className={`px-1.5 py-0.5 rounded transition ${
                        readerTheme === t.id ? 'bg-cyan-500/30 text-cyan-300 font-bold border border-cyan-500/40' : 'hover:bg-white/10'
                      }`}
                    >
                      {t.label}
                    </button>
                  ))}
                </div>

                {/* Print & Save Buttons */}
                <button
                  onClick={handlePrint}
                  className="p-1 rounded-lg bg-black/20 border border-white/10 hover:border-cyan-500/40 hover:text-cyan-300 transition"
                  title="Print Reader Article"
                >
                  <Printer className="w-3.5 h-3.5" />
                </button>

                <button
                  onClick={() => handleSavePageHtml(activeTab.id)}
                  className="p-1 rounded-lg bg-black/20 border border-white/10 hover:border-cyan-500/40 hover:text-cyan-300 transition"
                  title="Save Clean Article HTML"
                >
                  <Download className="w-3.5 h-3.5" />
                </button>

                {/* Exit Reader Mode */}
                <button
                  onClick={() => handleToggleReaderMode(activeTab.id)}
                  className="flex items-center gap-1 px-2 py-0.5 rounded-lg bg-cyan-600/20 border border-cyan-500/40 hover:bg-cyan-600/40 text-cyan-300 transition font-bold text-xs"
                  title="Exit Reader Mode (Escape)"
                >
                  <X className="w-3.5 h-3.5" />
                  <span>Exit</span>
                </button>
              </div>
            </div>

            {/* Reader Article Body */}
            <div className={`mx-auto px-6 py-8 w-full transition-all duration-200 ${
              readerLineWidth === 'narrow'
                ? 'max-w-2xl'
                : readerLineWidth === 'wide'
                ? 'max-w-4xl'
                : 'max-w-3xl'
            }`} style={{ fontSize: `${readerFontSize}px`, lineHeight: 1.75 }}>
              {isExtractingReader ? (
                <div className="flex flex-col items-center justify-center py-20 gap-3">
                  <Loader2 className="w-8 h-8 text-cyan-400 animate-spin" />
                  <div className="text-sm font-mono text-cyan-300">Extracting clean article content...</div>
                </div>
              ) : (
                <article className="space-y-6">
                  <h1 className="text-3xl font-bold tracking-tight mb-2 leading-tight">
                    {readerDocs[activeTab.id]?.title || activeTab.title}
                  </h1>

                  {(readerDocs[activeTab.id]?.byline || readerDocs[activeTab.id]?.published_time) && (
                    <div className="flex flex-wrap items-center gap-3 text-xs font-mono opacity-75 border-b pb-3 mb-6">
                      {readerDocs[activeTab.id]?.byline && <span>By <strong className="opacity-100">{readerDocs[activeTab.id].byline}</strong></span>}
                      {readerDocs[activeTab.id]?.published_time && <span>• {readerDocs[activeTab.id].published_time}</span>}
                      <span>• <a href={activeTab.url} target="_blank" rel="noreferrer" className="underline hover:opacity-100">Original Source</a></span>
                    </div>
                  )}

                  {readerDocs[activeTab.id]?.excerpt && (
                    <div className={`p-4 rounded-xl border italic text-sm ${
                      readerTheme === 'sepia'
                        ? 'bg-[#ede3cc] border-[#d3c2a6]'
                        : readerTheme === 'light'
                        ? 'bg-slate-100 border-slate-200 text-slate-700'
                        : 'bg-cyan-950/20 border-cyan-500/20 text-cyan-200'
                    }`}>
                      {readerDocs[activeTab.id].excerpt}
                    </div>
                  )}

                  <div
                    className="reader-content space-y-4 font-serif leading-relaxed"
                    dangerouslySetInnerHTML={{ __html: readerDocs[activeTab.id]?.content_html || `<p>${readerDocs[activeTab.id]?.text_content || 'No article text extracted.'}</p>` }}
                  />

                  <div className="pt-8 mt-8 border-t text-xs font-mono opacity-60 flex items-center justify-between">
                    <span>E.D.I.T.H. Reader Mode</span>
                    <button
                      onClick={() => handleToggleReaderMode(activeTab.id)}
                      className="underline hover:opacity-100"
                    >
                      Return to original page →
                    </button>
                  </div>
                </article>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Phase 5.6F-C Create Tab Group Modal */}
      {showCreateGroupModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm animate-fadeIn">
          <div className="bg-[#070c18] border border-cyan-500/40 shadow-2xl rounded-2xl p-6 w-full max-w-md font-mono text-slate-200">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-bold text-cyan-300 flex items-center gap-2">
                <FolderPlus className="w-4 h-4 text-cyan-400" />
                Create Tab Group
              </h3>
              <button
                onClick={() => {
                  setShowCreateGroupModal(false);
                  setTargetTabForGroup(null);
                }}
                className="text-slate-400 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form onSubmit={handleCreateGroup} className="space-y-4">
              <div>
                <label className="block text-xs text-slate-400 mb-1.5">Group Name</label>
                <input
                  type="text"
                  value={newGroupName}
                  onChange={(e) => setNewGroupName(e.target.value)}
                  placeholder="e.g. Research, Project Alpha, Work..."
                  autoFocus
                  className="w-full bg-black/60 border border-cyan-500/30 rounded-xl px-3 py-2 text-xs text-white focus:outline-none focus:border-cyan-400"
                />
              </div>
              <div>
                <label className="block text-xs text-slate-400 mb-1.5">Group Color</label>
                <div className="flex items-center gap-2">
                  {(['blue', 'purple', 'green', 'yellow', 'orange', 'red', 'gray'] as const).map((color) => {
                    const cDef = GROUP_COLORS[color];
                    return (
                      <button
                        key={color}
                        type="button"
                        onClick={() => setNewGroupColor(color)}
                        className={`w-7 h-7 rounded-full flex items-center justify-center transition border-2 ${
                          newGroupColor === color ? 'border-white scale-110 shadow-lg' : 'border-transparent opacity-70 hover:opacity-100'
                        }`}
                        style={{ backgroundColor: cDef.dot }}
                        title={color}
                      >
                        {newGroupColor === color && <Check className="w-3.5 h-3.5 text-white" />}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="flex items-center justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => {
                    setShowCreateGroupModal(false);
                    setTargetTabForGroup(null);
                  }}
                  className="px-4 py-2 rounded-xl bg-white/5 hover:bg-white/10 text-xs text-slate-300 transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!newGroupName.trim()}
                  className="px-4 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 text-xs font-bold text-white transition shadow-cyan-glow-xs"
                >
                  Create Group
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Phase 5.6F-C Edit Tab Group Modal */}
      {editingGroupId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm animate-fadeIn">
          <div className="bg-[#070c18] border border-cyan-500/40 shadow-2xl rounded-2xl p-6 w-full max-w-md font-mono text-slate-200">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-bold text-cyan-300 flex items-center gap-2">
                <Edit2 className="w-4 h-4 text-cyan-400" />
                Edit Tab Group
              </h3>
              <button
                onClick={() => {
                  setEditingGroupId(null);
                  setEditingGroupName('');
                }}
                className="text-slate-400 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form onSubmit={handleRenameGroup} className="space-y-4">
              <div>
                <label className="block text-xs text-slate-400 mb-1.5">Group Name</label>
                <input
                  type="text"
                  value={editingGroupName}
                  onChange={(e) => setEditingGroupName(e.target.value)}
                  placeholder="Group name..."
                  autoFocus
                  className="w-full bg-black/60 border border-cyan-500/30 rounded-xl px-3 py-2 text-xs text-white focus:outline-none focus:border-cyan-400"
                />
              </div>
              <div>
                <label className="block text-xs text-slate-400 mb-1.5">Group Color</label>
                <div className="flex items-center gap-2">
                  {(['blue', 'purple', 'green', 'yellow', 'orange', 'red', 'gray'] as const).map((color) => {
                    const cDef = GROUP_COLORS[color];
                    return (
                      <button
                        key={color}
                        type="button"
                        onClick={() => setEditingGroupColor(color)}
                        className={`w-7 h-7 rounded-full flex items-center justify-center transition border-2 ${
                          editingGroupColor === color ? 'border-white scale-110 shadow-lg' : 'border-transparent opacity-70 hover:opacity-100'
                        }`}
                        style={{ backgroundColor: cDef.dot }}
                        title={color}
                      >
                        {editingGroupColor === color && <Check className="w-3.5 h-3.5 text-white" />}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="flex items-center justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => {
                    setEditingGroupId(null);
                    setEditingGroupName('');
                  }}
                  className="px-4 py-2 rounded-xl bg-white/5 hover:bg-white/10 text-xs text-slate-300 transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!editingGroupName.trim()}
                  className="px-4 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 text-xs font-bold text-white transition shadow-cyan-glow-xs"
                >
                  Save Changes
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Phase 5.6F-C Tab Search HUD Modal (Ctrl+Shift+A) */}
      {showTabSearchModal && (
        <div
          className="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/60 backdrop-blur-sm animate-fadeIn"
          onClick={() => setShowTabSearchModal(false)}
        >
          <div
            className="bg-[#070c18] border border-cyan-500/40 shadow-2xl rounded-2xl p-4 w-full max-w-xl font-mono text-slate-200"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2 px-3 py-2 bg-black/60 border border-cyan-500/30 rounded-xl mb-3">
              <Compass className="w-4 h-4 text-cyan-400 shrink-0" />
              <input
                ref={tabSearchInputRef}
                type="text"
                value={tabSearchQuery}
                onChange={(e) => {
                  setTabSearchQuery(e.target.value);
                  setTabSearchSelectedIndex(0);
                }}
                onKeyDown={(e) => {
                  const filtered = browserState.tabs.filter((t) => {
                    const q = tabSearchQuery.toLowerCase();
                    const groupName = tabGroups.find((g) => g.id === t.group_id)?.name.toLowerCase() || '';
                    return (
                      (t.title && t.title.toLowerCase().includes(q)) ||
                      (t.url && t.url.toLowerCase().includes(q)) ||
                      groupName.includes(q)
                    );
                  });

                  if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    setTabSearchSelectedIndex((prev) => (prev + 1) % (filtered.length || 1));
                  } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    setTabSearchSelectedIndex((prev) => (prev - 1 + (filtered.length || 1)) % (filtered.length || 1));
                  } else if (e.key === 'Enter') {
                    e.preventDefault();
                    if (filtered[tabSearchSelectedIndex]) {
                      handleSwitchTab(filtered[tabSearchSelectedIndex].id);
                      setShowTabSearchModal(false);
                    }
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    setShowTabSearchModal(false);
                  }
                }}
                placeholder="Search tabs by title, URL, or group (Ctrl+Shift+A)..."
                autoFocus
                className="w-full bg-transparent border-none text-xs text-white placeholder-slate-500 focus:outline-none"
              />
              <span className="text-[10px] text-slate-500 px-1.5 py-0.5 rounded bg-white/5">ESC</span>
            </div>

            {/* Results List */}
            {(() => {
              const filtered = browserState.tabs.filter((t) => {
                const q = tabSearchQuery.toLowerCase();
                const groupName = tabGroups.find((g) => g.id === t.group_id)?.name.toLowerCase() || '';
                return (
                  (t.title && t.title.toLowerCase().includes(q)) ||
                  (t.url && t.url.toLowerCase().includes(q)) ||
                  groupName.includes(q)
                );
              });

              if (filtered.length === 0) {
                return (
                  <div className="py-8 text-center text-xs text-slate-500">
                    No open tabs match &quot;{tabSearchQuery}&quot;
                  </div>
                );
              }

              return (
                <div className="max-h-72 overflow-y-auto space-y-1 pr-1 custom-scrollbar">
                  {filtered.map((tab, idx) => {
                    const isSelected = idx === tabSearchSelectedIndex;
                    const isActive = tab.id === browserState.active_tab_id;
                    const group = tabGroups.find((g) => g.id === tab.group_id);
                    const cDef = group ? GROUP_COLORS[group.color] || GROUP_COLORS.blue : null;

                    return (
                      <div
                        key={tab.id}
                        onClick={() => {
                          handleSwitchTab(tab.id);
                          setShowTabSearchModal(false);
                        }}
                        onMouseEnter={() => setTabSearchSelectedIndex(idx)}
                        className={`flex items-center justify-between p-2.5 rounded-xl cursor-pointer transition select-none ${
                          isSelected
                            ? 'bg-cyan-950/60 border border-cyan-500/40 text-cyan-200'
                            : 'hover:bg-white/5 border border-transparent text-slate-300'
                        }`}
                      >
                        <div className="flex items-center gap-2.5 min-w-0 flex-1">
                          {tab.favicon ? (
                            <img
                              src={tab.favicon}
                              alt=""
                              className="w-4 h-4 shrink-0 rounded-sm"
                              onError={(e) => {
                                (e.target as HTMLElement).style.display = 'none';
                              }}
                            />
                          ) : (
                            <Globe className="w-4 h-4 shrink-0 text-slate-400" />
                          )}
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-1.5">
                              <span className="text-xs font-bold truncate">
                                {tab.title || tab.url || 'New Tab'}
                              </span>
                              {isActive && (
                                <span className="px-1.5 py-0.2 text-[9px] font-bold rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 shrink-0">
                                  CURRENT
                                </span>
                              )}
                              {tab.is_pinned && (
                                <span className="px-1.5 py-0.2 text-[9px] font-bold rounded bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shrink-0">
                                  PINNED
                                </span>
                              )}
                            </div>
                            <div className="text-[10px] text-slate-500 truncate mt-0.5">
                              {tab.url}
                            </div>
                          </div>
                        </div>

                        {/* Badges on Right */}
                        <div className="flex items-center gap-1.5 shrink-0 ml-2">
                          {group && cDef && (
                            <span
                              className={`flex items-center gap-1 px-2 py-0.5 rounded-lg text-[9px] font-bold border ${cDef.bg} ${cDef.border} ${cDef.text}`}
                            >
                              <span className="w-1.5 h-1.5 rounded-full" style={{ backgroundColor: cDef.dot }} />
                              {group.name}
                            </span>
                          )}
                          {tab.profile_id && tab.profile_id !== 'profile_default' && (
                            <span className="px-1.5 py-0.5 rounded bg-slate-900 border border-slate-700 text-slate-300 text-[9px] font-mono">
                              {tab.profile_id.startsWith('agent_') ? 'AI' : tab.profile_id.replace('profile_', '')}
                            </span>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              );
            })()}
          </div>
        </div>
      )}

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
          <span className="text-slate-500">Shortcuts: Ctrl+T, Ctrl+W, Ctrl+Shift+T, Ctrl+Shift+A, Ctrl+Tab, Ctrl+L</span>
          <span className="text-slate-600">|</span>
          <span className="text-purple-400/80">Phase 4C Autonomous Agent Loop</span>
        </div>
      </div>
    </div>
  );
};

export default BrowserView;
