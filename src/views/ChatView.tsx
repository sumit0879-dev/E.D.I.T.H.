import React, { useState, useEffect, useRef } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import type { Message } from '../types';
import { CodeBlock } from '../components/CodeBlock';
import { ArcReactor } from '../components/ArcReactor';
import { FloatingCommandBar } from '../components/FloatingCommandBar';
import {
  Send,
  Volume2,
  AlertOctagon,
  Plus,
  Trash2,
  Edit2,
  Check,
  X,
  Sparkles,
  Bot,
  User,
  Terminal,
  Search,
  Copy,
  Layers,
  ChevronLeft,
  ChevronRight,
  Radio,
  Zap,
  ArrowDown,
} from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

export const ChatView: React.FC = () => {
  const {
    sessions,
    activeSessionId,
    setActiveSessionId,
    createSession,
    deleteSession,
    renameSession,
    settings,
    showToast,
    speakText,
    isSpeaking,
    isRecording,
    toggleRecording,
  } = useApp();

  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [newTitle, setNewTitle] = useState('');
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [isSessionDrawerOpen, setIsSessionDrawerOpen] = useState(true);
  const [isAutoScroll, setIsAutoScroll] = useState(true);
  const [hasNewMessagesBelow, setHasNewMessagesBelow] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const pendingSessionIdRef = useRef<string | null>(null);

  const scrollToBottom = (behavior: ScrollBehavior = 'smooth') => {
    messagesEndRef.current?.scrollIntoView({ behavior });
  };

  const handleContainerScroll = () => {
    if (!scrollContainerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollContainerRef.current;
    const isNearBottom = scrollHeight - scrollTop - clientHeight < 120;
    setIsAutoScroll(isNearBottom);
    if (isNearBottom) {
      setHasNewMessagesBelow(false);
    }
  };

  useEffect(() => {
    if (!activeSessionId) {
      setMessages([]);
      return;
    }

    // Protect in-flight messages from being cleared during first message session creation
    if (pendingSessionIdRef.current === activeSessionId) {
      return;
    }

    let isMounted = true;
    setMessages([]);
    tauriService.getSessionMessages(activeSessionId)
      .then((loaded) => {
        if (isMounted) {
          setMessages(loaded || []);
          setIsAutoScroll(true);
          setHasNewMessagesBelow(false);
          setTimeout(() => scrollToBottom('auto'), 50);
        }
      })
      .catch((e) => {
        console.error('Error loading session messages:', e);
        if (isMounted) {
          setMessages([]);
        }
      });

    return () => {
      isMounted = false;
    };
  }, [activeSessionId]);

  // Backward compatibility fallback for legacy un-keyed stream chunks (yields to streamRouter)
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    tauriService.onChatChunk((chunk) => {
      if (tauriService.streamRouter.getActiveStreamCount() > 0) return;

      setMessages((prev) => {
        if (prev.length === 0) return prev;
        const lastIdx = prev.length - 1;
        const last = prev[lastIdx];
        if (last && last.role === 'assistant' && last.isStreaming) {
          return [
            ...prev.slice(0, lastIdx),
            {
              ...last,
              text: (last.text || '') + chunk,
              content: (last.content || '') + chunk,
            },
          ];
        }
        return prev;
      });

      if (isAutoScroll) {
        setTimeout(scrollToBottom, 50);
      } else {
        setHasNewMessagesBelow(true);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [isAutoScroll]);

  const handleSendMessage = async (text: string) => {
    if (!text.trim() || isLoading) return;

    let targetSessionId = activeSessionId;
    if (!targetSessionId) {
      targetSessionId = await createSession(text.slice(0, 26));
      pendingSessionIdRef.current = targetSessionId;
    }

    const timestamp = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const userMsgId = 'user-' + Date.now() + '-' + Math.random().toString(36).slice(2, 6);
    const assistantMsgId = 'assistant-' + Date.now() + '-' + Math.random().toString(36).slice(2, 6);

    const userMsg: Message = {
      id: userMsgId,
      role: 'user',
      text: text.trim(),
      content: text.trim(),
      time: timestamp,
      session_id: targetSessionId,
    };

    const assistantMsg: Message = {
      id: assistantMsgId,
      role: 'assistant',
      text: '',
      content: '',
      time: timestamp,
      session_id: targetSessionId,
      isStreaming: true,
    };

    setMessages((prev) => [...prev, userMsg, assistantMsg]);
    setIsLoading(true);
    setIsAutoScroll(true);
    setHasNewMessagesBelow(false);
    setTimeout(() => scrollToBottom('smooth'), 50);

    // Correlated stream subscription: strictly isolated to this assistantMsgId (turnId)
    const unsubscribeStream = tauriService.streamRouter.subscribeTurn(
      assistantMsgId,
      ({ text: chunkText }) => {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === assistantMsgId
              ? {
                  ...m,
                  text: (m.text || '') + chunkText,
                  content: (m.content || '') + chunkText,
                }
              : m
          )
        );

        if (isAutoScroll) {
          setTimeout(scrollToBottom, 50);
        } else {
          setHasNewMessagesBelow(true);
        }
      }
    );

    try {
      await tauriService.saveSessionMessage(targetSessionId, 'user', text.trim(), timestamp);

      const historyItems = messages.map((m) => ({
        role: m.role,
        text: m.text || m.content || '',
      }));

      const res = await tauriService.chatCommand(
        text.trim(),
        targetSessionId,
        historyItems,
        settings,
        assistantMsgId
      );

      setMessages((prev) =>
        prev.map((m) =>
          m.id === assistantMsgId
            ? {
                ...m,
                text: res.response || m.text,
                content: res.response || m.content,
                isStreaming: false,
                type: res.type as any,
              }
            : m
        )
      );

      await tauriService.saveSessionMessage(
        targetSessionId,
        'assistant',
        res.response,
        timestamp
      );

      if (settings.autoSpeak === 'true' && res.response && res.type !== 'error') {
        speakText(res.response);
      }
    } catch (e: any) {
      const errMsg = 'Error: ' + (e.message || e);
      setMessages((prev) =>
        prev.map((m) =>
          m.id === assistantMsgId
            ? {
                ...m,
                text: errMsg,
                content: errMsg,
                time: timestamp,
                session_id: targetSessionId,
                type: 'error',
                isStreaming: false,
              }
            : m
        )
      );
      showToast(errMsg, 'error');
    } finally {
      unsubscribeStream();
      setIsLoading(false);
      pendingSessionIdRef.current = null;
    }
  };

  const handleCopyMessage = async (text: string, index: number) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 2000);
      showToast('Copied to clipboard', 'info');
    } catch {}
  };

  const filteredSessions = sessions.filter((s) =>
    s.title.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const activeSessionObj = sessions.find((s) => s.id === activeSessionId);

  // Status for Arc Reactor
  const getHudStatus = () => {
    if (isRecording) return 'listening';
    if (isLoading) return 'processing';
    if (isSpeaking) return 'speaking';
    return 'standby';
  };

  return (
    <div className="flex-1 flex overflow-hidden h-full bg-[#030712] relative">
      {/* Collapsible Session List Sidebar with Smooth Sliding Animation */}
      <div
        className={`bg-[#030712]/95 border-r border-white/[0.08] flex flex-col justify-between shrink-0 z-10 select-none transition-all duration-300 ease-in-out overflow-hidden ${
          isSessionDrawerOpen ? 'w-64 opacity-100' : 'w-0 opacity-0 border-r-0 pointer-events-none'
        }`}
      >
        <div className="w-64 flex flex-col h-full justify-between">
          {/* Top Session Actions */}
          <div className="p-3 border-b border-white/[0.08] space-y-2">
            <button
              onClick={() => createSession()}
              className="w-full flex items-center justify-center gap-2 py-2 px-3 rounded-xl bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 text-white text-xs font-bold font-mono shadow-md shadow-cyan-500/20 transition duration-200"
            >
              <Plus className="w-4 h-4" />
              <span>NEW MISSION SESSION</span>
            </button>

            <div className="relative">
              <Search className="w-3.5 h-3.5 text-slate-400 absolute left-3 top-2.5" />
              <input
                type="text"
                placeholder="Search missions..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 rounded-xl glass-input text-xs font-mono text-slate-200 placeholder-slate-500 focus:outline-none"
              />
            </div>
          </div>

          {/* Sessions list */}
          <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar">
            {filteredSessions.length === 0 ? (
              <div className="text-center py-10 text-xs font-mono text-slate-500">
                No missions found
              </div>
            ) : (
              filteredSessions.map((session) => {
                const isSelected = activeSessionId === session.id;
                const isEditing = editingSessionId === session.id;

                return (
                  <div
                    key={session.id}
                    onClick={() => !isEditing && setActiveSessionId(session.id)}
                    className={
                      'group flex items-center justify-between px-3 py-2 rounded-xl cursor-pointer text-xs transition border font-mono ' +
                      (isSelected
                        ? 'bg-gradient-to-r from-cyan-950/80 to-blue-950/80 text-white font-bold border-cyan-500/60 shadow-md shadow-cyan-500/10'
                        : 'border-transparent text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]')
                    }
                  >
                    {isEditing ? (
                      <div className="flex items-center gap-1 flex-1 mr-1" onClick={(e) => e.stopPropagation()}>
                        <input
                          type="text"
                          value={newTitle}
                          onChange={(e) => setNewTitle(e.target.value)}
                          autoFocus
                          className="flex-1 px-2 py-1 bg-slate-900 border border-cyan-500 rounded-lg text-xs text-white focus:outline-none"
                        />
                        <button
                          onClick={() => {
                            renameSession(session.id, newTitle.trim() || session.title);
                            setEditingSessionId(null);
                          }}
                          className="p-1 text-emerald-400 hover:text-emerald-300"
                        >
                          <Check className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() => setEditingSessionId(null)}
                          className="p-1 text-slate-400 hover:text-slate-200"
                        >
                          <X className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    ) : (
                      <>
                        <div className="truncate flex-1 flex items-center gap-1.5">
                          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${isSelected ? 'bg-cyan-400 shadow-cyan-glow-sm' : 'bg-slate-600'}`} />
                          <span className="truncate">{session.title}</span>
                        </div>
                        <div className="hidden group-hover:flex items-center gap-1 shrink-0 ml-1">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setEditingSessionId(session.id);
                              setNewTitle(session.title);
                            }}
                            className="p-1 text-slate-400 hover:text-cyan-400 transition"
                            title="Rename"
                          >
                            <Edit2 className="w-3 h-3" />
                          </button>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteSession(session.id);
                            }}
                            className="p-1 text-slate-400 hover:text-rose-400 transition"
                            title="Delete"
                          >
                            <Trash2 className="w-3 h-3" />
                          </button>
                        </div>
                      </>
                    )}
                  </div>
                );
              })
            )}
          </div>
        </div>
      </div>

      {/* Main Adaptive Center Stage */}
      <div className="flex-1 flex flex-col bg-[#030712] overflow-hidden relative hud-grid-bg min-w-0 transition-all duration-300">
        {/* Active Session Mini Header */}
        <div className="px-5 py-2 bg-[#030712]/90 border-b border-white/[0.08] flex items-center justify-between text-xs backdrop-blur-xl z-10 select-none">
          <div className="flex items-center gap-2">
            <button
              onClick={() => setIsSessionDrawerOpen(!isSessionDrawerOpen)}
              className="p-1 rounded hover:bg-white/5 text-slate-400 hover:text-cyan-300 transition"
              title={isSessionDrawerOpen ? 'Collapse Sessions' : 'Expand Sessions'}
            >
              {isSessionDrawerOpen ? <ChevronLeft className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
            </button>

            <div className="flex items-center gap-2 text-slate-300 font-mono">
              <span className="font-bold text-white text-xs">{activeSessionObj?.title || 'Tactical Feed'}</span>
              <span className="text-slate-600">/</span>
              <span className="text-slate-500 text-[11px]">{messages.length} messages</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {messages.length > 0 && (
              <ArcReactor
                compact
                status={getHudStatus()}
                isListening={isRecording}
                onTriggerMic={() =>
                  toggleRecording((txt) => {
                    if (txt.trim()) handleSendMessage(txt);
                  })
                }
              />
            )}
          </div>
        </div>

        {/* Adaptive Viewport: Standby Mode vs Active Message Stream */}
        <div
          ref={scrollContainerRef}
          onScroll={handleContainerScroll}
          className={`flex-1 ${messages.length === 0 ? 'overflow-hidden justify-center' : 'overflow-y-auto custom-scrollbar'} p-4 sm:p-6 pb-6 space-y-6 flex flex-col min-h-0`}
        >
          {messages.length === 0 ? (
            /* Standby Hero View */
            <div className="my-auto flex flex-col items-center justify-center text-center max-w-2xl mx-auto space-y-6 animate-fade-in py-4 select-none">
              {/* Central Arc Reactor Visualizer */}
              <ArcReactor
                size={240}
                status={getHudStatus()}
                isListening={isRecording}
                onTriggerMic={() =>
                  toggleRecording((txt) => {
                    if (txt.trim()) handleSendMessage(txt);
                  })
                }
              />

              <div>
                <div className="flex items-center justify-center gap-2 mb-2">
                  <span className="text-2xl font-black text-white tracking-widest font-mono">
                    E.D.I.T.H.
                  </span>
                  <span className="text-[10px] font-mono font-bold text-cyan-300 bg-cyan-950/80 px-2 py-0.5 rounded-full border border-cyan-500/40">
                    MARK-85
                  </span>
                </div>
                <p className="text-xs text-slate-400 leading-relaxed font-mono max-w-lg mx-auto">
                  Even Dead, I'm The Hero — Tactical AI Assistant with multi-provider inference, live telemetry, and system controls. Click core or press shortcut to initiate voice command.
                </p>
              </div>
            </div>
          ) : (
            /* Active Conversational Message Stream (Dynamic Full-Width Viewport) */
            <div className="space-y-5 w-full px-3 sm:px-6 md:px-8 transition-all duration-300">
              {messages.map((msg, index) => {
                const isUser = msg.role === 'user';
                const isError = msg.type === 'error';

                return (
                  <div
                    key={msg.id || `msg-${index}-${msg.time || ''}`}
                    className={`flex items-start w-full transition-all duration-200 ${
                      isUser ? 'justify-end' : 'justify-start'
                    }`}
                  >
                    <div
                      className={`flex items-start gap-2.5 max-w-[88%] sm:max-w-[78%] ${
                        isUser ? 'flex-row-reverse' : 'flex-row'
                      }`}
                    >
                      {/* Role Avatar */}
                      <div
                        className={
                          'w-7 h-7 rounded-xl shrink-0 flex items-center justify-center shadow-lg mt-0.5 ' +
                          (isUser
                            ? 'bg-gradient-to-tr from-cyan-600 to-blue-600 text-white shadow-cyan-glow-sm'
                            : isError
                            ? 'bg-gradient-to-tr from-rose-950 to-slate-900 border border-rose-500/40 text-rose-400 shadow-rose-500/10'
                            : 'bg-gradient-to-tr from-cyan-950 to-slate-900 border border-cyan-500/40 text-cyan-400 shadow-cyan-glow-sm')
                        }
                      >
                        {isUser ? (
                          <User className="w-3.5 h-3.5" />
                        ) : isError ? (
                          <AlertOctagon className="w-3.5 h-3.5" />
                        ) : (
                          <Bot className="w-3.5 h-3.5" />
                        )}
                      </div>

                      {/* Message Bubble Card (Hugs Content, w-fit) */}
                      <div
                        className={
                          'rounded-2xl px-4 py-3 text-sm leading-relaxed border backdrop-blur-xl relative shadow-2xl w-fit max-w-full ' +
                          (isUser
                            ? 'bg-gradient-to-br from-cyan-950/80 via-[#0a1526] to-blue-950/70 border-cyan-500/40 text-white rounded-tr-none shadow-cyan-500/10'
                            : isError
                            ? 'bg-gradient-to-br from-rose-950/30 via-[#180b12] to-slate-900/90 border-rose-500/40 text-rose-200 rounded-tl-none shadow-rose-500/10'
                            : 'bg-[#090d16]/90 border-white/[0.08] text-slate-100 rounded-tl-none')
                        }
                      >
                        {/* Bubble Header */}
                        <div className="flex items-center justify-between gap-3 mb-1.5 text-xs text-slate-400 border-b border-white/5 pb-1">
                          <span className="font-bold text-[10px] uppercase tracking-wider text-slate-300 font-mono flex items-center gap-1.5">
                            <span>{isUser ? 'Commander' : isError ? 'SYSTEM ALERT' : 'E.D.I.T.H.'}</span>
                            {!isUser && (
                              <span
                                className={`text-[9px] px-1.5 py-0.2 rounded border font-bold ${
                                  isError
                                    ? 'bg-rose-950/80 text-rose-400 border-rose-500/40'
                                    : 'bg-cyan-950/70 text-cyan-400 border border-cyan-500/30'
                                }`}
                              >
                                {isError ? 'ERROR' : 'AI'}
                              </span>
                            )}
                          </span>

                          <div className="flex items-center gap-1.5 ml-auto shrink-0">
                            {msg.time && (
                              <span className="text-[10px] font-mono text-slate-500">{msg.time}</span>
                            )}
                            <button
                              onClick={() => handleCopyMessage(msg.text || msg.content || '', index)}
                              className="text-slate-400 hover:text-cyan-400 transition p-0.5"
                              title="Copy text"
                            >
                              {copiedIndex === index ? (
                                <Check className="w-3.5 h-3.5 text-emerald-400" />
                              ) : (
                                <Copy className="w-3.5 h-3.5" />
                              )}
                            </button>
                            {!isUser && msg.text && !isError && (
                              <button
                                onClick={() => speakText(msg.text)}
                                className="text-slate-400 hover:text-cyan-400 transition p-0.5"
                                title="Synthesize voice output"
                              >
                                <Volume2 className="w-3.5 h-3.5" />
                              </button>
                            )}
                          </div>
                        </div>

                        {/* Markdown Rendered Content */}
                        <div className={`prose-dark text-sm break-words whitespace-pre-wrap ${isError ? 'text-rose-200' : ''}`}>
                          <ReactMarkdown
                            remarkPlugins={[remarkGfm]}
                            components={{
                              code({ node, className, children, ...props }: any) {
                                const match = /language-(\w+)/.exec(className || '');
                                const codeValue = String(children).replace(/\n$/, '');
                                return match ? (
                                  <CodeBlock language={match[1]} value={codeValue} />
                                ) : (
                                  <code className={className} {...props}>
                                    {children}
                                  </code>
                                );
                              },
                            }}
                          >
                            {msg.text || msg.content || (msg.isStreaming ? 'Synthesizing response...' : '')}
                          </ReactMarkdown>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
              {/* Spacer so the last message is fully above the floating command bar when scrolled to bottom */}
              <div ref={messagesEndRef} className="h-36 sm:h-44 shrink-0 pointer-events-none" />
            </div>
          )}
        </div>

        {/* Floating Bottom Command Bar with Centered Scroll-to-Bottom */}
        <FloatingCommandBar
          onSendMessage={handleSendMessage}
          isLoading={isLoading}
          showScrollToBottom={!isAutoScroll && messages.length > 0}
          hasNewMessagesBelow={hasNewMessagesBelow}
          onScrollToBottom={() => {
            setIsAutoScroll(true);
            setHasNewMessagesBelow(false);
            scrollToBottom('smooth');
          }}
        />
      </div>
    </div>
  );
};
