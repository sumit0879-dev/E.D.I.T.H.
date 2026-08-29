import React, { useState, useEffect, useRef } from 'react';
import { useApp } from '../context/AppContext';
import * as tauriService from '../services/tauri';
import {
  Terminal,
  RotateCcw,
  FileCode,
  Sparkles,
  Send,
  FolderOpen,
  CheckCircle2,
  AlertTriangle,
  Play,
  FileText,
  Search,
  Code2,
} from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { CodeBlock } from '../components/CodeBlock';

interface AgentMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  type?: 'cmd' | 'file' | 'text';
}

const quickAgentPrompts = [
  { label: 'Analyze Architecture', prompt: 'Analyze the architecture, dependencies, and entrypoints of this project.' },
  { label: 'Browser: Observe Tab', prompt: 'Use the browser tool to observe tab_a and summarize its content and interactive elements.' },
  { label: 'Browser: Open URL', prompt: 'Use the browser tool to open https://example.com in tab_a.' },
  { label: 'Find Bugs & Issues', prompt: 'Audit the codebase for missing error handlers, memory leaks, or unhandled exceptions.' },
  { label: 'Performance Review', prompt: 'Review performance bottlenecks and recommend async / caching optimizations.' },
  { label: 'Generate Docs', prompt: 'Generate a clean technical architecture guide for this project.' },
];

export const DevAgentView: React.FC = () => {
  const { showToast } = useApp();
  const [projectPath, setProjectPath] = useState('E:\\Projects\\E.D.I.T.H');
  const [isReady, setIsReady] = useState(false);
  const [inputMessage, setInputMessage] = useState('');
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [liveStream, setLiveStream] = useState('');
  const chatEndRef = useRef<HTMLDivElement>(null);

  const [proposals, setProposals] = useState<Array<{
    proposal_id: string;
    session_id: string;
    command: string;
    working_dir: string;
    risk_level: string;
    expires_at: number;
    status: 'pending' | 'approved' | 'rejected' | 'done';
    result?: string;
  }>>([]);

  useEffect(() => {
    tauriService.agentStatus().then((status) => {
      setIsReady(status.is_ready);
      if (status.project_path) {
        setProjectPath(status.project_path);
      }
    });

    const unlistenPromise = tauriService.onChatChunk((chunk) => {
      setLiveStream((prev) => prev + chunk);
      chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    });

    const unlistenProposalPromise = tauriService.onToolProposal((p) => {
      setProposals((prev) => [
        ...prev.filter((item) => item.proposal_id !== p.proposal_id),
        { ...p, status: 'pending' },
      ]);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
      unlistenProposalPromise.then((unlisten) => unlisten());
    };
  }, []);

  const handleResolveProposal = async (proposalId: string, action: 'Approve' | 'Reject') => {
    try {
      const targetProp = proposals.find((p) => p.proposal_id === proposalId);
      const sessionId = targetProp?.session_id || 'default_session';
      const res = await tauriService.agentResolveProposal(proposalId, action, sessionId);
      setProposals((prev) =>
        prev.map((p) =>
          p.proposal_id === proposalId
            ? { ...p, status: action === 'Approve' ? 'done' : 'rejected', result: res.output }
            : p
        )
      );
      showToast(
        action === 'Approve' ? 'Command execution authorized' : 'Command execution rejected',
        action === 'Approve' ? 'success' : 'info'
      );
    } catch (err: any) {
      showToast('Proposal Error: ' + (err.message || err), 'error');
    }
  };

  const handleSetPath = async (pathToUse?: string) => {
    const target = (pathToUse || projectPath).trim();
    if (!target) return;
    try {
      await tauriService.agentSetPath(target);
      setIsReady(true);
      setProjectPath(target);
      showToast('Project workspace set for E.D.I.T.H. Dev Agent', 'success');
      setMessages((prev) => [
        ...prev,
        {
          role: 'assistant',
          content: 'Workspace context loaded: `' + target + '`.\n\nReady to analyze project files, read directories with `[READ_FILE: <path>]`, and execute terminal commands with `[RUN_CMD: <command>]`.',
        },
      ]);
    } catch (e: any) {
      showToast('Failed to set path: ' + (e.message || e), 'error');
    }
  };

  const handleReset = async () => {
    try {
      await tauriService.agentReset();
      setIsReady(false);
      setProjectPath('');
      setMessages([]);
      setLiveStream('');
      showToast('Dev Agent workspace context reset', 'info');
    } catch (e: any) {
      showToast('Reset error: ' + (e.message || e), 'error');
    }
  };

  const handleSend = async (textToSend?: string) => {
    const text = (textToSend || inputMessage).trim();
    if (!text || isLoading) return;

    setInputMessage('');
    setMessages((prev) => [...prev, { role: 'user', content: text }]);
    setIsLoading(true);
    setLiveStream('');

    try {
      const response = await tauriService.agentChat(text);
      setMessages((prev) => [
        ...prev,
        { role: 'assistant', content: response },
      ]);
    } catch (e: any) {
      const err = 'Agent Error: ' + (e.message || e);
      setMessages((prev) => [...prev, { role: 'assistant', content: err }]);
      showToast(err, 'error');
    } finally {
      setIsLoading(false);
      setLiveStream('');
      setTimeout(() => chatEndRef.current?.scrollIntoView({ behavior: 'smooth' }), 50);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-[#000000] overflow-hidden">
      {/* Top Workspace Path Bar */}
      <div className="p-4 border-b border-white/10 bg-[#0d1424] flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3 flex-1 min-w-[300px]">
          <div className="p-2.5 rounded-2xl bg-violet-500/20 text-violet-400 border border-violet-500/30">
            <Terminal className="w-5 h-5" />
          </div>
          <div className="flex-1 max-w-2xl">
            <div className="text-xs font-semibold text-slate-300 mb-1 flex items-center gap-2">
              <span>E.D.I.T.H. Dev Agent Project Root</span>
              {isReady && (
                <span className="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded font-mono font-bold">
                  ACTIVE
                </span>
              )}
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                placeholder="E:\Projects\MyProject or ./..."
                value={projectPath}
                onChange={(e) => setProjectPath(e.target.value)}
                disabled={isReady}
                className="flex-1 px-3.5 py-1.5 rounded-xl glass-input text-xs font-mono text-slate-200 placeholder-slate-500"
              />
              {!isReady ? (
                <button
                  onClick={() => handleSetPath()}
                  disabled={!projectPath.trim()}
                  className="px-4 py-1.5 rounded-xl bg-violet-600 hover:bg-violet-500 text-white text-xs font-bold transition shadow-lg shadow-violet-600/30 shrink-0"
                >
                  Set Workspace
                </button>
              ) : (
                <button
                  onClick={handleReset}
                  className="px-3.5 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white text-xs font-medium transition flex items-center gap-1.5 border border-white/10 shrink-0"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>Reset</span>
                </button>
              )}
            </div>
          </div>
        </div>

        <div className="hidden lg:flex items-center gap-2 text-xs text-slate-400">
          <Sparkles className="w-4 h-4 text-violet-400" />
          <span>Auto-executes `[RUN_CMD]` and reads `[READ_FILE]`</span>
        </div>
      </div>

      {/* Main Agent Terminal / Conversation */}
      <div className="flex-1 overflow-y-auto custom-scrollbar p-6 space-y-6">
        {messages.length === 0 && !liveStream ? (
          <div className="h-full flex flex-col items-center justify-center text-center max-w-xl mx-auto space-y-6">
            <div className="w-16 h-16 rounded-2xl bg-violet-600/20 border border-violet-500/30 flex items-center justify-center text-violet-400 shadow-xl shadow-violet-500/20">
              <Code2 className="w-8 h-8" />
            </div>
            <div>
              <h3 className="text-xl font-bold text-white">E.D.I.T.H. Developer Agent</h3>
              <p className="text-xs text-slate-400 mt-1.5 leading-relaxed max-w-md mx-auto">
                Autonomous coding engine that executes shell tasks, inspects codebase files, audits bugs, and refactors components.
              </p>
            </div>

            {/* Quick Action Chips */}
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 w-full">
              {quickAgentPrompts.map((qp, idx) => (
                <button
                  key={idx}
                  onClick={() => {
                    if (!isReady) {
                      handleSetPath();
                    }
                    handleSend(qp.prompt);
                  }}
                  className="p-3 rounded-xl glass-card hover:border-violet-500/50 text-left transition group border border-white/10"
                >
                  <div className="text-xs font-bold text-slate-200 group-hover:text-violet-300">
                    {qp.label}
                  </div>
                  <div className="text-[11px] text-slate-500 truncate mt-0.5">
                    "{qp.prompt}"
                  </div>
                </button>
              ))}
            </div>
          </div>
        ) : (
          messages.map((m, idx) => (
            <div
              key={idx}
              className={'flex gap-3 max-w-3xl ' + (m.role === 'user' ? 'ml-auto flex-row-reverse' : 'mr-auto')}
            >
              <div
                className={
                  'w-8 h-8 rounded-xl shrink-0 flex items-center justify-center shadow ' +
                  (m.role === 'user' ? 'bg-cyan-600 text-white' : 'bg-violet-600 text-white')
                }
              >
                {m.role === 'user' ? 'U' : <Terminal className="w-4 h-4" />}
              </div>

              <div
                className={
                  'rounded-2xl p-4 max-w-[85%] text-sm leading-relaxed border shadow-xl ' +
                  (m.role === 'user'
                    ? 'bg-cyan-950/60 border-cyan-500/30 text-white rounded-tr-none'
                    : 'bg-[#121929] border-white/10 text-slate-100 rounded-tl-none')
                }
              >
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
                  {m.content}
                </ReactMarkdown>
              </div>
            </div>
          ))
        )}

        {/* Live streaming indicator */}
        {liveStream && (
          <div className="flex gap-3 max-w-3xl mr-auto">
            <div className="w-8 h-8 rounded-xl bg-violet-600 text-white flex items-center justify-center animate-pulse">
              <Terminal className="w-4 h-4" />
            </div>
            <div className="rounded-2xl p-4 bg-[#121929] border border-violet-500/30 text-slate-100 rounded-tl-none shadow-xl text-sm leading-relaxed">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{liveStream}</ReactMarkdown>
            </div>
          </div>
        )}

        {/* Active Command Authorization Proposals (SEC-02 HITL) */}
        {proposals.map((prop) => (
          <div key={prop.proposal_id} className="max-w-2xl mx-auto my-3 p-4 rounded-2xl bg-[#0e1628] border-2 border-amber-500/40 shadow-2xl animate-fade-in">
            <div className="flex items-center justify-between gap-3 mb-2.5 pb-2.5 border-b border-white/10">
              <div className="flex items-center gap-2">
                <AlertTriangle className="w-4 h-4 text-amber-400" />
                <span className="text-xs font-bold uppercase tracking-wider text-amber-300">Command Authorization Request</span>
              </div>
              <span className="px-2 py-0.5 rounded-full text-[10px] font-mono uppercase bg-amber-500/20 text-amber-300 border border-amber-500/30">
                Risk: {prop.risk_level}
              </span>
            </div>

            <div className="space-y-1.5 mb-3 text-xs">
              <div className="flex items-start gap-2">
                <span className="text-slate-400 shrink-0 font-medium">Proposed Action:</span>
                <code className="font-mono bg-black/40 px-2 py-1 rounded text-cyan-300 font-semibold break-all border border-white/5">
                  {prop.command}
                </code>
              </div>
              <div className="flex items-center gap-2 text-slate-400">
                <span className="shrink-0 font-medium">Working Directory:</span>
                <span className="font-mono text-slate-300 truncate">{prop.working_dir}</span>
              </div>
              <div className="flex items-center gap-2 text-slate-500 text-[11px]">
                <span>Token ID:</span>
                <span className="font-mono truncate">{prop.proposal_id}</span>
              </div>
            </div>

            {prop.status === 'pending' ? (
              <div className="flex items-center justify-end gap-2 pt-2 border-t border-white/5">
                <button
                  onClick={() => handleResolveProposal(prop.proposal_id, 'Reject')}
                  className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-rose-500/20 hover:bg-rose-500/30 text-rose-300 border border-rose-500/30 transition"
                >
                  Reject & Abort
                </button>
                <button
                  onClick={() => handleResolveProposal(prop.proposal_id, 'Approve')}
                  className="px-4 py-1.5 rounded-xl text-xs font-bold bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white shadow-lg shadow-emerald-600/30 border border-emerald-400/30 transition flex items-center gap-1.5"
                >
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  Approve & Execute
                </button>
              </div>
            ) : (
              <div className="pt-2 border-t border-white/5 flex items-center justify-between text-xs">
                <span className="text-slate-400 font-medium">Resolution Status:</span>
                <span className={'font-bold uppercase tracking-wider ' + (prop.status === 'done' ? 'text-emerald-400' : 'text-rose-400')}>
                  {prop.status === 'done' ? 'Authorized & Executed' : 'Rejected by Operator'}
                </span>
              </div>
            )}

            {prop.result && (
              <div className="mt-2.5 p-2.5 rounded-xl bg-black/40 border border-white/10 text-xs font-mono text-slate-300 max-h-32 overflow-y-auto">
                <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1 font-sans font-bold">Execution Output:</div>
                <pre className="whitespace-pre-wrap">{prop.result}</pre>
              </div>
            )}
          </div>
        ))}

        <div ref={chatEndRef} />
      </div>

      {/* Input box */}
      <div className="p-4 bg-[#0d1424] border-t border-white/10">
        <div className="max-w-4xl mx-auto flex items-center gap-2 bg-[#090d16] rounded-2xl border border-white/10 p-2 focus-within:border-violet-500">
          <input
            type="text"
            placeholder={isReady ? "Describe the task (e.g. 'Build the frontend router and fix errors')..." : "Set workspace path or click a prompt above to begin"}
            value={inputMessage}
            onChange={(e) => setInputMessage(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSend()}
            disabled={isLoading}
            className="flex-1 bg-transparent px-3 py-2 text-sm text-slate-100 placeholder-slate-500 focus:outline-none"
          />
          <button
            onClick={() => handleSend()}
            disabled={!inputMessage.trim() || isLoading}
            className={
              'p-2.5 rounded-xl font-semibold transition ' +
              (inputMessage.trim() && !isLoading
                ? 'bg-violet-600 hover:bg-violet-500 text-white shadow-lg shadow-violet-600/30'
                : 'bg-white/5 text-slate-600 cursor-not-allowed')
            }
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
};
