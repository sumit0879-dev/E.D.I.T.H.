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

export interface DuplexVoiceState {
  session: 'disabled' | 'idle' | 'connecting' | 'connected' | 'reconnecting' | 'fallback' | 'error';
  input: 'inactive' | 'listening_ambient' | 'user_speaking' | 'muted';
  output: 'silent' | 'assistant_speaking' | 'interrupted_ducking';
  processing: 'idle' | 'model_inferring' | 'model_streaming' | 'tool_executing';
  active_turn_id?: string;
  generation_id: number;
}

export interface VisualizerEnergyData {
  rms: number;
  peak: number;
  bands: number[];
  is_speech: boolean;
  direction: string;
}

export interface DeviceChangedData {
  input_device_id?: string;
  output_device_id?: string;
  input_device_name?: string;
  output_device_name?: string;
}

export type VoiceEventData =
  | { voice_event: 'session_started'; data: { session_id: string } }
  | { voice_event: 'state_changed'; data: { state: string; decibel?: number } }
  | { voice_event: 'barge_in'; data: { interrupted_source: string } }
  | { voice_event: 'session_ended'; data: { session_id: string; reason?: string } }
  | { voice_event: 'duplex_state_changed'; data: DuplexVoiceState }
  | { voice_event: 'visualizer_energy'; data: VisualizerEnergyData }
  | { voice_event: 'device_changed'; data: DeviceChangedData };

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

export type SecurityPolicyEventData =
  | {
      security_event: 'policy_evaluated';
      data: {
        action_domain: string;
        action_operation: string;
        risk_level: string;
        outcome: string;
        reason: string;
        approval_id?: string;
      };
    }
  | {
      security_event: 'approval_requested';
      data: {
        approval_id: string;
        action_domain: string;
        action_operation: string;
        risk_level: string;
        reason: string;
        expires_at_ms: number;
      };
    }
  | {
      security_event: 'approval_resolved';
      data: {
        approval_id: string;
        status: string;
        notes?: string;
      };
    };

export type EdithPayload =
  | { category: 'stream'; data: StreamEventData }
  | { category: 'task'; data: TaskEventData }
  | { category: 'tool'; data: ToolEventData }
  | { category: 'voice'; data: VoiceEventData }
  | { category: 'runtime'; data: RuntimeEventData }
  | { category: 'security_policy'; data: SecurityPolicyEventData };

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
