import { useState, useEffect, useRef } from 'react';

const ChapterRangeSlider = ({ 
  min = 0, 
  max = 100, 
  onRangeChange, 
  initialStart = null, 
  initialEnd = null,
  step = 1,
  disabled = false 
}) => {
  const [start, setStart] = useState(initialStart || min);
  const [end, setEnd] = useState(initialEnd || max);
  const [isDragging, setIsDragging] = useState(false);
  const [dragType, setDragType] = useState(null); // 'start' or 'end'
  const sliderRef = useRef(null);

  useEffect(() => {
    if (onRangeChange) {
      onRangeChange({ start, end });
    }
  }, [start, end, onRangeChange]);

  const getPercentage = (value) => {
    return ((value - min) / (max - min)) * 100;
  };

  const getValueFromPercentage = (percentage) => {
    return Math.round((percentage / 100) * (max - min) + min);
  };

  const handleMouseDown = (e, type) => {
    if (disabled) return;
    e.preventDefault();
    setIsDragging(true);
    setDragType(type);
  };

  const handleMouseMove = (e) => {
    if (!isDragging || !sliderRef.current) return;

    const rect = sliderRef.current.getBoundingClientRect();
    const clientX = e.touches ? e.touches[0].clientX : e.clientX;
    const percentage = Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
    const value = getValueFromPercentage(percentage);

    if (dragType === 'start') {
      const newStart = Math.min(value, end - step);
      setStart(newStart);
    } else if (dragType === 'end') {
      const newEnd = Math.max(value, start + step);
      setEnd(newEnd);
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    setDragType(null);
  };

  const handleClick = (e) => {
    if (disabled || isDragging) return;

    const rect = sliderRef.current.getBoundingClientRect();
    const clientX = e.touches ? e.touches[0].clientX : e.clientX;
    const percentage = ((clientX - rect.left) / rect.width) * 100;
    const value = getValueFromPercentage(percentage);

    // Determine which handle to move based on which is closer
    const startDist = Math.abs(value - start);
    const endDist = Math.abs(value - end);

    if (startDist < endDist) {
      const newStart = Math.min(value, end - step);
      setStart(newStart);
    } else {
      const newEnd = Math.max(value, start + step);
      setEnd(newEnd);
    }
  };

  // Add global event listeners for smooth dragging
  useEffect(() => {
    if (isDragging) {
      const handleGlobalMouseMove = (e) => handleMouseMove(e);
      const handleGlobalMouseUp = () => handleMouseUp();
      const handleGlobalTouchMove = (e) => {
        e.preventDefault();
        handleMouseMove(e);
      };
      const handleGlobalTouchEnd = () => handleMouseUp();

      document.addEventListener('mousemove', handleGlobalMouseMove);
      document.addEventListener('mouseup', handleGlobalMouseUp);
      document.addEventListener('touchmove', handleGlobalTouchMove, { passive: false });
      document.addEventListener('touchend', handleGlobalTouchEnd);

      return () => {
        document.removeEventListener('mousemove', handleGlobalMouseMove);
        document.removeEventListener('mouseup', handleGlobalMouseUp);
        document.removeEventListener('touchmove', handleGlobalTouchMove);
        document.removeEventListener('touchend', handleGlobalTouchEnd);
      };
    }
  }, [isDragging, dragType, start, end, step]);

  const startPercentage = getPercentage(start);
  const endPercentage = getPercentage(end);

  return (
    <div className="w-full">
      <div className="mb-4">
        <div className="flex justify-between text-sm text-gray-600 dark:text-gray-400 mb-2">
          <span>Chapter {start}</span>
          <span>Chapter {end}</span>
        </div>
        <div className="flex justify-between text-xs text-gray-500 dark:text-gray-500">
          <span>Min: {min}</span>
          <span>Max: {max}</span>
        </div>
      </div>

      <div 
        ref={sliderRef}
        className={`relative h-6 w-full cursor-pointer ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
        onClick={handleClick}
      >
        {/* Track */}
        <div className="absolute top-1/2 transform -translate-y-1/2 w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full">
          {/* Selected range */}
          <div 
            className="absolute h-full bg-blue-500 dark:bg-blue-400 rounded-full"
            style={{
              left: `${startPercentage}%`,
              width: `${endPercentage - startPercentage}%`
            }}
          />
        </div>

        {/* Start handle */}
        <div
          className={`absolute top-1/2 transform -translate-y-1/2 w-4 h-4 bg-blue-600 dark:bg-blue-500 rounded-full border-2 border-white dark:border-gray-800 shadow-lg cursor-grab active:cursor-grabbing ${disabled ? 'cursor-not-allowed' : ''} ${isDragging && dragType === 'start' ? 'scale-110' : ''}`}
          style={{ left: `${startPercentage}%`, marginLeft: '-8px' }}
          onMouseDown={(e) => handleMouseDown(e, 'start')}
          onTouchStart={(e) => handleMouseDown(e, 'start')}
        />

        {/* End handle */}
        <div
          className={`absolute top-1/2 transform -translate-y-1/2 w-4 h-4 bg-blue-600 dark:bg-blue-500 rounded-full border-2 border-white dark:border-gray-800 shadow-lg cursor-grab active:cursor-grabbing ${disabled ? 'cursor-not-allowed' : ''} ${isDragging && dragType === 'end' ? 'scale-110' : ''}`}
          style={{ left: `${endPercentage}%`, marginLeft: '-8px' }}
          onMouseDown={(e) => handleMouseDown(e, 'end')}
          onTouchStart={(e) => handleMouseDown(e, 'end')}
        />
      </div>
    </div>
  );
};

export default ChapterRangeSlider; 