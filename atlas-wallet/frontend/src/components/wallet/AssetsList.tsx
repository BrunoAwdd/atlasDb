import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Pin, PinOff } from "lucide-react";
import { getAssetSymbol } from "@/lib/assets";
import { useFavorites } from "@/hooks/useFavorites";

interface AssetsListProps {
  balances: Record<string, string>;
}

export function AssetsList({ balances }: AssetsListProps) {
  const entries = Object.entries(balances);
  const { isFavorite, toggleFavorite } = useFavorites();

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
              <TableHead className="w-[50px] text-center text-xs uppercase font-semibold">
                Pin
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {entries.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={3}
                  className="h-24 text-center text-xs text-muted-foreground"
                >
                  No assets found.
                </TableCell>
              </TableRow>
            ) : (
              entries.map(([asset, amount]) => {
                const isPinned = isFavorite(asset);
                return (
                  <TableRow key={asset}>
                    <TableCell className="font-medium text-xs font-mono">
                      {getAssetSymbol(asset)}
                    </TableCell>
                    <TableCell className="text-right font-mono text-xs">
                      {new Intl.NumberFormat("en-US", {
                        minimumFractionDigits: 2,
                        maximumFractionDigits: 6,
                      }).format(Number(amount))}
                    </TableCell>
                    <TableCell className="text-center">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6"
                        onClick={() => toggleFavorite(asset)}
                        title={isPinned ? "Unpin asset" : "Pin asset"}
                      >
                        {isPinned ? (
                          <PinOff className="h-3 w-3 text-primary" />
                        ) : (
                          <Pin className="h-3 w-3 text-muted-foreground" />
                        )}
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
