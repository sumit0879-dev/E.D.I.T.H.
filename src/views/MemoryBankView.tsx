import React, { useState, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import type { MemoryChunk } from '../types';
import {
  Brain,
  Search,
  Plus,
  Trash2,
  Database,
  Tag,
  Sparkles,
  RefreshCw,
  Layers,
  FileCode,
  Sliders,
  CheckCircle2,
} from 'lucide-react';

const knowledgeTemplates = [
  { label: 'Personal Context', source: 'user_profile', text: 'User is a senior software engineer developing high-performance Tauri applications with Rust and React.' },
  { label: 'Coding Guidelines', source: 'coding_rules', text: 'All TypeScript code must follow strict typing. Wrap all generated code in Markdown triple backticks.' },
  { label: 'Project Architecture', source: 'project_architecture', text: 'E.D.I.T.H. Mark-85 uses Tauri 2 for system-level APIs, SQLite for state persistence, LanceDB for vector memory, and EdgeTTS for voice.' },
];

export const MemoryBankView: React.FC = () => {
  const { showToast } = useApp();
  const [memories, setMemories] = useState<MemoryChunk[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<MemoryChunk[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [newText, setNewText] = useState('');
  const [newSource, setNewSource] = useState('user_knowledge');
  const [isAdding, setIsAdding] = useState(false);
  const [activeSourceFilter, setActiveSourceFilter] = useState<string>('all');

  useEffect(() => {
    loadMemories();
  }, []);

  const loadMemories = async () => {
    try {
      const list = await tauriService.getMemories();
      setMemories(list);
    } catch (e) {
      console.error('Failed to load memories:', e);
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }
    try {
      setIsSearching(true);
      const res = await tauriService.searchMemory(searchQuery.trim());
      setSearchResults(res);
    } catch (e: any) {
      showToast('Search error: ' + (e.message || e), 'error');
    } finally {
      setIsSearching(false);
    }
  };

  const handleAddMemory = async () => {
    if (!newText.trim()) return;
    try {
      setIsAdding(true);
      await tauriService.saveToMemory(newText.trim(), newSource.trim() || 'user_knowledge');
      setNewText('');
      showToast('Knowledge chunk embedded and stored in LanceDB!', 'success');
      loadMemories();
    } catch (e: any) {
      showToast('Save error: ' + (e.message || e), 'error');
    } finally {
      setIsAdding(false);
    }
  };

  const handleDelete = async (source: string) => {
    try {
      await tauriService.deleteMemory(source);
      showToast("Memories from '" + source + "' deleted", 'info');
      loadMemories();
      if (searchResults.length > 0) {
        handleSearch();
      }
    } catch (e: any) {
      showToast('Delete error: ' + (e.message || e), 'error');
    }
  };

  const baseList = searchQuery.trim() ? searchResults : memories;
  const filteredList = baseList.filter((m) => {
    if (activeSourceFilter === 'all') return true;
    return m.source.toLowerCase().includes(activeSourceFilter.toLowerCase());
  });

  const uniqueSources = Array.from(new Set(memories.map((m) => m.source.split(':')[0])));

  return (
    <div className="flex-1 flex flex-col h-full bg-[#000000] overflow-hidden">
      {/* Header */}
      <div className="p-4 border-b border-white/10 bg-[#0c121e] flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-2xl bg-amber-500/20 text-amber-400 border border-amber-500/30">
            <Brain className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-white flex items-center gap-2">
              <span>LanceDB Vector Memory Bank</span>
              <span className="text-[10px] bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded font-mono font-bold">
                384-dim Embeddings
              </span>
            </h3>
            <p className="text-xs text-slate-400">
              Persistent semantic knowledge retrieved automatically during conversations
            </p>
          </div>
        </div>

        <button
          onClick={loadMemories}
          className="px-3.5 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-semibold border border-white/10 flex items-center gap-1.5 transition"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          <span>Refresh Knowledge</span>
        </button>
      </div>

      {/* Main Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Side: Add Knowledge Form & Templates */}
        <div className="w-[400px] bg-[#0a0f1c] border-r border-white/10 p-5 flex flex-col justify-between overflow-y-auto custom-scrollbar space-y-4">
          <div className="space-y-4">
            <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-slate-300">
              <Plus className="w-4 h-4 text-amber-400" />
              <span>Embed New Knowledge</span>
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-400 block mb-1">
                Source Tag Name
              </label>
              <input
                type="text"
                value={newSource}
                onChange={(e) => setNewSource(e.target.value)}
                placeholder="e.g. project_docs, client_faq, personal"
                className="w-full px-3 py-2 rounded-xl glass-input text-xs font-mono text-slate-200"
              />
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-400 block mb-1">
                Knowledge Paragraph / Text
              </label>
              <textarea
                rows={5}
                value={newText}
                onChange={(e) => setNewText(e.target.value)}
                placeholder="Paste code documentation, project guidelines, user preferences or facts to remember..."
                className="w-full p-3 rounded-2xl glass-input text-xs text-slate-200 resize-none focus:outline-none"
              />
            </div>

            {/* Quick Templates */}
            <div>
              <label className="text-xs font-semibold text-slate-400 block mb-1.5 flex items-center gap-1">
                <Sparkles className="w-3.5 h-3.5 text-amber-400" />
                <span>Knowledge Presets</span>
              </label>
              <div className="space-y-1.5">
                {knowledgeTemplates.map((kt, i) => (
                  <button
                    key={i}
                    onClick={() => {
                      setNewSource(kt.source);
                      setNewText(kt.text);
                    }}
                    className="w-full p-2 rounded-xl bg-white/[0.02] hover:bg-white/[0.05] border border-white/5 text-left transition"
                  >
                    <div className="text-xs font-bold text-amber-300">{kt.label}</div>
                    <div className="text-[10px] text-slate-500 font-mono truncate">{kt.source}</div>
                  </button>
                ))}
              </div>
            </div>
          </div>

          <button
            onClick={handleAddMemory}
            disabled={isAdding || !newText.trim()}
            className="w-full mt-4 py-3 px-4 rounded-2xl bg-gradient-to-r from-amber-500 to-yellow-600 hover:from-amber-400 hover:to-yellow-500 text-slate-950 text-xs font-bold shadow-lg shadow-amber-500/30 flex items-center justify-center gap-2 transition disabled:opacity-50"
          >
            <Sparkles className="w-4 h-4" />
            <span>Embed & Save to LanceDB</span>
          </button>
        </div>

        {/* Right Side: Semantic Search & Chunks Viewer */}
        <div className="flex-1 p-6 flex flex-col overflow-hidden bg-[#070b14]/50">
          {/* Search bar & Filter Pills */}
          <div className="space-y-3 mb-6">
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                <Search className="w-4 h-4 text-slate-400 absolute left-3.5 top-3" />
                <input
                  type="text"
                  placeholder="Semantic vector search across memory database..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                  className="w-full pl-10 pr-4 py-2.5 rounded-2xl glass-input text-xs text-slate-100 placeholder-slate-500 focus:outline-none"
                />
              </div>
              <button
                onClick={handleSearch}
                disabled={isSearching}
                className="px-5 py-2.5 rounded-2xl bg-amber-500/20 text-amber-300 hover:bg-amber-500/30 border border-amber-500/30 text-xs font-bold transition"
              >
                {isSearching ? 'Searching...' : 'Vector Search'}
              </button>
            </div>

            {/* Source Tag Filter Chips */}
            <div className="flex flex-wrap gap-1.5 items-center">
              <span className="text-[11px] text-slate-500 font-semibold mr-1">Filter Source:</span>
              <button
                onClick={() => setActiveSourceFilter('all')}
                className={
                  'px-2.5 py-0.5 rounded-lg text-[10px] font-semibold uppercase transition border ' +
                  (activeSourceFilter === 'all'
                    ? 'bg-amber-500/20 border-amber-500 text-amber-300'
                    : 'bg-white/[0.02] border-white/5 text-slate-400 hover:text-slate-200')
                }
              >
                All Sources ({memories.length})
              </button>

              {uniqueSources.map((src) => (
                <button
                  key={src}
                  onClick={() => setActiveSourceFilter(src)}
                  className={
                    'px-2.5 py-0.5 rounded-lg text-[10px] font-mono transition border ' +
                    (activeSourceFilter === src
                      ? 'bg-amber-500/20 border-amber-500 text-amber-300'
                      : 'bg-white/[0.02] border-white/5 text-slate-400 hover:text-slate-200')
                  }
                >
                  {src}
                </button>
              ))}
            </div>
          </div>

          {/* Chunks List */}
          <div className="flex-1 overflow-y-auto custom-scrollbar pb-32 space-y-3 pr-2">
            {filteredList.length === 0 ? (
              <div className="text-center py-16 text-slate-500 text-xs flex flex-col items-center gap-3">
                <Database className="w-14 h-14 text-slate-600 mb-1" />
                <span className="font-semibold text-slate-400">No vector memories found</span>
                <span className="max-w-md text-[11px]">
                  Add knowledge on the left or chat with E.D.I.T.H. to generate contextual memories automatically.
                </span>
              </div>
            ) : (
              filteredList.map((chunk, idx) => (
                <div
                  key={chunk.id || idx}
                  className="p-4 rounded-2xl glass-card border border-white/10 hover:border-amber-500/50 transition shadow-lg group"
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <Tag className="w-3.5 h-3.5 text-amber-400" />
                      <span className="font-mono text-xs text-amber-300 font-bold">
                        {chunk.source}
                      </span>
                      {chunk.score !== undefined && (
                        <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-950/60 text-emerald-400 border border-emerald-500/30 font-mono font-bold">
                          {(100 - Math.min(100, chunk.score * 100)).toFixed(0)}% Match
                        </span>
                      )}
                    </div>

                    <button
                      onClick={() => handleDelete(chunk.source)}
                      className="opacity-0 group-hover:opacity-100 p-1 text-slate-500 hover:text-rose-400 transition"
                      title="Delete source memories"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>

                  <p className="text-xs text-slate-300 leading-relaxed font-sans select-text">
                    {chunk.text}
                  </p>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
