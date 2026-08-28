import * as tauriService from './tauri';
import type { BrowserInfo, BrowserViewportBounds } from '../types';

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

class BrowserController {
  private isInitialized = false;
  private currentUrl = 'https://example.com';
  private currentTitle = 'Example Domain';

  public async create(url?: string, bounds?: BrowserViewportBounds): Promise<BrowserInfo> {
    const targetUrl = url ? normalizeBrowserUrl(url) : this.currentUrl;
    const res = await tauriService.browserCreate(targetUrl, bounds);
    this.isInitialized = true;
    this.currentUrl = res.current_url;
    this.currentTitle = res.title;
    return res;
  }

  public async destroy(): Promise<void> {
    await tauriService.browserDestroy();
    this.isInitialized = false;
  }

  public async show(): Promise<void> {
    if (this.isInitialized) {
      await tauriService.browserShow();
    }
  }

  public async hide(): Promise<void> {
    if (this.isInitialized) {
      await tauriService.browserHide();
    }
  }

  public async navigate(input: string): Promise<string> {
    const normalized = normalizeBrowserUrl(input);
    this.currentUrl = normalized;
    const res = await tauriService.browserNavigate(normalized);
    return res;
  }

  public async goBack(): Promise<void> {
    await tauriService.browserGoBack();
  }

  public async goForward(): Promise<void> {
    await tauriService.browserGoForward();
  }

  public async reload(): Promise<void> {
    await tauriService.browserReload();
  }

  public async setBounds(bounds: BrowserViewportBounds): Promise<void> {
    await tauriService.browserSetBounds(bounds);
  }

  public async getUrl(): Promise<string> {
    const url = await tauriService.browserGetUrl();
    if (url) this.currentUrl = url;
    return url;
  }

  public async getTitle(): Promise<string> {
    const title = await tauriService.browserGetTitle();
    if (title) this.currentTitle = title;
    return title;
  }

  public async getVisibleText(): Promise<string> {
    return await tauriService.browserGetVisibleText();
  }

  public getState() {
    return {
      isInitialized: this.isInitialized,
      currentUrl: this.currentUrl,
      currentTitle: this.currentTitle,
    };
  }
}

export const browserController = new BrowserController();
export default browserController;
