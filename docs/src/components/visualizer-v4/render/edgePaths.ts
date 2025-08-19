/**
 * @fileoverview Edge path utilities
 *
 * Shared SVG path generators for custom edge rendering.
 */

/**
 * Generates a wavy (sinusoidal) SVG path between two points.
 * Behavior preserved from original implementation in edges.tsx.
 */
export function getWavyPath({
  sourceX,
  sourceY,
  targetX,
  targetY,
  amplitude = 10,
  frequency = 6,
}: {
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  amplitude?: number;
  frequency?: number;
}): string {
  const deltaX = targetX - sourceX;
  const deltaY = targetY - sourceY;
  const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);

  if (distance === 0) {
    return `M ${sourceX} ${sourceY}`;
  }

  // Angle of the straight line
  const angle = Math.atan2(deltaY, deltaX);

  // Number of segments for smooth curve
  const segments = Math.max(20, Math.floor(distance / 5));

  let path = `M ${sourceX} ${sourceY}`;

  for (let i = 1; i <= segments; i++) {
    const t = i / segments;

    // Base position along the straight line
    const baseX = sourceX + deltaX * t;
    const baseY = sourceY + deltaY * t;

    // Use actual distance traveled for consistent wave density
    const distanceTraveled = distance * t;
    const waveOffset = amplitude * Math.sin((frequency * Math.PI * distanceTraveled) / 50);

    // Apply perpendicular offset
    const offsetX = -waveOffset * Math.sin(angle);
    const offsetY = waveOffset * Math.cos(angle);

    const x = baseX + offsetX;
    const y = baseY + offsetY;

    path += ` L ${x} ${y}`;
  }

  return path;
}
