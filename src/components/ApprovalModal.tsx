import React, { useState, useEffect, useCallback } from 'react';
import { ShieldAlert, CheckCircle, XCircle, Clock, ChevronRight, Terminal } from 'lucide-react';
import {
  listPendingApprovals,
  resolveApproval,
  type ApprovalRequest,
  type RiskLevel,
} from '../services/policyService';
import { eventRouter } from '../events';
import { useApp } from '../context/AppContext';

export const ApprovalModal: React.FC = () => {
  const { showToast } = useApp();
  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequest[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [resolving, setResolving] = useState(false);
  const [timeLeftSec, setTimeLeftSec] = useState<number | null>(null);

  const fetchApprovals = useCallback(async () => {
    try {
      const approvals = await listPendingApprovals();
      setPendingApprovals(approvals);
      if (currentIndex >= approvals.length && approvals.length > 0) {
        setCurrentIndex(0);
      }
    } catch (err) {
      console.error('Failed to list pending approvals:', err);
    }
  }, [currentIndex]);

  useEffect(() => {
    fetchApprovals();

    const unsub = eventRouter.onCategory('security_policy', (event) => {
      if (event.payload.category === 'security_policy') {
        const data = event.payload.data;
        if (data.security_event === 'approval_requested' || data.security_event === 'approval_resolved') {
          fetchApprovals();
        }
      }
    });

    const interval = setInterval(fetchApprovals, 3000);

    return () => {
      unsub();
      clearInterval(interval);
    };
  }, [fetchApprovals]);

  const activeApproval = pendingApprovals[currentIndex] || null;

  useEffect(() => {
    if (!activeApproval || !activeApproval.expires_at_ms) {
      setTimeLeftSec(null);
      return;
    }

    const updateTimer = () => {
      const remainingMs = activeApproval.expires_at_ms - Date.now();
      const sec = Math.max(0, Math.floor(remainingMs / 1000));
      setTimeLeftSec(sec);
      if (sec === 0) {
        fetchApprovals();
      }
    };

    updateTimer();
    const timer = setInterval(updateTimer, 1000);
    return () => clearInterval(timer);
  }, [activeApproval, fetchApprovals]);

  const handleResolve = async (decisionType: 'approve' | 'deny') => {
    if (!activeApproval || resolving) return;
    setResolving(true);
    try {
      await resolveApproval(activeApproval.approval_id, { type: decisionType });
      showToast(
        decisionType === 'approve'
          ? `Action approved: ${activeApproval.action_request.domain}.${activeApproval.action_request.operation}`
          : `Action denied: ${activeApproval.action_request.domain}.${activeApproval.action_request.operation}`,
        decisionType === 'approve' ? 'success' : 'info'
      );
      await fetchApprovals();
    } catch (err: any) {
      showToast(`Failed to resolve approval: ${err?.message || String(err)}`, 'error');
    } finally {
      setResolving(false);
    }
  };

  if (!activeApproval) {
    return null;
  }

  const getRiskBadge = (level: RiskLevel) => {
    switch (level) {
      case 'critical':
        return <span className="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-rose-500/20 text-rose-400 border border-rose-500/40">CRITICAL RISK</span>;
      case 'high':
        return <span className="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-amber-500/20 text-amber-400 border border-amber-500/40">HIGH RISK</span>;
      case 'medium':
        return <span className="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-yellow-500/20 text-yellow-400 border border-yellow-500/40">MEDIUM RISK</span>;
      case 'low':
      case 'safe':
      default:
        return <span className="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-cyan-500/20 text-cyan-400 border border-cyan-500/40">LOW RISK</span>;
    }
  };

  const formattedArgs = JSON.stringify(activeApproval.action_request.arguments, null, 2);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-md animate-fade-in">
      <div className="relative w-full max-w-xl bg-slate-900 border border-cyan-500/40 rounded-2xl shadow-2xl shadow-cyan-950/50 p-6 flex flex-col gap-5 text-slate-100 font-sans">
        <div className="flex items-start justify-between gap-4 border-b border-slate-800 pb-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-amber-500/20 border border-amber-500/40 flex items-center justify-center text-amber-400 shrink-0">
              <ShieldAlert className="w-6 h-6 animate-pulse" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-base font-bold text-slate-100 tracking-wide">
                  OPERATOR CONFIRMATION REQUIRED
                </h3>
                {getRiskBadge(activeApproval.risk_level)}
              </div>
              <p className="text-xs text-slate-400 mt-0.5">
                Host Policy Engine paused execution pending authoritative human approval
              </p>
            </div>
          </div>
          {pendingApprovals.length > 1 && (
            <div className="flex items-center gap-1.5 text-xs text-slate-400 bg-slate-800 px-2 py-1 rounded-md shrink-0">
              <span>{currentIndex + 1} of {pendingApprovals.length}</span>
              <button
                onClick={() => setCurrentIndex((prev) => (prev + 1) % pendingApprovals.length)}
                className="hover:text-cyan-400 transition"
                title="Next approval request"
              >
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>
          )}
        </div>

        <div className="flex flex-col gap-3 text-sm">
          <div className="flex items-center justify-between bg-slate-950/80 p-3 rounded-lg border border-slate-800">
            <div className="flex items-center gap-2">
              <Terminal className="w-4 h-4 text-cyan-400 shrink-0" />
              <span className="text-xs uppercase tracking-wider text-slate-400">Target Action:</span>
              <span className="font-mono font-semibold text-cyan-300">
                {activeApproval.action_request.domain}.{activeApproval.action_request.operation}
              </span>
            </div>
            {timeLeftSec !== null && (
              <div className="flex items-center gap-1 text-xs text-slate-400">
                <Clock className="w-3.5 h-3.5 text-amber-400" />
                <span>{timeLeftSec}s remaining</span>
              </div>
            )}
          </div>

          <div className="bg-slate-950/50 p-3 rounded-lg border border-slate-800/80 text-xs flex flex-col gap-1">
            <span className="text-slate-400 font-medium uppercase tracking-wider">Policy Evaluation Reason:</span>
            <p className="text-slate-300 leading-relaxed font-mono">
              {activeApproval.reason || 'Operation requires elevated confirmation per system security policy.'}
            </p>
          </div>

          {formattedArgs && formattedArgs !== '{}' && (
            <div className="flex flex-col gap-1">
              <span className="text-xs text-slate-400 font-medium uppercase tracking-wider">Action Arguments:</span>
              <pre className="max-h-40 overflow-y-auto p-3 rounded-lg bg-black/80 border border-slate-800 text-xs font-mono text-cyan-400/90 whitespace-pre-wrap break-all">
                {formattedArgs}
              </pre>
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-3 pt-3 border-t border-slate-800">
          <button
            onClick={() => handleResolve('deny')}
            disabled={resolving}
            className="flex items-center gap-2 px-4 py-2 rounded-xl bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border border-rose-500/30 text-xs font-semibold uppercase tracking-wider transition disabled:opacity-50"
          >
            <XCircle className="w-4 h-4" />
            Deny Action
          </button>
          <button
            onClick={() => handleResolve('approve')}
            disabled={resolving}
            className="flex items-center gap-2 px-5 py-2 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-slate-950 text-xs font-bold uppercase tracking-wider shadow-lg shadow-cyan-500/20 transition disabled:opacity-50"
          >
            <CheckCircle className="w-4 h-4" />
            Approve & Resume
          </button>
        </div>
      </div>
    </div>
  );
};
