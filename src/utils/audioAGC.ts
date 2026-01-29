/**
 * Automatic Gain Control (AGC) for audio visualization
 * Industry best practice: track recent peak levels and normalize display relative to that
 * This ensures the meter shows visible movement for any input level
 */
export class AudioAGC {
  private peakLevel = 0.1; // Start with a small baseline
  private readonly attackTime = 0.3; // Fast attack - quickly adapt to louder sounds
  private readonly releaseTime = 2.0; // Slow release - gradually reduce gain when quiet
  private readonly minPeak = 0.05; // Minimum peak to prevent division by very small numbers
  private readonly maxPeak = 1.0; // Maximum peak
  private lastUpdateTime = Date.now();

  /**
   * Process incoming levels and return normalized values (0-1 range, visually meaningful)
   */
  process(levels: number[]): number[] {
    const now = Date.now();
    const deltaTime = (now - this.lastUpdateTime) / 1000;
    this.lastUpdateTime = now;

    // Find current max level
    const currentMax = Math.max(...levels, 0.001);

    // Update peak with attack/release dynamics
    if (currentMax > this.peakLevel) {
      // Attack: quickly rise to new peak
      const attackRate = 1 - Math.exp(-deltaTime / this.attackTime);
      this.peakLevel += (currentMax - this.peakLevel) * attackRate;
    } else {
      // Release: slowly decay peak
      const releaseRate = 1 - Math.exp(-deltaTime / this.releaseTime);
      this.peakLevel -= (this.peakLevel - currentMax) * releaseRate * 0.5;
    }

    // Clamp peak to valid range
    this.peakLevel = Math.max(this.minPeak, Math.min(this.maxPeak, this.peakLevel));

    // Normalize levels relative to current peak (AGC effect)
    // This makes the bars show significant movement even for quiet speech
    const normalizedLevels = levels.map((level) => {
      const normalized = level / this.peakLevel;
      // Apply slight curve for more pleasing visual (emphasize mid-range)
      return Math.pow(Math.min(1, normalized), 0.8);
    });

    return normalizedLevels;
  }

  reset() {
    this.peakLevel = 0.1;
    this.lastUpdateTime = Date.now();
  }
}
