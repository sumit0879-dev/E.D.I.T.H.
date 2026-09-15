import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './tauri';

/**
 * Branded nominal type for cryptographic approval identifiers.
 * Prevents accidental mix-ups with turn or session identifiers (frontend-data-contracts).
 */
export type ApprovalId = string & { readonly __brand: unique symbol };

export function asApprovalId(id: string): ApprovalId {
  return id as ApprovalId;
}

export type RiskLevel = 'safe' | 'low' | 'medium' | 'high' | 'critical';

export type PolicyOutcome = 'allow' | 'confirmation_required' | 'restricted' | 'blocked';

export type ApprovalStatus = 'pending' | 'approved' | 'denied' | 'expired' | 'cancelled' | 'consumed';

export type SecurityMode = 'autonomous' | 'standard' | 'restricted';

export type ActionTarget =
  | { type: 'command'; program: string; args: string[] }
  | { type: 'path'; path: string; is_write: boolean }
  | { type: 'url'; url: string }
  | { type: 'browser_element'; selector: string; tag?: string; role?: string; text?: string }
  | { type: 'system_target'; target: string }
  | { type: 'none' };

export interface ActionRequest {
  domain: string;
  operation: string;
  target: ActionTarget;
  arguments: Record<string, unknown>;
}

export interface PolicyConstraints {
  allow_command_execution: boolean;
  allow_filesystem_write: boolean;
  allow_network_access: boolean;
  allow_external_services: boolean;
  sandbox_workspace_only: boolean;
}

export interface PolicyContext {
  session_id?: string;
  turn_id?: string;
  task_id?: string;
  source: 'autonomous_agent' | 'user_turn' | 'background_task' | 'test_runner';
  security_mode: SecurityMode;
  workspace_roots: string[];
  active_approval_id?: ApprovalId;
}

export interface PolicyDecision {
  outcome: PolicyOutcome;
  risk_level: RiskLevel;
  reason: string;
  approval_id?: ApprovalId;
  constraints?: PolicyConstraints;
}

export interface ApprovalRequest {
  approval_id: ApprovalId;
  action_request: ActionRequest;
  request_hash: string;
  risk_level: RiskLevel;
  reason: string;
  session_id?: string;
  turn_id?: string;
  task_id?: string;
  policy_version: number;
  created_at_ms: number;
  expires_at_ms: number;
  status: ApprovalStatus;
}

export interface AuditRecord {
  event_id: string;
  timestamp_ms: number;
  session_id?: string;
  turn_id?: string;
  task_id?: string;
  action_domain: string;
  action_operation: string;
  target: ActionTarget;
  sanitized_arguments: Record<string, unknown>;
  risk_level: RiskLevel;
  outcome: PolicyOutcome;
  reason: string;
  approval_id?: ApprovalId;
  policy_version: number;
}

export type OperatorDecision =
  | { type: 'approve'; notes?: string }
  | { type: 'deny'; notes?: string };

/**
 * Evaluates a proposed action through the central host-enforced Policy Engine.
 */
export async function evaluatePolicyAction(
  request: ActionRequest,
  context: PolicyContext
): Promise<PolicyDecision> {
  if (!isTauri()) {
    // Development browser mock: default safe allow unless sensitive
    const isDangerous =
      request.domain === 'system' &&
      (request.operation.includes('rm') || request.operation.includes('delete'));
    return {
      outcome: isDangerous ? 'confirmation_required' : 'allow',
      risk_level: isDangerous ? 'high' : 'low',
      reason: isDangerous ? 'Mock confirmation required' : 'Mock allowed in dev browser',
      approval_id: isDangerous ? asApprovalId('mock-approval-1') : undefined,
    };
  }

  return await invoke<PolicyDecision>('policy_evaluate_action', {
    request,
    context,
  });
}

/**
 * Lists all active human confirmation requests waiting for operator approval.
 */
export async function listPendingApprovals(): Promise<ApprovalRequest[]> {
  if (!isTauri()) {
    return [];
  }
  return await invoke<ApprovalRequest[]>('policy_list_pending_approvals');
}

/**
 * Resolves a pending human confirmation request (Approve or Deny).
 */
export async function resolveApproval(
  approvalId: ApprovalId | string,
  decision: OperatorDecision
): Promise<ApprovalRequest> {
  if (!isTauri()) {
    throw new Error('Approval resolution requires active Tauri host runtime.');
  }

  // Map to Rust enum shape
  const rustDecision =
    decision.type === 'approve'
      ? { approve: { notes: decision.notes ?? null } }
      : { deny: { notes: decision.notes ?? null } };

  return await invoke<ApprovalRequest>('policy_resolve_approval', {
    approvalId,
    decision: rustDecision,
  });
}

/**
 * Retrieves privacy-sanitized security audit logs for operator review.
 */
export async function getSecurityAuditLog(limit?: number): Promise<AuditRecord[]> {
  if (!isTauri()) {
    return [];
  }
  return await invoke<AuditRecord[]>('policy_get_audit_log', {
    limit: limit ?? 100,
  });
}
