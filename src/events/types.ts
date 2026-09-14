/**
 * E.D.I.T.H. Correlated Event Infrastructure - TypeScript Definitions
 * Aligned with Rust backend `edith::events` module.
 */

export interface EventCorrelation {
  conversation_id?: string;
  turn_id?: string;
  stream_id?: string;
  task_id?: string;
  tool_execution_id?: string;
  voice_session_id?: string;
}

export type StreamEventData =
  | { stream_event: 'started'; data: { model: string } }
  | {
      stream_event: 'chunk';
      data: {
        text: string;
        sequence_number: number;
        is_final: boolean;
      };
    }
  | {
      stream_event: 'finished';
      data: {
        total_tokens?: number;
        finish_reason?: string;
      };
    }
  | {
      stream_event: 'failed';
      data: {
        error: string;
        error_type?: string;
      };
    }
  | {
      stream_event: 'cancelled';
      data: {
        reason?: string;
      };
    };

export type TaskEventData =
  | { task_event: 'started'; data: { task_id: string; goal: string } }
  | {
      task_event: 'step_progress';
      data: {
        task_id: string;
        step: number;
        max_steps: number;
        status_text: string;
      };
    }
  | {
      task_event: 'finished';
      data: {
        task_id: string;
        success: boolean;
        summary: string;
      };
    }
  | { task_event: 'failed'; data: { task_id: string; error: string } }
  | { task_event: 'cancelled'; data: { task_id: string; reason?: string } };

export type ToolEventData =
  | {
      tool_event: 'proposed';
      data: {
        execution_id: string;
        tool_name: string;
        risk_level: string;
        summary: string;
      };
    }
  | { tool_event: 'started'; data: { execution_id: string; tool_name: string } }
  | {
      tool_event: 'completed';
      data: {
        execution_id: string;
        tool_name: string;
        success: boolean;
        duration_ms: number;
        result_summary?: string;
      };
    }
  | {
      tool_event: 'failed';
      data: {
        execution_id: string;
        tool_name: string;
        error: string;
      };
    };

export type VoiceEventData =
  | { voice_event: 'session_started'; data: { session_id: string } }
  | { voice_event: 'state_changed'; data: { state: string; decibel?: number } }
  | { voice_event: 'barge_in'; data: { interrupted_source: string } }
  | { voice_event: 'session_ended'; data: { session_id: string; reason?: string } };

export type RuntimeEventData =
  | {
      runtime_event: 'error';
      data: {
        code: string;
        message: string;
        details?: string;
      };
    }
  | { runtime_event: 'notification'; data: { level: string; message: string } };

export type EdithPayload =
  | { category: 'stream'; data: StreamEventData }
  | { category: 'task'; data: TaskEventData }
  | { category: 'tool'; data: ToolEventData }
  | { category: 'voice'; data: VoiceEventData }
  | { category: 'runtime'; data: RuntimeEventData };

export type EventCategory = EdithPayload['category'];

export interface EdithEventEnvelope<T = EdithPayload> {
  event_id: string;
  timestamp_ms: number;
  correlation: EventCorrelation;
  payload: T;
}

export type StreamChunkHandler = (chunk: {
  text: string;
  sequenceNumber: number;
  isFinal: boolean;
  correlation: EventCorrelation;
}) => void;

export type StreamLifecycleHandler = (event: {
  type: 'started' | 'finished' | 'failed' | 'cancelled';
  correlation: EventCorrelation;
  data?: any;
}) => void;

export interface StreamSubscription {
  onChunk: StreamChunkHandler;
  onLifecycle?: StreamLifecycleHandler;
  /** Optional filter for specific turn ID */
  turnId?: string;
  /** Optional filter for specific stream ID */
  streamId?: string;
}
