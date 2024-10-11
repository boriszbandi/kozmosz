import React from 'react';
import './Raketa.css'; // Ensure you have the animation in this CSS file

export function Raketa(props) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 16 16" {...props}>
      {/* Rocket body */}
      <path fill="currentColor" fillRule="evenodd" d="M15.808 0a10 10 0 0 0-7.142 3H6.75A2.75 2.75 0 0 0 4 5.75V8h2l2 2v2h2.25A2.75 2.75 0 0 0 13 9.25V7.334a10 10 0 0 0 3-7.142V0zM6.44 6.5a10 10 0 0 1 1.015-2H6.75c-.69 0-1.25.56-1.25 1.25v.75zm3.06 4v-.94a10 10 0 0 0 2-1.015v.705c0 .69-.56 1.25-1.25 1.25zm4.88-8.88a8.5 8.5 0 0 0-6.71 5.928l.782.783a8.5 8.5 0 0 0 5.928-6.71Z" clipRule="evenodd" className="rocket-body"></path>

      {/* Gas tail */}
      <g className="gas-tail">
        <path fill="orange" d="M7 12c0 1 0.5 1.5 1 2s1-1 1-2-0.5-1.5-1-2-1 1-1 2Z" />
        <path fill="red" d="M7.5 13c0 .5.25.75.5 1s.5-.5.5-1-.25-.75-.5-1-.5.5-.5 1Z" />
      </g>
    </svg>
  );
}
