import { useEffect, useState } from "react";
import { Users, Wallet } from "lucide-react";
import { Link } from "react-router-dom";

interface AccountState {
  balances: Record<string, number>;
  nonce: number;
}

interface AccountRow {
  address: string;
  balances: Record<string, number>;
  nonce: number;
}

export default function Accounts() {
  const [accounts, setAccounts] = useState<AccountRow[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("http://localhost:3001/api/accounts")
      .then((res) => res.json())
      .then((data: Record<string, AccountState>) => {
        // Convert Map { address: state } to Array [{ address, ...state }]
        const list = Object.entries(data).map(([address, state]) => ({
          address,
          ...state,
        }));

        // Sort by balance (heuristic: sorting by ATLAS balance if present, or just total)
        // For simplicity, let's sort by nonce or just alphabetical for now,
        // essentially we'd want a "Rich List" so sorting by main asset balance is better.
        // Assuming 'ATLAS' is the main asset.
        list.sort((a, b) => {
          const balA = a.balances["wallet:mint/ATLAS"] || 0;
          const balB = b.balances["wallet:mint/ATLAS"] || 0;
          return balB - balA;
        });

        setAccounts(list);
        setLoading(false);
      })
      .catch((err) => {
        console.error("Failed to fetch accounts:", err);
        setLoading(false);
      });
  }, []);

  if (loading)
    return (
      <div className="p-8 text-center animate-pulse">Loading Accounts...</div>
    );

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold flex items-center space-x-2">
          <Users className="text-purple-500" />
          <span>Top Accounts</span>
        </h2>
        <span className="text-sm text-muted-foreground">
          Total Accounts: {accounts.length}
        </span>
      </div>

      <div className="card overflow-hidden p-0">
        <table className="w-full">
          <thead>
            <tr className="bg-secondary/30 text-left">
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Rank
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Address
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Primary Balance (ATLAS)
              </th>
              <th className="p-4 text-sm font-semibold text-muted-foreground">
                Nonce
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border/30">
            {accounts.map((acc, index) => (
              <tr
                key={acc.address}
                className="hover:bg-secondary/10 transition-colors"
              >
                <td className="p-4 text-muted-foreground font-mono w-16">
                  {index + 1}
                </td>
                <td className="p-4">
                  <div className="flex items-center space-x-2">
                    <div className="p-2 bg-blue-500/10 rounded-lg text-blue-400">
                      <Wallet size={14} />
                    </div>
                    <Link
                      to={`/address/${acc.address}`}
                      className="font-mono text-blue-400 hover:text-blue-300 transition-colors"
                    >
                      {acc.address}
                    </Link>
                  </div>
                </td>
                <td className="p-4 font-mono font-bold text-green-400">
                  {acc.balances["wallet:mint/ATLAS"] || 0}
                </td>
                <td className="p-4 font-mono text-muted-foreground">
                  {acc.nonce}
                </td>
              </tr>
            ))}
            {accounts.length === 0 && (
              <tr>
                <td
                  colSpan={4}
                  className="p-8 text-center text-muted-foreground"
                >
                  No active accounts found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
