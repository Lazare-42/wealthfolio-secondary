import type { CompanyThesis } from "@/lib/types";
import { Badge } from "@wealthfolio/ui/components/ui/badge";

import { decimal } from "./formatters";

export function ThesesList({ theses }: { theses: CompanyThesis[] }) {
  return (
    <div className="space-y-2">
      {theses.map((thesis) => (
        <div key={thesis.id} className="rounded-md border p-3">
          <div className="mb-1 flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <span className="font-medium">{thesis.symbol}</span>
              {thesis.rating && <Badge variant="outline">{thesis.rating}</Badge>}
            </div>
            {thesis.confidence !== null && thesis.confidence !== undefined && (
              <span className="text-muted-foreground text-xs">
                {decimal.format(thesis.confidence)}
              </span>
            )}
          </div>
          <p className="text-muted-foreground line-clamp-3 text-sm">{thesis.thesis}</p>
        </div>
      ))}
      {theses.length === 0 && (
        <div className="text-muted-foreground rounded-md border border-dashed p-3 text-sm">
          Save a thesis above, or run an agent — model decisions are stored here with rating and
          confidence.
        </div>
      )}
    </div>
  );
}
