import { useEffect, useState } from "react";
import { Ticket } from "lucide-react";

interface TokenMetadata {
  symbol: string;
  name: string;
  decimals: number;
  issuer: string;
  logo?: string;
}

export default function Assets() {
  const [tokens, setTokens] = useState<TokenMetadata[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("http://localhost:3001/api/tokens")
      .then((res) => res.json())
      .then((data) => {
        // API returns a Map: { "ATLAS": { ... }, "USD": { ... } }
        // We convert it to an array of values
        const tokenList = Object.values(data) as TokenMetadata[];
        setTokens(tokenList);
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
          Total Assets: {tokens.length}
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
            {tokens.map((token) => (
              <tr
                key={token.symbol}
                className="hover:bg-secondary/10 transition-colors"
              >
                <td className="p-4 font-mono font-bold text-blue-400">
                  {token.symbol}
                </td>
                <td className="p-4">{token.name}</td>
                <td className="p-4 font-mono text-muted-foreground">
                  {token.decimals}
                </td>
                <td className="p-4 font-mono text-xs text-muted-foreground break-all">
                  {token.issuer}
                </td>
              </tr>
            ))}
            {tokens.length === 0 && (
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
