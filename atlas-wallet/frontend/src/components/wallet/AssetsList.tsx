import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

interface AssetsListProps {
  balances: Record<string, string>;
}

export function AssetsList({ balances }: AssetsListProps) {
  const entries = Object.entries(balances);

  return (
    <div className="space-y-4 pt-4">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          All Assets
        </h3>
        <span className="text-[10px] text-muted-foreground">
          {entries.length} assets found
        </span>
      </div>

      <div className="rounded-md border border-border/50 bg-secondary/20 overflow-hidden">
        <Table>
          <TableHeader className="bg-muted/50">
            <TableRow>
              <TableHead className="w-[100px] text-xs uppercase font-semibold">
                Asset
              </TableHead>
              <TableHead className="text-right text-xs uppercase font-semibold">
                Balance
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {entries.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={2}
                  className="h-24 text-center text-xs text-muted-foreground"
                >
                  No assets found.
                </TableCell>
              </TableRow>
            ) : (
              entries.map(([asset, amount]) => (
                <TableRow key={asset}>
                  <TableCell className="font-medium text-xs font-mono">
                    {asset}
                  </TableCell>
                  <TableCell className="text-right font-mono text-xs">
                    {new Intl.NumberFormat("en-US", {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 6,
                    }).format(Number(amount))}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
