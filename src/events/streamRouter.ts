import { eventRouter } from './eventRouter';
import type {
  EdithEventEnvelope,
  EventCorrelation,
  StreamEventData,
  StreamSubscription,
} from './types';

export interface ActiveStreamState {
  streamId?: string;
  turnId?: string;
  lastSequenceNumber: number;
  startedAt: number;
  isComplete: boolean;
}

/**
 * StreamRouter
 * Manages multiplexed streaming LLM responses with sequence ordering,
 * turn/stream isolation, and lifecycle tracking.
 */
export class StreamRouter {
  private subscriptions: Set<StreamSubscription> = new Set();
  private streamStates: Map<string, ActiveStreamState> = new Map();
  private unregisterRouter: (() => void) | null = null;

  constructor() {
    this.init();
  }

  private init(): void {
    if (this.unregisterRouter) return;

    this.unregisterRouter = eventRouter.onCategory('stream', (envelope) => {
      this.handleStreamEnvelope(envelope as EdithEventEnvelope<{ category: 'stream'; data: StreamEventData }>);
    });
  }

  private getStreamKey(correlation: EventCorrelation): string {
    return correlation.stream_id || correlation.turn_id || 'anonymous_stream';
  }

  private handleStreamEnvelope(
    envelope: EdithEventEnvelope<{ category: 'stream'; data: StreamEventData }>
  ): void {
    const { correlation, payload } = envelope;
    const streamData = payload.data;
    const streamKey = this.getStreamKey(correlation);

    // Initialize or get stream state
    let state = this.streamStates.get(streamKey);
    if (!state) {
      state = {
        streamId: correlation.stream_id,
        turnId: correlation.turn_id,
        lastSequenceNumber: 0,
        startedAt: envelope.timestamp_ms,
        isComplete: false,
      };
      this.streamStates.set(streamKey, state);
    }

    if (streamData.stream_event === 'chunk') {
      const { text, sequence_number, is_final } = streamData.data;

      // Monotonic sequence verification: reject duplicate or out-of-order chunks
      if (sequence_number <= state.lastSequenceNumber && sequence_number !== 0) {
        console.warn(
          `[StreamRouter] Dropping out-of-order chunk for ${streamKey}: seq=${sequence_number}, last=${state.lastSequenceNumber}`
        );
        return;
      }
      state.lastSequenceNumber = sequence_number;

      // Dispatch strictly to matching subscribers
      for (const sub of this.subscriptions) {
        if (this.isMatchingSubscription(sub, correlation)) {
          try {
            sub.onChunk({
              text,
              sequenceNumber: sequence_number,
              isFinal: is_final,
              correlation,
            });
          } catch (err) {
            console.error('[StreamRouter] Error in subscriber onChunk:', err);
          }
        }
      }
    } else {
      // Lifecycle events: started, finished, failed, cancelled
      const lifecycleType = streamData.stream_event;
      const isTerminal = ['finished', 'failed', 'cancelled'].includes(lifecycleType);

      if (isTerminal) {
        state.isComplete = true;
      }

      for (const sub of this.subscriptions) {
        if (this.isMatchingSubscription(sub, correlation)) {
          if (sub.onLifecycle) {
            try {
              sub.onLifecycle({
                type: lifecycleType,
                correlation,
                data: 'data' in streamData ? (streamData as any).data : undefined,
              });
            } catch (err) {
              console.error('[StreamRouter] Error in subscriber onLifecycle:', err);
            }
          }
        }
      }

      // Cleanup stream tracking state on completion
      if (isTerminal) {
        setTimeout(() => {
          this.streamStates.delete(streamKey);
        }, 5000);
      }
    }
  }

  private isMatchingSubscription(sub: StreamSubscription, correlation: EventCorrelation): boolean {
    if (sub.turnId && correlation.turn_id) {
      return sub.turnId === correlation.turn_id;
    }
    if (sub.streamId && correlation.stream_id) {
      return sub.streamId === correlation.stream_id;
    }
    // If neither turnId nor streamId is set on subscription, match all (wildcard)
    return !sub.turnId && !sub.streamId;
  }

  /**
   * Subscribe to stream events with turn/stream isolation.
   * Returns an unsubscribe function.
   */
  public subscribe(subscription: StreamSubscription): () => void {
    this.subscriptions.add(subscription);
    return () => {
      this.subscriptions.delete(subscription);
    };
  }

  /**
   * Helper to subscribe specifically for a turn ID.
   */
  public subscribeTurn(
    turnId: string,
    onChunk: StreamSubscription['onChunk'],
    onLifecycle?: StreamSubscription['onLifecycle']
  ): () => void {
    return this.subscribe({
      turnId,
      onChunk,
      onLifecycle,
    });
  }

  /**
   * Helper to subscribe specifically for a stream ID.
   */
  public subscribeStream(
    streamId: string,
    onChunk: StreamSubscription['onChunk'],
    onLifecycle?: StreamSubscription['onLifecycle']
  ): () => void {
    return this.subscribe({
      streamId,
      onChunk,
      onLifecycle,
    });
  }

  public getActiveStreamCount(): number {
    return this.streamStates.size;
  }

  public destroy(): void {
    if (this.unregisterRouter) {
      this.unregisterRouter();
      this.unregisterRouter = null;
    }
    this.subscriptions.clear();
    this.streamStates.clear();
  }
}

export const streamRouter = new StreamRouter();
