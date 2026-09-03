# E.D.I.T.H. Browser — Professional Browser UX / Airspace / UI / Architecture Overhaul (Version 2.1)

---

## 1. Scope

This specification defines the comprehensive architectural, user experience, and structural overhaul of the **E.D.I.T.H. Browser module**.

The non-negotiable objective is to establish layout stability, interaction quality, and visual coherence comparable to a modern desktop browser (such as Microsoft Edge or Google Chrome), while maintaining E.D.I.T.H.'s distinct brand identity, retaining valid existing capabilities, and avoiding any regressions.

### In Scope
- Browser chrome layout, fixed geometry, single coherent surface integration, and responsive toolbar behavior.
- Precise resolution of all user-visible Win32 WebView2 airspace defects without hiding or displacing active content.
- Four-tier popup and menu architecture distinguishing native command menus, rich browser flyouts, full internal management tabs, and conditional in-page overlays.
- Universal zero-displacement rule ensuring webpage stability across all popup interactions.
- Explicit separation of Tab lifecycle from Popup lifecycle.
- Real, interactive omnibox with full URL fidelity, caret positioning, editing, and comprehensive navigation synchronization.
- End-to-end search engine architecture (Google default, DuckDuckGo, Bing) with single source of truth across frontend and backend, including restart persistence.
- Architecture and implementation of dedicated internal browser pages (`edith://newtab`, `edith://history`, `edith://bookmarks`, `edith://downloads`, `edith://settings`).
- Complete elimination of experimental AI agent and developer console clutter from user-facing browser surfaces while strictly preserving shared programmatic backend capabilities in Rust.
- Comprehensive 28-state screenshot verification matrix and baseline vs. post-fix visual QA loop.
- Browser keyboard shortcut matrix and webpage input protection.

### Strict Scope Boundaries
This overhaul applies **exclusively** to the browser subsystem. The following modules and files are strictly out of scope and must not be touched:
- Voice & TTS synthesis (`src-tauri/src/tts.rs`, `src/services/tauri.ts` TTS player logic)
- Chat view and session management (`src/views/ChatView.tsx`)
- Global E.D.I.T.H. Settings view (`src/views/SettingsView.tsx`)
- Voice view (`src/views/VoiceView.tsx`)
- Terminal view (`src/views/TerminalView.tsx`)
- Data view (`src/views/DataView.tsx`)
- Vector Memory & LanceDB (`src-tauri/src/memory.rs`)

---

## 2. Non-Goals

1. **No Cloning of Edge or Chrome Proprietary Assets**:
   - Do not copy proprietary branding, logos, trademarks, proprietary assets, proprietary source code, or exact visual artwork from Google Chrome or Microsoft Edge. E.D.I.T.H. maintains its own tactical aesthetic while adopting modern browser interaction conventions.
2. **No Unrelated Code Refactoring**:
   - No opportunistic cleanup, lint fixes, or formatting changes in unrelated E.D.I.T.H. modules.
3. **No Deletion of Shared AI/Agent Backend Infrastructure**:
   - Programmatic AI agent tools (`browser_agent.rs`, `browser_orchestrator.rs`, `browser_tools.rs`) remain intact in the Rust backend for automated agent tasks. Only browser-specific manual UI buttons and redundant inspector panels are removed from the browser view.
4. **No Premature CoreWebView2CompositionController Migration**:
   - Do not introduce full DirectComposition hosting unless a requirement genuinely cannot be satisfied by top-level native popup surfaces and native menus.
5. **No Manipulation of Remote Webpage DOM**:
   - Remote website DOM and rendering inside child WebView2 instances remain untouched, with the exception of the existing reader mode extractor and accessibility live observer scripts.

---

## 3. Current Architecture

### 3.1 Component & Service Map
- **Frontend Container (`src/views/BrowserView.tsx`)**:
  - Hybrid React container managing the tab bar, navigation bar, omnibox, drawer panels, and the native viewport mounting canvas (`#edith-browser-viewport-container`).
- **Global Application Shell (`src/components/TopHudBar.tsx`)**:
  - Houses the global AI model selector. Currently uses Tauri native menus in browser context to avoid child webview occlusion.
- **Controller Layer (`src/services/browserController.ts`)**:
  - Manages tab state, navigation dispatch, viewport bounds synchronization, bookmarks, downloads, and profile state.
- **Tauri IPC Bridge (`src/services/tauri.ts`)**:
  - Exposes typed invoke functions for Rust backend browser commands.
- **Rust Core Engine (`src-tauri/src/browser.rs`)**:
  - Manages native Win32 `tauri::Webview` child instances attached to the main window (`window.add_child(...)`), tab lifecycle, bounds tracking (`BrowserViewportBounds`), session persistence, and navigation interception.

### 3.2 Current Window & Hierarchy Model
```
Main Tauri Window [HWND]
├── Parent Webview (E.D.I.T.H. Shell / React DOM)
│   ├── TopHudBar (Global Shell Header)
│   ├── Sidebar (Global Navigation)
│   └── BrowserView (Browser Shell)
│       ├── Tab Strip (React DOM)
│       ├── HUD Toolbar & Omnibox (React DOM)
│       ├── [IN-FLOW DRAWERS: Downloads, Bookmarks, Profiles, History, Risk, Agent HUD]
│       └── Viewport Container Div (#edith-browser-viewport-container)
└── Child Webview [HWND] (Tauri Webview / msedgewebview2.exe)
    └── Active Remote Webpage (e.g. Wikipedia, Google)
```

---

## 4. Root Cause Analysis

### 4.1 Vertical Webpage Displacement (Layout Shifting)
In [`BrowserView.tsx`](file:///e:/Projects/E.D.I.T.H/src/views/BrowserView.tsx#L2425-L2850), panels for Downloads, Bookmarks, Profiles, History, and Risk Audit were implemented as conditional, in-flow `<div>` elements inserted directly between the toolbar and `viewportRef`.
- Opening "History" mounted a 250px–320px high in-flow block.
- This shifted `viewportRef.current.getBoundingClientRect().top` downward by that exact amount.
- The bounds synchronization loop (`syncBounds`) immediately moved the child Win32 WebView2 position downward, causing the webpage to jump down.

### 4.2 Win32 Airspace Occlusion
Child WebView2 instances created via `window.add_child` are native Win32 child windows (`Intermediate D3D Window`).
- Any HTML element rendered by the parent React webview at coordinates overlapping the child HWND is occluded because the child window draws on top of the parent window's DirectComposition/GDI surface.
- Past revisions attempted to work around this by invoking `browserController.hideAll()` or `webview.hide()`, which caused the active webpage to turn black or flicker violently whenever menus opened.

### 4.3 Cluttered Toolbar & Start Page
The browser toolbar currently contains over 15 text-heavy pills and developer toggles (`AI Agent (4C)`, `Orchestrator (5.4)`, `Actions (4A)`, `Observe`, `Safety (5.3)`, `Take Control`). The new tab page similarly hosts redundant drawer buttons and debug widgets, presenting an engineering console rather than a consumer browser.

### 4.4 Hardcoded Search Fallback Across Stack
Search query fallback to DuckDuckGo was hardcoded independently in three places:
1. `src/services/browserController.ts` (`normalizeBrowserUrl`)
2. `src/views/BrowserView.tsx` (`handleNavigate` and search inputs)
3. `src-tauri/src/browser.rs` (`normalize_url`, line 366)
There was no unified registry, user preference persistence, or configuration UI.

---

## 5. Target Browser Architecture

The target architecture establishes a clean, unified browser surface where browser chrome remains fixed, the webpage remains fixed and stable, and all popups float gracefully over the surface.

```
┌────────────────────────────────────────────────────────────┐
│ Top Window Titlebar / Global TopHudBar                     │
├────────────────────────────────────────────────────────────┤
│ FIXED BROWSER CHROME                                       │
│ ├── Native Tab Strip (Tabs, Groups, New Tab +)             │
│ └── Compact Navigation Toolbar (Back, Fwd, Reload,         │
│     Security, Omnibox, Star, Quick Icons, Overflow ⋮)      │
├────────────────────────────────────────────────────────────┤
│ FIXED CONTENT VIEWPORT (Starts directly under toolbar)     │
│                                                            │
│ ┌────────────────────────┐  ┌────────────────────────────┐ │
│ │ EXTERNAL TABS          │  │ INTERNAL TABS (edith://)   │ │
│ │ Active Native WebView2 │  │ Dedicated React Surfaces:  │ │
│ │ • Visible & Alive      │  │ • edith://newtab           │ │
│ │ • Zero Layout Shifting │  │ • edith://history          │ │
│ │ • Zero Airspace Occl.  │  │ • edith://bookmarks        │ │
│ │ • Unchanged Bounds     │  │ • edith://downloads        │ │
│ └────────────────────────┘  │ • edith://settings         │ │
│                             └────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
         ▲                                   ▲
         │ (Native Win32 Topmost Menus)      │ (Anchored Custom Flyouts)
   Tier A: Simple Menus               Tier B: Rich Popups
   • Tab Context Menu                 • History Quick Flyout
   • Model Switcher                   • Bookmarks Quick Flyout
   • Search Engine Selector           • Downloads Quick Flyout
   • Overflow Menu (⋮)                • Profiles Quick Flyout
```

---

## 6. Browser Chrome Architecture

1. **Fixed Chrome Invariant**:
   - Browser chrome consists of two immutable vertical sections:
     - Top Tab Strip (38px height)
     - Navigation & Omnibox Toolbar (42px height)
   - Chrome elements never expand, collapse, or inject vertical blocks into the document flow during popup activation.
2. **Coherent Single Surface**:
   - The webpage viewport container begins immediately below the toolbar border with `0px` margin, `0px` padding, and `0px` artificial border framing.
   - Eliminates nested cards, dashboard drop-shadows, and fake inner borders around the viewport.
   - Visually presents a single continuous application window.

---

## 7. WebView2 Content Architecture

1. **Active Webview Stability**:
   - The active child WebView2 maintains an exact 1-to-1 geometric match with the viewport container div (`#edith-browser-viewport-container`).
   - The native child webview is never hidden, shifted, resized, destroyed, or recreated when opening any popup, menu, or dialog.
2. **State & Media Preservation**:
   - Remote page DOM, active audio/video playback, WebGL context, form entries, JavaScript timers, and scroll position remain completely uninterrupted during all browser chrome interactions.

---

## 8. Popup & Menu Architecture (The 4-Tier Model)

Rather than forcing every interaction into an OS menu or an in-flow DOM element, interactions are structured into four distinct architectural tiers:

### Tier A: Simple Command & Context Menus
- **Use Case**: Tab context menu, Global Model switcher, Browser Overflow (⋮) menu, Search Engine quick switcher.
- **Mechanism**: Tauri Native OS Menus (`@tauri-apps/api/menu` / Win32 `TrackPopupMenuEx`).
- **Rationale**: Native menus are OS-level topmost windows (`#32768`) drawn directly by the Desktop Window Manager (DWM). They float natively above child WebView2 HWNDs with zero airspace occlusion, zero layout shift, and instant response.

### Tier B: Rich Browser Popups / Flyouts
- **Use Case**: Quick History flyout (with search input and recent items), Quick Bookmarks flyout, Quick Downloads flyout (with progress indicators and open buttons), Quick Profiles flyout, Site Security/Privacy flyout.
- **Mechanism**:
  - Must support browser-quality UI: scrolling, rich rows, icons, search fields, status indicators, buttons, grouping, and custom styling.
  - Implemented using an appropriately anchored top-level surface (such as a lightweight frameless top-level popup window or custom floating surface positioned over the parent window) that renders above the child WebView2 HWND without displacing or hiding the webview.
  - Closes on outside click, blur, or `Escape`.
- **Constraint**: Under no circumstances shall an in-flow DOM element be inserted above or within the viewport container to display these flyouts.

### Tier C: Full Management Pages
- **Use Case**: Comprehensive history search & date filtering, bookmark tree reorganization, complete download history audit, extensible browser preferences.
- **Mechanism**: Dedicated internal browser tabs running internal routes (`edith://history`, `edith://bookmarks`, `edith://downloads`, `edith://settings`).
- **Behavior**: Clicking "Open full manager..." from any Tier B flyout or pressing standard shortcuts (`Ctrl+H`, `Ctrl+J`, `Ctrl+Shift+O`) navigates to or focuses the corresponding internal tab.

### Tier D: Advanced Arbitrary Overlays (Conditional)
- **Use Case**: Advanced Find in page HUD (`Ctrl+F`), save-page toast notifications, isolated reader mode.
- **Mechanism**:
  - Find in Page: Compact floating toolstrip anchored strictly within the browser chrome toolbar bounds, or native WebView2 script injection bridge.
  - Reader Mode: Full-viewport internal reader surface rendered when the tab enters reader mode.
  - CompositionController is evaluated only if arbitrary custom graphics must be dynamically blended directly over live remote web content. It is not used for standard menus or flyouts.

---

## 9. Airspace Strategy (Precise Technical Framing)

### 9.1 Accurate Technical Definition
Native popup windows and OS menus do not alter the underlying DirectComposition architecture of WebView2; rather, they operate as separate topmost HWNDs managed by the Windows Desktop Window Manager (DWM). 
- **Requirement**: All supported browser popup interactions must be free from user-visible airspace defects.
- **Prohibited Workarounds**:
  - ❌ `browserController.hideAll()`
  - ❌ `webview.hide()` on popup trigger
  - ❌ Destroying or recreating WebView2 instances to reset Z-order
  - ❌ Shrinking or shifting `viewportRef` bounds

### 9.2 The Valid Architectural Solutions
1. **OS Topmost Floating Surfaces**: Native Win32 menus and top-level popup windows reside above the child HWND in the OS window hierarchy, eliminating airspace clipping entirely.
2. **Internal Tab Surface Delegation**: When navigating to internal pages (`edith://`), the child webview is hidden *only because the active tab is an internal application surface*, allowing React to render the full management UI in the viewport without remote webview conflict.

---

## 10. Tab Lifecycle vs. Popup Lifecycle

The architecture explicitly decouples tab switching from popup management:

| Lifecycle Event | Child WebView2 Action | Rationale |
| :--- | :--- | :--- |
| **Open Menu / Popup** (History, Bookmarks, Downloads, Profiles, Settings, Overflow) | **STAY VISIBLE & UNTOUCHED** (`show()`, bounds unaltered) | User is inspecting browser chrome while viewing active page. |
| **Close Menu / Popup** | **NO-OP** (remain visible) | Zero flicker, zero state loss. |
| **Switch to Inactive Tab** (External URL) | Hide previous tab webview, Show target tab webview | Standard multi-tab isolation. |
| **Switch to Internal Tab** (`edith://newtab`, `edith://settings`, etc.) | Hide native webview for target tab | Viewport renders internal React surface cleanly. |
| **Window Resize / Move** | Synchronize position and size via `set_position` / `set_size` | Webview tracks container bounds smoothly. |

---

## 11. Omnibox Architecture: URL Fidelity & Synchronization

### 11.1 URL Fidelity Invariant
The omnibox must faithfully represent the actual active navigation URL.
- **No Silent Mutation**: Do NOT strip query parameters, fragments (`#hash`), paths, ports, or tracking parameters from the URL.
- **Unfocused State**: May visually ellipsize extremely long URLs or highlight the registrable domain for readability, but must represent the true current destination.
- **Focused State**: Exposes the complete, unadorned, raw URL string. Automatically selects the entire address upon initial focus for rapid replacement or copying.
- **Interactive Surface**: Real HTML `<input>` with full caret navigation, text selection, copy, paste, and undo/redo.

### 11.2 Comprehensive Navigation Synchronization
The omnibox must update immediately and synchronously across all navigation events:
- Direct user URL navigation
- Search query submission
- Client-side navigation (`history.pushState`, `history.replaceState`, hash changes)
- HTTP server-side redirects (e.g. `301`, `302`)
- Browser Back and Forward navigation (`Alt+Left`, `Alt+Right`)
- Page reload (`Ctrl+R`, `F5`)
- Tab switching (active tab URL immediately reflected)
- Internal page navigation (`edith://newtab`, `edith://history`, etc.)
- In-page link clicks and window.open / programmatic dispatches
- Never displays internal Tauri labels (`edith_tab_tab_xyz`) or memory pointers.

---

## 12. Search Engine Architecture: End-to-End Flow & Persistence

### 12.1 End-to-End Data Flow
A single source of truth governs search engine selection across the entire stack:

```
User Input (Omnibox or New Tab Search)
    │
    ▼
URL-vs-Query Classification
    ├── If valid URL / domain / internal route ──► Navigate directly
    │
    └── If search query string ──► Selected SearchEngine Configuration
                                      │
                                      ▼
                               Navigation URL Generation (e.g. https://www.google.com/search?q=%s)
                                      │
                                      ▼
                               browserController.navigateTab()
                                      │
                                      ▼
                               Rust Backend (browser_navigate_tab)
                                      │
                                      ▼
                               Active Child WebView2 Navigation
```

### 12.2 Single Source of Truth & Backend Alignment
- **Frontend Source of Truth**: `SEARCH_ENGINES` registry in `browserController.ts`.
- **Default Engine**: **Google** (`https://www.google.com/search?q=%s`).
- **Supported Engines**: Google, DuckDuckGo (`https://duckduckgo.com/?q=%s`), Bing (`https://www.bing.com/search?q=%s`).
- **Persistent Storage**: Saved in `localStorage` under `edith_browser_search_engine`.
- **Backend Alignment**:
  - Audit and update `src-tauri/src/browser.rs` line 366 (`normalize_url`) to eliminate the hardcoded DuckDuckGo fallback.
  - Ensure the backend receives the normalized URL from the frontend or queries the shared preference, guaranteeing zero frontend/backend search divergence.

### 12.3 Persistence Verification Test
1. Select DuckDuckGo in Browser Settings.
2. Close the E.D.I.T.H. application completely.
3. Relaunch E.D.I.T.H. and open Browser.
4. Verify omnibox and New Tab searches dispatch to DuckDuckGo.
5. Select Google, close application, reopen, and confirm Google is active.

---

## 13. Browser Settings Architecture

### 13.1 Strict Separation of Concerns
- **Global E.D.I.T.H. Settings** (`src/views/SettingsView.tsx`): Manages AI models, API keys, system speech synthesis, and application appearance. Remains untouched.
- **Browser Settings** (`edith://settings`): Dedicated internal browser page managing browser-specific preferences only.

### 13.2 Architecture Ready for Extension
The initial overhaul implements:
- Default Search Engine selector (Google, DuckDuckGo, Bing) with immediate persistence.
- Clear Browsing Data button (history, cache, cookies).
- Extensible schema structured to support future options (startup behavior, default zoom, custom homepage, download directory) without refactoring core settings plumbing.

---

## 14. New-Tab Page Architecture (`edith://newtab`)

1. **Clean Visual Hierarchy**:
   - Centered E.D.I.T.H. Browser brandmark.
   - Prominent, auto-focused search bar utilizing the configured search engine.
   - Grid of quick shortcuts (Google, GitHub, Wikipedia, Rust Docs, Tauri v2, YouTube) with custom shortcut addition.
   - Clean recent pages and bookmarks strip.
2. **Complete Removal of Clutter**:
   - Eliminate all autonomous agent buttons, developer console drawers, and experimental HUD cards.
   - Present a calm, modern start page experience matching modern desktop standards.

---

## 15. AI / Developer Clutter Removal & Dependency Tracing

### 15.1 UI Elements Removed from Browser Shell
The following user-facing controls, panels, and state hooks are removed from `BrowserView.tsx`:
- `AI Agent (4C)` toolbar button and inline drawer
- `Orchestrator (5.4)` toolbar button and inline drawer
- `Actions (4A)` playground button and inline drawer
- `Observe` DOM inspector button and cards
- `Safety (5.3)` risk audit button and drawer
- `Grant AI` / `Take Control` takeover pills
- Bottom debug status panels and screenshot preview bars

### 15.2 Backend Preservation Invariant
- All underlying Rust backend commands in `src-tauri/src/browser_agent.rs`, `browser_orchestrator.rs`, and `browser_tools.rs` are **strictly preserved** without deletion.
- These commands remain available for programmatic AI agent workflows and background automated operations.
- Only manual user-facing browser UI bindings and dead imports are cleaned up.

---

## 16. Global Model Selector vs. Browser Controls

1. **Global Shell Belonging**:
   - The AI Model Selector belongs to the **Global E.D.I.T.H. Shell Header** (`src/components/TopHudBar.tsx`), not the browser toolbar.
2. **No Duplication**:
   - The model selector will NOT be duplicated inside the browser toolbar.
3. **Preserve Native Menu Behavior**:
   - The existing fix in `TopHudBar.tsx` (using Tauri Native OS menus when in browser context) is preserved to prevent child webview occlusion when switching models globally.

---

## 17. Browser Toolbar Redesign & Responsive Behavior

### 17.1 Toolbar Layout
The toolbar transitions to a compact, icon-first desktop browser toolbar:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [◀] [▶] [↻]  [🔒 https://en.wikipedia.org/wiki/Rust  ★]  [⏱] [↓] [★] [👤] [⋮]       │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

1. **Left Navigation Cluster**:
   - Back button (`Alt+Left`) with disabled styling when history stack is empty.
   - Forward button (`Alt+Right`) with disabled styling.
   - Reload button (`Ctrl+R` / `F5`) displaying subtle spinner during active document loading.
2. **Omnibox Centerpiece**:
   - Site Security Lock icon (Green/Cyan for HTTPS, Neutral for local, Amber for HTTP).
   - Omnibox Input Field (`Ctrl+L`).
   - Bookmark Star Toggle (Filled amber when current page is bookmarked).
3. **Right Action Cluster**:
   - History Icon (`Ctrl+H`): Quick popup & tab trigger.
   - Downloads Icon (`Ctrl+J`): Quick popup with badge indicator when downloads are active.
   - Bookmarks Icon (`Ctrl+Shift+O`): Quick popup & manager trigger.
   - Profile Avatar: Active profile indicator and fast switcher.
   - Overflow / More (⋮): Comprehensive browser menu.

### 17.2 Responsive Toolbar Behavior (No Two-Line Wrapping)
- The toolbar must remain a single, coherent row at all practical desktop window widths (down to minimum window width of 900px).
- The omnibox acts as the flexible `flex-1` region, shrinking gracefully as window width decreases.
- If window width reaches extreme constraints, secondary action icons move cleanly into the Overflow (⋮) menu rather than wrapping into an awkward two-line dashboard.

---

## 18. Internal Browser Pages (`edith://`)

Internal pages are rendered directly by React inside the viewport container with native child webviews hidden for that specific tab:

1. **`edith://newtab`**: Minimal start page with Google search and quick shortcut tiles.
2. **`edith://history`**: Full searchable history log with date clustering, individual item deletion, and clear all history actions.
3. **`edith://bookmarks`**: Bookmark manager with search, folder organization, URL editing, and deletion.
4. **`edith://downloads`**: Dedicated download audit panel with progress tracking, open file, and show-in-folder actions.
5. **`edith://settings`**: Browser-specific configuration panel.

---

## 19. Geometry, Bounds & Edge Repositioning

1. **Deterministic Coordinate Calculation**:
   - `syncBounds` calculates coordinates strictly from `viewportRef.current.getBoundingClientRect()`.
   - Because no in-flow drawers exist, `rect.top` remains constant across all popup and menu invocations.
2. **Window State Transitions**:
   - Window resize, maximize, and restore events trigger debounced bounds updates via `ResizeObserver` and native window event listeners.
   - Sub-pixel rounding issues are prevented via deterministic `Math.round()`.
3. **Smart Screen-Edge Repositioning for Popups**:
   - Tier B rich popups compute trigger element rects and available screen space.
   - If a trigger button is close to the right or bottom screen edge, the popup flips alignment (e.g. aligns right edges, or clamps coordinates) to prevent off-screen clipping without shifting the underlying webpage.

---

## 20. Input, Focus & Webpage Passthrough

1. **Focus Handoff**:
   - Clicking inside the native child webview transfers keyboard and mouse focus seamlessly to the remote document.
   - Pressing `Ctrl+L` transfers focus back to the parent window and selects the omnibox content.
2. **Escape Key Handling**:
   - Pressing `Escape` while an omnibox is focused restores original URL and blurs input.
   - Pressing `Escape` while a native popup is open closes the popup without affecting the active webview.
3. **Webpage Keystroke Passthrough Invariant**:
   - Normal typing inside web applications (e.g. Google Docs, YouTube search, code editors, textareas, contenteditable elements) is never intercepted by browser shortcuts.

---

## 21. Browser Keyboard Shortcut Matrix

| Shortcut | Description | Implementation Status | Focus / Passthrough Condition |
| :--- | :--- | :--- | :--- |
| **Ctrl+L** / **Alt+D** | Focus and select Omnibox address | To be implemented | Global in browser view; transfers focus to omnibox |
| **Alt+Left** | Navigate Back | To be implemented | Global; passthrough when editing text inside webpage |
| **Alt+Right** | Navigate Forward | To be implemented | Global; passthrough when editing text inside webpage |
| **Ctrl+R** / **F5** | Reload active tab | Already implemented | Global browser shortcut |
| **Ctrl+T** | Open New Tab | Already implemented | Global browser shortcut |
| **Ctrl+W** | Close active tab | Already implemented | Global browser shortcut |
| **Ctrl+Shift+T** | Reopen last closed tab | Already implemented | Global browser shortcut |
| **Ctrl+Tab** | Switch to next tab | Already implemented | Global browser shortcut |
| **Ctrl+Shift+Tab** | Switch to previous tab | Already implemented | Global browser shortcut |
| **Ctrl+H** | Open History (Flyout / Manager) | To be implemented | Global browser shortcut |
| **Ctrl+J** | Open Downloads (Flyout / Manager) | To be implemented | Global browser shortcut |
| **Ctrl+D** | Bookmark current page | Already implemented | Global browser shortcut |
| **Ctrl+Shift+O** | Open Bookmark Manager | To be implemented | Global browser shortcut |
| **Ctrl+F** | Find in page | Already implemented | Global browser shortcut |
| **Ctrl+P** | Print active tab | Already implemented | Global browser shortcut |
| **Ctrl++** | Zoom in (increment 10%) | Already implemented | Global browser shortcut |
| **Ctrl+-** | Zoom out (decrement 10%) | Already implemented | Global browser shortcut |
| **Ctrl+0** | Reset zoom to 100% | Already implemented | Global browser shortcut |

---

## 22. Performance & Stability Invariants

- **Zero Webpage Reloads**: Opening, closing, or hovering browser menus/popups shall never cause the active remote webpage to reload.
- **Zero WebView2 Recreation**: Native child webviews are not destroyed or recreated during popup lifecycles.
- **No Native Window / Surface Leaks**: Popup instances, submenus, and event listeners must be properly disposed on close; no orphaned HWNDs or detached listeners.
- **No Layout Thrashing**: Viewport bounds are synchronized on genuine resize events, not on popup hover or focus events.
- **Stress-Test Stability**: Rapidly opening and closing popups 10 times consecutively must maintain 60fps UI responsiveness without memory accumulation or visual tearing.

---

## 23. Code Cleanup

- Remove obsolete state hooks in `BrowserView.tsx` (`showAgentPanel`, `showOrchestratorPanel`, `showActionPanel`, `showRiskPanel`, `selectedAction`, etc.).
- Remove unused icons and dead imports from `lucide-react`.
- Delete obsolete inline drawer markup and dead CSS utility classes.
- Ensure 100% clean TypeScript compilation with zero lint warnings.

---

## 24. Testing Strategy

### 24.1 Automated Validation
1. **Frontend Compilation**: `npm run build` (`tsc && vite build`) must execute with **0 errors**.
2. **Backend Compilation**: `cargo check --manifest-path src-tauri/Cargo.toml` must execute with **0 errors**.

### 24.2 Functional Test Suite
- Tab lifecycle: creation, closure, duplication, pinning, group collapse/expand.
- Navigation lifecycle: direct URL, query search, redirect tracking, back/forward history stack.
- Omnibox fidelity: URL formatting, text selection, copy/paste, `Ctrl+L`, Escape restore.
- Search engine persistence: engine switch, app termination, relaunch verification.
- Internal tabs: `edith://` route dispatch and React manager rendering.

---

## 25. Screenshot & Visual QA Loop (Evidence-Based Acceptance)

Visual quality cannot be guaranteed by compilation alone. An iterative screenshot verification loop is strictly mandated during implementation.

```
┌─────────────────────────────────────────────────────────────┐
│ 1. REPRODUCE BROWSER STATE                                  │
│    Launch desktop app -> Navigate to target view state      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. CAPTURE HIGH-RES DESKTOP SCREENSHOT                      │
│    Capture full application window to artifacts directory   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. VISUAL DEFECT INSPECTION                                 │
│    Inspect via view_file for:                               │
│    • Webpage displacement / layout shifts                   │
│    • WebView2 blanking or black screen                      │
│    • Unanchored or clipped popups                           │
│    • Visual seams, nested frames, or fake borders           │
│    • Toolbar button crowding or text pills                  │
│    • Stale or truncated omnibox URL                         │
└──────────────────────────────┬──────────────────────────────┘
                               │
                ┌──────────────┴──────────────┐
                ▼                             ▼
        [Defects Found]               [Passes Standard]
                │                             │
                ▼                             ▼
┌──────────────────────────────┐  ┌───────────────────────────┐
│ 4. REFINE & RE-TEST          │  │ PROCEED TO NEXT STATE     │
│    Adjust code -> Rebuild -> │  │ Record baseline comparison│
│    Retake screenshot         │  └───────────────────────────┘
└──────────────────────────────┘
```

### 25.1 Mandatory 28-State Screenshot Matrix
1. **New Tab Page**: Clean start page, Google search box, quick launch tiles, zero AI clutter.
2. **Real Webpage**: Active site (e.g. Wikipedia), compact toolbar, fixed chrome, webpage starting directly below chrome.
3. **History Quick Popup**: Quick flyout over Wikipedia; Wikipedia completely visible and unshifted.
4. **Bookmarks Quick Popup**: Starred bookmarks list floating over active webpage.
5. **Downloads Quick Popup**: Recent download items floating over active webpage.
6. **Profiles Quick Popup**: Active profile checked, switch options floating over active webpage.
7. **Security/Privacy Popup**: HTTPS connection details and shield status over active page.
8. **Overflow (More ⋮) Menu**: Comprehensive menu over active page.
9. **Tab Context Menu**: Right-click on tab; native menu over active page.
10. **Global Model Selector Menu**: TopHudBar model menu over active page.
11. **History Manager Tab (`edith://history`)**: Full internal tab view with search.
12. **Bookmark Manager Tab (`edith://bookmarks`)**: Full internal tab view.
13. **Downloads Manager Tab (`edith://downloads`)**: Full internal tab view.
14. **Browser Settings Tab (`edith://settings`)**: Search engine selector and preferences.
15. **Focused Omnibox**: Full URL highlighted and ready for typing.
16. **Long URL Handling**: Clean display when unfocused; complete URL when focused.
17. **Search Query Submission**: Omnibox search navigating directly to Google search results.
18. **Multiple Tabs Open**: Clean tab strip with active tab styling and favicon display.
19. **Window Resize State**: Scaled down window with proper toolbar wrapping and webview tracking.
20. **Maximized Window State**: Edge-to-edge layout with zero visual gaps or seams.
21. **Popup Stress Testing**: Opening and closing History, Bookmarks, and Overflow 5 times rapidly.
22. **Post-Popup Stability**: Webpage scroll and video/media state verified identical before and after popup activation.
23. **Ctrl+L Behavior**: Address highlighted instantly upon shortcut execution.
24. **Browser Settings Persistence After Restart**: Settings verified after quitting and restarting app.
25. **Search Engine Persistence After Restart**: Search preference verified after quitting and restarting app.
26. **Toolbar at Narrow Window Width**: Single-row coherence verified at 900px width.
27. **Popup Near Right Screen Edge**: Verified repositioning without off-screen clipping.
28. **Popup Near Bottom Screen Edge**: Verified repositioning without off-screen clipping.

---

## 26. Acceptance Criteria

The browser overhaul is accepted **only** when all of the following criteria are met:

- [ ] Browser feels like one coherent desktop browser with zero artificial inner frames.
- [ ] E.D.I.T.H. identity remains unique; no Edge/Chrome cloning.
- [ ] Browser chrome stays fixed; webpage stays fixed.
- [ ] Opening any popup does NOT push webpage downward.
- [ ] Opening any popup does NOT resize webpage vertically.
- [ ] Opening any popup does NOT shift WebView2 bounds.
- [ ] Opening any popup does NOT hide WebView2.
- [ ] Opening any popup does NOT recreate WebView2.
- [ ] Opening any popup does NOT reload webpage.
- [ ] No visible flicker during popup lifecycle.
- [ ] History quick flyout works over active webpage.
- [ ] Bookmarks quick flyout works over active webpage.
- [ ] Downloads quick flyout works over active webpage.
- [ ] Profiles quick flyout works over active webpage.
- [ ] Security/privacy popup works over active webpage.
- [ ] Overflow menu works over active webpage.
- [ ] Tab context menu works over active webpage.
- [ ] Global Model selector retains valid native-menu behavior in browser view.
- [ ] Dedicated internal management pages exist (`edith://history`, `edith://bookmarks`, `edith://downloads`, `edith://settings`).
- [ ] Toolbar is icon-first, compact, and accessible.
- [ ] AI Agent removed from browser UI.
- [ ] Orchestrator removed from browser UI.
- [ ] Actions removed from browser UI.
- [ ] Observe removed from browser UI.
- [ ] Safety removed from browser UI.
- [ ] Grant AI / Take Control browser clutter removed.
- [ ] Shared backend capabilities in Rust are preserved intact.
- [ ] Real editable omnibox exists with full URL fidelity.
- [ ] Long URLs are usable and cleanly formatted.
- [ ] Ctrl+L shortcut works reliably.
- [ ] Back/Forward navigation synchronizes omnibox URL.
- [ ] Tab switching synchronizes omnibox URL.
- [ ] Server-side and client-side redirects synchronize omnibox URL.
- [ ] Search query classification works seamlessly.
- [ ] Google is default search engine.
- [ ] Search engine preference is configurable.
- [ ] Search engine preference persists after application restart.
- [ ] Frontend/backend search engine behavior cannot diverge.
- [ ] Browser Settings exists and is separate from global E.D.I.T.H. Settings.
- [ ] New-tab page is clean, minimal, and branded.
- [ ] New-tab search uses configured search engine.
- [ ] Narrow-width toolbar remains coherent without two-line wrapping.
- [ ] Popups near screen edges reposition intelligently without clipping.
- [ ] All 20 browser keyboard shortcuts work properly.
- [ ] Webpage keyboard input is preserved in web forms and editors.
- [ ] Browser popup stress testing passes with zero leaks.
- [ ] Window resize, maximize, and restore maintain exact bounds.
- [ ] Browser content geometry remains completely stable.
- [ ] Complete 28-state screenshot matrix is captured.
- [ ] Baseline and post-fix screenshots are visually compared.
- [ ] Visual defects are explicitly logged, addressed, and verified.
- [ ] Final visual inspection passes.

---

## 27. Risks and Mitigations

| Risk | Mitigation |
| :--- | :--- |
| **Airspace Occlusion on Custom HTML Popups** | Deploy top-level native surfaces (OS menus and frameless top-level popup windows) that render at the DWM level above child WebView2 HWNDs. Avoid in-flow DOM elements above the viewport. |
| **Frontend/Backend Search Engine Divergence** | Establish a single source of truth in `browserController`, update hardcoded DuckDuckGo fallback in `src-tauri/src/browser.rs` line 366, and pass the selected engine preference across IPC. |
| **Accidental Deletion of Shared AI Primitives** | Perform strict dependency tracing. Keep all Rust backend commands in `browser_agent.rs`, `browser_orchestrator.rs`, and `browser_tools.rs`. Remove only frontend UI bindings in `BrowserView.tsx`. |
| **Keyboard Shortcut Collisions with Web Apps** | Check event targets and active element types (`INPUT`, `TEXTAREA`, `contenteditable`) before intercepting navigation shortcuts to preserve native web application typing. |
| **Popup Edge Clipping on Small Displays** | Compute trigger button bounding rects against window viewport dimensions and clamp/flip popup alignment near right and bottom boundaries. |
| **Unrelated Code Regression** | Strictly enforce the out-of-scope boundary. Prohibit modifications to TTS, Chat, Voice, Terminal, Memory, or global application settings. |

---

## 28. Files Likely to Change

- `src/views/BrowserView.tsx` (Toolbar redesign, popup integration, omnibox overhaul, clutter removal, internal tab rendering)
- `src/services/browserController.ts` (Search engine registry, persistent preference, URL normalization, internal route handling)
- `src-tauri/src/browser.rs` (Backend URL normalization update, internal `edith://` route lifecycle handling)
- `src/types.ts` (Search engine configuration types, browser preference interfaces)

---

## 29. Files That Must NOT Change

- `src-tauri/src/tts.rs` (Audio synthesis & Rodio playback — UNTOUCHED)
- `src/services/tauri.ts` (TTS audio player & global IPC — UNTOUCHED)
- `src/views/ChatView.tsx` (Chat sessions & message state — UNTOUCHED)
- `src/views/SettingsView.tsx` (Global application settings — UNTOUCHED)
- `src/views/VoiceView.tsx` (Voice assistant — UNTOUCHED)
- `src/views/TerminalView.tsx` (Terminal execution — UNTOUCHED)
- `src/views/DataView.tsx` (Data workspace — UNTOUCHED)
- `src-tauri/src/memory.rs` (Vector memory & LanceDB — UNTOUCHED)

---

## 30. Final Implementation Loop & Verification Protocol

The future implementation agent must execute the overhaul according to this strict protocol:

1. **Baseline State Verification**: Audit current browser state, launch local dev environment, and capture initial baseline screenshots.
2. **Atomic Implementation**: Execute the browser changes according to the 4-tier popup model, fixed chrome hierarchy, real omnibox, and search engine architecture.
3. **Automated Validation**: Run `npm run build` and `cargo check --manifest-path src-tauri/Cargo.toml` to verify zero compiler errors.
4. **Desktop Execution**: Launch the application in native desktop mode.
5. **Visual Capture & Defect Logging**: Execute the 28-state screenshot matrix. Inspect each screenshot for layout shifts, clipping, blanking, or alignment defects.
6. **Defect Remediation**: Fix all identified visual defects and re-execute screenshots until every state satisfies acceptance criteria.
7. **Final Acceptance**: Confirm all 55 acceptance checkboxes pass before declaring completion.
