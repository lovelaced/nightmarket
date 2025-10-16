'use client';

import { useEffect, useState } from 'react';

export function NightStatus() {
  const [isNight, setIsNight] = useState(false);

  useEffect(() => {
    const checkTime = () => {
      const now = new Date();
      const hour = now.getUTCHours();
      // Market hours: 6 AM - 5 AM UTC
      setIsNight(hour >= 6 || hour < 5);
    };

    checkTime();
    const interval = setInterval(checkTime, 60000); // Check every minute

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex items-center gap-2 text-xs">
      <div
        className={`w-2 h-2 rounded-full transition-all duration-500 ${
          isNight
            ? 'bg-green-500/60 shadow-[0_0_8px_rgba(34,197,94,0.4)]'
            : 'bg-red-500/60 shadow-[0_0_8px_rgba(239,68,68,0.4)]'
        }`}
      />
      <span className={`font-mono tracking-wider ${isNight ? 'text-green-500/80' : 'text-red-500/80'}`}>
        {isNight ? 'open' : 'closed'}
      </span>
    </div>
  );
}
