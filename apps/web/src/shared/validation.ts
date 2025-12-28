import type { PricingRule } from "../domain/types";

export type PricingIssue = {
  index: number;
  message: string;
  field?: "model_pattern" | "input_per_1m" | "cached_input_per_1m" | "output_per_1m" | "range";
};

export function validatePricingRules(rules: PricingRule[]): PricingIssue[] {
  const issues: PricingIssue[] = [];
  const rangesByModel = new Map<
    string,
    Array<{ index: number; start: number; end: number | null }>
  >();

  rules.forEach((rule, index) => {
    if (!rule.model_pattern.trim()) {
      issues.push({ index, message: "Model pattern is required.", field: "model_pattern" });
    }
    if (rule.input_per_1m < 0) {
      issues.push({ index, message: "Input price must be zero or higher.", field: "input_per_1m" });
    }
    if (rule.cached_input_per_1m < 0) {
      issues.push({
        index,
        message: "Cached input price must be zero or higher.",
        field: "cached_input_per_1m"
      });
    }
    if (rule.output_per_1m < 0) {
      issues.push({
        index,
        message: "Output price must be zero or higher.",
        field: "output_per_1m"
      });
    }
    const start = new Date(rule.effective_from).getTime();
    const end = rule.effective_to ? new Date(rule.effective_to).getTime() : null;
    if (!Number.isNaN(start) && end !== null && !Number.isNaN(end) && end < start) {
      issues.push({
        index,
        message: "Effective end must be after the start date.",
        field: "range"
      });
    }
    const list = rangesByModel.get(rule.model_pattern) ?? [];
    list.push({ index, start, end });
    rangesByModel.set(rule.model_pattern, list);
  });

  rangesByModel.forEach((ranges) => {
    ranges.sort((a, b) => (a.start || 0) - (b.start || 0));
    for (let i = 0; i < ranges.length; i += 1) {
      const current = ranges[i];
      if (Number.isNaN(current.start)) {
        continue;
      }
      for (let j = i + 1; j < ranges.length; j += 1) {
        const next = ranges[j];
        if (Number.isNaN(next.start)) {
          continue;
        }
        const currentEnd = current.end ?? Number.POSITIVE_INFINITY;
        const nextEnd = next.end ?? Number.POSITIVE_INFINITY;
        const overlaps = current.start <= nextEnd && next.start <= currentEnd;
        if (overlaps) {
          issues.push({
            index: current.index,
            message: "Overlapping effective ranges for this model pattern.",
            field: "range"
          });
          issues.push({
            index: next.index,
            message: "Overlapping effective ranges for this model pattern.",
            field: "range"
          });
        }
      }
    }
  });

  return issues;
}
