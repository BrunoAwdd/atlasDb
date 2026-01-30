import { useEffect, useState } from "react";
import { Ticket } from "lucide-react";

interface AssetDefinition {
  id?: string;
  issuer: string;
  asset_type: any; // Simplified for display, or define enum
  name: string;
  symbol: string;
  asset_standard?: string;
  decimals: number;
  resource_url?: string;
}

export default function Assets() {
  const [assets, setAssets] = useState<AssetDefinition[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("http://localhost:3001/api/tokens")
      .then((res) => res.json())
      .then((data) => {
        console.log("API Data:", data);
        // API returns a Map: { "Issuer/Symbol": { ... } }
        if (!data || typeof data !== "object") {
          console.error("Invalid data format:", data);
          setAssets([]);
          setLoading(false);
          return;
        }

        // We convert it to an array of values
        const assetList = Object.entries(data).map(([key, val]) => {
          const record = val as any; // Cast to any to safely access properties if type mismatch matches
          return {
            id: key,
            issuer: record.issuer || "Unknown",
            asset_type: record.asset_type,
            name: record.name,
            symbol: record.symbol,
            asset_standard: record.asset_standard || undefined,
            decimals: record.decimals,
            resource_url: record.resource_url || undefined,
          } as AssetDefinition;
        });

        console.log("Parsed Assets:", assetList);
        setAssets(assetList);
        setLoading(false);
      })
      .catch((err) => {
        console.error("Failed to fetch tokens:", err);
        setLoading(false);
      });
  }, []);

  if (loading)
    return (
      <div className="p-8 text-center animate-pulse">Loading Assets...</div>
    );

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold flex items-center space-x-2">
          <Ticket className="text-green-500" />
          <span>Token Registry</span>
        </h2>
        <span className="text-sm text-muted-foreground">
          Total Assets: {assets.length}
        </span>
      </div>

      <div className="card overflow-hidden p-0">
        <table className="w-full">
          <thead>
            <tr className="bg-secondary/30 text-left">
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Symbol
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Name
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Decimals
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Issuer
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border/30">
            {assets.map((asset) => (
              <tr
                key={asset.id}
                className="hover:bg-secondary/10 transition-colors"
              >
                <td className="p-4 font-mono font-bold text-blue-400">
                  {asset.symbol}
                </td>
                <td className="p-4">{asset.name}</td>
                <td className="p-4 font-mono text-muted-foreground">
                  {asset.decimals}
                </td>
                <td className="p-4 font-mono text-xs text-muted-foreground break-all">
                  {asset.issuer}
                </td>
              </tr>
            ))}
            {assets.length === 0 && (
              <tr>
                <td
                  colSpan={4}
                  className="p-8 text-center text-muted-foreground"
                >
                  No registered assets found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
