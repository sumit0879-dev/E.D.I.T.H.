import React, { useState, useRef, useEffect } from 'react';
import { useApp } from '../context/AppContext';
import {
  Send,
  Mic,
  MicOff,
  Sparkles,
  ArrowDown,
  Radio,
} from 'lucide-react';

interface FloatingCommandBarProps {
  onSendMessage: (text: string) => void;
  isLoading?: boolean;
  showScrollToBottom?: boolean;
  hasNewMessagesBelow?: boolean;
  onScrollToBottom?: () => void;
  className?: string;
}

export const FloatingCommandBar: React.FC<FloatingCommandBarProps> = ({
  onSendMessage,
  isLoading = false,
  showScrollToBottom = false,
  hasNewMessagesBelow = false,
  onScrollToBottom,
  className = '',
}) => {
  const { isRecording, toggleRecording, settings } = useApp();
  const [inputText, setInputText] = useState('');
  const [decibels, setDecibels] = useState<number[]>([35, 60, 45, 80, 50]);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Audio decibel meter level simulation when recording
  useEffect(() => {
    if (!isRecording) {
      setDecibels([20, 30, 25, 35, 20]);
      return;
    }

    const interval = setInterval(() => {
      setDecibels([
        Math.floor(Math.random() * 60) + 30,
        Math.floor(Math.random() * 80) + 20,
        Math.floor(Math.random() * 95) + 35,
        Math.floor(Math.random() * 75) + 25,
        Math.floor(Math.random() * 65) + 30,
      ]);
    }, 90);

    return () => clearInterval(interval);
  }, [isRecording]);

  // Auto-resize input height up to 192px (max-h-48) expanding upward
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      const scrollH = textareaRef.current.scrollHeight;
      textareaRef.current.style.height = `${Math.min(Math.max(scrollH, 36), 192)}px`;
    }
  }, [inputText]);

  const handleSend = () => {
    if (!inputText.trim() || isLoading) return;
    onSendMessage(inputText.trim());
    setInputText('');
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div
      className={`absolute bottom-4 left-3 right-3 sm:left-6 sm:right-6 max-w-2xl mx-auto z-30 pointer-events-none select-none ${className}`}
    >
      <div className="pointer-events-auto flex flex-col w-full">
        {/* Centered Scroll to Bottom Button */}
        {showScrollToBottom && onScrollToBottom && (
          <div className="flex items-center justify-center mb-2.5">
            <button
              onClick={onScrollToBottom}
              className="px-4 py-1.5 rounded-full bg-[#090d16]/95 border border-cyan-500/60 text-cyan-300 text-xs font-mono font-bold shadow-cyan-glow-md hover:bg-cyan-950/90 transition flex items-center gap-2 backdrop-blur-xl animate-bounce pointer-events-auto"
            >
              <ArrowDown className="w-3.5 h-3.5 text-cyan-400 animate-pulse" />
              <span>{hasNewMessagesBelow ? 'New messages below ↓' : 'Scroll to bottom ↓'}</span>
            </button>
          </div>
        )}

        {/* Futuristic Glass Input Pill */}
        <div className="relative rounded-2xl bg-[#090d16]/95 backdrop-blur-2xl border border-cyan-500/30 p-2 shadow-2xl transition-all duration-200 focus-within:border-cyan-400 focus-within:shadow-cyan-glow-md">
          <div className="flex items-end gap-2">
            {/* Microphone Toggle Button with Decibel Visualizer */}
            <div className="flex items-center gap-1 shrink-0 pb-1">
              <button
                type="button"
                onClick={() =>
                  toggleRecording((transcript) => {
                    if (transcript.trim()) {
                      setInputText(transcript);
                    }
                  })
                }
                aria-label={isRecording ? 'Stop voice recording' : 'Start voice recording'}
                className={`p-2 rounded-xl transition flex items-center justify-center shrink-0 ${
                  isRecording
                    ? 'bg-rose-500 text-white shadow-red-glow animate-pulse'
                    : 'bg-white/[0.04] text-slate-400 hover:text-cyan-300 hover:bg-cyan-500/20 border border-white/5'
                }`}
                title={isRecording ? 'Listening... click to stop' : 'Click to speak'}
              >
                {isRecording ? <MicOff className="w-4 h-4" /> : <Mic className="w-4 h-4" />}
              </button>

              {/* Decibel Level Bars (active when recording) */}
              {isRecording && (
                <div
                  className="flex items-end gap-0.5 h-6 px-1.5 py-1 rounded-lg bg-rose-950/60 border border-rose-500/40"
                  title="Audio Input Level"
                >
                  {decibels.map((lvl, idx) => (
                    <span
                      key={idx}
                      className="w-1 bg-rose-400 rounded-full transition-all duration-100"
                      style={{ height: `${Math.max(lvl, 15)}%` }}
                    />
                  ))}
                </div>
              )}
            </div>

            {/* Expanding Textarea */}
            <textarea
              ref={textareaRef}
              rows={1}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Direct instruction to E.D.I.T.H... (Enter to send, Shift+Enter for newline)"
              className="flex-1 bg-transparent text-xs sm:text-sm text-slate-100 placeholder-slate-500 resize-none focus:outline-none py-1.5 px-1 max-h-48 overflow-y-auto custom-scrollbar font-sans leading-relaxed"
              style={{ minHeight: '36px', maxHeight: '192px' }}
            />

            {/* Send Button */}
            <button
              onClick={handleSend}
              disabled={!inputText.trim() || isLoading}
              aria-label="Send Message"
              className="p-2.5 rounded-xl bg-gradient-to-tr from-cyan-500 via-teal-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 font-bold transition flex items-center justify-center shrink-0 shadow-md shadow-cyan-500/20 disabled:opacity-30 disabled:cursor-not-allowed mb-0.5"
            >
              <Send className="w-4 h-4" />
            </button>
          </div>

          {/* Bottom Status / Hint Bar */}
          <div className="flex items-center justify-between px-2 pt-1.5 mt-1 border-t border-white/5 text-[10px] font-mono text-slate-500">
            <span className="flex items-center gap-1.5 truncate mr-2">
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 shrink-0" />
              <span className="truncate">Active Model: {settings.selectedModel || 'llama-3.3-70b'}</span>
            </span>
            <span className="shrink-0 text-slate-400">Press Enter ↵ to dispatch</span>
          </div>
        </div>
      </div>
    </div>
  );
};
