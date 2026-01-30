import { useState } from "react";
import {
  Search,
  FileJson,
  Table as TableIcon,
  Copy,
  Code,
  Landmark,
  Building2,
  Wallet,
} from "lucide-react";
import { getAssetSymbol } from "../lib/assets";

interface AccountState {
  address: string;
  asset: string;
  balance: string;
  balances: Record<string, string>;
  nonce: number;
}

interface AssetDefinition {
  issuer: string;
  asset_type: string; // "A1_1_1", "L2_1_3", etc.
  name: string;
  symbol: string;
  decimals: number;
  asset_standard?: string;
}

// COA Groupings for Demo
const COA_GROUPS: Record<
  string,
  { label: string; color: string; type: "active" | "passive" | "equity" }
> = {
  // ATIVO
  A1_1_1: {
    label: "1.1.1 Caixa e Equivalentes",
    color: "text-green-400",
    type: "active",
  },
  A1_1_2: {
    label: "1.1.2 Contas a Receber",
    color: "text-emerald-400",
    type: "active",
  },
  A1_2_3: {
    label: "1.2.3 Imobilizado (Real Assets)",
    color: "text-teal-400",
    type: "active",
  },

  // PASSIVO
  L2_1_1: {
    label: "2.1.1 Fornecedores",
    color: "text-red-400",
    type: "passive",
  },
  L2_1_3: {
    label: "2.1.3 Obrigações com Clientes (Deposits)",
    color: "text-rose-400",
    type: "passive",
  },

  // PATRIMÔNIO
  EQ3_1: {
    label: "3.1 Capital Social",
    color: "text-blue-300",
    type: "equity",
  },
  EQ3_2: { label: "3.2 Reservas", color: "text-indigo-300", type: "equity" },
  EQ3_3: { label: "3.3 Ajustes", color: "text-violet-300", type: "equity" },
};

export default function Inspector() {
  const [address, setAddress] = useState("patrimonio:issuance");
  const [data, setData] = useState<AccountState | null>(null);
  const [registry, setRegistry] = useState<Record<string, AssetDefinition>>({});
  const [loading, setLoading] = useState(false);
  const [viewMode, setViewMode] = useState<"table" | "json">("table");
  const [error, setError] = useState("");

  const handleSearch = async () => {
    if (!address) return;
    setLoading(true);
    setError("");
    setData(null);
    try {
      // 1. Fetch Registry for Metadata
      const tokensRes = await fetch("http://localhost:3001/api/tokens");
      if (tokensRes.ok) {
        setRegistry(await tokensRes.json());
      }

      // 2. Fetch Balances
      const res = await fetch(
        `http://localhost:3001/api/balance?query=${encodeURIComponent(address)}`,
      );
      if (!res.ok) throw new Error("Failed to fetch data");
      const json = await res.json();
      setData(json);
    } catch (e) {
      console.error(e);
      setError("Could not fetch account data. Is the node running?");
    } finally {
      setLoading(false);
    }
  };

  const groupedAssets = () => {
    if (!data || !data.balances) return { active: [], passive: [], equity: [] };

    const groups: Record<"active" | "passive" | "equity", any[]> = {
      active: [],
      passive: [],
      equity: [],
    };

    Object.entries(data.balances).forEach(([assetId, amount]) => {
      const def = registry[assetId];
      // Default to "Other" if not found in registry (should not happen if synced)
      const coaCode = def?.asset_type || "A1_1_1";
      const groupInfo = COA_GROUPS[coaCode] || {
        label: "Unknown",
        color: "text-gray-400",
        type: "active",
      };

      groups[groupInfo.type].push({
        id: assetId,
        amount,
        def,
        group: groupInfo,
      });
    });

    return groups;
  };

  const { active, passive, equity } = groupedAssets();

  return (
    <div className="max-w-6xl mx-auto space-y-8 pb-12">
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
            <div className="flex items-center gap-3">
              <span className="font-mono text-lg font-bold text-white bg-secondary/30 px-3 py-1 rounded border border-white/10">
                {data.address}
              </span>
              <button
                onClick={() => navigator.clipboard.writeText(data.address)}
                className="p-1.5 hover:bg-white/10 rounded transition-colors"
                title="Copy Address"
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
          <div className="card border border-border/50 bg-[#0d1117] rounded-xl overflow-hidden shadow-2xl min-h-[500px]">
            {viewMode === "json" ? (
              <pre className="p-6 overflow-x-auto text-sm font-mono text-green-400 bg-[#0d1117] m-0">
                {JSON.stringify(data, null, 2)}
              </pre>
            ) : (
              <div className="p-8 grid grid-cols-2 gap-12">
                {/* Left Side: ASSETS */}
                <div>
                  <div className="flex items-center gap-2 border-b-2 border-green-500/50 pb-2 mb-6">
                    <Landmark className="text-green-400" />
                    <h3 className="text-lg font-bold uppercase tracking-wider text-green-400">
                      Ativo (Assets)
                    </h3>
                  </div>

                  <div className="space-y-6">
                    {active.length === 0 && (
                      <p className="text-muted-foreground italic text-sm">
                        No active assets.
                      </p>
                    )}
                    {active.map((item) => (
                      <div key={item.id} className="relative group">
                        <div className="flex justify-between items-end mb-1">
                          <div>
                            <span
                              className={`text-xs font-bold uppercase ${item.group.color} mb-0.5 block`}
                            >
                              {item.group.label}
                            </span>
                            <span className="font-mono text-white text-lg">
                              {item.def
                                ? item.def.name
                                : getAssetSymbol(item.id)}
                            </span>
                          </div>
                          <span className="font-mono text-xl font-bold">
                            {parseInt(item.amount).toLocaleString()}
                          </span>
                        </div>
                        <div className="w-full h-px bg-white/10 group-hover:bg-white/20 transition-colors"></div>
                        <div className="text-[10px] font-mono text-muted-foreground mt-1">
                          {item.id}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Right Side: LIABILITIES & EQUITY */}
                <div className="space-y-8">
                  {/* Liabilities */}
                  <div>
                    <div className="flex items-center gap-2 border-b-2 border-red-500/50 pb-2 mb-6">
                      <Wallet className="text-red-400" />
                      <h3 className="text-lg font-bold uppercase tracking-wider text-red-400">
                        Passivo (Liabilities)
                      </h3>
                    </div>
                    <div className="space-y-6">
                      {passive.length === 0 && (
                        <p className="text-muted-foreground italic text-sm">
                          No liabilities.
                        </p>
                      )}
                      {passive.map((item) => (
                        <div key={item.id} className="relative group">
                          <div className="flex justify-between items-end mb-1">
                            <div>
                              <span
                                className={`text-xs font-bold uppercase ${item.group.color} mb-0.5 block`}
                              >
                                {item.group.label}
                              </span>
                              <span className="font-mono text-white text-lg">
                                {item.def
                                  ? item.def.name
                                  : getAssetSymbol(item.id)}
                              </span>
                            </div>
                            <span className="font-mono text-xl font-bold">
                              {parseInt(item.amount).toLocaleString()}
                            </span>
                          </div>
                          <div className="w-full h-px bg-white/10 group-hover:bg-white/20 transition-colors"></div>
                          <div className="text-[10px] font-mono text-muted-foreground mt-1">
                            {item.id}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  {/* Equity */}
                  <div className="pt-8">
                    <div className="flex items-center gap-2 border-b-2 border-blue-500/50 pb-2 mb-6">
                      <Building2 className="text-blue-400" />
                      <h3 className="text-lg font-bold uppercase tracking-wider text-blue-400">
                        Patrimônio Líquido (Equity)
                      </h3>
                    </div>
                    <div className="space-y-6">
                      {equity.length === 0 && (
                        <p className="text-muted-foreground italic text-sm">
                          No equity accounts.
                        </p>
                      )}
                      {equity.map((item) => (
                        <div key={item.id} className="relative group">
                          <div className="flex justify-between items-end mb-1">
                            <div>
                              <span
                                className={`text-xs font-bold uppercase ${item.group.color} mb-0.5 block`}
                              >
                                {item.group.label}
                              </span>
                              <span className="font-mono text-white text-lg">
                                {item.def
                                  ? item.def.name
                                  : getAssetSymbol(item.id)}
                              </span>
                            </div>
                            <span className="font-mono text-xl font-bold">
                              {parseInt(item.amount).toLocaleString()}
                            </span>
                          </div>
                          <div className="w-full h-px bg-white/10 group-hover:bg-white/20 transition-colors"></div>
                          <div className="text-[10px] font-mono text-muted-foreground mt-1">
                            {item.id}
                          </div>
                        </div>
                      ))}
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
