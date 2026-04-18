function getDurationParts(totalSeconds: number) {
  const safeSeconds = Math.max(0, Math.floor(totalSeconds));

  return {
    hours: Math.floor(safeSeconds / 3600),
    minutes: Math.floor((safeSeconds % 3600) / 60),
    seconds: safeSeconds % 60,
  };
}

function pad2(value: number) {
  return String(value).padStart(2, '0');
}

export function formatClockTime(totalSeconds: number) {
  const { hours, minutes, seconds } = getDurationParts(totalSeconds);

  if (hours > 0) {
    return `${hours}:${pad2(minutes)}:${pad2(seconds)}`;
  }

  return `${pad2(minutes)}:${pad2(seconds)}`;
}

export function formatCompactDuration(totalSeconds: number) {
  const { hours, minutes, seconds } = getDurationParts(totalSeconds);

  if (hours > 0) {
    return `${hours}:${pad2(minutes)}:${pad2(seconds)}`;
  }

  if (minutes > 0) {
    return `${minutes}:${pad2(seconds)}`;
  }

  if (seconds > 0) {
    return `0:${pad2(seconds)}`;
  }

  return '';
}
