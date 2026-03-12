export function formatAverageWpm(wpm: number): string {
  if (!Number.isFinite(wpm) || wpm <= 0) {
    return "0";
  }

  return wpm.toFixed(1);
}
