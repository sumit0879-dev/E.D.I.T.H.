/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'Plus Jakarta Sans', 'ui-sans-serif', 'system-ui', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco', 'Consolas', 'monospace'],
      },
      fontSize: {
        'xs': '11px',
        'sm': '12px',
        'md': '14px',
        'base': '15px',
        'lg': '18px',
        'xl': '22px',
        '2xl': '32px',
      },
      borderRadius: {
        'xs': '4px',
        'sm': '8px',
        'md': '12px',
        'full': '9999px',
      },
      colors: {
        background: '#000000',
        surface: {
          base: '#000000',
          raised: '#030712',
          muted: '#090d16',
          overlay: 'rgba(15, 23, 42, 0.75)',
          accent: 'rgba(6, 182, 212, 0.08)',
          50: '#1e293b',
          100: '#161e2e',
          200: '#0f172a',
          300: '#0b1120',
          400: '#070b14',
          900: '#030712',
        },
        border: {
          glass: 'rgba(255, 255, 255, 0.08)',
          neon: '#06b6d4',
          neonGlow: '#22d3ee',
        },
        hud: {
          cyan: '#06b6d4',
          cyanGlow: '#22d3ee',
          cyanDim: 'rgba(6, 182, 212, 0.30)',
          cyanBg: 'rgba(6, 182, 212, 0.06)',
          border: 'rgba(255, 255, 255, 0.08)',
          borderGlow: 'rgba(6, 182, 212, 0.4)',
        },
        status: {
          online: '#10b981',
          standby: '#06b6d4',
          busy: '#f59e0b',
          alert: '#ef4444',
        },
        brand: {
          cyan: '#06b6d4',
          violet: '#8b5cf6',
          indigo: '#6366f1',
          rose: '#f43f5e',
          emerald: '#10b981',
          amber: '#f59e0b',
        }
      },
      boxShadow: {
        'glass': '0 8px 32px 0 rgba(0, 0, 0, 0.37)',
        'cyanGlow': '0 0 15px rgba(6, 182, 212, 0.35)',
        'cyan-glow-sm': '0 0 10px rgba(6, 182, 212, 0.35)',
        'cyan-glow-md': '0 0 18px rgba(6, 182, 212, 0.45)',
        'cyan-glow-lg': '0 0 25px rgba(6, 182, 212, 0.60), inset 0 0 15px rgba(6, 182, 212, 0.2)',
        'amber-glow': '0 0 15px rgba(245, 158, 11, 0.45)',
        'red-glow': '0 0 15px rgba(239, 68, 68, 0.55)',
      },
      animation: {
        'spin-slow': 'spin 30s linear infinite',
        'spin-reverse-slow': 'spin-reverse 20s linear infinite',
        'spin-reverse-fast': 'spin-reverse 10s linear infinite',
        'pulse-glow': 'pulse-glow 2.5s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'pulse-fast': 'pulse-glow 1s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'waveform': 'waveform 1.2s ease-in-out infinite alternate',
      },
      keyframes: {
        'spin-reverse': {
          '0%': { transform: 'rotate(0deg)' },
          '100%': { transform: 'rotate(-360deg)' },
        },
        'pulse-glow': {
          '0%, 100%': { opacity: '0.4', transform: 'scale(1)' },
          '50%': { opacity: '0.9', transform: 'scale(1.03)' },
        },
        'waveform': {
          '0%': { height: '15%' },
          '100%': { height: '100%' },
        }
      }
    },
  },
  plugins: [],
};
