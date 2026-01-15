import { useState, useRef, useEffect, useCallback, memo } from "react";

interface VolumeFaderProps {
  value: number;
  onChange: (value: number) => void; // Called on drag end (for backend)
  onValueChange?: (value: number) => void; // Called during drag (for display)
  min?: number;
  max?: number;
  disabled?: boolean;
  className?: string;
}

export function VolumeFader({
  value,
  onChange,
  onValueChange,
  min = -60,
  max = 18,  // Changed to +18dB
  disabled = false,
  className = "",
}: VolumeFaderProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [showTooltip, setShowTooltip] = useState(false);
  const [localValue, setLocalValue] = useState(value);
  const trackRef = useRef<HTMLDivElement>(null);
  const dragStartYRef = useRef(0);
  const dragStartValueRef = useRef(0);

  // Update local value when prop changes (but not during drag)
  useEffect(() => {
    if (!isDragging) {
      setLocalValue(value);
    }
  }, [value, isDragging]);

  // Convert dB to percentage (0-100)
  const dbToPercent = useCallback((db: number) => {
    return ((db - min) / (max - min)) * 100;
  }, [min, max]);

  // Convert percentage to dB
  const percentToDb = useCallback((percent: number) => {
    return min + (percent / 100) * (max - min);
  }, [min, max]);

  // Clamp value between min and max
  const clampValue = useCallback((val: number) => {
    return Math.max(min, Math.min(max, val));
  }, [min, max]);

  // Handle mouse/touch down
  const handleDragStart = useCallback((e: React.MouseEvent | React.TouchEvent) => {
    if (disabled) return;

    const clientY = 'touches' in e ? e.touches[0].clientY : e.clientY;
    dragStartYRef.current = clientY;
    dragStartValueRef.current = localValue;

    setIsDragging(true);
    setShowTooltip(true);

    // Add global event listeners
    document.addEventListener('mousemove', handleDragMove);
    document.addEventListener('mouseup', handleDragEnd);
    document.addEventListener('touchmove', handleDragMove, { passive: false });
    document.addEventListener('touchend', handleDragEnd);
  }, [disabled, localValue]);

  // Handle mouse/touch move
  const handleDragMove = useCallback((e: MouseEvent | TouchEvent) => {
    if (!isDragging || !trackRef.current) return;

    e.preventDefault();

    const clientY = 'touches' in e ? e.touches[0].clientY : e.clientY;
    const deltaY = dragStartYRef.current - clientY; // Inverted: up = increase
    const rect = trackRef.current.getBoundingClientRect();
    const trackHeight = rect.height;

    // Calculate sensitivity (Shift key for fine control)
    const isFineControl = (e as MouseEvent).shiftKey || false;
    const sensitivity = isFineControl ? 0.1 : 1.0;

    // Convert pixel movement to dB change
    const dbPerPixel = (max - min) / trackHeight;
    const dbChange = deltaY * dbPerPixel * sensitivity;
    const newValue = clampValue(dragStartValueRef.current + dbChange);

    setLocalValue(newValue);
    // Call onValueChange for display update during drag (don't call backend)
    onValueChange?.(newValue);
  }, [isDragging, max, min, clampValue, onValueChange]);

  // Handle mouse/touch end
  const handleDragEnd = useCallback(() => {
    setIsDragging(false);
    setShowTooltip(false);

    // Remove global event listeners
    document.removeEventListener('mousemove', handleDragMove);
    document.removeEventListener('mouseup', handleDragEnd);
    document.removeEventListener('touchmove', handleDragMove);
    document.removeEventListener('touchend', handleDragEnd);

    // Call onChange only on drag end to update backend
    onChange(localValue);
  }, [handleDragMove, localValue, onChange]);

  // Handle click on track
  const handleTrackClick = useCallback((e: React.MouseEvent) => {
    if (disabled || !trackRef.current) return;

    const rect = trackRef.current.getBoundingClientRect();
    const clickY = e.clientY - rect.top;
    const percent = 100 - (clickY / rect.height * 100); // Inverted
    const newValue = clampValue(percentToDb(percent));

    setLocalValue(newValue);
    onChange(newValue);
  }, [disabled, clampValue, percentToDb, onChange]);

  // Handle double-click to reset to 0dB
  const handleDoubleClick = useCallback(() => {
    if (disabled) return;

    setLocalValue(0);
    onChange(0);
  }, [disabled, onChange]);

  // Handle scroll wheel
  const handleWheel = useCallback((e: React.WheelEvent) => {
    if (disabled) return;

    e.preventDefault();
    const isFineControl = e.shiftKey;
    const step = isFineControl ? 0.1 : 1.0;
    const direction = e.deltaY > 0 ? -1 : 1;
    const newValue = clampValue(localValue + direction * step);

    setLocalValue(newValue);
    onChange(newValue);
  }, [disabled, localValue, clampValue, onChange]);

  // Handle keyboard
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (disabled) return;

    const step = e.shiftKey ? 0.1 : 1.0;

    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault();
        const upValue = clampValue(localValue + step);
        setLocalValue(upValue);
        onChange(upValue);
        break;
      case 'ArrowDown':
        e.preventDefault();
        const downValue = clampValue(localValue - step);
        setLocalValue(downValue);
        onChange(downValue);
        break;
      case 'Home':
        e.preventDefault();
        setLocalValue(max);
        onChange(max);
        break;
      case 'End':
        e.preventDefault();
        setLocalValue(min);
        onChange(min);
        break;
      case 'PageUp':
        e.preventDefault();
        const pgUpValue = clampValue(localValue + 6);
        setLocalValue(pgUpValue);
        onChange(pgUpValue);
        break;
      case 'PageDown':
        e.preventDefault();
        const pgDownValue = clampValue(localValue - 6);
        setLocalValue(pgDownValue);
        onChange(pgDownValue);
        break;
    }
  }, [disabled, localValue, clampValue, onChange, max, min]);

  const percent = dbToPercent(localValue);

  // 0dB marker position (75% from bottom when range is -60 to +18)
  const zeroDbPercent = dbToPercent(0);

  return (
    <div
      className={`relative ${className}`}
      onWheel={handleWheel}
      onKeyDown={handleKeyDown}
      tabIndex={disabled ? -1 : 0}
      role="slider"
      aria-label="Volume fader"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={localValue}
      aria-valuetext={`${localValue.toFixed(1)} dB`}
      aria-disabled={disabled}
    >
      {/* Tooltip */}
      {showTooltip && (
        <div className="absolute -top-10 left-1/2 -translate-x-1/2 z-50 pointer-events-none">
          <div className="px-3 py-1.5 bg-black text-white text-sm font-mono font-bold rounded shadow-lg whitespace-nowrap">
            {localValue.toFixed(1)} dB
          </div>
          {/* Arrow */}
          <div className="absolute left-1/2 -translate-x-1/2 top-full w-0 h-0 border-l-4 border-r-4 border-t-4 border-l-transparent border-r-transparent border-t-black" />
        </div>
      )}

      {/* Thin-line fader track */}
      <div
        ref={trackRef}
        className={`
          relative w-12 h-48 select-none touch-none
          ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
        `}
        onClick={handleTrackClick}
        onMouseDown={handleDragStart}
        onTouchStart={handleDragStart}
        onDoubleClick={handleDoubleClick}
        title={disabled ? undefined : "Drag to adjust • Double-click to reset to 0dB • Scroll wheel to fine-tune • Shift for fine control"}
      >
        {/* Thin track line */}
        <div className="absolute left-1/2 top-0 bottom-0 w-0.5 bg-slate-600 -translate-x-1/2" />

        {/* 0dB marker */}
        <div
          className="absolute left-0 right-0 h-px bg-slate-400"
          style={{ bottom: `${zeroDbPercent}%` }}
        />

        {/* Fader thumb (easy to grab) */}
        <div
          className={`
            absolute left-1/2 -translate-x-1/2 w-8 h-3
            bg-blue-500 rounded cursor-grab shadow-md
            transition-all duration-75
            ${isDragging ? 'bg-blue-400 scale-105 shadow-lg' : 'hover:bg-blue-400'}
            ${disabled ? 'opacity-50' : ''}
          `}
          style={{ bottom: `calc(${percent}% - 6px)` }}
        />
      </div>
    </div>
  );
}

// Memoize VolumeFader to prevent unnecessary re-renders
export const MemoizedVolumeFader = memo(VolumeFader, (prevProps, nextProps) => {
  return (
    prevProps.value === nextProps.value &&
    prevProps.min === nextProps.min &&
    prevProps.max === nextProps.max &&
    prevProps.disabled === nextProps.disabled &&
    prevProps.onChange === nextProps.onChange
  );
});
