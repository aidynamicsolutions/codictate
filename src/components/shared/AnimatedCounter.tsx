import React, { useEffect, useState, useRef } from "react";

interface AnimatedCounterProps {
  value: number;
  duration?: number;
  formatter?: (value: number) => React.ReactNode;
  className?: string;
  start?: number;
}

export const AnimatedCounter: React.FC<AnimatedCounterProps> = ({
  value,
  duration = 2000,
  formatter = (val) => Math.round(val).toString(),
  className,
  start = 0,
}) => {
  const [displayValue, setDisplayValue] = useState(start);
  const startTimeRef = useRef<number | null>(null);
  const startValueRef = useRef(start);
  const endValueRef = useRef(value);
  const animationFrameRef = useRef<number | null>(null);

  useEffect(() => {
    // If value hasn't effectively changed (allowing for small float diffs if needed, though exact check is usually fine), 
    // we don't need to re-animate unless it's the first mount.
    if (endValueRef.current === value && startTimeRef.current !== null) {
      return;
    }

    startValueRef.current = displayValue;
    endValueRef.current = value;
    startTimeRef.current = null;
    
    // Check if reduced motion is preferred
    const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    if (prefersReducedMotion) {
      setDisplayValue(value);
      return;
    }

    const animate = (timestamp: number) => {
      if (!startTimeRef.current) startTimeRef.current = timestamp;
      const progress = timestamp - startTimeRef.current;
      const progressRatio = Math.min(progress / duration, 1);

      // Easing function: Ease Out Expo (smoother, more gradual stop)
      const easeOutExpo = (x: number): number => {
        return x === 1 ? 1 : 1 - Math.pow(2, -10 * x);
      };

      const easedProgress = easeOutExpo(progressRatio);
      const current = startValueRef.current + (endValueRef.current - startValueRef.current) * easedProgress;

      setDisplayValue(current);

      if (progress < duration) {
        animationFrameRef.current = requestAnimationFrame(animate);
      } else {
        setDisplayValue(endValueRef.current);
      }
    };

    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [value, duration]);

  return <span className={className}>{formatter(displayValue)}</span>;
};
