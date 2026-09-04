import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  RotateCw,
  Search,
  Globe,
  Lock,
  Plus,
  X,
  Check,
  CheckCircle2,
  AlertTriangle,
  Star,
  Bookmark,
  History,
  Trash2,
  Download,
  Folder,
  ExternalLink,
  FileText,
  XCircle,
  Users,
  User,
  Edit2,
  Pin,
  PinOff,
  Copy,
  Terminal,
  Cpu,
  Printer,
  ZoomIn,
  ZoomOut,
  ChevronUp,
  ChevronDown,
  BookOpen,
  FileDown,
  ChevronRight,
  Shield,
  ShieldCheck,
  MoreVertical,
  Layers,
  Settings,
  Code2,
  FolderPlus,
  Loader2,
} from 'lucide-react';
import { browserController, SEARCH_ENGINES } from '../services/browserController';
import { useApp } from '../context/AppContext';
import { isTauri } from '../services/tauri';
import { listen } from '@tauri-apps/api/event';
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import type {
  SearchEngineId,
  SearchEngineConfig,
  BrowserTabInfo,
  BrowserMultiStateInfo,
  BrowserViewportBounds,
  BrowserHistoryEntry,
  BrowserBookmark,
  BrowserDownload,
  BrowserProfile,
  PrivacyStatus,
  FindResult,
  ReaderDocument,
  BrowserTabGroup,
} from '../types';

export const BrowserView: React.FC = () => {
  const { isTelemetryOpen } = useApp();
  const [browserState, setBrowserState] = useState<BrowserMultiStateInfo>({
    tabs: [],
    active_tab_id: null,
    is_visible: false,
  });
  const [inputUrl, setInputUrl] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isOmniboxFocused, setIsOmniboxFocused] = useState(false);

  // Search Engine state (Google by default, persistent)
  const [activeEngineId, setActiveEngineId] = useState<SearchEngineId>(() => browserController.getSearchEngineId());
  const activeSearchEngine = useMemo(() => (SEARCH_ENGINES as Record<string, SearchEngineConfig>)[activeEngineId] || SEARCH_ENGINES.google, [activeEngineId]);

  // History, Bookmarks, Downloads, Profiles, Privacy states
  const [historyList, setHistoryList] = useState<BrowserHistoryEntry[]>([]);
  const [isFetchingHistory, setIsFetchingHistory] = useState(false);
  const [historySearchQuery, setHistorySearchQuery] = useState('');

  const [bookmarksList, setBookmarksList] = useState<BrowserBookmark[]>([]);
  const [isFetchingBookmarks, setIsFetchingBookmarks] = useState(false);
  const [bookmarksSearchQuery, setBookmarksSearchQuery] = useState('');
  const [showAddBookmarkModal, setShowAddBookmarkModal] = useState(false);
  const [newBookmarkTitle, setNewBookmarkTitle] = useState('');
  const [newBookmarkUrl, setNewBookmarkUrl] = useState('');

  const [downloadsList, setDownloadsList] = useState<BrowserDownload[]>([]);
  const [isFetchingDownloads, setIsFetchingDownloads] = useState(false);

  const [profilesList, setProfilesList] = useState<BrowserProfile[]>([]);
  const [isFetchingProfiles, setIsFetchingProfiles] = useState(false);

  const [privacyStatus, setPrivacyStatus] = useState<PrivacyStatus | null>(null);

  // Utilities: Find in Page, Reader Mode, Zoom, Save
  const [showFindHud, setShowFindHud] = useState(false);
  const [findQuery, setFindQuery] = useState('');
  const [findResult, setFindResult] = useState<FindResult | null>(null);
  const [findCaseSensitive, setFindCaseSensitive] = useState(false);
  const findInputRef = useRef<HTMLInputElement>(null);

  const [readerDocs, setReaderDocs] = useState<Record<string, ReaderDocument>>({});
  const [isExtractingReader, setIsExtractingReader] = useState(false);
  const [readerFontSize, setReaderFontSize] = useState<number>(18);
  const [readerLineWidth, setReaderLineWidth] = useState<'narrow' | 'normal' | 'wide'>('normal');
  const [readerTheme, setReaderTheme] = useState<'dark' | 'sepia' | 'onyx' | 'light'>('dark');
  const [saveStatusToast, setSaveStatusToast] = useState<string | null>(null);

  // Tab Groups
  const [tabGroups, setTabGroups] = useState<BrowserTabGroup[]>([]);
  const [showCreateGroupModal, setShowCreateGroupModal] = useState(false);
  const [targetTabForGroup, setTargetTabForGroup] = useState<string | null>(null);
  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupColor, setNewGroupColor] = useState('blue');
  const [groupContextMenu, setGroupContextMenu] = useState<{ groupId: string; x: number; y: number } | null>(null);

  // New Tab search
  const [newTabSearchQuery, setNewTabSearchQuery] = useState('');

  // Context Menu fallback for non-Tauri
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; tabId: string } | null>(null);

  // Viewport & Omnibox Refs
  const viewportRef = useRef<HTMLDivElement>(null);
  const omniboxInputRef = useRef<HTMLInputElement>(null);
  const isSyncingRef = useRef(false);
  const pendingBoundsRef = useRef<BrowserViewportBounds | null>(null);
  const rafIdRef = useRef<number | null>(null);

  const activeTab = useMemo(
    () => browserState.tabs.find((t) => t.id === browserState.active_tab_id),
    [browserState.tabs, browserState.active_tab_id]
  );

  const activeProfileName = useMemo(() => {
    const active = profilesList.find((p) => p.is_active);
    return active ? active.name : activeTab?.profile_id || 'Default';
  }, [profilesList, activeTab?.profile_id]);

  const activeDownloadsCount = useMemo(
    () => downloadsList.filter((d) => d.status === 'DOWNLOADING').length,
    [downloadsList]
  );

  const isActiveTabBookmarked = useMemo(() => {
    if (!activeTab || !activeTab.url || activeTab.url.startsWith('edith://')) return false;
    return bookmarksList.some((bm) => bm.url === activeTab.url);
  }, [activeTab, bookmarksList]);

  // Determine internal tab pages
  const isNewTab = !activeTab || !activeTab.url || activeTab.url === 'edith://newtab' || activeTab.url === 'about:blank';
  const isHistoryPage = activeTab?.url === 'edith://history';
  const isBookmarksPage = activeTab?.url === 'edith://bookmarks';
  const isDownloadsPage = activeTab?.url === 'edith://downloads';
  const isSettingsPage = activeTab?.url === 'edith://settings';
  const isInternalPage = isNewTab || isHistoryPage || isBookmarksPage || isDownloadsPage || isSettingsPage;

  // --- Data Fetching Callbacks ---
  const fetchHistory = useCallback(async (query?: string) => {
    setIsFetchingHistory(true);
    try {
      const res = query && query.trim()
        ? await browserController.searchHistory(query.trim())
        : await browserController.getHistory(50);
      setHistoryList(res);
    } catch (e) {
      console.warn('Failed to fetch history', e);
    } finally {
      setIsFetchingHistory(false);
    }
  }, []);

  const fetchBookmarks = useCallback(async (query?: string) => {
    setIsFetchingBookmarks(true);
    try {
      const res = query && query.trim()
        ? await browserController.searchBookmarks(query.trim())
        : await browserController.getBookmarks();
      setBookmarksList(res);
    } catch (e) {
      console.warn('Failed to fetch bookmarks', e);
    } finally {
      setIsFetchingBookmarks(false);
    }
  }, []);

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

  const fetchPrivacyStatus = useCallback(async (tabId?: string) => {
    try {
      const status = await browserController.getPrivacyStatus(tabId);
      setPrivacyStatus(status);
    } catch (e) {
      console.warn('Failed to fetch privacy status', e);
    }
  }, []);

  const fetchTabGroups = useCallback(async (profileId?: string) => {
    try {
      const groups = await browserController.listTabGroups(profileId);
      setTabGroups(groups);
    } catch (e) {
      console.warn('Failed to fetch tab groups:', e);
    }
  }, []);

  // --- Viewport Bounds Synchronization ---
  const performSync = useCallback(async (bounds: BrowserViewportBounds) => {
    if (isSyncingRef.current) {
      pendingBoundsRef.current = bounds;
      return;
    }
    isSyncingRef.current = true;
    try {
      await browserController.setBoundsAll(bounds);
    } catch (e) {
      console.warn('Failed to sync browser bounds:', e);
    } finally {
      isSyncingRef.current = false;
      if (pendingBoundsRef.current) {
        const next = pendingBoundsRef.current;
        pendingBoundsRef.current = null;
        performSync(next);
      }
    }
  }, []);

  const syncBounds = useCallback(() => {
    if (rafIdRef.current !== null) {
      cancelAnimationFrame(rafIdRef.current);
    }
    rafIdRef.current = requestAnimationFrame(() => {
      rafIdRef.current = null;
      if (!viewportRef.current) return;
      const rect = viewportRef.current.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        const bounds: BrowserViewportBounds = {
          x: Math.round(rect.left),
          y: Math.round(rect.top),
          width: Math.round(rect.width),
          height: Math.round(rect.height),
        };
        performSync(bounds);
      }
    });
  }, [performSync]);

  // Synchronize on mount, telemetry change, and window resize
  useEffect(() => {
    syncBounds();
    const t1 = setTimeout(() => syncBounds(), 150);
    const t2 = setTimeout(() => syncBounds(), 350);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  }, [isTelemetryOpen, syncBounds]);

  // Subscribe to BrowserController state changes
  useEffect(() => {
    const unsub = browserController.subscribe((s) => {
      setBrowserState(s);
      const active = s.tabs.find((t) => t.id === s.active_tab_id);
      setIsLoading(active?.is_loading || false);
      setActiveEngineId(browserController.getSearchEngineId());
    });

    // Initial data fetch
    fetchHistory();
    fetchBookmarks();
    fetchDownloads();
    fetchProfiles();
    fetchTabGroups();

    // Initial tab creation if none exist
    const initTabs = async () => {
      const current = browserController.getState();
      if (current.tabs.length === 0) {
        let b;
        if (viewportRef.current) {
          const rect = viewportRef.current.getBoundingClientRect();
          b = { x: Math.round(rect.left), y: Math.round(rect.top), width: Math.round(rect.width), height: Math.round(rect.height) };
        }
        await browserController.createTab('tab_1', 'edith://newtab', b);
      }
    };
    initTabs();

    const ro = new ResizeObserver(() => syncBounds());
    if (viewportRef.current) ro.observe(viewportRef.current);
    window.addEventListener('resize', syncBounds);

    return () => {
      unsub();
      ro.disconnect();
      window.removeEventListener('resize', syncBounds);
    };
  }, [fetchHistory, fetchBookmarks, fetchDownloads, fetchProfiles, fetchTabGroups, syncBounds]);

  // Synchronize Omnibox URL with active tab
  useEffect(() => {
    if (!isOmniboxFocused && activeTab) {
      if (activeTab.url === 'edith://newtab' || !activeTab.url) {
        setInputUrl('');
      } else {
        setInputUrl(activeTab.url);
      }
    }
  }, [activeTab?.url, activeTab?.id, isOmniboxFocused]);

  // --- Search Engine Selection Handler ---
  const handleSetSearchEngine = useCallback((id: SearchEngineId) => {
    browserController.setSearchEngineId(id);
    setActiveEngineId(id);
  }, []);

  // --- Navigation Handlers ---
  const handleNavigate = useCallback(async (e?: React.FormEvent, targetUrl?: string) => {
    if (e) e.preventDefault();
    const raw = targetUrl || inputUrl;
    if (!raw || !raw.trim()) return;

    const activeId = browserState.active_tab_id;
    if (!activeId) return;

    setIsLoading(true);
    try {
      await browserController.navigateTab(activeId, raw.trim());
      fetchHistory();
    } catch (err) {
      console.error('Navigation error:', err);
    } finally {
      setIsLoading(false);
      omniboxInputRef.current?.blur();
    }
  }, [inputUrl, browserState.active_tab_id, fetchHistory]);

  const handleCreateNewTab = useCallback(async (url = 'edith://newtab') => {
    try {
      const newId = `tab_${Date.now()}`;
      let b;
      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        b = { x: Math.round(rect.left), y: Math.round(rect.top), width: Math.round(rect.width), height: Math.round(rect.height) };
      }
      await browserController.createTab(newId, url, b);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to create tab:', err);
    }
  }, []);

  const handleCloseTab = useCallback(async (e?: React.MouseEvent, tabId?: string) => {
    if (e) e.stopPropagation();
    const targetId = tabId || browserState.active_tab_id;
    if (!targetId) return;
    try {
      await browserController.closeTab(targetId);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to close tab:', err);
    }
  }, [browserState.active_tab_id]);

  const handleSwitchTab = useCallback(async (tabId: string) => {
    if (tabId === browserState.active_tab_id) return;
    try {
      let b;
      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        b = { x: Math.round(rect.left), y: Math.round(rect.top), width: Math.round(rect.width), height: Math.round(rect.height) };
      }
      await browserController.switchTab(tabId, b);
      fetchPrivacyStatus(tabId);
    } catch (err) {
      console.error('Failed to switch tab:', err);
    }
  }, [browserState.active_tab_id, fetchPrivacyStatus]);

  const handleDuplicateTab = useCallback(async (tabId: string) => {
    try {
      await browserController.duplicateTab(tabId);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to duplicate tab:', err);
    }
  }, []);

  const handleTogglePinTab = useCallback(async (tabId: string) => {
    try {
      await browserController.togglePinTab(tabId);
    } catch (err) {
      console.error('Failed to pin tab:', err);
    }
  }, []);

  const handleCloseOtherTabs = useCallback(async (tabId: string) => {
    try {
      await browserController.closeOtherTabs(tabId);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to close other tabs:', err);
    }
  }, []);

  const handleCloseTabsToRight = useCallback(async (tabId: string) => {
    try {
      await browserController.closeTabsToRight(tabId);
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to close tabs to right:', err);
    }
  }, []);

  const handleReopenTab = useCallback(async () => {
    try {
      await browserController.reopenLastClosedTab();
      browserController.saveSession();
    } catch (err) {
      console.error('Failed to reopen closed tab:', err);
    }
  }, []);

  const handleToggleBookmarkActiveTab = useCallback(async () => {
    if (!activeTab || !activeTab.url || activeTab.url.startsWith('edith://')) return;
    try {
      const existing = bookmarksList.find((b) => b.url === activeTab.url);
      if (existing) {
        await browserController.deleteBookmark(existing.id);
      } else {
        await browserController.addBookmark(activeTab.title || activeTab.url, activeTab.url);
      }
      fetchBookmarks();
    } catch (err) {
      console.error('Failed to toggle bookmark:', err);
    }
  }, [activeTab, bookmarksList, fetchBookmarks]);

  const handlePrint = useCallback(async () => {
    if (!browserState.active_tab_id) return;
    try {
      await browserController.printTab(browserState.active_tab_id);
    } catch (e) {
      console.error('Print failed:', e);
    }
  }, [browserState.active_tab_id]);

  const handleZoomIn = useCallback(async () => {
    if (!browserState.active_tab_id) return;
    await browserController.zoomIn(browserState.active_tab_id);
  }, [browserState.active_tab_id]);

  const handleZoomOut = useCallback(async () => {
    if (!browserState.active_tab_id) return;
    await browserController.zoomOut(browserState.active_tab_id);
  }, [browserState.active_tab_id]);

  const handleZoomReset = useCallback(async () => {
    if (!browserState.active_tab_id) return;
    await browserController.zoomReset(browserState.active_tab_id);
  }, [browserState.active_tab_id]);

  const handleToggleReaderMode = useCallback(async (tabId?: string) => {
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
  }, [browserState.tabs, browserState.active_tab_id]);

  const handleSavePageHtml = useCallback(async (tabId?: string) => {
    const targetId = tabId || browserState.active_tab_id;
    if (!targetId) return;
    try {
      const path = await browserController.savePageHtml(targetId);
      setSaveStatusToast(`Saved: ${path}`);
      setTimeout(() => setSaveStatusToast(null), 4000);
      fetchDownloads();
    } catch (e) {
      console.error('Failed to save page:', e);
    }
  }, [browserState.active_tab_id, fetchDownloads]);

  const handleOpenFind = useCallback(() => {
    setShowFindHud(true);
    setTimeout(() => {
      findInputRef.current?.focus();
      findInputRef.current?.select();
    }, 50);
  }, []);

  const handleCloseFind = useCallback(async () => {
    setShowFindHud(false);
    setFindQuery('');
    setFindResult(null);
    if (browserState.active_tab_id) {
      await browserController.clearFind(browserState.active_tab_id);
    }
  }, [browserState.active_tab_id]);

  const handleFind = useCallback(async (forward = true) => {
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
  }, [browserState.active_tab_id, findQuery, findCaseSensitive]);

  // --- Tier A & Tier B Native Menus (Zero Airspace Occlusion & Zero Webpage Shifting) ---

  // 1. Tab Context Menu
  const handleTabContextMenu = useCallback(async (e: React.MouseEvent, tabId: string) => {
    e.preventDefault();
    e.stopPropagation();

    if (isTauri()) {
      try {
        const targetTab = browserState.tabs.find((t) => t.id === tabId);
        const items: any[] = [
          await MenuItem.new({
            text: 'New Tab (Ctrl+T)',
            action: () => handleCreateNewTab('edith://newtab'),
          }),
          await MenuItem.new({
            text: 'Reload (Ctrl+R)',
            action: () => browserController.reload(),
          }),
          await MenuItem.new({
            text: 'Duplicate Tab',
            action: () => handleDuplicateTab(tabId),
          }),
          await MenuItem.new({
            text: targetTab?.is_pinned ? 'Unpin Tab' : 'Pin Tab',
            action: () => handleTogglePinTab(tabId),
          }),
          await PredefinedMenuItem.new({ item: 'Separator' }),
          await MenuItem.new({
            text: 'Copy Tab URL',
            action: () => {
              if (targetTab?.url) navigator.clipboard.writeText(targetTab.url);
            },
          }),
          await MenuItem.new({
            text: 'Find in Page... (Ctrl+F)',
            action: () => handleOpenFind(),
          }),
          await MenuItem.new({
            text: 'Print Tab... (Ctrl+P)',
            action: () => handlePrint(),
          }),
          await MenuItem.new({
            text: 'Toggle Reader Mode (Ctrl+Shift+R)',
            action: () => handleToggleReaderMode(tabId),
          }),
          await MenuItem.new({
            text: 'Save Page HTML...',
            action: () => handleSavePageHtml(tabId),
          }),
          await PredefinedMenuItem.new({ item: 'Separator' }),
          await MenuItem.new({
            text: 'Close Tab (Ctrl+W)',
            action: () => handleCloseTab(undefined, tabId),
          }),
          await MenuItem.new({
            text: 'Close Other Tabs',
            action: () => handleCloseOtherTabs(tabId),
          }),
          await MenuItem.new({
            text: 'Close Tabs to Right',
            action: () => handleCloseTabsToRight(tabId),
          }),
          await MenuItem.new({
            text: 'Reopen Closed Tab (Ctrl+Shift+T)',
            action: () => handleReopenTab(),
          }),
        ];

        const menu = await Menu.new({ items });
        await menu.popup();
        return;
      } catch (err) {
        console.warn('Native context menu failed:', err);
      }
    }

    setContextMenu({ x: e.clientX, y: e.clientY, tabId });
  }, [browserState.tabs, handleCreateNewTab, handleDuplicateTab, handleTogglePinTab, handleOpenFind, handlePrint, handleToggleReaderMode, handleSavePageHtml, handleCloseTab, handleCloseOtherTabs, handleCloseTabsToRight, handleReopenTab]);

  // 2. History Quick Flyout (Native Tier B Menu)
  const handleHistoryMenu = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!isTauri()) {
      handleNavigate(undefined, 'edith://history');
      return;
    }

    try {
      const recent = historyList.slice(0, 15);
      const items: any[] = [
        await MenuItem.new({
          text: '🔍 Open History Manager (Ctrl+H)',
          action: () => handleNavigate(undefined, 'edith://history'),
        }),
        await MenuItem.new({
          text: '🗑️ Clear All Browsing History...',
          action: async () => {
            if (confirm('Clear all browsing history?')) {
              await browserController.clearHistory();
              fetchHistory();
            }
          },
        }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
      ];

      if (recent.length === 0) {
        items.push(
          await MenuItem.new({
            text: '(No recent history)',
            enabled: false,
          })
        );
      } else {
        for (const item of recent) {
          const display = (item.title || item.url).slice(0, 48);
          items.push(
            await MenuItem.new({
              text: display,
              action: () => handleNavigate(undefined, item.url),
            })
          );
        }
      }

      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (err) {
      console.warn('Failed to open history native menu:', err);
      handleNavigate(undefined, 'edith://history');
    }
  }, [historyList, handleNavigate, fetchHistory]);

  // 3. Bookmarks Quick Flyout (Native Tier B Menu)
  const handleBookmarksMenu = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!isTauri()) {
      handleNavigate(undefined, 'edith://bookmarks');
      return;
    }

    try {
      const items: any[] = [
        await MenuItem.new({
          text: isActiveTabBookmarked ? '⭐ Remove Bookmark (Ctrl+D)' : '☆ Bookmark Current Tab (Ctrl+D)',
          action: () => handleToggleBookmarkActiveTab(),
        }),
        await MenuItem.new({
          text: '📁 Open Bookmark Manager (Ctrl+Shift+O)',
          action: () => handleNavigate(undefined, 'edith://bookmarks'),
        }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
      ];

      const recents = bookmarksList.slice(0, 20);
      if (recents.length === 0) {
        items.push(
          await MenuItem.new({
            text: '(No saved bookmarks)',
            enabled: false,
          })
        );
      } else {
        for (const bm of recents) {
          const display = (bm.title || bm.url).slice(0, 48);
          items.push(
            await MenuItem.new({
              text: display,
              action: () => handleNavigate(undefined, bm.url),
            })
          );
        }
      }

      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (err) {
      console.warn('Failed to open bookmarks native menu:', err);
      handleNavigate(undefined, 'edith://bookmarks');
    }
  }, [isActiveTabBookmarked, bookmarksList, handleToggleBookmarkActiveTab, handleNavigate]);

  // 4. Downloads Quick Flyout (Native Tier B Menu)
  const handleDownloadsMenu = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!isTauri()) {
      handleNavigate(undefined, 'edith://downloads');
      return;
    }

    try {
      const items: any[] = [
        await MenuItem.new({
          text: '📂 Open Downloads Folder',
          action: async () => {
            if (downloadsList.length > 0) {
              await browserController.showDownloadInFolder(downloadsList[0].id);
            } else {
              handleNavigate(undefined, 'edith://downloads');
            }
          },
        }),
        await MenuItem.new({
          text: '🗑️ Clear Download Records',
          action: async () => {
            await browserController.clearDownloads();
            fetchDownloads();
          },
        }),
        await MenuItem.new({
          text: '⬇️ Open Downloads Manager (Ctrl+J)',
          action: () => handleNavigate(undefined, 'edith://downloads'),
        }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
      ];

      const recents = downloadsList.slice(0, 10);
      if (recents.length === 0) {
        items.push(
          await MenuItem.new({
            text: '(No recent downloads)',
            enabled: false,
          })
        );
      } else {
        for (const dl of recents) {
          const statusText = dl.status === 'DOWNLOADING' ? `[${Math.round(dl.progress * 100)}%]` : `[${dl.status}]`;
          items.push(
            await MenuItem.new({
              text: `${statusText} ${dl.filename.slice(0, 36)}`,
              action: () => browserController.openDownloadedFile(dl.id),
            })
          );
        }
      }

      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (err) {
      console.warn('Failed to open downloads native menu:', err);
      handleNavigate(undefined, 'edith://downloads');
    }
  }, [downloadsList, handleNavigate, fetchDownloads]);

  // 5. Profiles Quick Flyout (Native Tier B Menu)
  const handleProfilesMenu = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!isTauri()) return;

    try {
      const items: any[] = [
        await MenuItem.new({
          text: `Active Profile: ${activeProfileName}`,
          enabled: false,
        }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
      ];

      for (const p of profilesList) {
        const isCurrent = p.is_active || p.name === activeProfileName;
        items.push(
          await MenuItem.new({
            text: isCurrent ? `✓ ${p.name}` : `   ${p.name}`,
            action: async () => {
              await browserController.switchProfile(p.id);
              fetchProfiles();
            },
          })
        );
      }

      items.push(
        await PredefinedMenuItem.new({ item: 'Separator' }),
        await MenuItem.new({
          text: '➕ Create New Profile...',
          action: async () => {
            const name = prompt('Enter profile name:');
            if (name && name.trim()) {
              await browserController.createProfile(name.trim(), 'STANDARD');
              fetchProfiles();
            }
          },
        })
      );

      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (err) {
      console.warn('Failed to open profiles native menu:', err);
    }
  }, [activeProfileName, profilesList, fetchProfiles]);

  // 6. Security / Lock Icon Quick Menu
  const handleSecurityMenu = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!isTauri()) return;

    try {
      const isHttps = activeTab?.url?.toLowerCase().startsWith('https://');
      const isHttp = activeTab?.url?.toLowerCase().startsWith('http://');
      const domain = activeTab?.url ? new URL(activeTab.url).hostname : 'local';

      const items: any[] = [
        await MenuItem.new({
          text: isHttps ? '🔒 Connection is Secure (HTTPS)' : isHttp ? '⚠️ Connection is Not Secure (HTTP)' : 'ℹ️ Internal Browser Page',
          enabled: false,
        }),
        await MenuItem.new({
          text: `Host: ${domain}`,
          enabled: false,
        }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
        await MenuItem.new({
          text: privacyStatus?.enabled !== false ? '🛡️ Tracker Blocker: Active (✓)' : '🛡️ Tracker Blocker: Paused',
          action: async () => {
            const next = privacyStatus?.enabled === false;
            await browserController.togglePrivacyProtection(next);
            fetchPrivacyStatus(activeTab?.id);
          },
        }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
        await MenuItem.new({
          text: `🛡️ Allowlist Domain: ${domain}`,
          action: async () => {
            await browserController.allowlistDomain(domain);
            fetchPrivacyStatus(activeTab?.id);
          },
        }),
      ];

      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (err) {
      console.warn('Failed to open security native menu:', err);
    }
  }, [activeTab, privacyStatus, fetchPrivacyStatus]);

  // 7. Overflow / More Menu (Native Tier A Menu)
  const handleOverflowMenu = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    if (!isTauri()) return;

    try {
      // Build search engine submenu items
      const searchItems: any[] = [];
      const currentEngine = browserController.getSearchEngineId();
      for (const [key, engine] of Object.entries(SEARCH_ENGINES)) {
        const isCurrent = key === currentEngine;
        searchItems.push(
          await MenuItem.new({
            text: isCurrent ? `✓ ${engine.name}` : `   ${engine.name}`,
            action: () => handleSetSearchEngine(key as SearchEngineId),
          })
        );
      }

      const searchSubmenu = await Submenu.new({
        text: `Search Engine (${SEARCH_ENGINES[currentEngine]?.name || 'Google'})`,
        items: searchItems,
      });

      const currentZoom = Math.round((activeTab?.zoom_level || 1.0) * 100);
      const zoomSubmenu = await Submenu.new({
        text: `Zoom (${currentZoom}%)`,
        items: [
          await MenuItem.new({ text: 'Zoom In (Ctrl++)', action: () => handleZoomIn() }),
          await MenuItem.new({ text: 'Zoom Out (Ctrl+-)', action: () => handleZoomOut() }),
          await MenuItem.new({ text: 'Reset Zoom (Ctrl+0)', action: () => handleZoomReset() }),
        ],
      });

      const items: any[] = [
        await MenuItem.new({ text: 'New Tab (Ctrl+T)', action: () => handleCreateNewTab('edith://newtab') }),
        await MenuItem.new({ text: 'Reload (Ctrl+R)', action: () => browserController.reload() }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
        zoomSubmenu,
        await MenuItem.new({ text: 'Print Page... (Ctrl+P)', action: () => handlePrint() }),
        await MenuItem.new({ text: 'Find in Page... (Ctrl+F)', action: () => handleOpenFind() }),
        await MenuItem.new({ text: 'Toggle Reader Mode (Ctrl+Shift+R)', action: () => handleToggleReaderMode() }),
        await MenuItem.new({ text: 'Save Page as HTML...', action: () => handleSavePageHtml() }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
        await MenuItem.new({ text: 'History (Ctrl+H)', action: () => handleNavigate(undefined, 'edith://history') }),
        await MenuItem.new({ text: 'Bookmarks (Ctrl+Shift+O)', action: () => handleNavigate(undefined, 'edith://bookmarks') }),
        await MenuItem.new({ text: 'Downloads (Ctrl+J)', action: () => handleNavigate(undefined, 'edith://downloads') }),
        await PredefinedMenuItem.new({ item: 'Separator' }),
        searchSubmenu,
        await MenuItem.new({ text: '⚙️ Browser Settings', action: () => handleNavigate(undefined, 'edith://settings') }),
      ];

      const menu = await Menu.new({ items });
      await menu.popup();
    } catch (err) {
      console.warn('Failed to open overflow native menu:', err);
    }
  }, [activeTab?.zoom_level, handleSetSearchEngine, handleZoomIn, handleZoomOut, handleZoomReset, handleCreateNewTab, handlePrint, handleOpenFind, handleToggleReaderMode, handleSavePageHtml, handleNavigate]);

  // --- Keyboard Shortcuts Matrix Dispatcher ---
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check if user is typing inside an editable field outside omnibox
      const target = e.target as HTMLElement;
      const isInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
      const isOmnibox = target === omniboxInputRef.current;

      // Global navigation shortcuts
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'l') {
        e.preventDefault();
        omniboxInputRef.current?.focus();
        omniboxInputRef.current?.select();
        return;
      }

      if (e.altKey && e.key.toLowerCase() === 'd') {
        e.preventDefault();
        omniboxInputRef.current?.focus();
        omniboxInputRef.current?.select();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 't') {
        e.preventDefault();
        handleCreateNewTab('edith://newtab');
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        handleReopenTab();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'w') {
        e.preventDefault();
        handleCloseTab();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === 'Tab') {
        e.preventDefault();
        const tabs = browserState.tabs;
        if (tabs.length > 1) {
          const idx = tabs.findIndex((t) => t.id === browserState.active_tab_id);
          const nextIdx = e.shiftKey
            ? (idx - 1 + tabs.length) % tabs.length
            : (idx + 1) % tabs.length;
          handleSwitchTab(tabs[nextIdx].id);
        }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r') {
        e.preventDefault();
        browserController.reload();
        return;
      }

      if (e.key === 'F5') {
        e.preventDefault();
        browserController.reload();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'h') {
        e.preventDefault();
        handleNavigate(undefined, 'edith://history');
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'j') {
        e.preventDefault();
        handleNavigate(undefined, 'edith://downloads');
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        handleNavigate(undefined, 'edith://bookmarks');
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
        e.preventDefault();
        handleToggleBookmarkActiveTab();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        handleOpenFind();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'p') {
        e.preventDefault();
        handlePrint();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handleZoomIn();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === '-' || e.key === '_')) {
        e.preventDefault();
        handleZoomOut();
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key === '0') {
        e.preventDefault();
        handleZoomReset();
        return;
      }

      // Alt+Left / Alt+Right history navigation (only if not typing in web forms)
      if (e.altKey && e.key === 'ArrowLeft' && !isInput) {
        e.preventDefault();
        browserController.goBack();
        return;
      }

      if (e.altKey && e.key === 'ArrowRight' && !isInput) {
        e.preventDefault();
        browserController.goForward();
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleCreateNewTab, handleReopenTab, handleCloseTab, browserState.tabs, browserState.active_tab_id, handleSwitchTab, handleNavigate, handleToggleBookmarkActiveTab, handleOpenFind, handlePrint, handleZoomIn, handleZoomOut, handleZoomReset]);

  // Omnibox Escape & Enter Handling
  const handleOmniboxKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      if (activeTab) {
        setInputUrl(activeTab.url === 'edith://newtab' ? '' : activeTab.url);
      }
      omniboxInputRef.current?.blur();
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#02050e] text-slate-100 select-none overflow-hidden font-sans">
      {/* ========================================================================= */}
      {/* 1. FIXED TOP BROWSER CHROME: TAB STRIP                                   */}
      {/* ========================================================================= */}
      <div className="h-9 bg-[#040813] border-b border-white/[0.08] flex items-center px-2 shrink-0 z-30 overflow-x-auto no-scrollbar">
        <div className="flex items-center space-x-1.5 flex-1 min-w-0">
          {browserState.tabs.map((tab) => {
            const isActive = tab.id === browserState.active_tab_id;
            return (
              <div
                key={tab.id}
                onClick={() => handleSwitchTab(tab.id)}
                onContextMenu={(e) => handleTabContextMenu(e, tab.id)}
                className={`group relative flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs transition cursor-pointer max-w-[180px] min-w-[90px] border ${
                  isActive
                    ? 'bg-[#0a1124] border-white/[0.12] text-white shadow-sm'
                    : 'bg-[#060c1c]/70 border-white/[0.04] text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]'
                }`}
                title={tab.title || tab.url}
              >
                {/* Favicon or Globe */}
                {tab.favicon ? (
                  <img src={tab.favicon} alt="" className="w-3.5 h-3.5 rounded shrink-0 object-contain" />
                ) : tab.is_loading ? (
                  <Loader2 className="w-3.5 h-3.5 text-cyan-400 animate-spin shrink-0" />
                ) : (
                  <Globe className={`w-3.5 h-3.5 shrink-0 ${isActive ? 'text-cyan-400' : 'text-slate-500'}`} />
                )}

                {/* Tab Title */}
                <span className="truncate flex-1 font-medium text-[11px]">
                  {tab.title && tab.title !== 'Simulated Tab'
                    ? tab.title
                    : (tab.url === 'edith://newtab' || !tab.url ? 'New Tab' : tab.url)}
                </span>

                {/* Pin Indicator */}
                {tab.is_pinned && <Pin className="w-3 h-3 text-cyan-400 shrink-0 rotate-45" />}

                {/* Close Tab Button */}
                <button
                  onClick={(e) => handleCloseTab(e, tab.id)}
                  className="opacity-0 group-hover:opacity-100 hover:bg-white/10 rounded p-0.5 text-slate-400 hover:text-white transition"
                  title="Close Tab (Ctrl+W)"
                >
                  <X className="w-3 h-3" />
                </button>
              </div>
            );
          })}

          {/* New Tab Button */}
          <button
            onClick={() => handleCreateNewTab('edith://newtab')}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.06] transition shrink-0"
            title="New Tab (Ctrl+T)"
            aria-label="New Tab"
          >
            <Plus className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* ========================================================================= */}
      {/* 2. FIXED BROWSER CHROME: COMPACT NAVIGATION & OMNIBOX TOOLBAR             */}
      {/* ========================================================================= */}
      <div className="h-11 bg-[#050914] border-b border-white/[0.08] px-3 flex items-center gap-2 shrink-0 z-30">
        {/* Left Navigation Controls */}
        <div className="flex items-center space-x-1 shrink-0">
          <button
            onClick={() => browserController.goBack()}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Back (Alt+Left)"
            aria-label="Back"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <button
            onClick={() => browserController.goForward()}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Forward (Alt+Right)"
            aria-label="Forward"
          >
            <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={() => browserController.reload()}
            className={`w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition ${
              isLoading ? 'animate-spin text-cyan-400' : ''
            }`}
            title="Reload (Ctrl+R)"
            aria-label="Reload"
          >
            <RotateCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Center: Real Omnibox Address Bar */}
        <form onSubmit={handleNavigate} className="flex-1 min-w-[200px] flex items-center">
          <div className="w-full flex items-center bg-[#090e1a] border border-white/[0.1] focus-within:border-cyan-500/60 rounded-xl px-2.5 py-1 transition shadow-inner">
            {/* Site Security Lock Icon */}
            <button
              type="button"
              onClick={handleSecurityMenu}
              className="text-slate-400 hover:text-emerald-400 mr-2 shrink-0 transition"
              title="Site Security & Connection Info"
              aria-label="Site Security"
            >
              <Lock className="w-3.5 h-3.5 text-emerald-400" />
            </button>

            {/* Editable Omnibox Input */}
            <input
              ref={omniboxInputRef}
              type="text"
              value={inputUrl}
              onFocus={() => {
                setIsOmniboxFocused(true);
                omniboxInputRef.current?.select();
              }}
              onBlur={() => {
                setIsOmniboxFocused(false);
                if (!inputUrl.trim() && activeTab) {
                  setInputUrl(activeTab.url === 'edith://newtab' ? '' : activeTab.url);
                }
              }}
              onChange={(e) => setInputUrl(e.target.value)}
              onKeyDown={handleOmniboxKeyDown}
              placeholder={`Search with ${activeSearchEngine.name} or enter address (Ctrl+L)...`}
              className="w-full bg-transparent text-xs text-slate-100 placeholder-slate-500 focus:outline-none font-mono"
              spellCheck={false}
            />

            {/* Bookmark Star Toggle */}
            <button
              type="button"
              onClick={handleToggleBookmarkActiveTab}
              className={`transition shrink-0 ml-1.5 p-0.5 rounded hover:bg-white/[0.08] ${
                isActiveTabBookmarked ? 'text-amber-400' : 'text-slate-500 hover:text-amber-400'
              }`}
              title={isActiveTabBookmarked ? 'Bookmarked (Click to remove)' : 'Bookmark this tab (Ctrl+D)'}
              aria-label="Bookmark"
            >
              <Star className={`w-3.5 h-3.5 ${isActiveTabBookmarked ? 'fill-current' : ''}`} />
            </button>
          </div>
        </form>

        {/* Right: Compact Browser Actions (Tier B Flyouts & Overflow) */}
        <div className="flex items-center space-x-1 shrink-0">
          {/* History Icon */}
          <button
            onClick={handleHistoryMenu}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="History (Ctrl+H)"
            aria-label="History"
          >
            <History className="w-4 h-4" />
          </button>

          {/* Downloads Icon */}
          <button
            onClick={handleDownloadsMenu}
            className="relative w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-emerald-400 hover:bg-white/[0.05] transition"
            title="Downloads (Ctrl+J)"
            aria-label="Downloads"
          >
            <Download className="w-4 h-4" />
            {activeDownloadsCount > 0 && (
              <span className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-emerald-400 rounded-full animate-pulse" />
            )}
          </button>

          {/* Bookmarks Icon */}
          <button
            onClick={handleBookmarksMenu}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-amber-400 hover:bg-white/[0.05] transition"
            title="Bookmarks (Ctrl+Shift+O)"
            aria-label="Bookmarks"
          >
            <Bookmark className="w-4 h-4" />
          </button>

          {/* Profiles Icon */}
          <button
            onClick={handleProfilesMenu}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title={`Profile: ${activeProfileName}`}
            aria-label="Profiles"
          >
            <User className="w-4 h-4" />
          </button>

          {/* Overflow Menu (⋮) */}
          <button
            onClick={handleOverflowMenu}
            className="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-white hover:bg-white/[0.05] transition"
            title="More Tools & Settings"
            aria-label="More"
          >
            <MoreVertical className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* ========================================================================= */}
      {/* 3. FIXED CONTENT VIEWPORT (Zero Gap, Zero Displacement, Single Surface)  */}
      {/* ========================================================================= */}
      <div
        ref={viewportRef}
        id="edith-browser-viewport-container"
        className="flex-1 w-full bg-[#02050e] relative overflow-hidden flex flex-col items-center justify-start"
      >
        {/* Tier C: Dedicated Internal Browser Pages */}
        {isNewTab ? (
          /* --- edith://newtab --- */
          <div className="w-full h-full overflow-y-auto px-6 py-10 flex flex-col items-center animate-fadeIn z-20">
            <div className="max-w-3xl w-full flex flex-col items-center gap-7">
              {/* E.D.I.T.H. Branding */}
              <div className="flex flex-col items-center text-center gap-1.5 mt-4">
                <div className="flex items-center gap-2.5">
                  <Globe className="w-9 h-9 text-cyan-400 animate-pulse" />
                  <h1 className="text-2xl font-black tracking-widest text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 via-blue-400 to-indigo-300 font-mono">
                    E.D.I.T.H. BROWSER
                  </h1>
                </div>
                <div className="flex items-center gap-2 text-xs text-slate-500 font-mono">
                  <span>Fast</span>
                  <span>•</span>
                  <span>Isolated Profiles</span>
                  <span>•</span>
                  <span>Native Core</span>
                </div>
              </div>

              {/* Central Search Box */}
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  if (newTabSearchQuery.trim()) {
                    handleNavigate(e, newTabSearchQuery.trim());
                  }
                }}
                className="w-full max-w-2xl relative flex items-center shadow-2xl"
              >
                <div className="w-full flex items-center bg-[#070e1c]/90 border border-cyan-500/30 focus-within:border-cyan-400 rounded-2xl px-4 py-3 backdrop-blur-xl transition">
                  <Search className="w-5 h-5 text-cyan-400 mr-3 shrink-0" />
                  <input
                    type="text"
                    value={newTabSearchQuery}
                    onChange={(e) => setNewTabSearchQuery(e.target.value)}
                    placeholder={`Search with ${activeSearchEngine.name} or enter address...`}
                    className="bg-transparent text-sm text-slate-100 placeholder-slate-500 focus:outline-none flex-1 font-mono"
                    autoFocus
                  />
                  <button
                    type="submit"
                    className="px-3.5 py-1 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-black font-bold text-xs transition font-mono shadow-md"
                  >
                    Go
                  </button>
                </div>
              </form>

              {/* Quick Launch Shortcut Tiles */}
              <div className="w-full grid grid-cols-3 sm:grid-cols-6 gap-2.5">
                {[
                  { name: 'Google', url: 'https://www.google.com', icon: Globe, color: 'text-blue-400' },
                  { name: 'GitHub', url: 'https://github.com', icon: Code2, color: 'text-purple-400' },
                  { name: 'Wikipedia', url: 'https://en.wikipedia.org', icon: Globe, color: 'text-slate-300' },
                  { name: 'Rust Docs', url: 'https://doc.rust-lang.org', icon: Terminal, color: 'text-amber-400' },
                  { name: 'Tauri v2', url: 'https://v2.tauri.app', icon: Cpu, color: 'text-cyan-400' },
                  { name: 'YouTube', url: 'https://www.youtube.com', icon: Search, color: 'text-red-400' },
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

              {/* Bookmarks & Recent History Quick Strip */}
              <div className="w-full grid grid-cols-1 md:grid-cols-2 gap-4 mt-1">
                {/* Bookmarks */}
                <div className="p-4 rounded-2xl bg-[#040813]/80 border border-amber-500/20 backdrop-blur-md flex flex-col gap-2.5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 text-xs font-bold text-amber-300 font-mono">
                      <Bookmark className="w-4 h-4 text-amber-400" />
                      Bookmarks ({bookmarksList.length})
                    </div>
                    <button
                      onClick={() => handleNavigate(undefined, 'edith://bookmarks')}
                      className="text-[10px] text-amber-400/80 hover:text-amber-200 font-mono transition"
                    >
                      View All →
                    </button>
                  </div>
                  {bookmarksList.length === 0 ? (
                    <div className="text-center py-6 text-slate-500 text-xs font-mono">
                      No bookmarks saved yet. Star pages from the address bar.
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

                {/* Recent History */}
                <div className="p-4 rounded-2xl bg-[#040813]/80 border border-blue-500/20 backdrop-blur-md flex flex-col gap-2.5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 text-xs font-bold text-blue-300 font-mono">
                      <History className="w-4 h-4 text-blue-400" />
                      Recent History ({historyList.length})
                    </div>
                    <button
                      onClick={() => handleNavigate(undefined, 'edith://history')}
                      className="text-[10px] text-blue-400/80 hover:text-blue-200 font-mono transition"
                    >
                      View All →
                    </button>
                  </div>
                  {historyList.length === 0 ? (
                    <div className="text-center py-6 text-slate-500 text-xs font-mono">
                      No recent browsing history recorded.
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
            </div>
          </div>
        ) : isHistoryPage ? (
          /* --- edith://history --- */
          <div className="w-full h-full overflow-y-auto p-8 flex flex-col items-center animate-fadeIn z-20 font-mono">
            <div className="max-w-3xl w-full flex flex-col gap-5">
              <div className="flex items-center justify-between border-b border-white/10 pb-4">
                <div className="flex items-center gap-2.5 text-lg font-bold text-cyan-300">
                  <History className="w-5 h-5 text-cyan-400" />
                  Browsing History ({historyList.length} items)
                </div>
                <button
                  onClick={async () => {
                    if (confirm('Clear all browsing history?')) {
                      await browserController.clearHistory();
                      fetchHistory();
                    }
                  }}
                  className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-red-950/40 hover:bg-red-900/40 text-red-300 border border-red-500/30 text-xs transition"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  Clear All History
                </button>
              </div>

              {/* History Search */}
              <div className="relative flex items-center">
                <Search className="w-4 h-4 text-slate-400 absolute left-3" />
                <input
                  type="text"
                  value={historySearchQuery}
                  onChange={(e) => {
                    setHistorySearchQuery(e.target.value);
                    fetchHistory(e.target.value);
                  }}
                  placeholder="Search history by title or URL..."
                  className="w-full bg-[#080e1c] border border-white/10 rounded-xl pl-9 pr-3 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-cyan-500/50"
                />
              </div>

              {/* History Items */}
              {historyList.length === 0 ? (
                <div className="text-center py-12 text-slate-500 text-xs">No matching history entries found.</div>
              ) : (
                <div className="flex flex-col gap-1.5">
                  {historyList.map((item) => (
                    <div
                      key={item.id}
                      className="flex items-center justify-between p-2.5 rounded-xl bg-black/40 border border-white/5 hover:border-cyan-500/30 group transition"
                    >
                      <div
                        onClick={() => handleNavigate(undefined, item.url)}
                        className="flex items-center gap-2.5 truncate flex-1 cursor-pointer"
                      >
                        <Globe className="w-4 h-4 text-cyan-400 shrink-0" />
                        <div className="truncate flex-1">
                          <div className="text-xs text-slate-200 font-medium truncate group-hover:text-cyan-200">{item.title}</div>
                          <div className="text-[10px] text-slate-500 truncate">{item.url}</div>
                        </div>
                      </div>
                      <div className="flex items-center gap-3 shrink-0 ml-3">
                        <span className="text-[10px] text-slate-500">
                          {new Date(item.last_visited_at).toLocaleDateString([], { month: 'short', day: 'numeric' })}{' '}
                          {new Date(item.last_visited_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </span>
                        <button
                          onClick={async () => {
                            await browserController.deleteHistory(item.id);
                            fetchHistory(historySearchQuery);
                          }}
                          className="text-slate-600 hover:text-red-400 p-1 transition"
                          title="Remove from history"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ) : isBookmarksPage ? (
          /* --- edith://bookmarks --- */
          <div className="w-full h-full overflow-y-auto p-8 flex flex-col items-center animate-fadeIn z-20 font-mono">
            <div className="max-w-3xl w-full flex flex-col gap-5">
              <div className="flex items-center justify-between border-b border-white/10 pb-4">
                <div className="flex items-center gap-2.5 text-lg font-bold text-amber-300">
                  <Bookmark className="w-5 h-5 text-amber-400" />
                  Saved Bookmarks ({bookmarksList.length})
                </div>
                <button
                  onClick={() => setShowAddBookmarkModal(true)}
                  className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 border border-amber-500/30 text-xs transition"
                >
                  <Plus className="w-3.5 h-3.5" />
                  Add Bookmark
                </button>
              </div>

              {/* Bookmarks Search */}
              <div className="relative flex items-center">
                <Search className="w-4 h-4 text-slate-400 absolute left-3" />
                <input
                  type="text"
                  value={bookmarksSearchQuery}
                  onChange={(e) => {
                    setBookmarksSearchQuery(e.target.value);
                    fetchBookmarks(e.target.value);
                  }}
                  placeholder="Search bookmarks..."
                  className="w-full bg-[#080e1c] border border-white/10 rounded-xl pl-9 pr-3 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                />
              </div>

              {/* Bookmarks List */}
              {bookmarksList.length === 0 ? (
                <div className="text-center py-12 text-slate-500 text-xs">No bookmarks saved yet.</div>
              ) : (
                <div className="flex flex-col gap-1.5">
                  {bookmarksList.map((bm) => (
                    <div
                      key={bm.id}
                      className="flex items-center justify-between p-2.5 rounded-xl bg-black/40 border border-white/5 hover:border-amber-500/30 group transition"
                    >
                      <div
                        onClick={() => handleNavigate(undefined, bm.url)}
                        className="flex items-center gap-2.5 truncate flex-1 cursor-pointer"
                      >
                        <Bookmark className="w-4 h-4 text-amber-400 shrink-0" />
                        <div className="truncate flex-1">
                          <div className="text-xs text-slate-200 font-medium truncate group-hover:text-amber-200">{bm.title}</div>
                          <div className="text-[10px] text-slate-500 truncate">{bm.url}</div>
                        </div>
                      </div>
                      <div className="flex items-center gap-2 shrink-0 ml-3">
                        <button
                          onClick={async () => {
                            await browserController.deleteBookmark(bm.id);
                            fetchBookmarks(bookmarksSearchQuery);
                          }}
                          className="text-slate-600 hover:text-red-400 p-1 transition"
                          title="Delete bookmark"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ) : isDownloadsPage ? (
          /* --- edith://downloads --- */
          <div className="w-full h-full overflow-y-auto p-8 flex flex-col items-center animate-fadeIn z-20 font-mono">
            <div className="max-w-3xl w-full flex flex-col gap-5">
              <div className="flex items-center justify-between border-b border-white/10 pb-4">
                <div className="flex items-center gap-2.5 text-lg font-bold text-emerald-300">
                  <Download className="w-5 h-5 text-emerald-400" />
                  Downloads Manager ({downloadsList.length} items)
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={async () => {
                      await browserController.clearDownloads();
                      fetchDownloads();
                    }}
                    className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-white/5 hover:bg-white/10 text-slate-300 border border-white/10 text-xs transition"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    Clear Records
                  </button>
                </div>
              </div>

              {/* Downloads List */}
              {downloadsList.length === 0 ? (
                <div className="text-center py-12 text-slate-500 text-xs">No downloads recorded yet.</div>
              ) : (
                <div className="flex flex-col gap-2">
                  {downloadsList.map((dl) => (
                    <div
                      key={dl.id}
                      className="flex flex-col p-3 rounded-xl bg-black/40 border border-white/5 hover:border-emerald-500/30 gap-1.5 transition"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2 truncate flex-1">
                          <FileText className="w-4 h-4 text-emerald-400 shrink-0" />
                          <span className="text-xs text-slate-200 font-bold truncate">{dl.filename}</span>
                        </div>
                        <span className={`text-[9px] px-1.5 py-0.5 rounded font-bold ${
                          dl.status === 'COMPLETED'
                            ? 'bg-emerald-950 text-emerald-400 border border-emerald-500/40'
                            : dl.status === 'DOWNLOADING'
                            ? 'bg-cyan-950 text-cyan-300 border border-cyan-500/40 animate-pulse'
                            : 'bg-red-950 text-red-400 border border-red-500/40'
                        }`}>
                          {dl.status}
                        </span>
                      </div>

                      {dl.status === 'DOWNLOADING' && (
                        <div className="w-full bg-black/60 rounded-full h-1.5 overflow-hidden border border-white/10 my-1">
                          <div
                            className="bg-emerald-400 h-full transition-all duration-200"
                            style={{ width: `${Math.round(dl.progress * 100)}%` }}
                          />
                        </div>
                      )}

                      <div className="flex items-center justify-between text-[10px] text-slate-500 mt-1">
                        <span>{(dl.received_bytes / 1024).toFixed(1)} KB{dl.total_bytes ? ` / ${(dl.total_bytes / 1024).toFixed(1)} KB` : ''}</span>
                        <div className="flex items-center gap-2">
                          {dl.status === 'COMPLETED' && (
                            <>
                              <button
                                onClick={() => browserController.openDownloadedFile(dl.id)}
                                className="text-emerald-400 hover:text-emerald-300 flex items-center gap-1"
                              >
                                <ExternalLink className="w-3 h-3" /> Open
                              </button>
                              <button
                                onClick={() => browserController.showDownloadInFolder(dl.id)}
                                className="text-slate-400 hover:text-white flex items-center gap-1"
                              >
                                <Folder className="w-3 h-3" /> Folder
                              </button>
                            </>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ) : isSettingsPage ? (
          /* --- edith://settings --- */
          <div className="w-full h-full overflow-y-auto p-8 flex flex-col items-center animate-fadeIn z-20 font-mono">
            <div className="max-w-2xl w-full flex flex-col gap-6">
              <div className="border-b border-white/10 pb-4">
                <div className="flex items-center gap-2.5 text-lg font-bold text-cyan-300">
                  <Settings className="w-5 h-5 text-cyan-400" />
                  Browser Settings
                </div>
                <div className="text-xs text-slate-500 mt-1">
                  Manage search engine preferences, browsing data, and browser startup.
                </div>
              </div>

              {/* 1. Default Search Engine */}
              <div className="p-4 rounded-2xl bg-[#040813]/80 border border-white/10 flex flex-col gap-3">
                <div className="font-bold text-xs text-slate-200 flex items-center gap-2">
                  <Search className="w-4 h-4 text-cyan-400" />
                  Default Search Engine
                </div>
                <div className="text-[11px] text-slate-400">
                  Select the search engine used for queries entered into the address bar or start page:
                </div>
                <div className="flex flex-col gap-2 mt-1">
                  {[
                    { id: 'google' as SearchEngineId, name: 'Google (Default)', desc: 'Fast, comprehensive global search engine', url: 'https://www.google.com' },
                    { id: 'duckduckgo' as SearchEngineId, name: 'DuckDuckGo', desc: 'Privacy-focused search engine with zero tracking', url: 'https://duckduckgo.com' },
                    { id: 'bing' as SearchEngineId, name: 'Microsoft Bing', desc: 'Microsoft web search with rich media integration', url: 'https://www.bing.com' },
                  ].map((engine) => (
                    <label
                      key={engine.id}
                      onClick={() => handleSetSearchEngine(engine.id)}
                      className={`flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition ${
                        activeEngineId === engine.id
                          ? 'bg-cyan-950/40 border-cyan-500/50 text-cyan-200'
                          : 'bg-black/40 border-white/5 text-slate-300 hover:border-white/20'
                      }`}
                    >
                      <input
                        type="radio"
                        name="search_engine"
                        checked={activeEngineId === engine.id}
                        onChange={() => handleSetSearchEngine(engine.id)}
                        className="mt-0.5"
                      />
                      <div className="flex flex-col gap-0.5">
                        <span className="font-bold text-xs">{engine.name}</span>
                        <span className="text-[10px] text-slate-500">{engine.desc}</span>
                      </div>
                    </label>
                  ))}
                </div>
              </div>

              {/* 2. Browsing Data */}
              <div className="p-4 rounded-2xl bg-[#040813]/80 border border-white/10 flex flex-col gap-3">
                <div className="font-bold text-xs text-slate-200 flex items-center gap-2">
                  <Trash2 className="w-4 h-4 text-red-400" />
                  Browsing Data & Storage
                </div>
                <div className="text-[11px] text-slate-400">
                  Clear local session data, history records, and browser downloads:
                </div>
                <div className="flex items-center gap-2.5 mt-1">
                  <button
                    onClick={async () => {
                      if (confirm('Clear all browsing history?')) {
                        await browserController.clearHistory();
                        fetchHistory();
                        alert('History cleared.');
                      }
                    }}
                    className="px-3 py-1.5 rounded-xl bg-white/5 hover:bg-white/10 text-slate-200 border border-white/10 text-xs transition"
                  >
                    Clear History
                  </button>
                  <button
                    onClick={async () => {
                      await browserController.clearDownloads();
                      fetchDownloads();
                      alert('Download records cleared.');
                    }}
                    className="px-3 py-1.5 rounded-xl bg-white/5 hover:bg-white/10 text-slate-200 border border-white/10 text-xs transition"
                  >
                    Clear Downloads
                  </button>
                </div>
              </div>

              {/* 3. About Browser */}
              <div className="p-4 rounded-2xl bg-[#040813]/80 border border-white/10 flex flex-col gap-2 text-xs text-slate-400">
                <div className="font-bold text-slate-200">About E.D.I.T.H. Browser</div>
                <div>Engine: Microsoft WebView2 via Tauri v2 Core</div>
                <div>Isolation: Active profile directory ({activeProfileName})</div>
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
                  {findResult.match_found ? `${findResult.active_match_ordinal}/${findResult.matches_count}` : '0/0'}
                </span>
              )}
            </div>

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
            <button
              onClick={() => setFindCaseSensitive(!findCaseSensitive)}
              title="Match Case"
              className={`px-1.5 py-0.5 rounded text-[10px] font-bold border transition ${
                findCaseSensitive ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300' : 'bg-white/5 border-transparent text-slate-400 hover:text-slate-200'
              }`}
            >
              Aa
            </button>
            <button
              onClick={handleCloseFind}
              title="Close (Escape)"
              className="p-1 rounded hover:bg-red-950/60 hover:text-red-300 text-slate-400 transition"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        )}

        {/* Save Toast */}
        {saveStatusToast && (
          <div className="absolute top-3 left-1/2 transform -translate-x-1/2 z-50 bg-[#06111f] border border-cyan-500/50 text-cyan-200 px-4 py-2 rounded-xl shadow-2xl backdrop-blur-md text-xs font-mono flex items-center gap-2 animate-fadeIn">
            <CheckCircle2 className="w-4 h-4 text-cyan-400" />
            <span>{saveStatusToast}</span>
          </div>
        )}

        {/* Reader Mode Surface */}
        {activeTab?.is_reader_mode && !isInternalPage && (
          <div className={`absolute inset-0 z-30 overflow-y-auto flex flex-col transition-colors duration-200 ${
            readerTheme === 'sepia' ? 'bg-[#f4ecd8] text-[#5b4636]' : readerTheme === 'onyx' ? 'bg-black text-[#e0e0e0]' : 'bg-[#0a0f1d] text-slate-100'
          }`}>
            <div className="sticky top-0 z-10 px-6 py-2 border-b border-white/10 backdrop-blur-md flex items-center justify-between">
              <span className="font-bold text-xs">Reader Mode</span>
              <button
                onClick={() => handleToggleReaderMode(activeTab?.id)}
                className="px-2 py-1 rounded bg-white/10 text-xs hover:bg-white/20"
              >
                Exit
              </button>
            </div>
            <div className="max-w-2xl mx-auto px-6 py-10">
              <h1 className="text-2xl font-bold mb-4">{readerDocs[activeTab?.id || '']?.title}</h1>
              <div
                dangerouslySetInnerHTML={{ __html: readerDocs[activeTab?.id || '']?.content_html || '' }}
                className="prose prose-invert max-w-none text-sm leading-relaxed"
              />
            </div>
          </div>
        )}
      </div>

      {/* Add Bookmark Modal */}
      {showAddBookmarkModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm animate-fadeIn">
          <div className="bg-[#080e1c] border border-white/10 rounded-2xl p-5 max-w-md w-full mx-4 shadow-2xl flex flex-col gap-4 font-mono text-xs">
            <div className="flex items-center justify-between">
              <span className="font-bold text-amber-300 flex items-center gap-2">
                <Bookmark className="w-4 h-4 text-amber-400" /> Add Bookmark
              </span>
              <button onClick={() => setShowAddBookmarkModal(false)} className="text-slate-400 hover:text-white">
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="flex flex-col gap-3">
              <div>
                <label className="text-slate-400 text-[10px] block mb-1">Title</label>
                <input
                  type="text"
                  value={newBookmarkTitle}
                  onChange={(e) => setNewBookmarkTitle(e.target.value)}
                  placeholder="Bookmark Title..."
                  className="w-full bg-black/60 border border-white/10 rounded-xl px-3 py-2 text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                />
              </div>
              <div>
                <label className="text-slate-400 text-[10px] block mb-1">URL</label>
                <input
                  type="text"
                  value={newBookmarkUrl}
                  onChange={(e) => setNewBookmarkUrl(e.target.value)}
                  placeholder="https://..."
                  className="w-full bg-black/60 border border-white/10 rounded-xl px-3 py-2 text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                />
              </div>
            </div>
            <div className="flex items-center justify-end gap-2 mt-2">
              <button
                onClick={() => setShowAddBookmarkModal(false)}
                className="px-3 py-1.5 rounded-xl bg-white/5 hover:bg-white/10 text-slate-300"
              >
                Cancel
              </button>
              <button
                onClick={async () => {
                  if (newBookmarkUrl.trim()) {
                    await browserController.addBookmark(newBookmarkTitle.trim() || newBookmarkUrl.trim(), newBookmarkUrl.trim());
                    setShowAddBookmarkModal(false);
                    setNewBookmarkTitle('');
                    setNewBookmarkUrl('');
                    fetchBookmarks();
                  }
                }}
                className="px-4 py-1.5 rounded-xl bg-amber-500 hover:bg-amber-400 text-black font-bold"
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
