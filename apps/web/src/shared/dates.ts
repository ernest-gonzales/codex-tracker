export function formatDateOnlyLocal(value?: string | null) {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  const offset = parsed.getTimezoneOffset() * 60 * 1000;
  return new Date(parsed.getTime() - offset).toISOString().slice(0, 10);
}

export function parseDateOnlyLocal(value: string) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toISOString();
}

export function parseDateInput(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  if (trimmed.includes("T")) {
    const parsed = new Date(trimmed);
    if (!Number.isNaN(parsed.getTime())) {
      return parsed;
    }
  }
  const parts = trimmed.split("-");
  if (parts.length === 3) {
    const [year, month, day] = parts.map((part) => Number(part));
    if ([year, month, day].every((part) => Number.isFinite(part))) {
      return new Date(year, month - 1, day);
    }
  }
  const parsed = new Date(trimmed);
  if (!Number.isNaN(parsed.getTime())) {
    return parsed;
  }
  return null;
}

export function formatDateInputValue(value?: string | null) {
  if (!value) return "";
  const parsed = parseDateInput(value);
  if (!parsed) {
    return value;
  }
  const year = String(parsed.getFullYear());
  const month = String(parsed.getMonth() + 1).padStart(2, "0");
  const day = String(parsed.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function toRangeStart(value?: string) {
  if (!value) return undefined;
  const parsed = parseDateInput(value);
  if (!parsed) return undefined;
  const start = new Date(parsed.getFullYear(), parsed.getMonth(), parsed.getDate(), 0, 0, 0, 0);
  return start.toISOString();
}

export function toRangeEndExclusive(value?: string) {
  if (!value) return undefined;
  const parsed = parseDateInput(value);
  if (!parsed) return undefined;
  const end = new Date(parsed.getFullYear(), parsed.getMonth(), parsed.getDate(), 0, 0, 0, 0);
  // Shift to next-day start so the selected end date is treated as inclusive.
  end.setDate(end.getDate() + 1);
  return end.toISOString();
}
