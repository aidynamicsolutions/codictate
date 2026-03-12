import { describe, expect, it } from "vitest";
import { formatAverageWpm } from "@/components/home/statsDisplayUtils";

describe("formatAverageWpm", () => {
  it("shows a single decimal place for positive averages", () => {
    expect(formatAverageWpm(115.19872488440114)).toBe("115.2");
  });

  it("returns zero for missing or invalid values", () => {
    expect(formatAverageWpm(0)).toBe("0");
    expect(formatAverageWpm(Number.NaN)).toBe("0");
    expect(formatAverageWpm(Number.POSITIVE_INFINITY)).toBe("0");
  });
});
