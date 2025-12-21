import type { SVGProps } from "react";

export function CodexTrackerLogo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 64 64"
      width="100%"
      height="100%"
      aria-hidden="true"
      focusable="false"
      {...props}
    >
      <defs>
        <linearGradient id="ctg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#7df9ff" />
          <stop offset="0.55" stopColor="#4bb0ff" />
          <stop offset="1" stopColor="#ffb347" />
        </linearGradient>
      </defs>
      <circle cx="32" cy="32" r="24" fill="none" stroke="url(#ctg)" strokeWidth="6" />
      <g fill="url(#ctg)">
        <rect x="20" y="31" width="6" height="15" rx="3" />
        <rect x="29" y="25" width="6" height="21" rx="3" />
        <rect x="38" y="19" width="6" height="27" rx="3" />
      </g>
      <path
        d="M13 35c3.5 8.5 10.6 14 19.1 15.2"
        stroke="rgba(253, 250, 255, 0.45)"
        strokeWidth="2"
        strokeLinecap="round"
        fill="none"
      />
    </svg>
  );
}
