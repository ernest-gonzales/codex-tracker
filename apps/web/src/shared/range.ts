import type { RangeParams } from "../domain/types";
import { RANGE_OPTIONS, type RangeValue } from "./constants";
import { toRangeEndExclusive, toRangeStart } from "./dates";

export function buildRangeParams(range: RangeValue, start?: string, end?: string): RangeParams {
  if (range === "custom") {
    return {
      start: toRangeStart(start),
      end: toRangeEndExclusive(end)
    };
  }
  return { range };
}

export function rangeLabel(value: RangeValue) {
  return RANGE_OPTIONS.find((option) => option.value === value)?.label ?? value;
}
