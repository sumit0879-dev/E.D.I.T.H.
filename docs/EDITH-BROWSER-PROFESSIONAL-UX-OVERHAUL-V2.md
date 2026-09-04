# E.D.I.T.H. Browser — Professional Browser UX / Airspace / UI / Architecture Overhaul (Version 2)

---

## 1. Scope

This document specifies the complete architectural and user experience overhaul of the **E.D.I.T.H. Browser module**.

The objective is to establish an interaction model, visual stability, and layout behavior comparable to a modern desktop browser (such as Microsoft Edge or Google Chrome) while preserving E.D.I.T.H.'s distinct visual identity and existing valid features.

### In Scope
- Browser chrome layout, visual hierarchy, and toolbar geometry.
- Complete resolution of the Win32 WebView2 airspace and webpage displacement defects.
- Categorization and implementation strategy for all browser popups, menus, and flyouts.
- Implementation of a real, interactive, synchronized omnibox (address bar).
- End-to-end configurable search engine architecture (Google default, DuckDuckGo, Bing) with persistent storage.
- Architecture and implementation of dedicated internal browser pages (`edith://newtab`, `edith://history`, `edith://bookmarks`, `edith://downloads`, `edith://settings`).
- Elimination of developer/agent console clutter from user-facing browser surfaces.
- Comprehensive visual regression testing and screenshot-driven verification loop.

### Strict Scope Boundary
This overhaul applies **exclusively** to the browser subsystem. No unrelated application modules shall be modified, refactored, or cleaned up.

---

## 2. Non-Goals

1. **No Cloning of Edge or Chrome Assets**:
   - Do not copy proprietary branding, logos, icons, trademarks, artwork, color schemes, or proprietary source code from Google Chrome, Chromium, or Microsoft Edge. The goal is behavioral and interaction excellence, not visual plagiarism.
2. **No Modifications to Unrelated E.D.I.T.H. Features**:
   - Absolutely no changes to Voice/TTS synthesis, Chat sessions, AI Core reasoning, Memory/LanceDB indexing, Terminal execution, Data connectors, or Global application settings.
   - Preserves all TTS and audio playback improvements from previous revisions.
3. **No Blind Deletion of Shared AI Capabilities**:
   - Underlying autonomous agent tools, multi-tab orchestration primitives, and safety audit logging in the Rust backend (`browser_agent.rs`, `browser_orchestrator.rs`, `browser_tools.rs`) remain intact for programmatic AI agent workflows. Only browser-specific manual UI clutter is removed from the user shell.
4. **No Mandatory Full-Engine Migration to CompositionController**:
   - Avoid unnecessary architectural risk. CompositionController shall remain a conditional strategy evaluated only where strict requirements cannot be fulfilled by native top-level surfaces and native menus.
5. **No Manipulation of Webpage DOM Content**:
   - Do not inject CSS or scripts that modify remote websites rendered inside child WebView2 instances, except for the existing reader mode and DOM accessibility observer scripts.

---

## 3. Current Architecture

### 3.1 Component & Service Map
- **Frontend View (`src/views/BrowserView.tsx`)**:
  - Acts as a hybrid React container managing the tab bar, navigation bar, omnibox, drawer panels, and the native viewport mounting canvas (`#edith-browser-viewport-container`).
- **Global HUD (`src/components/TopHudBar.tsx`)**:
  - Houses the global model selector. Currently triggers native OS menus when in browser view to prevent webview occlusion.
- **Controller Layer (`src/services/browserController.ts`)**:
  - Manages tab state, navigation dispatch, viewport bounds synchronization, bookmarks, downloads, and profile state.
- **Tauri IPC Bridge (`src/services/tauri.ts`)**:
  - Exposes typed invoke functions for Rust backend browser commands.
- **Rust Core (`src-tauri/src/browser.rs`)**:
  - Manages native Win32 `tauri::Webview` child instances attached to the main window (`window.add_child(...)`).
  - Stores tab metadata, viewport bounds (`BrowserViewportBounds`), session history, and IPC listeners.

### 3.2 Current Window & Hierarchy Model
```
Main Tauri Window [HWND]
├── Parent Webview (E.D.I.T.H. Shell / React DOM)
│   ├── TopHudBar (Global Header)
│   ├── Sidebar (Global Navigation)
│   └── BrowserView (Browser Shell)
│       ├── Tab Strip (React DOM)
│       ├── HUD Toolbar & Omnibox (React DOM)
│       ├── [IN-FLOW DRAWERS: Downloads, Bookmarks, Profiles, History, Risk, Agent HUD]
│       └── Viewport Container Div (Ref target for bounds)
└── Child Webview [HWND] (Tauri Webview / msedgewebview2.exe)
    └── Active Remote Webpage (e.g. Wikipedia, Google)
```

---

## 4. Root Cause Analysis

### 4.1 Vertical Webpage Displacement (Layout Shifting)
In [`BrowserView.tsx`](file:///e:/Projects/E.D.I.T.H/src/views/BrowserView.tsx#L2425-L2850), panels for Downloads, Bookmarks, Profiles, History, and Risk Audit were implemented as conditional, in-flow `<div>` elements inserted directly between the toolbar and `viewportRef`.
- When a user clicked "History", React mounted a 250px–320px high in-flow block.
- This shifted `viewportRef.current.getBoundingClientRect().top` downward by that exact height.
- The bounds synchronization loop (`syncBounds`) immediately adjusted the child Win32 WebView2 position downward, creating the forbidden "toolbar → history panel → pushed webpage" layout.

### 4.2 Win32 Airspace Occlusion
On Windows, child WebView2 instances created via `window.add_child` are native Win32 child windows (`Intermediate D3D Window`).
- Any HTML element rendered by the parent React webview at coordinates overlapping the child HWND is occluded because the child window draws on top of the parent window's DirectComposition/GDI surface.
- Past revisions attempted to work around this by invoking `browserController.hideAll()` or `webview.hide()`, which caused the active webpage to turn black or flicker violently whenever menus opened.

### 4.3 Cluttered Toolbar & Start Page
The browser toolbar currently contains over 15 text-heavy pills and developer toggles (`AI Agent (4C)`, `Orchestrator (5.4)`, `Actions (4A)`, `Observe`, `Safety (5.3)`, `Take Control`). The new tab page similarly hosts redundant drawer buttons and debug widgets, presenting an engineering console rather than a consumer browser.

### 4.4 Hardcoded Search Fallback Across Stack
Search query fallback to DuckDuckGo was hardcoded independently in three places:
1. `src/services/browserController.ts` (`normalizeBrowserUrl`)
2. `src/views/BrowserView.tsx` (`handleNavigate` and search inputs)
3. `src-tauri/src/browser.rs` (`normalize_url`)
There was no unified registry, user preference persistence, or configuration UI.

---

## 5. Target Browser Architecture

The target architecture establishes a clean, unified browser surface where the browser chrome remains fixed, the webpage remains fixed and stable, and all popups float gracefully over the surface.

```
┌────────────────────────────────────────────────────────────┐
│ Top Window Titlebar / Global TopHudBar                    │
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
   Simple Menus:                      Rich Popups:
   • Tab Context Menu                 • History Quick Flyout
   • Model Switcher                   • Bookmarks Quick Flyout
   • Search Engine Selector           • Downloads Quick Flyout
   • Overflow Menu (⋮)                • Profiles Quick Flyout
```

---

## 6. Browser Chrome Architecture

1. **Fixed Chrome Invariant**:
   - The browser chrome consists of two immutable vertical sections:
     - Top Tab Strip (38px height)
     - Navigation & Omnibox Toolbar (42px height)
   - Chrome elements never expand, collapse, or inject vertical blocks into the document flow during popup activation.
2. **Coherent Single Surface**:
   - The webpage viewport container begins at `top: 80px` (or immediately below the toolbar border) with `0px` margin, `0px` padding, and `0px` artificial border framing.
   - Eliminates nested cards, dashboard drop-shadows, and fake inner borders around the viewport.
   - Visually presents a single continuous application window.

---

## 7. WebView2 Content Architecture

1. **Active Webview Stability**:
   - The active child WebView2 maintains a 1-to-1 geometric match with the viewport container div (`#edith-browser-viewport-container`).
   - The native child webview is never hidden, shifted, resized, destroyed, or recreated when opening any popup, menu, or dialog.
2. **State & Media Preservation**:
   - Remote page DOM, active audio/video playback, WebGL context, form entries, JavaScript timers, and scroll position remain completely uninterrupted during all browser chrome interactions.

---

## 8. Popup & Menu Architecture

To avoid the binary trap of "everything must be a native menu" or "everything must be React in-flow", browser interactions are categorized into four distinct architectural tiers:

### Tier A: Simple Command & Context Menus
- **Use Case**: Tab context menu, Global Model switcher, Browser Overflow (⋮) menu, Search Engine switcher.
- **Mechanism**: Tauri Native OS Menus (`@tauri-apps/api/menu` / Win32 `TrackPopupMenuEx`).
- **Rationale**: Native menus are OS-level topmost windows (`#32768`) drawn directly by the Desktop Window Manager (DWM). They float natively above child WebView2 HWNDs with zero airspace occlusion, zero layout shift, and instant response.

### Tier B: Rich Browser Popups / Flyouts
- **Use Case**: Quick History flyout (with search and recent items), Quick Bookmarks flyout, Quick Downloads flyout (with progress indicators and open buttons), Quick Profiles flyout, Site Security flyout.
- **Mechanism**:
  - *Option 1 (Primary)*: Anchored Native Menu Hierarchies with dynamic submenus and action shortcuts (e.g. search prompt, recent items, clear actions, and direct jump to full manager).
  - *Option 2 (Rich Floating Surface)*: Top-level frameless popup window or native overlay anchored directly beneath the trigger icon, capturing focus and auto-closing on blur or `Escape`.
- **Constraint**: Under no circumstances shall an in-flow DOM element be inserted above the viewport container.

### Tier C: Full Management Pages
- **Use Case**: Comprehensive history search & date filtering, bookmark tree reorganization, complete download history audit, extensible browser preferences.
- **Mechanism**: Dedicated internal browser tabs running internal routes (`edith://history`, `edith://bookmarks`, `edith://downloads`, `edith://settings`).
- **Behavior**: Clicking "Manage all..." from any quick popup or pressing standard shortcuts (`Ctrl+H`, `Ctrl+J`, `Ctrl+Shift+O`) navigates to or focuses the corresponding internal tab.

### Tier D: Advanced In-Page Overlays (Conditional)
- **Use Case**: Find in page HUD (`Ctrl+F`), save-page toast notifications, isolated reader mode.
- **Mechanism**:
  - For Find in page: Native WebView2 script injection bridge or high-Z floating toolstrip anchored strictly within chrome bounds.
  - Reader Mode: Isolated full-viewport rendering surface loaded when the active tab is in reader mode.

---

## 9. Airspace Strategy

### 9.1 The Airspace Rule
Under Win32 architecture, child HWNDs draw on top of parent HTML content. 
The following solutions are strictly forbidden:
- ❌ `browserController.hideAll()`
- ❌ `webview.hide()` on popup trigger
- ❌ Recreating WebView2 instances to reset Z-order
- ❌ Shrinking or shifting `viewportRef` bounds

### 9.2 The Valid Airspace Solutions
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

## 11. Omnibox Architecture

### 11.1 Behavioral Specifications
- **Real Interactive Surface**: An actual HTML `<input>` element with continuous state binding, full caret positioning, click-to-select, copy, paste, and text selection.
- **Display vs. Edit State**:
  - *Unfocused*: Displays clean, human-readable URL (or internal tab title), hiding unnecessary tracking query parameters if configured, and showing site security indicators (HTTPS lock / alert).
  - *Focused*: Exposes the complete, unadorned URL string with automatic full selection for immediate typing or replacement.
- **Keyboard Shortcuts**:
  - `Ctrl+L` / `Alt+D`: Instantly focuses omnibox and highlights the entire address.
  - `Escape`: Reverts edited text back to active tab's current URL and blurs omnibox.
  - `Enter`: Commits navigation.
- **Input Classification**:
  - Direct URL detection (protocol prefixes `https://`, `http://`, domain heuristics `example.com`, `localhost:port`, internal `edith://`).
  - Search query fallback: Plain text queries automatically dispatch to the active search engine URL format.
- **Synchronization Guarantee**:
  - Omnibox updates automatically on remote page redirects, pushState/replaceState navigation, Back/Forward actions, and tab switches.
  - Never displays internal Tauri labels (e.g. `edith_tab_tab_xyz`) or memory pointers.

---

## 12. Search Engine Architecture

### 12.1 Single Source of Truth
The selected search engine must be centrally managed across the application.

```typescript
export interface SearchEngineConfig {
  id: string;
  name: string;
  searchUrlTemplate: string; // e.g. "https://www.google.com/search?q=%s"
  homepageUrl: string;
}

export const SEARCH_ENGINES: Record<string, SearchEngineConfig> = {
  google: {
    id: 'google',
    name: 'Google',
    searchUrlTemplate: 'https://www.google.com/search?q=%s',
    homepageUrl: 'https://www.google.com',
  },
  duckduckgo: {
    id: 'duckduckgo',
    name: 'DuckDuckGo',
    searchUrlTemplate: 'https://duckduckgo.com/?q=%s',
    homepageUrl: 'https://duckduckgo.com',
  },
  bing: {
    id: 'bing',
    name: 'Bing',
    searchUrlTemplate: 'https://www.bing.com/search?q=%s',
    homepageUrl: 'https://www.bing.com',
  },
};
```

### 12.2 End-to-End Tracing & Hardcoded Fallback Removal
1. **Frontend (`browserController.ts`)**:
   - `normalizeBrowserUrl(input: string, engineId?: string)` replaces hardcoded DuckDuckGo with active search engine from persistent storage.
   - Persistence key: `edith_browser_search_engine` (defaults to `'google'`).
2. **Frontend View (`BrowserView.tsx`)**:
   - Omnibox submission and New Tab central search use `browserController.getSearchEngine()`.
   - Omnibox placeholder dynamically reflects engine name (e.g. `"Search with Google or enter address..."`).
3. **Backend Rust Core (`src-tauri/src/browser.rs`)**:
   - Audit and update line 366 of `browser.rs` where DuckDuckGo was hardcoded.
   - Ensure backend URL normalization respects the user's configured search preference or defaults to Google HTTPS query format.

---

## 13. Browser Settings Architecture

### 13.1 Strict Separation of Concerns
- **Global E.D.I.T.H. Settings** (`src/views/SettingsView.tsx`): Manages AI models, API keys, system speech synthesis, and application appearance. Remains untouched.
- **Browser Settings** (`edith://settings`): Manages browser-specific functionality only.

### 13.2 Extensible Browser Settings Schema
- **Search Preferences**: Default search engine selection (Google, DuckDuckGo, Bing).
- **Startup Behavior**: Open new tab page vs. Restore previous session tabs.
- **Privacy & Content**: Content blocking / tracking protection toggle, clear browsing data (history, cache, cookies).
- **Appearance & Navigation**: Show/hide bookmarks bar, default zoom level (100%).

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

## 15. AI / Developer Clutter Removal

### 15.1 Elements to Remove from Browser UI
The following user-facing controls and panels are removed from `BrowserView.tsx`:
- `AI Agent (4C)` toolbar button and inline drawer
- `Orchestrator (5.4)` toolbar button and inline drawer
- `Actions (4A)` playground button and inline drawer
- `Observe` DOM inspector button and cards
- `Safety (5.3)` risk audit button and drawer
- `Grant AI` / `Take Control` takeover pills
- Bottom debug status panels and screenshot preview bars

### 15.2 Backend Preservation (Dependency Tracing)
- All corresponding Rust backend commands in `src-tauri/src/browser_agent.rs`, `browser_orchestrator.rs`, and `browser_tools.rs` are **retained** without deletion to ensure automated or headless E.D.I.T.H. background capabilities remain functional.
- Only frontend browser UI bindings and dead imports are cleaned up.

---

## 16. Browser Toolbar Redesign

The toolbar transitions from a text-heavy dashboard row to a compact, icon-first desktop browser toolbar:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [◀] [▶] [↻]  [🔒 https://en.wikipedia.org/wiki/Rust  ★]  [⏱] [↓] [★] [👤] [⋮]       │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

1. **Left Navigation Cluster**:
   - Back button (`Alt+Left`) with disabled state when history stack is empty.
   - Forward button (`Alt+Right`) with disabled state.
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

---

## 17. Internal Browser Pages (`edith://`)

Internal pages are rendered directly by React inside the viewport container with native child webviews hidden for that specific tab:

1. **`edith://newtab`**: Minimal start page with Google search and quick shortcut tiles.
2. **`edith://history`**: Full searchable history log with date clustering, individual item deletion, and clear all history actions.
3. **`edith://bookmarks`**: Bookmark manager with search, folder organization, URL editing, and deletion.
4. **`edith://downloads`**: Dedicated download audit panel with progress tracking, open file, and show-in-folder actions.
5. **`edith://settings`**: Browser-specific configuration panel.

---

## 18. Geometry & Bounds Strategy

1. **Deterministic Coordinate Calculation**:
   - `syncBounds` calculates coordinates strictly from `viewportRef.current.getBoundingClientRect()`.
   - Because no in-flow drawers exist, `rect.top` remains constant across all popup and menu invocations.
2. **Window State Transitions**:
   - Window resize, maximize, and restore events trigger debounced bounds updates via `ResizeObserver` and native window event listeners.
   - Sub-pixel rounding issues are prevented via deterministic `Math.round()`.
3. **DPI & Multi-Monitor Scaling**:
   - Tauri logical coordinates are preserved to prevent physical pixel scaling drift on high-DPI Windows displays.

---

## 19. Input & Focus Behavior

1. **Focus Handoff**:
   - Clicking inside the native child webview transfers keyboard and mouse focus seamlessly to the remote document.
   - Pressing `Ctrl+L` transfers focus back to the parent window and selects the omnibox content.
2. **Escape Key Handling**:
   - Pressing `Escape` while an omnibox is focused restores original URL and blurs input.
   - Pressing `Escape` while a native popup is open closes the popup without affecting the active webview.
3. **Webpage Keystroke Passthrough**:
   - Normal typing inside web applications (e.g. Google Docs, YouTube search, code editors) is never intercepted by browser shortcuts.

---

## 20. Accessibility

- All icon buttons in the toolbar include descriptive `aria-label` attributes and native HTML `title` tooltips displaying shortcut keys (e.g. `"Back (Alt+Left)"`, `"Reload (Ctrl+R)"`).
- Focus rings conform to high-contrast cyan/emerald themes without breaking keyboard-only navigation.
- Native menus leverage standard Windows accessibility APIs for screen readers.

---

## 21. Error Handling

- **Invalid URL Input**: Graciously converts non-URL text to active search engine queries.
- **Failed Navigation**: Renders clean internal error state rather than crashing or freezing webview.
- **Database/Storage Faults**: History and bookmark SQLite read/write errors are logged with graceful in-memory fallbacks to prevent UI blocking.

---

## 22. Code Cleanup

- Remove obsolete state hooks in `BrowserView.tsx` (`showAgentPanel`, `showOrchestratorPanel`, `showActionPanel`, `showRiskPanel`, `selectedAction`, etc.).
- Remove unused icons and dead imports from `lucide-react`.
- Delete obsolete inline drawer markup and dead CSS utility classes.
- Ensure 100% clean TypeScript compilation with zero lint warnings.

---

## 23. Testing Strategy

### 23.1 Automated Validation
1. **Frontend Compilation**: `npm run build` (`tsc && vite build`) must execute with **0 errors**.
2. **Backend Compilation**: `cargo check --manifest-path src-tauri/Cargo.toml` must execute with **0 errors**.

### 23.2 Functional Test Suite
- Test tab creation, closure, duplication, pinning, and group management.
- Test URL navigation, back, forward, reload, and redirect tracking.
- Test bookmark addition, deletion, and status synchronization.
- Test download tracking, cancellation, file opening, and folder display.
- Test search engine selection persistence across app reloads.

---

## 24. Screenshot & Visual QA Loop

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

### 24.1 Mandatory 22-State Screenshot Matrix
1. **New Tab Page**: Clean start page, Google search box, quick launch tiles, zero AI clutter.
2. **Real Webpage**: Active site (e.g. Wikipedia), compact toolbar, fixed chrome, webpage starting directly below chrome.
3. **History Quick Popup**: Native menu floating over Wikipedia; Wikipedia completely visible and unshifted.
4. **Bookmarks Quick Popup**: Starred bookmarks list floating over active webpage.
5. **Downloads Quick Popup**: Recent download items floating over active webpage.
6. **Profiles Quick Popup**: Active profile checked, switch options floating over active webpage.
7. **Site Security Popup**: HTTPS connection details and shield status over active page.
8. **Overflow (More ⋮) Menu**: Comprehensive menu over active page.
9. **Tab Context Menu**: Right-click on tab; native menu over active page.
10. **Model Selector Menu**: TopHudBar model menu over active page.
11. **History Manager Tab (`edith://history`)**: Full internal tab view with search.
12. **Bookmark Manager Tab (`edith://bookmarks`)**: Full internal tab view.
13. **Downloads Manager Tab (`edith://downloads`)**: Full internal tab view.
14. **Browser Settings Tab (`edith://settings`)**: Search engine selector and preferences.
15. **Focused Omnibox**: Full URL highlighted and ready for typing.
16. **Long URL Handling**: Clean truncation when unfocused; complete URL when focused.
17. **Search Query Submission**: Omnibox search navigating directly to Google search results.
18. **Multiple Tabs Open**: Clean tab strip with active tab styling and favicon display.
19. **Window Resize State**: Scaled down window with proper toolbar wrapping and webview tracking.
20. **Maximized Window State**: Edge-to-edge layout with zero visual gaps or seams.
21. **Popup Stress Testing**: Opening and closing History, Bookmarks, and Overflow 5 times rapidly.
22. **Post-Popup Stability**: Webpage scroll and video/media state verified identical before and after popup activation.

---

## 25. Acceptance Criteria

The browser overhaul is accepted **only** when all of the following criteria are met:

- [ ] Browser feels like **one coherent desktop application surface** with zero artificial inner frames.
- [ ] Active webpage **never disappears, blanks, or turns black** when opening any menu or popup.
- [ ] Active webpage **never moves downward or resizes** when opening any menu or popup.
- [ ] Active WebView2 is never hidden or destroyed merely to display a browser menu.
- [ ] History, Bookmarks, Downloads, Profiles, Security, and Overflow behave as floating popups.
- [ ] Dedicated internal tabs exist for full management (`edith://history`, `edith://bookmarks`, `edith://downloads`, `edith://settings`).
- [ ] Tab context menu and TopHudBar model menu retain verified native OS menu behavior.
- [ ] All browser-toolbar AI buttons (`AI Agent (4C)`, `Orchestrator`, `Actions`, `Observe`, `Safety`, `Grant AI`) are removed from the browser UI.
- [ ] Underlying shared backend commands in Rust are preserved intact.
- [ ] Toolbar is compact, icon-first, with tooltips and accessible labels.
- [ ] Real omnibox is fully editable, supports `Ctrl+L`, text selection, copy/paste, and syncs on navigation/redirects/tab switch.
- [ ] Default search engine is Google, with persistent configuration for DuckDuckGo and Bing.
- [ ] All 22 screenshot states in the test matrix are visually verified and free of layout defects.

---

## 26. Risks and Mitigations

| Risk | Mitigation |
| :--- | :--- |
| **Airspace Occlusion on Custom HTML Popups** | Leverage Tauri Native OS Menus (`@tauri-apps/api/menu`) for floating toolbar popups, which render at the OS level directly above child WebView2 HWNDs, combined with full `edith://` internal tabs for rich management. |
| **Breaking Programmatic AI Agent Features** | Retain all backend Rust commands in `browser_agent.rs`, `browser_orchestrator.rs`, and `browser_tools.rs`. Remove only manual user-facing browser UI buttons in `BrowserView.tsx`. |
| **Search Engine Desynchronization** | Implement single source of truth in `browserController` persisted via `localStorage`, and update hardcoded DuckDuckGo fallback in `src-tauri/src/browser.rs`. |
| **Accidental Modification of Unrelated Code** | Enforce strict file boundaries. Prohibit edits to audio/TTS, Chat, Voice, Memory, or global application settings. |

---

## 27. Files Likely to Change

- `src/views/BrowserView.tsx` (Toolbar redesign, popup integration, omnibox overhaul, clutter removal, internal tab rendering)
- `src/services/browserController.ts` (Search engine registry, persistent preference, URL normalization, internal route handling)
- `src-tauri/src/browser.rs` (Backend URL normalization update, internal `edith://` route lifecycle handling)
- `src/types.ts` (Search engine configuration types, browser preference interfaces)

---

## 28. Files That Must NOT Change

- `src-tauri/src/tts.rs` (Audio synthesis & Rodio playback — UNTOUCHED)
- `src/services/tauri.ts` (TTS audio player & global IPC — UNTOUCHED)
- `src/views/ChatView.tsx` (Chat sessions & message state — UNTOUCHED)
- `src/views/SettingsView.tsx` (Global application settings — UNTOUCHED)
- `src/views/VoiceView.tsx` (Voice assistant — UNTOUCHED)
- `src/views/TerminalView.tsx` (Terminal execution — UNTOUCHED)
- `src/views/DataView.tsx` (Data workspace — UNTOUCHED)
- `src-tauri/src/memory.rs` (Vector memory & LanceDB — UNTOUCHED)

---

## 29. Final Verification Checklist

- [ ] Plan reviewed against Base Plan requirements (all base requirements preserved).
- [ ] Plan reviewed against Rule 1 through Rule 25.
- [ ] Distinct popup categories defined (native command menus vs. rich popups vs. internal manager tabs).
- [ ] End-to-end search engine architecture traced through frontend and backend.
- [ ] Complete 22-state screenshot matrix documented.
- [ ] No source code modified during this planning step.
- [ ] Plan saved as standalone Markdown document `docs/EDITH-BROWSER-PROFESSIONAL-UX-OVERHAUL-V2.md`.
