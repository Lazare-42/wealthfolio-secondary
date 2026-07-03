import { TableCell, TableRow } from "@wealthfolio/ui/components/ui/table";

export function EmptyRow({ colSpan, label }: { colSpan: number; label: string }) {
  return (
    <TableRow>
      <TableCell colSpan={colSpan} className="text-muted-foreground h-24 text-center text-sm">
        {label}
      </TableCell>
    </TableRow>
  );
}
