import { useState, useEffect } from "react";
import { useSearchParams } from "react-router-dom";
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
  view?: {
    type: string;
    assets: Record<string, string>;
    liabilities: Record<string, string>;
    equity: Record<string, string>;
  };
  // Extended for Consolidated View
  system_balances?: Record<string, number>; // Left Side (Assets/Contra)
  user_balances?: Record<string, number>; // Right Side (Liabilities)
  equity_balances?: Record<string, number>; // Equity (Capital + Retained Earnings)
}

interface AssetDefinition {
  issuer: string;
  // asset_type removed
  name: string;
  symbol: string;
  decimals: number;
  asset_standard?: string;
}

// Fixed Group Definitions (Heuristic Targets)
const GROUPS = {
  // EQUITY
  CAPITAL: {
    label: "3.1 Capital Social (Equity)",
    color: "text-blue-300",
    type: "equity" as const,
  },
  RESERVES: {
    label: "3.2 Reservas",
    color: "text-indigo-300",
    type: "equity" as const,
  },
  NET_WORTH: {
    label: "3.4 Patrimônio Líquido (Net Worth)",
    color: "text-purple-300",
    type: "equity" as const,
  },

  // LIABILITIES
  DEPOSITS: {
    label: "2.1.3 Obrigações com Clientes (Deposits)",
    color: "text-rose-400",
    type: "passive" as const,
  },
  PAYABLES: {
    label: "2.1.1 Fornecedores",
    color: "text-red-400",
    type: "passive" as const,
  },

  // ASSETS
  CASH: {
    label: "1.1.1 Caixa e Equivalentes",
    color: "text-green-400",
    type: "active" as const,
  },
  SYSTEM_RESERVE: {
    label: "System Reserves / Treasury",
    color: "text-teal-400",
    type: "active" as const,
  },
};

export default function Inspector() {
  const [searchParams] = useSearchParams();
  const initialAddress = searchParams.get("address") || "vault:issuance";
  const [address, setAddress] = useState(initialAddress);
  const [data, setData] = useState<AccountState | null>(null);
  const [registry, setRegistry] = useState<Record<string, AssetDefinition>>({});
  const [loading, setLoading] = useState(false);
  const [viewMode, setViewMode] = useState<"table" | "json">("table");
  const [isConsolidated, setIsConsolidated] = useState(false);
  const [error, setError] = useState("");

  const handleSearch = async (forceConsolidated?: boolean) => {
    const useConsolidated =
      forceConsolidated !== undefined ? forceConsolidated : isConsolidated;
    setLoading(true);
    setError("");
    setData(null);
    try {
      // 1. Fetch Registry for Metadata
      const tokensRes = await fetch(
        `${import.meta.env.VITE_NODE_URL}/api/tokens`,
      );
      if (tokensRes.ok) {
        setRegistry(await tokensRes.json());
      }

      if (useConsolidated) {
        // Consolidated Mode: Fetch ALL accounts
        const accountsRes = await fetch(
          `${import.meta.env.VITE_NODE_URL}/api/accounts`,
        );
        if (!accountsRes.ok) throw new Error("Failed to fetch accounts");
        const accounts: Record<
          string,
          { balances: Record<string, string | number> }
        > = await accountsRes.json();

        // Split Aggregation
        // systemMap = Assets (Left Side)
        // userMap = Liabilities (Right Side - what we owe to users)
        // equityMap = Equity (Right Side - accumulated value / capital)
        const systemMap: Record<string, number> = {};
        const userMap: Record<string, number> = {};
        const equityMap: Record<string, number> = {};

        Object.entries(accounts).forEach(([accAddr, accState]) => {
          Object.entries(accState.balances).forEach(([asset, amount]) => {
            const val = Number(amount);

            // Classification Logic for Consolidated View:
            //
            // INTERNAL/CONTROL ACCOUNTS (IGNORED):
            // - vault:unissued = Contra-account (cancels out)
            // - wallet:mint = Token issuer account (internal)
            //
            // REAL BALANCE SHEET:
            // - vault:mint:*, vault:issuance, vault:genesis = ASSETS (token reserves)
            // - vault:capital:*, vault:fees, vault:treasury = EQUITY (capital + revenue)
            // - User wallets (wallet:*) = LIABILITIES (what we owe to users)

            // Skip only truly internal accounts
            if (
              accAddr === "vault:unissued" ||
              accAddr.startsWith("wallet:mint")
            ) {
              return;
            }

            // EQUITY: capital accounts + fees + treasury
            if (
              accAddr.startsWith("vault:capital:") ||
              accAddr === "vault:fees" ||
              accAddr === "vault:treasury"
            ) {
              equityMap[asset] = (equityMap[asset] || 0) + val;
            }
            // ASSETS: mint pools + issuance + genesis
            else if (
              accAddr.startsWith("vault:mint:") ||
              accAddr === "vault:issuance" ||
              accAddr === "vault:genesis" ||
              accAddr.startsWith("vault:")
            ) {
              systemMap[asset] = (systemMap[asset] || 0) + val;
            }
            // LIABILITIES: user wallets
            else {
              userMap[asset] = (userMap[asset] || 0) + val;
            }
          });
        });

        const balancesStr: Record<string, string> = {};

        // Calculate Equity as Assets - Liabilities (ensures A = L + E always balances)
        const calculatedEquity: Record<string, number> = {};
        const allAssets = new Set([
          ...Object.keys(systemMap),
          ...Object.keys(userMap),
        ]);
        allAssets.forEach((asset) => {
          const assets = systemMap[asset] || 0;
          const liabilities = userMap[asset] || 0;
          calculatedEquity[asset] = assets - liabilities;
        });

        setData({
          address: "Global Protocol Balance Sheet",
          asset: "MULTI",
          balance: "0",
          balances: balancesStr,
          nonce: 0,
          system_balances: systemMap,
          user_balances: userMap,
          equity_balances: calculatedEquity,
        });
      } else {
        // Single Account Mode
        if (!address) return;
        const res = await fetch(
          `${import.meta.env.VITE_NODE_URL}/api/balance?query=${encodeURIComponent(address)}`,
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

  // Auto-search on mount if address is provided via query params
  useEffect(() => {
    // Check for consolidated param
    const consolidatedParam = searchParams.get("consolidated");
    const shouldConsolidate = consolidatedParam === "true";
    if (shouldConsolidate) {
      setIsConsolidated(true);
    }
    // Small delay to ensure UI is ready, then search with correct mode
    const timer = setTimeout(() => {
      handleSearch(shouldConsolidate);
    }, 100);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Only run on mount

  const groupedAssets = () => {
    if (!data || (!data.balances && !data.view))
      return { active: {}, passive: {}, equity: {} };

    const sections: Record<
      "active" | "passive" | "equity",
      Record<string, { group: any; items: any[] }>
    > = {
      active: {},
      passive: {},
      equity: {},
    };

    // 1. Backend-Driven Classification (Single Account View)
    if (data.view) {
      const processViewMap = (
        map: Record<string, string>,
        section: "active" | "passive" | "equity",
      ) => {
        Object.entries(map).forEach(([assetId, amtStr]) => {
          const amount = Number(amtStr);
          const def = registry[assetId];
          const symbol = def?.symbol || getAssetSymbol(assetId);

          // Allow group to be active, passive or equity
          let group: {
            label: string;
            color: string;
            type: "active" | "passive" | "equity";
          } = GROUPS.CASH;

          if (section === "active") {
            if (symbol === "ATLAS" && data.view?.type === "user") {
              group = {
                ...GROUPS.CAPITAL,
                label: "Atlas Token (Equity Share)",
                type: "active" as const,
              };
            } else if (symbol === "ATLAS") {
              group = GROUPS.SYSTEM_RESERVE;
            } else {
              group = GROUPS.CASH;
            }
          } else if (section === "passive") {
            group = GROUPS.DEPOSITS;
          } else if (section === "equity") {
            if (symbol === "ATLAS") {
              group = GROUPS.CAPITAL;
            } else {
              group = GROUPS.NET_WORTH;
            }
          }

          if (!sections[section][group.label]) {
            sections[section][group.label] = { group, items: [] };
          }
          sections[section][group.label].items.push({
            id: assetId,
            amount,
            def,
          });
        });
      };

      processViewMap(data.view.assets, "active");
      processViewMap(data.view.liabilities, "passive");
      processViewMap(data.view.equity, "equity");

      return sections;
    }

    // 2. Client-Side Heuristic (Consolidated View Fallback)
    const processItem = (
      assetId: string,
      amount: number,
      itemType: "system" | "user" | "equity",
    ) => {
      const def = registry[assetId];
      const symbol = def?.symbol || getAssetSymbol(assetId);

      // Heuristic Classification
      let effectiveGroup;

      if (itemType === "system") {
        // System Side (Assets)
        if (symbol === "ATLAS") {
          effectiveGroup = GROUPS.SYSTEM_RESERVE;
        } else {
          effectiveGroup = GROUPS.CASH;
        }
      } else if (itemType === "user") {
        // User Side (Liabilities)
        effectiveGroup = GROUPS.DEPOSITS;
      } else {
        // Equity
        if (symbol === "ATLAS") {
          effectiveGroup = GROUPS.CAPITAL;
        } else {
          effectiveGroup = GROUPS.NET_WORTH;
        }
      }

      const type = effectiveGroup.type;
      const groupKey = effectiveGroup.label;

      if (!sections[type][groupKey]) {
        sections[type][groupKey] = { group: effectiveGroup, items: [] };
      }
      sections[type][groupKey].items.push({ id: assetId, amount, def });
    };

    if (isConsolidated && data.system_balances && data.user_balances) {
      Object.entries(data.system_balances).forEach(([id, amt]) =>
        processItem(id, amt, "system"),
      );
      Object.entries(data.user_balances).forEach(([id, amt]) =>
        processItem(id, amt, "user"),
      );
      // Process equity_balances if available
      if (data.equity_balances) {
        Object.entries(data.equity_balances).forEach(([id, amt]) =>
          processItem(id, amt, "equity"),
        );
      }
    } else {
      Object.entries(data.balances).forEach(([id, amtStr]) => {
        // Fallback for failed view fetch
        const isSystemAccount =
          address.startsWith("vault:") || address.startsWith("wallet:mint");
        processItem(id, Number(amtStr), isSystemAccount ? "system" : "user");
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
              placeholder="Enter Account Address (e.g., vault:issuance)"
              disabled={isConsolidated}
              className={`w-full bg-background border border-border rounded-lg pl-10 pr-4 py-2 focus:ring-2 focus:ring-blue-500 outline-none font-mono transition-opacity ${
                isConsolidated ? "opacity-50" : ""
              }`}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            />
          </div>
          <button
            onClick={() => handleSearch()}
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
              (Sums all 'vault:*' and 'wallet:mint*' accounts)
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
