import * as tauriService from './tauri';
import type { BrowserTabInfo, BrowserMultiStateInfo, BrowserViewportBounds, BrowserInfo } from '../types';

/**
 * Normalizes user input into a direct HTTPS URL or a deterministic search engine query URL.
 */
export function normalizeBrowserUrl(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) {
    return 'https://example.com';
  }

  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
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

  public async createTab(
    tabId: string,
    url?: string,
    bounds?: BrowserViewportBounds
  ): Promise<BrowserTabInfo> {
    if (bounds) this.currentBounds = bounds;
    const normUrl = url ? normalizeBrowserUrl(url) : 'https://example.com';
    const res = await tauriService.browserCreateTab(tabId, normUrl, this.currentBounds || undefined);

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

  public async navigateTab(tabId: string, input: string): Promise<string> {
    const normalized = normalizeBrowserUrl(input);
    const res = await tauriService.browserNavigateTab(tabId, normalized);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab) {
      tab.url = res;
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

  public async getTabUrl(tabId: string): Promise<string> {
    const url = await tauriService.browserGetTabUrl(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab && url) {
      tab.url = url;
      this.notify();
    }
    return url;
  }

  public async getTabTitle(tabId: string): Promise<string> {
    const title = await tauriService.browserGetTabTitle(tabId);
    const tab = this.tabs.find((t) => t.id === tabId);
    if (tab && title) {
      tab.title = title;
      this.notify();
    }
    return title;
  }

  public async getTabVisibleText(tabId: string): Promise<string> {
    return await tauriService.browserGetTabVisibleText(tabId);
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
}

export const browserController = new BrowserController();
export default browserController;
