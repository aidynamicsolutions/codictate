import React from "react";

const CodictateLogo = ({
  width,
  height,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) => (
  <svg
    width={width || 100}
    height={height || 100}
    viewBox="0 0 100 100"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
  >
    {/* Border */}
    <rect
      x="5"
      y="5"
      width="90"
      height="90"
      rx="20"
      stroke="currentColor"
      strokeWidth="4"
      fill="none"
    />
    {/* Icon Content */}
    <g transform="scale(0.8) translate(12.5, 12.5)">
      <rect x="25" y="25" width="12" height="12" rx="5" fill="currentColor" />
      <path
        d="M63 36 L70 29 L77 36"
        stroke="currentColor"
        strokeWidth="5"
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
      />
      <rect x="25" y="60" width="8" height="12" rx="4" fill="currentColor" />
      <rect x="39" y="55" width="8" height="17" rx="4" fill="currentColor" />
      <rect x="53" y="55" width="8" height="17" rx="4" fill="currentColor" />
      <rect x="67" y="60" width="8" height="12" rx="4" fill="currentColor" />
    </g>
  </svg>
);

export default CodictateLogo;
