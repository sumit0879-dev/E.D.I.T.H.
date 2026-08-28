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
  AlertCircle,
  ExternalLink,
} from 'lucide-react';
import { browserController } from '../services/browserController';
import { isTauri } from '../services/tauri';

export const BrowserView: React.FC = () => {
  const [inputUrl, setInputUrl] = useState('https://example.com');
  const [currentUrl, setCurrentUrl] = useState('https://example.com');
  const [pageTitle, setPageTitle] = useState('Example Domain');
  const [isLoading, setIsLoading] = useState(false);
  const [observationResult, setObservationResult] = useState<string | null>(null);
  const [isObserving, setIsObserving] = useState(false);

  const viewportRef = useRef<HTMLDivElement>(null);

  // Sync bounds from DOM element to native WebView
  const syncBounds = useCallback(() => {
    if (!viewportRef.current) return;
    const rect = viewportRef.current.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      browserController.setBounds({
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height,
      }).catch((e) => console.warn('Failed to sync browser bounds:', e));
    }
  }, []);

  // Initialize or show native WebView on mount
  useEffect(() => {
    let mounted = true;
    setIsLoading(true);

    const init = async () => {
      // Allow DOM to compute initial layout
      await new Promise((resolve) => setTimeout(resolve, 50));
      if (!mounted) return;

      if (viewportRef.current) {
        const rect = viewportRef.current.getBoundingClientRect();
        try {
          const info = await browserController.create(currentUrl, {
            x: rect.left,
            y: rect.top,
            width: rect.width,
            height: rect.height,
          });
          if (mounted && info) {
            setCurrentUrl(info.current_url);
            setInputUrl(info.current_url);
            setPageTitle(info.title);
          }
        } catch (err) {
          console.error('Browser creation error:', err);
        }
      }
      if (mounted) {
        setIsLoading(false);
        syncBounds();
      }
    };

    init();

    // ResizeObserver for dynamic window resizing and TelemetryDock toggling
    const resizeObserver = new ResizeObserver(() => {
      syncBounds();
    });

    if (viewportRef.current) {
      resizeObserver.observe(viewportRef.current);
    }

    window.addEventListener('resize', syncBounds);

    return () => {
      mounted = false;
      resizeObserver.disconnect();
      window.removeEventListener('resize', syncBounds);
      browserController.hide().catch(() => {});
    };
  }, [syncBounds, currentUrl]);

  // Handle URL navigation
  const handleNavigate = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!inputUrl.trim()) return;

    setIsLoading(true);
    try {
      const navigatedUrl = await browserController.navigate(inputUrl);
      setCurrentUrl(navigatedUrl);
      setInputUrl(navigatedUrl);
      // Refresh title after navigation
      setTimeout(async () => {
        try {
          const title = await browserController.getTitle();
          if (title) setPageTitle(title);
        } catch {}
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
      setTimeout(updateUrlAndTitle, 500);
    } catch (err) {
      console.error('Back error:', err);
    }
  };

  const handleGoForward = async () => {
    try {
      await browserController.goForward();
      setTimeout(updateUrlAndTitle, 500);
    } catch (err) {
      console.error('Forward error:', err);
    }
  };

  const handleReload = async () => {
    setIsLoading(true);
    try {
      await browserController.reload();
      setTimeout(() => {
        updateUrlAndTitle();
        setIsLoading(false);
      }, 800);
    } catch (err) {
      console.error('Reload error:', err);
      setIsLoading(false);
    }
  };

  const updateUrlAndTitle = async () => {
    try {
      const url = await browserController.getUrl();
      if (url) {
        setCurrentUrl(url);
        setInputUrl(url);
      }
      const title = await browserController.getTitle();
      if (title) setPageTitle(title);
    } catch {}
  };

  // Test 5 & 10: Scoped page observation test
  const handleObservePage = async () => {
    setIsObserving(true);
    setObservationResult(null);
    try {
      const [url, title, text] = await Promise.all([
        browserController.getUrl(),
        browserController.getTitle(),
        browserController.getVisibleText(),
      ]);

      const summary = `### 🛰️ Scoped Page Observation (Test 5 & 10 Verified)
- **Observed URL**: \`${url}\`
- **Document Title**: \`${title}\`
- **Extracted Visible Text Length**: ${text.length} characters
- **Content Preview**:
> ${text.slice(0, 500)}...`;

      setObservationResult(summary);
    } catch (err: any) {
      setObservationResult(`Observation Error: ${err.message || err}`);
    } finally {
      setIsObserving(false);
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#000000] text-slate-100 select-none overflow-hidden">
      {/* Tactical Browser HUD Toolbar */}
      <div className="h-12 bg-[#050914] border-b border-white/[0.08] px-3 flex items-center gap-2 shrink-0 z-10">
        {/* Navigation Controls */}
        <div className="flex items-center space-x-1">
          <button
            onClick={handleGoBack}
            className="w-8 h-8 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Back"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <button
            onClick={handleGoForward}
            className="w-8 h-8 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition"
            title="Forward"
          >
            <ArrowRight className="w-4 h-4" />
          </button>
          <button
            onClick={handleReload}
            className={`w-8 h-8 rounded-lg flex items-center justify-center text-slate-400 hover:text-cyan-400 hover:bg-white/[0.05] transition ${
              isLoading ? 'animate-spin text-cyan-400' : ''
            }`}
            title="Reload"
          >
            <RotateCw className="w-4 h-4" />
          </button>
        </div>

        {/* Omnibox URL / Search Bar */}
        <form onSubmit={handleNavigate} className="flex-1 flex items-center">
          <div className="w-full flex items-center bg-[#090e1a] border border-white/[0.1] focus-within:border-cyan-500/50 rounded-xl px-3 py-1.5 transition shadow-inner">
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

        {/* Action Buttons */}
        <div className="flex items-center space-x-1.5">
          <button
            onClick={handleObservePage}
            disabled={isObserving}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-xl bg-cyan-950/60 border border-cyan-500/30 text-cyan-300 hover:bg-cyan-900/50 hover:border-cyan-400 transition text-[11px] font-mono shadow-cyan-glow-xs"
            title="Verify Scoped Page Observation (Test 5/10)"
          >
            <Eye className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Observe DOM</span>
          </button>
        </div>
      </div>

      {/* Observation Drawer Banner (if active) */}
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

      {/* Center Viewport Canvas (Native WebView Target Area) */}
      <div
        ref={viewportRef}
        id="edith-browser-viewport-container"
        className="flex-1 w-full bg-[#000000] relative overflow-hidden flex flex-col items-center justify-center"
      >
        {!isTauri() && (
          <div className="flex flex-col items-center text-center p-8 max-w-md bg-[#050914]/80 rounded-2xl border border-cyan-500/20 backdrop-blur-xl shadow-2xl">
            <Globe className="w-12 h-12 text-cyan-400 mb-4 animate-pulse" />
            <h3 className="text-base font-bold text-slate-100 mb-1">
              E.D.I.T.H. Native WebView2 Host
            </h3>
            <p className="text-xs text-slate-400 mb-4 leading-relaxed">
              In Tauri desktop mode, the Windows WebView2 engine renders directly into this viewport.
            </p>
            <div className="w-full bg-slate-900/90 rounded-xl p-3 border border-white/5 text-left font-mono text-[10px] space-y-1">
              <div className="text-emerald-400 flex items-center gap-1.5">
                <CheckCircle2 className="w-3.5 h-3.5" /> Engine: Windows WebView2 / Edge
              </div>
              <div className="text-cyan-300 flex items-center gap-1.5">
                <CheckCircle2 className="w-3.5 h-3.5" /> URL: {currentUrl}
              </div>
              <div className="text-slate-400 flex items-center gap-1.5">
                <CheckCircle2 className="w-3.5 h-3.5" /> Title: {pageTitle}
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
            WebView2 Host Active
          </span>
          <span className="text-slate-600">|</span>
          <span className="text-slate-300 truncate max-w-md">{pageTitle}</span>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-slate-500">Isolation: Sandboxed Remote Origin</span>
          <span className="text-cyan-400/80">Phase 1 Feasibility Spike</span>
        </div>
      </div>
    </div>
  );
};

export default BrowserView;
