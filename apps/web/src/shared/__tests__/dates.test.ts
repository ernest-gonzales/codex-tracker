import { describe, expect, it } from "vitest";
import { buildRangeParams } from "../range";
import {
  formatDateInputValue,
  formatDateOnlyLocal,
  parseDateOnlyLocal,
  toRangeEndExclusive,
  toRangeStart
} from "../dates";

describe("date helpers", () => {
  it("formats date input values to YYYY-MM-DD", () => {
    expect(formatDateInputValue("2024-01-02T12:34:56.000Z")).toBe("2024-01-02");
  });

  it("builds inclusive custom ranges with day boundaries", () => {
    const start = toRangeStart("2024-02-10");
    const end = toRangeEndExclusive("2024-02-10");
    expect(start).toBe(new Date(2024, 1, 10, 0, 0, 0, 0).toISOString());
    expect(end).toBe(new Date(2024, 1, 11, 0, 0, 0, 0).toISOString());
  });

  it("round-trips date-only values without shifting the day", () => {
    const input = "2024-03-10";
    const expectedIso = new Date(2024, 2, 10).toISOString();
    expect(parseDateOnlyLocal(input)).toBe(expectedIso);
    expect(formatDateOnlyLocal(expectedIso)).toBe(input);

    const offsetMinutes = new Date(2024, 2, 10).getTimezoneOffset();
    if (offsetMinutes > 0) {
      const legacyIso = new Date(input).toISOString();
      expect(formatDateOnlyLocal(legacyIso)).not.toBe(input);
    }
  });

  it("builds preset ranges without custom dates", () => {
    expect(buildRangeParams("today", "2024-01-01", "2024-01-02")).toEqual({
      range: "today"
    });
  });
});
