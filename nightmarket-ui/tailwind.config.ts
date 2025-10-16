import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './pages/**/*.{js,ts,jsx,tsx,mdx}',
    './components/**/*.{js,ts,jsx,tsx,mdx}',
    './app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        moonlight: {
          DEFAULT: '#6b8cff',
          dim: '#4a6bcc',
        },
      },
      animation: {
        'night-pulse': 'nightPulse 3s ease-in-out infinite',
        'moon-glow': 'moonGlow 2s ease-in-out infinite',
        'shadow-fade': 'shadowFade 1s cubic-bezier(0.4, 0, 0.2, 1)',
        'emerge': 'emerge 800ms cubic-bezier(0.4, 0, 0.2, 1)',
      },
    },
  },
  plugins: [],
};

export default config;
