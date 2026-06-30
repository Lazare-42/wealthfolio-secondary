import { memo } from "react";

import { cn } from "@/lib/utils";
import type { ArtifactColumn, ArtifactTableData } from "../../types";

const numberFormat = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });
const decimalFormat = new Intl.NumberFormat(undefined, {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

/**
 * Render a cell value using the column's format hint. Pre-formatted strings
 * from the model pass through untouched; only raw numbers are formatted, so the
 * model stays in control when it wants exact text.
 */
function formatCell(value: string | number | null | undefined, column: ArtifactColumn): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "string") return value;

  switch (column.format) {
    case "currency":
      return decimalFormat.format(value);
    case "percent":
      return `${numberFormat.format(value)}%`;
    case "number":
      return numberFormat.format(value);
    default:
      return String(value);
  }
}

function alignClass(column: ArtifactColumn): string {
  const align = column.align ?? (column.format && column.format !== "text" ? "right" : "left");
  if (align === "right") return "text-right tabular-nums";
  if (align === "center") return "text-center";
  return "text-left";
}

function ArtifactTableImpl({ table }: { table: ArtifactTableData }) {
  const { columns, rows } = table;
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-separate border-spacing-0 text-sm">
        <thead>
          <tr>
            {columns.map((column) => (
              <th
                key={column.key}
                className={cn(
                  "bg-muted text-muted-foreground sticky top-0 whitespace-nowrap border-b px-3 py-2 text-xs font-semibold",
                  alignClass(column),
                )}
              >
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, rowIndex) => (
            <tr key={rowIndex} className="hover:bg-muted/40">
              {columns.map((column) => (
                <td
                  key={column.key}
                  className={cn("whitespace-nowrap border-b px-3 py-2", alignClass(column))}
                >
                  {formatCell(row[column.key], column)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length === 0 ? (
        <p className="text-muted-foreground px-3 py-6 text-center text-sm">No rows.</p>
      ) : null}
    </div>
  );
}

export const ArtifactTable = memo(ArtifactTableImpl);
