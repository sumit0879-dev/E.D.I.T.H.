import React, { useEffect, useState } from 'react';
import { Mic, Sparkles, Activity, Radio, Zap } from 'lucide-react';

interface ArcReactorProps {
  status?: 'standby' | 'listening' | 'processing' | 'speaking' | 'online';
  isListening?: boolean;
  onTriggerMic?: () => void;
  size?: number;
  compact?: boolean;
  className?: string;
}

export const ArcReactor: React.FC<ArcReactorProps> = ({
  status = 'standby',
  isListening = false,
  onTriggerMic,
  size = 280,
  compact = false,
  className = '',
}) => {
  const [waveLevels, setWaveLevels] = useState<number[]>([40, 65, 30, 80, 55, 90, 45, 70, 35, 85, 60, 40]);

  // Audio wave simulation when listening or speaking
  useEffect(() => {
    if (status !== 'listening' && status !== 'speaking') return;

    const interval = setInterval(() => {
      setWaveLevels((prev) =>
        prev.map(() => Math.floor(Math.random() * 75) + 25)
      );
    }, 120);

    return () => clearInterval(interval);
  }, [status]);

  const effectiveStatus = isListening ? 'listening' : status;

  // Determine glow color and speed based on status
  const getCoreColors = () => {
    switch (effectiveStatus) {
      case 'listening':
        return {
          glow: 'rgba(6, 182, 212, 0.9)',
          core: 'from-cyan-400 via-teal-300 to-cyan-500',
          border: 'border-cyan-400',
          shadow: '0 0 35px rgba(6, 182, 212, 0.75), inset 0 0 20px rgba(6, 182, 212, 0.5)',
          pulseSpeed: 'animate-pulse',
        };
      case 'processing':
        return {
          glow: 'rgba(245, 158, 11, 0.85)',
          core: 'from-amber-400 via-orange-300 to-cyan-400',
          border: 'border-amber-400',
          shadow: '0 0 30px rgba(245, 158, 11, 0.65), inset 0 0 15px rgba(245, 158, 11, 0.4)',
          pulseSpeed: 'animate-ping',
        };
      case 'speaking':
        return {
          glow: 'rgba(56, 189, 248, 0.9)',
          core: 'from-sky-400 via-cyan-300 to-indigo-400',
          border: 'border-sky-400',
          shadow: '0 0 30px rgba(56, 189, 248, 0.7), inset 0 0 20px rgba(56, 189, 248, 0.4)',
          pulseSpeed: 'animate-pulse',
        };
      default:
        return {
          glow: 'rgba(6, 182, 212, 0.5)',
          core: 'from-cyan-500 via-teal-400 to-blue-600',
          border: 'border-cyan-500/60',
          shadow: '0 0 25px rgba(6, 182, 212, 0.5), inset 0 0 15px rgba(6, 182, 212, 0.25)',
          pulseSpeed: 'animate-pulse-glow',
        };
    }
  };

  const colors = getCoreColors();

  if (compact) {
    return (
      <div
        onClick={onTriggerMic}
        className={`relative flex items-center justify-center cursor-pointer group ${className}`}
        style={{ width: 44, height: 44 }}
        title={isListening ? 'E.D.I.T.H. is Listening...' : 'Click to activate voice'}
      >
        {/* Compact Orbit Ring */}
        <div className="absolute inset-0 rounded-full border border-dashed border-cyan-500/40 animate-spin-slow" />
        <div className="absolute inset-1 rounded-full border border-cyan-400/20 animate-spin-reverse-slow" />
        
        {/* Core */}
        <div
          className={`w-7 h-7 rounded-full bg-gradient-to-tr ${colors.core} flex items-center justify-center shadow-lg transition-all duration-300 group-hover:scale-110`}
          style={{ boxShadow: colors.shadow }}
        >
          {effectiveStatus === 'listening' ? (
            <Radio className="w-3.5 h-3.5 text-slate-950 animate-pulse" />
          ) : effectiveStatus === 'processing' ? (
            <Activity className="w-3.5 h-3.5 text-slate-950 animate-spin" />
          ) : (
            <Sparkles className="w-3.5 h-3.5 text-slate-950" />
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      className={`relative flex items-center justify-center select-none ${className}`}
      style={{ width: size, height: size }}
    >
      {/* Dynamic Audio Waveform Spikes (Radiating 360) */}
      {(effectiveStatus === 'listening' || effectiveStatus === 'speaking') && (
        <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
          {waveLevels.map((lvl, idx) => {
            const angle = (idx * 360) / waveLevels.length;
            return (
              <div
                key={idx}
                className="absolute w-1 rounded-full bg-cyan-400/80 transition-all duration-100"
                style={{
                  height: `${lvl}%`,
                  transform: `rotate(${angle}deg) translateY(-${size * 0.46}px)`,
                  boxShadow: '0 0 8px rgba(6, 182, 212, 0.8)',
                }}
              />
            );
          })}
        </div>
      )}

      {/* Layer 1: Outermost Tech Coordinate Ring */}
      <svg
        className="absolute inset-0 w-full h-full animate-spin-slow pointer-events-none opacity-60"
        viewBox="0 0 300 300"
      >
        <circle
          cx="150"
          cy="150"
          r="140"
          fill="none"
          stroke="rgba(6, 182, 212, 0.2)"
          strokeWidth="1"
          strokeDasharray="4 8 12 8"
        />
        <circle
          cx="150"
          cy="150"
          r="132"
          fill="none"
          stroke="rgba(6, 182, 212, 0.4)"
          strokeWidth="1.5"
          strokeDasharray="80 15 30 15 120 20"
        />
        {/* Cardinal Markers */}
        <text x="145" y="18" fill="#22d3ee" fontSize="8" fontFamily="JetBrains Mono" opacity="0.8">000°</text>
        <text x="274" y="153" fill="#22d3ee" fontSize="8" fontFamily="JetBrains Mono" opacity="0.8">090°</text>
        <text x="145" y="292" fill="#22d3ee" fontSize="8" fontFamily="JetBrains Mono" opacity="0.8">180°</text>
        <text x="8" y="153" fill="#22d3ee" fontSize="8" fontFamily="JetBrains Mono" opacity="0.8">270°</text>
      </svg>

      {/* Layer 2: Counter-rotating Geometrical Ring */}
      <svg
        className="absolute inset-4 w-[calc(100%-2rem)] h-[calc(100%-2rem)] animate-spin-reverse-slow pointer-events-none opacity-75"
        viewBox="0 0 260 260"
      >
        <circle
          cx="130"
          cy="130"
          r="115"
          fill="none"
          stroke="rgba(6, 182, 212, 0.35)"
          strokeWidth="2"
          strokeDasharray="20 40 60 40"
        />
        {/* Chevron Arcs */}
        {[0, 60, 120, 180, 240, 300].map((deg) => (
          <path
            key={deg}
            d="M 130 20 L 135 28 L 125 28 Z"
            fill="#06b6d4"
            transform={`rotate(${deg} 130 130)`}
            opacity="0.85"
          />
        ))}
      </svg>

      {/* Layer 3: Inner High-Frequency Ring */}
      <svg
        className="absolute inset-10 w-[calc(100%-5rem)] h-[calc(100%-5rem)] animate-spin-slow pointer-events-none opacity-85"
        viewBox="0 0 200 200"
      >
        <circle
          cx="100"
          cy="100"
          r="82"
          fill="none"
          stroke="rgba(34, 211, 238, 0.5)"
          strokeWidth="1.5"
          strokeDasharray="6 6"
        />
        <circle
          cx="100"
          cy="100"
          r="74"
          fill="none"
          stroke="rgba(6, 182, 212, 0.25)"
          strokeWidth="4"
          strokeDasharray="30 20 50 15"
        />
      </svg>

      {/* Center Core: Interactive Arc Reactor Button */}
      <div
        onClick={onTriggerMic}
        role="button"
        tabIndex={0}
        aria-label="Activate Voice Assistant E.D.I.T.H."
        className={`relative z-10 w-28 h-28 rounded-full bg-gradient-to-tr ${colors.core} flex flex-col items-center justify-center cursor-pointer transition-all duration-300 transform hover:scale-105 active:scale-95 group focus:outline-none focus:ring-2 focus:ring-cyan-400 focus:ring-offset-4 focus:ring-offset-[#030712]`}
        style={{
          boxShadow: colors.shadow,
        }}
      >
        {/* Core Tech Segment Overlay */}
        <div className="absolute inset-1.5 rounded-full border-2 border-dashed border-slate-950/40 animate-spin-slow" />
        <div className="absolute inset-3 rounded-full border border-white/40 opacity-80" />

        {/* Center Indicator Icon & Status */}
        <div className="relative z-10 flex flex-col items-center justify-center text-slate-950">
          {effectiveStatus === 'listening' ? (
            <>
              <Radio className="w-8 h-8 animate-bounce mb-0.5" />
              <span className="text-[10px] font-black tracking-widest font-mono uppercase">
                LISTENING
              </span>
            </>
          ) : effectiveStatus === 'processing' ? (
            <>
              <Activity className="w-8 h-8 animate-spin mb-0.5" />
              <span className="text-[10px] font-black tracking-widest font-mono uppercase">
                PROCESSING
              </span>
            </>
          ) : effectiveStatus === 'speaking' ? (
            <>
              <Zap className="w-8 h-8 animate-pulse mb-0.5" />
              <span className="text-[10px] font-black tracking-widest font-mono uppercase">
                SPEAKING
              </span>
            </>
          ) : (
            <>
              <Mic className="w-8 h-8 group-hover:scale-110 transition-transform mb-0.5" />
              <span className="text-[10px] font-black tracking-widest font-mono uppercase opacity-90">
                E.D.I.T.H.
              </span>
            </>
          )}
        </div>

        {/* Outer Glow Pulse Effect */}
        <div
          className={`absolute -inset-2 rounded-full border border-cyan-400/50 opacity-40 ${colors.pulseSpeed} pointer-events-none`}
        />
      </div>
    </div>
  );
};
