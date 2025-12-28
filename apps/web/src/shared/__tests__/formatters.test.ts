import { describe, expect, it } from "vitest";
import { formatCurrency, formatNumber, formatPercent, formatPercentWhole } from "../formatters";

describe("formatters", () => {
  it("returns placeholders for empty values", () => {
    expect(formatCurrency(null)).toBe("n/a");
    expect(formatNumber(undefined)).toBe("-");
  });

  it("formats percent helpers", () => {
    expect(formatPercent(12.34)).toBe("12.3%");
    expect(formatPercentWhole(12.34)).toBe("12%");
  });
});
