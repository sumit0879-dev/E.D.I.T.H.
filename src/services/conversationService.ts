import { invoke } from '@tauri-apps/api/core';
import { isTauri } from './tauri';

export interface SubmitTurnRequest {
  sessionId: string;
  message: string;
  providerId?: string;
  modelId?: string;
  temperature?: number;
  clientTurnId?: string;
}

export interface SubmitTurnResult {
  turn_id: string;
  session_id: string;
  stream_id: string;
  user_message_text: string;
}

export interface TurnStatusSnapshot {
  turn_id: string;
  session_id: string;
  status: 'created' | 'input_accepted' | 'processing' | 'streaming' | 'completed' | 'failed' | 'cancelled';
  model_selection: {
    provider_id: string;
    model_id: string;
    temperature: number;
  };
  created_at_ms: number;
  completed_at_ms?: number;
  error?: string;
  final_response?: string;
}

export interface TaskSnapshot {
  task_id: string;
  task_type: string | { custom: string };
  owner: any;
  goal: string;
  status: 'created' | 'queued' | 'running' | 'completing' | 'completed' | 'failed' | 'cancelled';
  progress: {
    step: number;
    max_steps: number;
    status_text: string;
  };
  created_at_ms: number;
  started_at_ms?: number;
  completed_at_ms?: number;
  error?: string;
  result_summary?: string;
}

/**
 * Submits a new user message to Conversation Core.
 * The backend authoritatively generates the TurnId, prepares the turn lifecycle,
 * and saves the user message to persistent storage.
 */
export async function submitConversationTurn(req: SubmitTurnRequest): Promise<SubmitTurnResult> {
  if (!isTauri()) {
    const turnId = req.clientTurnId || 'turn-' + Date.now() + '-' + Math.random().toString(36).slice(2, 6);
    const streamId = 'stream-' + Date.now() + '-' + Math.random().toString(36).slice(2, 6);
    return {
      turn_id: turnId,
      session_id: req.sessionId,
      stream_id: streamId,
      user_message_text: req.message,
    };
  }

  return await invoke<SubmitTurnResult>('conversation_submit_turn', {
    sessionId: req.sessionId,
    message: req.message,
    providerId: req.providerId,
    modelId: req.modelId,
    temperature: req.temperature,
    clientTurnId: req.clientTurnId,
  });
}

/**
 * Executes model generation and streaming for an accepted turn.
 */
export async function executeConversationTurn(
  turnId: string,
  streamId: string,
  appSettings?: Record<string, any>
): Promise<string> {
  if (!isTauri()) {
    return 'Simulated browser response for turn ' + turnId;
  }

  return await invoke<string>('conversation_execute_turn', {
    turnId,
    streamId,
    appSettings,
  });
}

/**
 * Cancels a specific in-flight turn using its scoped cancellation token.
 */
export async function cancelConversationTurn(turnId: string, reason?: string): Promise<void> {
  if (!isTauri()) {
    console.log(`[ConversationService] Cancelled turn ${turnId} (simulated):`, reason);
    return;
  }

  await invoke('conversation_cancel_turn', {
    turnId,
    reason,
  });
}

/**
 * Queries the current lifecycle state of a turn.
 */
export async function getTurnStatus(turnId: string): Promise<TurnStatusSnapshot | null> {
  if (!isTauri()) return null;
  return await invoke<TurnStatusSnapshot | null>('conversation_get_turn_status', {
    turnId,
  });
}

/**
 * Creates an asynchronous background task managed by Task Runtime.
 */
export async function createTask(
  taskType: string,
  goal: string,
  sessionId?: string,
  turnId?: string
): Promise<string> {
  if (!isTauri()) {
    return 'sim-task-' + Date.now();
  }

  return await invoke<string>('task_create', {
    taskType,
    goal,
    sessionId,
    turnId,
  });
}

/**
 * Cancels an active task in Task Runtime.
 */
export async function cancelTask(taskId: string, reason?: string): Promise<void> {
  if (!isTauri()) {
    console.log(`[TaskRuntime] Cancelled task ${taskId} (simulated):`, reason);
    return;
  }

  await invoke('task_cancel', {
    taskId,
    reason,
  });
}

/**
 * Queries a task snapshot from Task Runtime.
 */
export async function getTaskStatus(taskId: string): Promise<TaskSnapshot | null> {
  if (!isTauri()) return null;
  return await invoke<TaskSnapshot | null>('task_get_status', {
    taskId,
  });
}

/**
 * Lists all currently active tasks in Task Runtime.
 */
export async function listActiveTasks(): Promise<TaskSnapshot[]> {
  if (!isTauri()) return [];
  return await invoke<TaskSnapshot[]>('task_list_active');
}
