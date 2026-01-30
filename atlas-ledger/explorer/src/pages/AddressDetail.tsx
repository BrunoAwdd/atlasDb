import { useParams } from "react-router-dom";
import { useEffect, useState } from "react";
import { Copy, Wallet, ArrowRightLeft } from "lucide-react";
import { getAssetSymbol } from "../lib/assets";

interface AccountState {
  balances: Record<string, number>;
  nonce: number;
  last_transaction_hash?: string;
  last_entry_id?: string;
}

interface TxDto {
  tx_hash: string;
  from: string;
  to: string;
  amount: string;
  asset: string;
  timestamp: number;
  memo: string;
  fee_payer?: string;
}

export default function AddressDetail() {
  const { address } = useParams();
  const [data, setData] = useState<AccountState | null>(null);
  const [history, setHistory] = useState<TxDto[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!address) return;

    const fetchData = async () => {
      setLoading(true);
      try {
        // 1. Fetch Balances (Using legacy balance endpoint or querying accounts directly?)
        // The /api/accounts endpoint returns a map of ALL accounts. That's inefficient but currently what we have.
        // But we also have /api/balance?query=address

        const balRes = await fetch(
          `http://localhost:3001/api/balance?query=${encodeURIComponent(address)}`,
        );
        const balJson = await balRes.json();

        // Reconstruct AccountState-like object from balance endpoint??
        // Actually, /api/balance returns { address, balance, asset, nonce }.
        // It only returns ONE asset (ATLAS or USD default?).

        // To get ALL balances, we might need a new endpoint or scan /api/accounts.
        // For now, let's use /api/balance for at least the main Asset (ATLAS) and Nonce.
        // AND also fetch history.

        const listRes = await fetch(
          `http://localhost:3001/api/transactions?query=${encodeURIComponent(address)}&limit=50`,
        );
        const listJson = await listRes.json();
        setHistory(listJson.transactions);

        // API now returns full balances map in `balances` field (strings)
        // We need to parse strings to numbers if interface assumes numbers,
        // OR update interface. Let's cast to number for compatibility or keep as string?
        // Backend rest.rs sends string values.
        // Let's update the interface AccountState to use string or handle conversion.
        // For visual simplicity, keep as string in UI or display directly.

        // But AccountState interface (line 6) defines balances: Record<string, number>.
        // I should update line 6. For now, parseInt/Float.

        const balancesMap: Record<string, number> = {};
        if (balJson.balances) {
          Object.entries(balJson.balances).forEach(([k, v]) => {
            balancesMap[k] = Number(v);
          });
        } else {
          // Fallback for legacy response?
          balancesMap[balJson.asset] = parseFloat(balJson.balance);
        }

        setData({
          balances: balancesMap,
          nonce: balJson.nonce || 0,
        });
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, [address]);

  if (loading)
    return (
      <div className="p-8 text-center animate-pulse">
        Loading Address Data...
      </div>
    );
  if (!data)
    return (
      <div className="p-8 text-center text-red-500">Address Not Found</div>
    );

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div className="card p-6 border border-border/50 bg-secondary/20 rounded-xl space-y-4">
        <div className="flex items-center space-x-3">
          <div className="p-3 bg-blue-500/20 rounded-lg text-blue-400">
            <Wallet size={24} />
          </div>
          <div>
            <h2 className="text-xl font-bold">Address Details</h2>
            <div className="flex items-center space-x-2 text-muted-foreground font-mono text-sm">
              <span>{address}</span>
              <Copy size={14} className="cursor-pointer hover:text-white" />
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
          <div className="p-4 bg-background/50 rounded-lg border border-border/30">
            <p className="text-sm text-muted-foreground uppercase font-semibold">
              Nonce
            </p>
            <p className="text-2xl font-mono">{data.nonce}</p>
          </div>
          <div className="p-4 bg-background/50 rounded-lg border border-border/30">
            <p className="text-sm text-muted-foreground uppercase font-semibold">
              Primary Balance
            </p>
            <p className="text-2xl font-mono">
              {data.balances["passivo:wallet:mint/ATLAS"] ||
                Object.values(data.balances)[0] ||
                0}{" "}
              {data.balances["passivo:wallet:mint/ATLAS"]
                ? "ATLAS"
                : getAssetSymbol(Object.keys(data.balances)[0] || "")}
            </p>
          </div>

          <div className="col-span-1 md:col-span-3 p-4 bg-background/50 rounded-lg border border-border/30">
            <p className="text-sm text-muted-foreground uppercase font-semibold mb-3">
              Asset Portfolio
            </p>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {Object.entries(data.balances)
                .filter(([assetId]) => assetId !== "passivo:wallet:mint/ATLAS")
                .map(([assetId, bal]) => (
                  <div
                    key={assetId}
                    className="flex flex-col bg-secondary/20 p-3 rounded border border-border/10"
                  >
                    <span className="text-xs text-muted-foreground font-mono mb-1">
                      {getAssetSymbol(assetId)}
                    </span>
                    <span className="font-mono font-bold text-lg">{bal}</span>
                  </div>
                ))}
              {Object.keys(data.balances).filter(
                (k) => k !== "passivo:wallet:mint/ATLAS",
              ).length === 0 && (
                <span className="text-muted-foreground text-sm">
                  No other assets found
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="card overflow-hidden p-0">
        <div className="p-6 flex items-center space-x-2 border-b border-border/50">
          <ArrowRightLeft size={20} className="text-muted-foreground" />
          <h3 className="text-lg font-bold">Transactions</h3>
        </div>

        <table className="w-full">
          <thead>
            <tr className="bg-secondary/30 text-left">
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Hash
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Method
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Related
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Amount
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Asset
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Date
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border/30">
            {history.map((tx) => {
              const isIn =
                tx.to.replace("passivo:wallet:", "").toLowerCase() ===
                (address || "").replace("passivo:wallet:", "").toLowerCase();
              return (
                <tr
                  key={tx.tx_hash}
                  className="hover:bg-secondary/10 transition-colors"
                >
                  <td className="p-4">
                    <span className="font-mono text-blue-400 hover:text-blue-300 transition-colors cursor-pointer">
                      {tx.tx_hash.substring(0, 12)}...
                    </span>
                  </td>
                  <td className="p-4">
                    <span
                      className={`badge ${isIn ? "bg-green-500/20 text-green-400" : "bg-yellow-500/20 text-yellow-400"}`}
                    >
                      {isIn ? "IN" : "OUT"}
                    </span>
                  </td>
                  <td className="p-4">
                    <span className="font-mono text-muted-foreground">
                      {isIn
                        ? tx.from
                            .replace("passivo:wallet:", "")
                            .substring(0, 12)
                        : tx.to.replace("passivo:wallet:", "").substring(0, 12)}
                      ...
                    </span>
                  </td>
                  <td className="p-4 font-mono">{tx.amount}</td>
                  <td className="p-4">
                    <span className="text-xs text-muted-foreground">
                      {getAssetSymbol(tx.asset)}
                    </span>
                  </td>
                  <td className="p-4 text-xs text-muted-foreground">
                    {new Date(tx.timestamp * 1000).toLocaleString()}
                  </td>
                </tr>
              );
            })}
            {history.length === 0 && (
              <tr>
                <td
                  colSpan={6}
                  className="p-8 text-center text-muted-foreground"
                >
                  No transactions found
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
