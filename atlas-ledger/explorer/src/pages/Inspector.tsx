import { useState } from "react";
import { Search, FileJson, Table as TableIcon, Copy, Code } from "lucide-react";
import { getAssetSymbol } from "../lib/assets";

interface AccountState {
  address: string;
  asset: string;
  balance: string;
  balances: Record<string, string>;
  nonce: number;
}

export default function Inspector() {
  const [address, setAddress] = useState("patrimonio:issuance");
  const [data, setData] = useState<AccountState | null>(null);
  const [loading, setLoading] = useState(false);
  const [viewMode, setViewMode] = useState<"table" | "json">("json");
  const [error, setError] = useState("");

  const handleSearch = async () => {
    if (!address) return;
    setLoading(true);
    setError("");
    setData(null);
    try {
      // Use the raw balance endpoint which returns the full portfolio in `balances`
      const res = await fetch(
        `http://localhost:3001/api/balance?query=${encodeURIComponent(address)}`,
      );
      if (!res.ok) throw new Error("Failed to fetch data");
      const json = await res.json();

      // If the backend returns empty/default, it might be 0.
      setData(json);
    } catch (e) {
      console.error(e);
      setError("Could not fetch account data. Is the node running?");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="max-w-6xl mx-auto space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2 flex items-center gap-2">
          <Code className="text-blue-400" /> Ledger Inspector
        </h1>
        <p className="text-muted-foreground">
          Low-level state inspection and balance sheet verification.
        </p>
      </div>

      {/* Search Bar */}
      <div className="card p-6 bg-secondary/10 border border-border/50 rounded-xl">
        <div className="flex gap-4">
          <div className="flex-1 relative">
            <Search
              className="absolute left-3 top-3 text-muted-foreground"
              size={20}
            />
            <input
              type="text"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="Enter Account Address (e.g., patrimonio:issuance)"
              className="w-full bg-background border border-border rounded-lg pl-10 pr-4 py-2 focus:ring-2 focus:ring-blue-500 outline-none font-mono"
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            />
          </div>
          <button
            onClick={handleSearch}
            disabled={loading}
            className="btn btn-primary px-6 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 font-semibold disabled:opacity-50"
          >
            {loading ? "Loading..." : "Inspect"}
          </button>
        </div>
        {error && <p className="text-red-400 mt-2 text-sm">{error}</p>}
      </div>

      {/* Results */}
      {data && (
        <div className="space-y-4">
          {/* Controls */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="font-mono text-lg font-bold text-white">
                {data.address}
              </span>
              <button
                onClick={() => navigator.clipboard.writeText(data.address)}
                className="p-1 hover:bg-white/10 rounded"
              >
                <Copy size={16} className="text-muted-foreground" />
              </button>
            </div>

            <div className="flex bg-secondary/30 rounded-lg p-1">
              <button
                onClick={() => setViewMode("table")}
                className={`px-4 py-1.5 rounded-md text-sm font-medium transition-colors flex items-center gap-2 ${
                  viewMode === "table"
                    ? "bg-blue-600 text-white shadow-sm"
                    : "text-muted-foreground hover:text-white"
                }`}
              >
                <TableIcon size={16} /> Balance Sheet
              </button>
              <button
                onClick={() => setViewMode("json")}
                className={`px-4 py-1.5 rounded-md text-sm font-medium transition-colors flex items-center gap-2 ${
                  viewMode === "json"
                    ? "bg-blue-600 text-white shadow-sm"
                    : "text-muted-foreground hover:text-white"
                }`}
              >
                <FileJson size={16} /> Raw JSON
              </button>
            </div>
          </div>

          {/* Views */}
          <div className="card border border-border/50 bg-[#0d1117] rounded-xl overflow-hidden shadow-2xl">
            {viewMode === "json" ? (
              <pre className="p-6 overflow-x-auto text-sm font-mono text-green-400 bg-[#0d1117] m-0">
                {JSON.stringify(data, null, 2)}
              </pre>
            ) : (
              <div className="p-6">
                <div className="grid grid-cols-2 gap-8">
                  {/* Assets Side */}
                  <div>
                    <h3 className="text-sm uppercase font-bold text-muted-foreground mb-4 pb-2 border-b border-white/10">
                      Assets / Credits
                    </h3>
                    <div className="space-y-3">
                      {Object.keys(data.balances || {}).length === 0 ? (
                        <div className="text-muted-foreground italic">
                          No assets found
                        </div>
                      ) : (
                        Object.entries(data.balances || {}).map(
                          ([assetId, amount]) => (
                            <div
                              key={assetId}
                              className="flex justify-between items-center p-3 bg-white/5 rounded border border-white/5"
                            >
                              <div className="flex flex-col">
                                <span className="font-bold text-blue-300">
                                  {getAssetSymbol(assetId)}
                                </span>
                                <span className="text-[10px] text-muted-foreground font-mono">
                                  {assetId}
                                </span>
                              </div>
                              <span className="font-mono text-xl">
                                {parseInt(amount).toLocaleString()}
                              </span>
                            </div>
                          ),
                        )
                      )}
                    </div>
                  </div>

                  {/* Metadata Side (Simulating Liabilities/Equity or just Info) */}
                  <div>
                    <h3 className="text-sm uppercase font-bold text-muted-foreground mb-4 pb-2 border-b border-white/10">
                      Account Metadata
                    </h3>
                    <div className="space-y-3">
                      <div className="flex justify-between items-center p-3 bg-white/5 rounded border border-white/5">
                        <span className="text-muted-foreground">Nonce</span>
                        <span className="font-mono text-xl">{data.nonce}</span>
                      </div>
                      <div className="flex justify-between items-center p-3 bg-white/5 rounded border border-white/5">
                        <span className="text-muted-foreground">Status</span>
                        <span className="badge bg-green-500/20 text-green-400">
                          Active
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
