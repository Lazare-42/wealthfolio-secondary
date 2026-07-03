const money = new Intl.NumberFormat(undefined, {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

export const decimal = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 2,
});

export function formatMoney(value: number) {
  return money.format(Number.isFinite(value) ? value : 0);
}

export function formatPct(value: number) {
  const safe = Number.isFinite(value) ? value : 0;
  return `${safe >= 0 ? "+" : ""}${decimal.format(safe)}%`;
}

export function statusVariant(
  status: string,
): "default" | "secondary" | "success" | "warning" | "outline" {
  if (status === "active" || status === "completed" || status === "executed") return "success";
  if (status === "running") return "warning";
  if (status === "settled") return "secondary";
  return "outline";
}
