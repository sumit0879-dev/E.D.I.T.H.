import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './tauri';
import { RiskLevel } from './policyService';

/**
 * Branded nominal type for tool execution identifiers.
 * Prevents accidental mix-ups with turn, session, or task identifiers (frontend-data-contracts).
 */
export type ToolExecutionId = string & { readonly __brand: unique symbol };

export function asToolExecutionId(id: string): ToolExecutionId {
  return id as ToolExecutionId;
}

export type ToolDomain =
  | 'browser'
  | 'system'
  | 'file'
  | 'git'
  | 'network'
  | { custom: string };

export type ToolStatus =
  | 'queued'
  | 'running'
  | 'approval_required'
  | 'success'
  | 'failed'
  | 'cancelled';

export interface ToolDefinition {
  name: string;
  domain: ToolDomain;
  description: string;
  parameters_schema: Record<string, unknown>;
  is_read_only: boolean;
  requires_approval: boolean;
  default_timeout_ms: number;
  risk_level: RiskLevel;
}

export interface EventCorrelation {
  trace_id?: string;
  conversation_id?: string;
  turn_id?: string;
  task_id?: string;
  span_id?: string;
  parent_span_id?: string;
  causation_id?: string;
}

export interface ToolRequest {
  tool_name: string;
  arguments: Record<string, unknown>;
  execution_id?: ToolExecutionId;
  session_id?: string;
  turn_id?: string;
  task_id?: string;
  active_approval_id?: string;
  timeout_ms?: number;
  correlation?: EventCorrelation;
}

export type ToolExecutionError =
  | { type: 'not_found'; tool_name: string }
  | { type: 'already_exists'; tool_name: string }
  | { type: 'validation_failed'; reason: string }
  | { type: 'malformed_arguments'; reason: string }
  | { type: 'access_denied'; reason: string; risk_level: RiskLevel }
  | { type: 'approval_required'; approval_id: string; reason: string }
  | { type: 'timeout'; timeout_ms: number }
  | { type: 'cancelled'; reason: string }
  | { type: 'domain_unavailable'; domain: ToolDomain; reason: string }
  | { type: 'execution_failed'; reason: string }
  | { type: 'internal'; reason: string };

export interface ToolExecutionResult {
  execution_id: ToolExecutionId;
  tool_name: string;
  domain: ToolDomain;
  status: ToolStatus;
  output?: unknown;
  error?: ToolExecutionError;
  duration_ms: number;
  approval_id?: string;
  correlation: EventCorrelation;
}

// Fallback definitions for web / mock environments
const MOCK_BROWSER_DEFINITIONS: ToolDefinition[] = [
  {
    name: 'browser.observe',
    domain: 'browser',
    description: 'Captures interactive DOM elements, forms, links, and accessibility tree state',
    parameters_schema: {
      type: 'object',
      properties: {
        tab_id: { type: 'string', description: 'Tab to observe' },
        depth: { type: 'integer', description: 'Max DOM traversal depth' },
      },
    },
    is_read_only: true,
    requires_approval: false,
    default_timeout_ms: 10000,
    risk_level: 'safe',
  },
  {
    name: 'browser.navigate',
    domain: 'browser',
    description: 'Navigates tab to the specified URL',
    parameters_schema: {
      type: 'object',
      properties: {
        url: { type: 'string', description: 'Target destination URL' },
        tab_id: { type: 'string', description: 'Tab to navigate' },
      },
      required: ['url'],
    },
    is_read_only: false,
    requires_approval: false,
    default_timeout_ms: 30000,
    risk_level: 'low',
  },
  {
    name: 'browser.click',
    domain: 'browser',
    description: 'Dispatches click to the specified element selector or coordinates',
    parameters_schema: {
      type: 'object',
      properties: {
        selector: { type: 'string', description: 'CSS selector or XPath' },
        tab_id: { type: 'string', description: 'Tab containing the element' },
      },
      required: ['selector'],
    },
    is_read_only: false,
    requires_approval: false,
    default_timeout_ms: 10000,
    risk_level: 'medium',
  },
];

/**
 * Lists all registered tool definitions in the host Universal Tool Runtime.
 */
export async function listTools(): Promise<ToolDefinition[]> {
  if (isTauri()) {
    return invoke<ToolDefinition[]>('tools_list_definitions');
  }
  return [...MOCK_BROWSER_DEFINITIONS];
}

/**
 * Gets a specific tool definition by name.
 */
export async function getTool(name: string): Promise<ToolDefinition | null> {
  if (isTauri()) {
    return invoke<ToolDefinition | null>('tools_get_definition', { name });
  }
  return MOCK_BROWSER_DEFINITIONS.find((t) => t.name === name) || null;
}

/**
 * Dispatches tool execution through the Universal Tool Runtime.
 * Validates schema, routes through PolicyEngine, and executes on the domain handler.
 */
export async function executeTool(request: ToolRequest): Promise<ToolExecutionResult> {
  if (isTauri()) {
    return invoke<ToolExecutionResult>('tools_execute', { request });
  }

  // Mock execution fallback
  const start = performance.now();
  const tool = MOCK_BROWSER_DEFINITIONS.find((t) => t.name === request.tool_name);
  if (!tool) {
    return {
      execution_id: asToolExecutionId(request.execution_id || `exec_${Date.now()}`),
      tool_name: request.tool_name,
      domain: 'browser',
      status: 'failed',
      error: { type: 'not_found', tool_name: request.tool_name },
      duration_ms: Math.round(performance.now() - start),
      correlation: request.correlation || {},
    };
  }

  return {
    execution_id: asToolExecutionId(request.execution_id || `exec_${Date.now()}`),
    tool_name: request.tool_name,
    domain: tool.domain,
    status: 'success',
    output: { mock: true, executed: request.tool_name, args: request.arguments },
    duration_ms: Math.round(performance.now() - start),
    correlation: request.correlation || {},
  };
}

/**
 * Cancels an in-flight tool execution across the hierarchical cancellation tree.
 */
export async function cancelTool(executionId: ToolExecutionId | string): Promise<boolean> {
  if (isTauri()) {
    return invoke<boolean>('tools_cancel_execution', { executionId });
  }
  return true;
}
