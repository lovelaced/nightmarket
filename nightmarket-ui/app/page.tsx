'use client';

import { useState, useEffect } from 'react';

export default function LandingPage() {
  const [visible, setVisible] = useState(false);
  const [fadeOut, setFadeOut] = useState(false);

  useEffect(() => {
    const fadeInTimer = setTimeout(() => setVisible(true), 200);
    return () => clearTimeout(fadeInTimer);
  }, []);

  const handleEnter = () => {
    setFadeOut(true);
    setTimeout(() => {
      window.location.href = '/market';
    }, 1000);
  };

  return (
    <div className="fixed inset-0 bg-black flex items-center justify-center overflow-hidden">
      {/* Subtle background gradient */}
      <div className="absolute inset-0 bg-gradient-to-b from-black via-black to-blue-950/10" />

      <div
        className={`relative flex flex-col items-center gap-16 transition-all duration-1000 ${
          visible && !fadeOut ? 'opacity-100' : 'opacity-0'
        } ${fadeOut ? 'scale-95 blur-sm' : 'scale-100'}`}
      >
        {/* Main Title */}
        <div className="flex flex-col items-center gap-4">
          <h1 className="text-3xl md:text-4xl font-extralight tracking-[0.3em] text-center uppercase">
            nightmarket
          </h1>
          <div className="h-px w-32 bg-gradient-to-r from-transparent via-white/20 to-transparent" />
          <p className="text-sm font-light text-gray-500 tracking-wider font-serif italic">
            anonymous commerce. ephemeral exchanges.
          </p>
        </div>

        {/* Time Window */}
        <div className="flex items-center gap-3 text-gray-600 font-mono text-xs">
          <div className="w-2 h-2 rounded-full bg-blue-500/40 animate-night-pulse" />
          <span>06:00 — 05:00 utc</span>
        </div>

        {/* Enter Button */}
        <button
          onClick={handleEnter}
          className="group px-10 py-4 text-xs tracking-[0.4em] uppercase
                     border border-white/10 hover:border-white/30
                     transition-all duration-700
                     hover:bg-white/5 hover:shadow-moonlight
                     relative overflow-hidden"
        >
          <span className="relative z-10">enter</span>
          <div className="absolute inset-0 bg-gradient-to-r from-blue-500/0 via-blue-500/5 to-blue-500/0
                          translate-x-[-100%] group-hover:translate-x-[100%] transition-transform duration-1000" />
        </button>

        {/* Info */}
        <div className="text-[10px] text-gray-700 tracking-wide font-mono max-w-md text-center leading-relaxed space-y-2">
          <p>zero-knowledge location proofs. encrypted listings.</p>
          <p>all exchanges via dead drop protocol.</p>
          <p>transactions irreversible. privacy absolute.</p>
        </div>

        {/* Disclaimer */}
        <div className="text-[9px] text-gray-800 tracking-wide font-mono max-w-lg text-center leading-relaxed mt-8 border-t border-white/5 pt-4">
          <p className="text-red-400/60 mb-2">experimental protocol. use at your own risk.</p>
          <p>no central authority. no customer support. no refunds.</p>
        </div>
      </div>
    </div>
  );
}
