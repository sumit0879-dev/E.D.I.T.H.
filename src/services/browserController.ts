import * as tauriService from './tauri';
import type {
  BrowserTabInfo,
  BrowserMultiStateInfo,
  BrowserViewportBounds,
  BrowserInfo,
  PageObservationSnapshot,
  ScreenshotResult,
  BrowserActionResult,
  BrowserToolDefinition,
  BrowserToolExecutionResult,
  BrowserTaskState,
  BrowserTaskResult,
  BrowserActionContext,
  BrowserRiskAssessment,
  BrowserRiskAuditEntry,
  PendingBrowserActionApproval,
  BrowserTabWork,
  BrowserOrchestrationTask,
  BrowserSubtaskResult,
  BrowserOrchestrationResult,
  BrowserControlState,
  TabControlInfo,
  BrowserHistoryEntry,
  BrowserBookmarkFolder,
  BrowserBookmark,
  BrowserDownload,
  BrowserProfile,
} from '../types';

/**
 * Normalizes user input into a direct HTTPS URL or a deterministic search engine query URL.
 */
export function normalizeBrowserUrl(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) {
    return 'https://example.com';
  }

  // Security check: Block javascript: execution
  if (trimmed.toLowerCase().startsWith('javascript:')) {
    console.warn('Blocked execution of javascript: URL from omnibox.');
    return 'about:blank';
  }

  if (trimmed.startsWith('http://') || trimmed.startsWith('https://') || trimmed.startsWith('about:')) {
    return trimmed;
  }

  // Detect domain format (e.g. example.com, sub.domain.org/path, localhost:1420)
  const isDomain =
    (trimmed.includes('.') && !trimmed.includes(' ') && !trimmed.startsWith('.')) ||
    trimmed.startsWith('localhost');

  if (isDomain) {
    return `https://${trimmed}`;
  }

  // Fallback to DuckDuckGo search query
  return `https://duckduckgo.com/?q=${encodeURIComponent(trimmed)}`;
}

export type BrowserStateListener = (state: BrowserMultiStateInfo) => void;

class BrowserController {
  private tabs: BrowserTabInfo[] = [];
  private activeTabId: string | null = null;
  private isVisible = false;
  private currentBounds: BrowserViewportBounds | null = null;
  private listeners: Set<BrowserStateListener> = new Set();

  public subscribe(listener: BrowserStateListener): () => void {
    this.listeners.add(listener);
    listener(this.getState());
    return () => this.listeners.delete(listener);
  }

  private notify() {
    const s = this.getState();
    this.listeners.forEach((l) => l(s));
  }

  public getState(): BrowserMultiStateInfo {
    return {
      tabs: [...this.tabs],
      active_tab_id: this.activeTabId,
      is_visible: this.isVisible,
      bounds: this.currentBounds || undefined,
    };
  }

  public getActiveTab(): BrowserTabInfo | undefined {
    return this.tabs.find((t) => t.id === this.activeTabId);
  }

  public async refreshMultiState(): Promise<BrowserMultiStateInfo> {
    const multi = await tauriService.browserGetMultiState();
    this.tabs = multi.tabs;
    this.activeTabId = multi.active_tab_id;
    this.isVisible = multi.is_visible;
    if (multi.bounds) this.currentBounds = multi.bounds;
    this.notify();
    return multi;
  }

  public async createTab(
    tabId: string,
    url?: string,
    bounds?: BrowserViewportBounds,
    profileId?: string
  ): Promise<BrowserTabInfo> {
    if (bounds) this.currentBounds = bounds;
    const normUrl = url ? normalizeBrowserUrl(url) : 'https://example.com';
    const res = await tauriService.browserCreateTab(tabId, normUrl, this.currentBounds || undefined, profileId);

    const existingIdx = this.tabs.findIndex((t) => t.id === tabId);
    this.tabs.forEach((t) => (t.is_active = false));
    if (existingIdx >= 0) {
      this.tabs[existingIdx] = res;
    } else {
      this.tabs.push(res);
    }
    this.activeTabId = tabId;
    this.isVisible = true;
    this.notify();
    return res;
  }

  public async switchTab(tabId: string, bounds?: BrowserViewportBounds): Promise<BrowserTabInfo> {
    if (bounds) this.currentBounds = bounds;
    const res = await tauriService.browserSwitchTab(tabId, this.currentBounds || undefined);
    this.tabs.forEach((t) => {
      t.is_active = t.id === tabId;
      if (t.id === tabId) {
        t.url = res.url;
        t.title = res.title;
        t.favicon = res.favicon;
      }
    });
    this.activeTabId = tabId;
    this.isVisible = true;
    this.notify();
    return res;
  }

  public async closeTab(tabId: string): Promise<BrowserTabInfo | null> {
    const res = await tauriService.browserCloseTab(tabId);
    this.tabs = this.tabs.filter((t) => t.id !== tabId);
    if (res) {
      this.activeTabId = res.id;
      this.tabs.forEach((t) => (t.is_active = t.id === res.id));
    } else {
      this.activeTabId = null;
      this.isVisible = false;
    }
    this.notify();
    return res;
  }

  public async reopenLastClosedTab(bounds?: BrowserViewportBounds): Promise<BrowserTabInfo | null> {
    if (bounds) this.currentBounds = bounds;
    const res = await tauriService.browserReopenLastClosedTab(this.currentBounds || undefined);
    if (res) {
      this.tabs.forEach((t) => (t.is_active = false));
      this.tabs.push(res);
      this.activeTabId = res.id;
      this.isVisible = true;
      this.notify();
    }
    return res;
  }

  public async switchToNextTab(): Promise<BrowserTabInfo | undefined> {
    if (this.tabs.length <= 1 || !this.activeTabId) return this.getActiveTab();
    const idx = this.tabs.findIndex((t) => t.id === this.activeTabId);
    const nextIdx = (idx + 1) % this.tabs.length;
    return await this.switchTab(this.tabs[nextIdx].id);
  }

  public async switchToPrevTab(): Promise<BrowserTabInfo | undefined> {
    if (this.tabs.length <= 1 || !this.activeTabId) return this.getActiveTab();
    const idx = this.tabs.findIndex((t) => t.id === this.activeTabId);
    const prevIdx = (idx - 1 + this.tabs.length) % this.tabs.length;
    return await this.switchTab(this.tabs[prevIdx].id);
  }

  public async navigateTab(tabId: string, input: string): Promise<string> {
    const normalized = normalizeBrowserUrl(input);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) {
      tab.is_loading = true;
      this.notify();
    }
    const res = await tauriService.browserNavigateTab(tabId, normalized);
    if (tab) {
      tab.url = res;
      tab.is_loading = false;
      this.notify();
    }
    return res;
  }

  public async goBack(tabId?: string): Promise<void> {
    const targetId = tabId || this.activeTabId;
    if (targetId) {
      await tauriService.browserGoBackTab(targetId);
    }
  }

  public async goForward(tabId?: string): Promise<void> {
    const targetId = tabId || this.activeTabId;
    if (targetId) {
      await tauriService.browserGoForwardTab(targetId);
    }
  }

  public async reload(tabId?: string): Promise<void> {
    const targetId = tabId || this.activeTabId;
    if (targetId) {
      await tauriService.browserReloadTab(targetId);
    }
  }

  public async setBoundsAll(bounds: BrowserViewportBounds): Promise<void> {
    this.currentBounds = bounds;
    await tauriService.browserSetBoundsAll(bounds);
  }

  public async hideAll(): Promise<void> {
    this.isVisible = false;
    await tauriService.browserHideAll();
    this.notify();
  }

  public async showActive(bounds?: BrowserViewportBounds): Promise<void> {
    if (bounds) this.currentBounds = bounds;
    if (this.activeTabId) {
      this.isVisible = true;
      await tauriService.browserShowActive(this.currentBounds || undefined);
      this.notify();
    }
  }

  // --- Phase 4A Browser Interaction & Action Layer APIs ---
  public async clickElement(elementId: string, tabId?: string): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    const result = await tauriService.browserClickElement(targetId, elementId);
    if (result.url_changed && result.resulting_url) {
      const tab = this.tabs.find((t) => t.id === targetId);
      if (tab) {
        tab.url = result.resulting_url;
        this.notify();
      }
    }
    return result;
  }

  public async typeElement(
    elementId: string,
    text: string,
    clearFirst?: boolean,
    tabId?: string
  ): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    return await tauriService.browserTypeElement(targetId, elementId, text, clearFirst);
  }

  public async scroll(
    direction: string,
    amount?: number,
    tabId?: string
  ): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    return await tauriService.browserScroll(targetId, direction, amount);
  }

  public async pressKey(key: string, tabId?: string): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    return await tauriService.browserPressKey(targetId, key);
  }

  public async focusElement(elementId: string, tabId?: string): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    return await tauriService.browserFocusElement(targetId, elementId);
  }

  public async wait(
    condition: string,
    target?: string,
    timeoutMs?: number,
    tabId?: string
  ): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    return await tauriService.browserWait(targetId, condition, target, timeoutMs);
  }

  public async selectOption(
    elementId: string,
    value: string,
    tabId?: string
  ): Promise<BrowserActionResult> {
    const targetId = tabId || this.activeTabId || 'tab_a';
    return await tauriService.browserSelectOption(targetId, elementId, value);
  }

  // --- Phase 3 & 5.2 Live Observation & Screenshot APIs ---
  public async observeTab(tabId: string, scope?: string): Promise<PageObservationSnapshot> {
    const obs = await tauriService.browserObserveTab(tabId, scope);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) {
      tab.url = obs.url;
      tab.title = obs.title;
      tab.is_loading = false;
      this.notify();
    }
    return obs;
  }

  public async screenshotTab(tabId: string): Promise<ScreenshotResult> {
    return await tauriService.browserScreenshotTab(tabId, this.currentBounds || undefined);
  }

  public async getTabUrl(tabId: string): Promise<string> {
    const obs = await this.observeTab(tabId);
    return obs.url;
  }

  public async getTabTitle(tabId: string): Promise<string> {
    const obs = await this.observeTab(tabId);
    return obs.title;
  }

  public async getTabVisibleText(tabId: string): Promise<string> {
    const obs = await this.observeTab(tabId);
    return obs.visible_text;
  }

  // --- Phase 4B AI Browser Tool Integration APIs ---
  public async getToolDefinitions(): Promise<BrowserToolDefinition[]> {
    return await tauriService.browserGetToolDefinitions();
  }

  public async executeTool(
    toolName: string,
    argumentsObj: Record<string, any>
  ): Promise<BrowserToolExecutionResult> {
    const res = await tauriService.browserExecuteTool(toolName, argumentsObj);
    if (res.success) {
      await this.refreshMultiState();
    }
    return res;
  }

  // --- Phase 4C Autonomous Browser Agent APIs ---
  public async runAgentTask(
    goal: string,
    maxSteps?: number,
    timeoutMs?: number
  ): Promise<BrowserTaskResult> {
    const res = await tauriService.browserAgentRunTask(goal, maxSteps, timeoutMs);
    await this.refreshMultiState();
    return res;
  }

  public async cancelAgentTask(taskId: string): Promise<boolean> {
    return await tauriService.browserAgentCancelTask(taskId);
  }

  public async getCurrentAgentTask(): Promise<BrowserTaskState | null> {
    return await tauriService.browserAgentGetCurrentTask();
  }

  // --- Legacy Phase 1 Delegate Methods ---
  public async create(url?: string, bounds?: BrowserViewportBounds): Promise<BrowserInfo> {
    const tab = await this.createTab('tab_a', url, bounds);
    return {
      is_created: true,
      is_visible: true,
      current_url: tab.url,
      title: tab.title,
      bounds,
    };
  }

  public async destroy(): Promise<void> {
    await this.hideAll();
  }

  public async show(): Promise<void> {
    await this.showActive();
  }

  public async hide(): Promise<void> {
    await this.hideAll();
  }

  public async navigate(input: string): Promise<string> {
    const targetId = this.activeTabId || 'tab_a';
    return await this.navigateTab(targetId, input);
  }

  public async getUrl(): Promise<string> {
    const targetId = this.activeTabId || 'tab_a';
    return await this.getTabUrl(targetId);
  }

  public async getTitle(): Promise<string> {
    const targetId = this.activeTabId || 'tab_a';
    return await this.getTabTitle(targetId);
  }

  public async getVisibleText(): Promise<string> {
    const targetId = this.activeTabId || 'tab_a';
    return await this.getTabVisibleText(targetId);
  }

  // --- Phase 5.3 Browser Action Risk & Safety Engine APIs ---
  public async assessActionRisk(context: BrowserActionContext): Promise<BrowserRiskAssessment> {
    return await tauriService.browserAssessActionRisk(context);
  }

  public async getRiskAuditLog(): Promise<BrowserRiskAuditEntry[]> {
    return await tauriService.browserGetRiskAuditLog();
  }

  public async resolveActionApproval(
    approvalId: string,
    decision: string
  ): Promise<PendingBrowserActionApproval> {
    return await tauriService.browserResolveActionApproval(approvalId, decision);
  }

  // --- Phase 5.4 Autonomous Multi-Tab Task Orchestration APIs ---
  public async runMultiTabOrchestration(
    goal: string,
    subtaskGoals?: string[],
    globalMaxSteps?: number,
    timeoutMs?: number
  ): Promise<BrowserOrchestrationResult> {
    return await tauriService.browserOrchestratorRunTask(
      goal,
      subtaskGoals,
      globalMaxSteps,
      timeoutMs
    );
  }

  public async cancelOrchestration(orchestrationId: string): Promise<boolean> {
    return await tauriService.browserOrchestratorCancelTask(orchestrationId);
  }

  public async getCurrentOrchestration(): Promise<BrowserOrchestrationTask | null> {
    return await tauriService.browserOrchestratorGetCurrentTask();
  }

  // --- Phase 5.5 Human <-> AI Browser Control / Takeover APIs ---
  public async requestAiControl(tabId: string, taskId?: string): Promise<TabControlInfo> {
    return await tauriService.browserRequestAiControl(tabId, taskId);
  }

  public async takeoverTab(tabId: string, reason?: string): Promise<TabControlInfo> {
    return await tauriService.browserTakeoverTab(tabId, reason);
  }

  public async releaseAiControl(tabId: string): Promise<TabControlInfo> {
    return await tauriService.browserReleaseAiControl(tabId);
  }

  public async pauseAiControl(tabId: string): Promise<TabControlInfo> {
    return await tauriService.browserPauseAiControl(tabId);
  }

  public async resumeAiControl(tabId: string): Promise<TabControlInfo> {
    return await tauriService.browserResumeAiControl(tabId);
  }

  public async getTabControlInfo(tabId: string): Promise<TabControlInfo> {
    return await tauriService.browserGetTabControlInfo(tabId);
  }

  public async getAllTabControls(): Promise<TabControlInfo[]> {
    return await tauriService.browserGetAllTabControls();
  }

  // --- Phase 5.6A Browser History & Bookmarks APIs ---
  public async getRecentHistory(limit?: number): Promise<BrowserHistoryEntry[]> {
    return await tauriService.browserHistoryGetRecent(limit);
  }

  public async searchHistory(query: string, limit?: number): Promise<BrowserHistoryEntry[]> {
    return await tauriService.browserHistorySearch(query, limit);
  }

  public async deleteHistory(id: string): Promise<boolean> {
    return await tauriService.browserHistoryDelete(id);
  }

  public async clearHistory(): Promise<number> {
    return await tauriService.browserHistoryClear();
  }

  public async addBookmark(
    title: string,
    url: string,
    folderId?: string,
    favicon?: string
  ): Promise<BrowserBookmark> {
    return await tauriService.browserBookmarkAdd(title, url, folderId, favicon);
  }

  public async updateBookmark(
    id: string,
    title: string,
    url: string,
    folderId?: string
  ): Promise<boolean> {
    return await tauriService.browserBookmarkUpdate(id, title, url, folderId);
  }

  public async deleteBookmark(id: string): Promise<boolean> {
    return await tauriService.browserBookmarkDelete(id);
  }

  public async getBookmarks(): Promise<BrowserBookmark[]> {
    return await tauriService.browserBookmarksList();
  }

  public async searchBookmarks(query: string): Promise<BrowserBookmark[]> {
    return await tauriService.browserBookmarksSearch(query);
  }

  public async isBookmarked(url: string): Promise<boolean> {
    return await tauriService.browserBookmarkIsBookmarked(url);
  }

  public async createBookmarkFolder(name: string, parentId?: string): Promise<BrowserBookmarkFolder> {
    return await tauriService.browserBookmarkCreateFolder(name, parentId);
  }

  public async deleteBookmarkFolder(folderId: string): Promise<boolean> {
    return await tauriService.browserBookmarkDeleteFolder(folderId);
  }

  // --- Phase 5.6B Browser Download Manager Methods ---
  public async startDownload(
    url: string,
    tabId?: string,
    suggestedFilename?: string
  ): Promise<BrowserDownload> {
    return await tauriService.browserDownloadStart(url, tabId, suggestedFilename);
  }

  public async cancelDownload(downloadId: string): Promise<boolean> {
    return await tauriService.browserDownloadCancel(downloadId);
  }

  public async getDownloads(limit?: number): Promise<BrowserDownload[]> {
    return await tauriService.browserDownloadList(limit);
  }

  public async getDownload(downloadId: string): Promise<BrowserDownload | null> {
    return await tauriService.browserDownloadGet(downloadId);
  }

  public async deleteDownloadRecord(downloadId: string): Promise<boolean> {
    return await tauriService.browserDownloadDeleteRecord(downloadId);
  }

  public async clearDownloadRecords(): Promise<number> {
    return await tauriService.browserDownloadClearRecords();
  }

  public async showDownloadInFolder(downloadId: string): Promise<boolean> {
    return await tauriService.browserDownloadShowInFolder(downloadId);
  }

  public async openDownloadFile(downloadId: string): Promise<boolean> {
    return await tauriService.browserDownloadOpenFile(downloadId);
  }

  // --- Phase 5.6C Browser Profiles & Session Storage Isolation Methods ---
  public async getProfiles(): Promise<BrowserProfile[]> {
    return await tauriService.browserProfilesList();
  }

  public async getProfile(profileId: string): Promise<BrowserProfile | null> {
    return await tauriService.browserProfileGet(profileId);
  }

  public async createProfile(name: string, profileType?: string): Promise<BrowserProfile> {
    const profile = await tauriService.browserProfileCreate(name, profileType);
    this.notify();
    return profile;
  }

  public async switchProfile(profileId: string): Promise<BrowserProfile> {
    const profile = await tauriService.browserProfileSwitch(profileId);
    this.notify();
    return profile;
  }

  public async renameProfile(profileId: string, newName: string): Promise<BrowserProfile> {
    const profile = await tauriService.browserProfileRename(profileId, newName);
    this.notify();
    return profile;
  }

  public async deleteProfile(profileId: string): Promise<boolean> {
    const success = await tauriService.browserProfileDelete(profileId);
    this.notify();
    return success;
  }

  public async createTemporaryProfile(taskId: string): Promise<BrowserProfile> {
    return await tauriService.browserProfileCreateTemporary(taskId);
  }

  public async cleanupTemporaryProfile(profileId: string): Promise<boolean> {
    return await tauriService.browserProfileCleanupTemporary(profileId);
  }

  // Phase 5.6D: Tab Duplication, Pinning, Mass Close & Session Restoration
  public async duplicateTab(tabId: string): Promise<BrowserTabInfo> {
    const tab = await tauriService.browserDuplicateTab(tabId, this.currentBounds || undefined);
    this.tabs.push(tab);
    this.activeTabId = tab.id;
    this.notify();
    return tab;
  }

  public async togglePinTab(tabId: string): Promise<BrowserTabInfo> {
    const tab = await tauriService.browserTogglePinTab(tabId);
    const idx = this.tabs.findIndex((t) => t.id === tabId);
    if (idx !== -1) {
      this.tabs[idx] = tab;
    }
    this.notify();
    return tab;
  }

  public async closeOtherTabs(tabId: string): Promise<void> {
    await tauriService.browserCloseOtherTabs(tabId);
    this.tabs = this.tabs.filter((t) => t.id === tabId || t.is_pinned);
    this.activeTabId = tabId;
    this.notify();
  }

  public async closeTabsToRight(tabId: string): Promise<void> {
    await tauriService.browserCloseTabsToRight(tabId);
    const idx = this.tabs.findIndex((t) => t.id === tabId);
    if (idx !== -1) {
      this.tabs = this.tabs.filter((t, i) => i <= idx || t.is_pinned);
    }
    this.notify();
  }

  public async saveSession(): Promise<boolean> {
    return await tauriService.browserSaveSession();
  }

  public async restoreSession(): Promise<BrowserTabInfo[]> {
    const restored = await tauriService.browserRestoreSession(this.currentBounds || undefined);
    if (restored.length > 0) {
      this.tabs = restored;
      this.activeTabId = restored[0].id;
      this.notify();
    }
    return restored;
  }

  // --- Phase 5.6E Content Blocking & Privacy Policy ---
  public async getPrivacyStatus(tabId?: string, profileId?: string) {
    return await tauriService.browserPrivacyGetStatus(tabId, profileId);
  }

  public async togglePrivacyProtection(enabled: boolean, profileId?: string) {
    return await tauriService.browserPrivacyToggleProtection(enabled, profileId);
  }

  public async allowlistDomain(domain: string, profileId?: string) {
    return await tauriService.browserPrivacyAllowlistDomain(domain, profileId);
  }

  public async removeAllowlistDomain(domain: string, profileId?: string) {
    return await tauriService.browserPrivacyRemoveAllowlist(domain, profileId);
  }

  public async addPrivacyRule(pattern: string, ruleType?: string, category?: string, profileId?: string) {
    return await tauriService.browserPrivacyAddBlockRule(pattern, ruleType, category, profileId);
  }

  public async removePrivacyRule(ruleId: string) {
    return await tauriService.browserPrivacyRemoveBlockRule(ruleId);
  }

  public async listPrivacyRules(profileId?: string) {
    return await tauriService.browserPrivacyListRules(profileId);
  }

  public async getTabPrivacyStats(tabId: string) {
    return await tauriService.browserPrivacyGetTabStats(tabId);
  }

  public async resetTabPrivacyStats(tabId: string) {
    return await tauriService.browserPrivacyResetStats(tabId);
  }

  // --- Phase 5.6F-A Advanced Browser Utilities ---
  public async findInPage(tabId: string, query: string, forward?: boolean, caseSensitive?: boolean) {
    return await tauriService.browserFindInPage(tabId, query, forward, caseSensitive);
  }

  public async clearFind(tabId: string) {
    return await tauriService.browserClearFind(tabId);
  }

  public async zoomSet(tabId: string, level: number) {
    const res = await tauriService.browserZoomSet(tabId, level);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.zoom_level = res;
    this.notify();
    return res;
  }

  public async zoomIn(tabId: string) {
    const res = await tauriService.browserZoomIn(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.zoom_level = res;
    this.notify();
    return res;
  }

  public async zoomOut(tabId: string) {
    const res = await tauriService.browserZoomOut(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.zoom_level = res;
    this.notify();
    return res;
  }

  public async zoomReset(tabId: string) {
    const res = await tauriService.browserZoomReset(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.zoom_level = res;
    this.notify();
    return res;
  }

  public async printTab(tabId: string) {
    return await tauriService.browserPrintTab(tabId);
  }

  public async openLinkInNewTab(url: string, sourceTabId?: string) {
    const tab = await tauriService.browserOpenLinkTab(url, sourceTabId, this.currentBounds || undefined);
    this.tabs.push(tab);
    this.activeTabId = tab.id;
    this.notify();
    return tab;
  }

  // --- Phase 5.6F-B Save Page + PDF + Reader Mode ---
  public async savePageHtml(tabId: string, customFilename?: string) {
    return await tauriService.browserSavePageHtml(tabId, customFilename);
  }

  public async readerModeEnter(tabId: string) {
    const doc = await tauriService.browserReaderModeEnter(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.is_reader_mode = true;
    this.notify();
    return doc;
  }

  public async readerModeExit(tabId: string) {
    const res = await tauriService.browserReaderModeExit(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) tab.is_reader_mode = false;
    this.notify();
    return res;
  }

  public async readerModeGet(tabId: string) {
    return await tauriService.browserReaderModeGet(tabId);
  }

  public async readerExtract(tabId: string) {
    return await tauriService.browserReaderExtract(tabId);
  }
}

export const browserController = new BrowserController();
export default browserController;

