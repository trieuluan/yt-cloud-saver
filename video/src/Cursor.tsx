import React from 'react';

interface CursorProps {
  x: number;
  y: number;
  clicking?: boolean;
}

export const Cursor: React.FC<CursorProps> = ({ x, y, clicking = false }) => {
  return (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: y,
        zIndex: 100,
        pointerEvents: 'none',
        transform: `scale(${clicking ? 0.82 : 1})`,
        transformOrigin: 'top left',
      }}
    >
      {clicking && (
        <div
          style={{
            position: 'absolute',
            left: -12,
            top: -12,
            width: 30,
            height: 30,
            borderRadius: '50%',
            border: '2px solid #60a5fa',
            background: 'rgba(96, 165, 250, 0.25)',
          }}
        />
      )}
      <svg width="26" height="26" viewBox="0 0 24 24" fill="none">
        <path
          d="M4 2L20 11.5L12.5 13.5L9.5 21L4 2Z"
          fill="#ffffff"
          stroke="#0f172a"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
};
