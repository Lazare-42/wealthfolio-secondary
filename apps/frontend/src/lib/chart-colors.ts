/**
 * Generate distinct colors for accounts in charts.
 * Uses equidistant hues to maximise distinction between accounts; saturation
 * and lightness are fixed to stay consistent with the theme.
 */
export function generateDistinctColors(count: number): string[] {
  const colors: string[] = [];
  // Start at 210 (blue) — nicer than red (0) for a finance chart.
  const startHue = 210;

  for (let i = 0; i < count; i++) {
    // Evenly space hues around the 360° wheel.
    const hue = (startHue + (i * 360) / Math.max(count, 1)) % 360;
    colors.push(`hsl(${hue} 70% 50%)`);
  }

  return colors;
}
