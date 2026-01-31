import { useEffect, useState } from "react";
import { Activity } from "lucide-react";
import { Link } from "react-router-dom";
import StatusBadge from "../components/StatusBadge";
import { getAssetSymbol } from "../lib/assets";

interface TxDto {
  tx_hash: string;
  from: string;
  to: string;
  amount: string;
  asset: string;
  timestamp: number;
  memo: string;
}

export default function Transactions() {
  const [transactions, setTransactions] = useState<TxDto[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("http://localhost:3001/api/transactions?limit=20")
      .then((res) => res.json())
      .then((data) => {
        setTransactions(data.transactions);
        setLoading(false);
      })
      .catch((err) => {
        console.error("Failed to fetch transactions:", err);
        setLoading(false);
      });
  }, []);

  if (loading)
    return (
      <div className="p-8 text-center animate-pulse">
        Loading Transaction History...
      </div>
    );

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold flex items-center space-x-2">
          <Activity className="text-blue-500" />
          <span>Latest Transactions</span>
        </h2>
      </div>

      <div className="card overflow-hidden p-0">
        <table className="w-full">
          <thead>
            <tr className="bg-secondary/30 text-left">
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Tx Hash
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Method
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                From
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                To
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Amount
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Time
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border/30">
            {transactions.map((tx) => (
              <tr
                key={tx.tx_hash}
                className="hover:bg-secondary/10 transition-colors"
              >
                <td className="p-4">
                  <Link
                    to={`/tx/${tx.tx_hash}`}
                    className="font-mono text-blue-400 hover:text-blue-300 transition-colors"
                  >
                    {tx.tx_hash.substring(0, 14)}...
                  </Link>
                </td>
                <td className="p-4">
                  <StatusBadge status="success" text="Transfer" />
                </td>
                <td className="p-4">
                  <Link
                    to={`/address/${tx.from}`}
                    className="font-mono text-xs text-muted-foreground hover:text-white transition-colors"
                  >
                    {tx.from.replace("wallet:", "").substring(0, 10)}...
                  </Link>
                </td>
                <td className="p-4">
                  <Link
                    to={`/address/${tx.to}`}
                    className="font-mono text-xs text-muted-foreground hover:text-white transition-colors"
                  >
                    {tx.to.replace("wallet:", "").substring(0, 10)}...
                  </Link>
                </td>
                <td className="p-4 font-mono">
                  {tx.amount}{" "}
                  <span className="text-xs text-muted-foreground">
                    {getAssetSymbol(tx.asset)}
                  </span>
                </td>
                <td className="p-4 text-xs text-muted-foreground">
                  {new Date(tx.timestamp * 1000).toLocaleString()}
                </td>
              </tr>
            ))}
            {transactions.length === 0 && (
              <tr>
                <td
                  colSpan={6}
                  className="p-8 text-center text-muted-foreground"
                >
                  No recent transactions.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
