const resolvedLocale = typeof navigator !== "undefined" ? navigator.language : "en-US";

const currency = new Intl.NumberFormat(resolvedLocale, {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2
});
const numberFormat = new Intl.NumberFormat(resolvedLocale);
const compactNumberFormat = new Intl.NumberFormat(resolvedLocale, {
  notation: "compact",
  compactDisplay: "short",
  maximumFractionDigits: 1
});
const dateTimeFormat = new Intl.DateTimeFormat(resolvedLocale, {
  month: "short",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit"
});
const dateFormat = new Intl.DateTimeFormat(resolvedLocale, {
  year: "numeric",
  month: "short",
  day: "2-digit"
});
const hourFormat = new Intl.DateTimeFormat(resolvedLocale, {
  hour: "2-digit",
  minute: "2-digit"
});

export function formatCurrency(value: number | null | undefined) {
  if (value === null || value === undefined) return "n/a";
  return currency.format(value);
}

export function formatNumber(value: number | null | undefined) {
  if (value === null || value === undefined) return "-";
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) {
    return compactNumberFormat.format(value);
  }
  return numberFormat.format(value);
}

export function formatBucketLabel(value: string, bucket?: "hour" | "day") {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  if (bucket === "hour") {
    return hourFormat.format(parsed);
  }
  if (bucket === "day") {
    return dateFormat.format(parsed);
  }
  if (
    parsed.getHours() === 0 &&
    parsed.getMinutes() === 0 &&
    parsed.getSeconds() === 0
  ) {
    return dateFormat.format(parsed);
  }
  return dateTimeFormat.format(parsed);
}

export function formatPercent(value: number | null | undefined) {
  if (value === null || value === undefined) return "-";
  return `${value.toFixed(1)}%`;
}

export function formatPercentWhole(value: number | null | undefined) {
  if (value === null || value === undefined) return "-";
  return `${Math.round(value)}%`;
}

export function formatLimitPercentLeft(value: number | null | undefined) {
  if (value === null || value === undefined) return "100%";
  if (value === 0) return "100%";
  return formatPercentWhole(value);
}

export function formatCostPerMillion(
  cost: number | null | undefined,
  tokens: number | null | undefined
) {
  if (cost === null || cost === undefined || !tokens) {
    return "-";
  }
  const perMillion = (cost / tokens) * 1_000_000;
  return currency.format(perMillion);
}

export function formatResetLabel(value: string | null | undefined) {
  if (!value) return "-";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return dateTimeFormat.format(parsed);
}

export function formatRelativeReset(value: string | null | undefined) {
  if (!value) return "-";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  const diffMs = parsed.getTime() - Date.now();
  const diffMinutes = Math.round(Math.abs(diffMs) / 60000);
  const hours = Math.floor(diffMinutes / 60);
  const minutes = diffMinutes % 60;
  const label = hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  return diffMs >= 0 ? `in ${label}` : `${label} ago`;
}

export function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return dateTimeFormat.format(parsed);
}

export function formatSessionLabel(sessionId: string) {
  const trimmed = sessionId.trim();
  if (trimmed.includes("/")) {
    const parts = trimmed.split("/").filter(Boolean);
    return parts[parts.length - 1] ?? trimmed;
  }
  if (trimmed.length > 18) {
    return `${trimmed.slice(0, 8)}...${trimmed.slice(-6)}`;
  }
  return trimmed;
}

export function formatEffort(value: string | null | undefined) {
  if (!value) return "unknown";
  return value;
}
