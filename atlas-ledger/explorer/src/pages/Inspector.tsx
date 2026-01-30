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
  // Extended for Consolidated View
  system_balances?: Record<string, number>; // Left Side (Assets/Contra)
  user_balances?: Record<string, number>; // Right Side (Liabilities/Equity)
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
  const [isConsolidated, setIsConsolidated] = useState(false);
  const [error, setError] = useState("");

  const handleSearch = async () => {
    setLoading(true);
    setError("");
    setData(null);
    try {
      // 1. Fetch Registry for Metadata
      const tokensRes = await fetch("http://localhost:3001/api/tokens");
      if (tokensRes.ok) {
        setRegistry(await tokensRes.json());
      }

      if (isConsolidated) {
        // Consolidated Mode: Fetch ALL accounts and sum `patrimonio:*`
        const accountsRes = await fetch("http://localhost:3001/api/accounts");
        if (!accountsRes.ok) throw new Error("Failed to fetch accounts");
        const accounts: Record<
          string,
          { balances: Record<string, string | number> }
        > = await accountsRes.json();

        // Split Aggregation
        const systemMap: Record<string, number> = {};
        const userMap: Record<string, number> = {};

        Object.entries(accounts).forEach(([accAddr, accState]) => {
          Object.entries(accState.balances).forEach(([asset, amount]) => {
            const val = Number(amount);

            // Logic:
            // Patrimonio (Issuance/Treasury) -> System Side (Left/Assets)
            // Wallets -> User Side (Right/Liabilities)
            if (accAddr.startsWith("patrimonio:")) {
              systemMap[asset] = (systemMap[asset] || 0) + val;
            } else {
              userMap[asset] = (userMap[asset] || 0) + val;
            }
          });
        });

        // Create dummy balances map for compatibility
        const balancesStr: Record<string, string> = {};

        setData({
          address: "Global Protocol Balance Sheet",
          asset: "MULTI",
          balance: "0",
          balances: balancesStr,
          nonce: 0,
          system_balances: systemMap,
          user_balances: userMap,
        });
      } else {
        // Single Account Mode
        if (!address) return;
        const res = await fetch(
          `http://localhost:3001/api/balance?query=${encodeURIComponent(address)}`,
        );
        if (!res.ok) throw new Error("Failed to fetch data");
        const json = await res.json();
        setData(json);
      }
    } catch (e) {
      console.error(e);
      setError("Could not fetch account data. Is the node running?");
    } finally {
      setLoading(false);
    }
  };

  const groupedAssets = () => {
    if (!data || !data.balances) return { active: {}, passive: {}, equity: {} };

    const sections: Record<
      "active" | "passive" | "equity",
      Record<string, { group: any; items: any[] }>
    > = {
      active: {},
      passive: {},
      equity: {},
    };

    const processItem = (
      assetId: string,
      amount: number,
      sourceMap: Record<string, number>,
      isSystem: boolean,
    ) => {
      const def = registry[assetId];
      let coaCode = def?.asset_type;

      // Fallback/Logic
      if (isConsolidated) {
        if (isSystem) {
          // System holds = Active
          coaCode = coaCode || "A1_1_1";
        } else {
          // User holds = Passive (usually)
          coaCode = coaCode || "L2_1_3";
        }
      } else {
        coaCode = coaCode || "A1_1_1";
      }

      const originalGroup = COA_GROUPS[coaCode];
      let type: "active" | "passive" | "equity" = "active";
      let effectiveGroup = originalGroup;

      if (isConsolidated) {
        if (isSystem) {
          type = "active";
          effectiveGroup = {
            label: originalGroup
              ? `(System Hold) ${originalGroup.label}`
              : "System Reserve",
            color: "text-blue-400",
            type: "active",
          };
        } else {
          // User Side
          if (originalGroup?.type === "equity") {
            type = "equity";
            effectiveGroup = originalGroup;
          } else {
            type = "passive";
            effectiveGroup = originalGroup
              ? { ...originalGroup, type: "passive" }
              : {
                  label: "User Holdings",
                  color: "text-red-400",
                  type: "passive",
                };
          }
        }
      } else {
        // Single Account View
        type = originalGroup?.type || "active";
        effectiveGroup = originalGroup || {
          label: "Unknown",
          color: "text-gray-400",
          type: "active",
        };
      }

      const groupKey = effectiveGroup.label;
      if (!sections[type][groupKey]) {
        sections[type][groupKey] = { group: effectiveGroup, items: [] };
      }
      sections[type][groupKey].items.push({ id: assetId, amount, def });
    };

    if (isConsolidated && data.system_balances && data.user_balances) {
      Object.entries(data.system_balances).forEach(([id, amt]) =>
        processItem(id, amt, data.system_balances!, true),
      );
      Object.entries(data.user_balances).forEach(([id, amt]) =>
        processItem(id, amt, data.user_balances!, false),
      );
    } else {
      Object.entries(data.balances).forEach(([id, amtStr]) => {
        processItem(id, Number(amtStr), {}, false); // isSystem arg ignored for single view logic mainly
      });
    }

    return sections;
  };

  const { active, passive, equity } = groupedAssets();

  const renderSection = (
    title: string,
    icon: React.ReactNode,
    groups: Record<string, { group: any; items: any[] }>,
    titleColor: string,
  ) => {
    const groupList = Object.values(groups);
    return (
      <div>
        <div
          className={`flex items-center gap-2 border-b-2 ${titleColor.replace(
            "text-",
            "border-",
          )}/50 pb-2 mb-6`}
        >
          {icon}
          <h3
            className={`text-lg font-bold uppercase tracking-wider ${titleColor}`}
          >
            {title}
          </h3>
        </div>

        <div className="space-y-8">
          {groupList.length === 0 && (
            <p className="text-muted-foreground italic text-sm">No accounts.</p>
          )}
          {groupList
            .sort((a, b) => a.group.label.localeCompare(b.group.label))
            .map((g) => (
              <div key={g.group.label}>
                <h4
                  className={`text-xs font-bold uppercase ${g.group.color} mb-3 border-b border-white/5 pb-1`}
                >
                  {g.group.label}
                </h4>
                <div className="space-y-4 pl-2">
                  {g.items.map((item: any) => (
                    <div key={item.id} className="relative group/item">
                      <div className="flex justify-between items-end mb-1">
                        <div>
                          <span className="font-mono text-white text-lg">
                            {item.def ? item.def.name : getAssetSymbol(item.id)}
                          </span>
                        </div>
                        <span className="font-mono text-xl font-bold">
                          {Math.floor(item.amount).toLocaleString()}
                        </span>
                      </div>
                      <div className="w-full h-px bg-white/10 group-hover/item:bg-white/20 transition-colors"></div>
                      <div className="text-[10px] font-mono text-muted-foreground mt-1">
                        {item.id}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
        </div>
      </div>
    );
  };

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
        <div className="flex gap-4 mb-4">
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
              disabled={isConsolidated}
              className={`w-full bg-background border border-border rounded-lg pl-10 pr-4 py-2 focus:ring-2 focus:ring-blue-500 outline-none font-mono transition-opacity ${
                isConsolidated ? "opacity-50" : ""
              }`}
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

        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="consolidate"
            checked={isConsolidated}
            onChange={(e) => setIsConsolidated(e.target.checked)}
            className="w-4 h-4 rounded border-gray-600 bg-gray-700 text-blue-600 focus:ring-blue-500"
          />
          <label
            htmlFor="consolidate"
            className="text-sm text-gray-300 select-none cursor-pointer"
          >
            Consolidated Protocol View{" "}
            <span className="text-xs text-muted-foreground">
              (Sums all 'patrimonio:*' accounts)
            </span>
          </label>
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
                {renderSection(
                  "Ativo (Assets)",
                  <Landmark className="text-green-400" />,
                  active,
                  "text-green-400",
                )}

                {/* Right Side: LIABILITIES & EQUITY */}
                <div className="space-y-8">
                  {renderSection(
                    "Passivo (Liabilities)",
                    <Wallet className="text-red-400" />,
                    passive,
                    "text-red-400",
                  )}

                  <div className="pt-8">
                    {renderSection(
                      "Patrimônio Líquido (Equity)",
                      <Building2 className="text-blue-400" />,
                      equity,
                      "text-blue-400",
                    )}
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
