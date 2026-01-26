import { useEffect, useState } from "react";
import "./App.css";

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

interface ListResponse {
  transactions: TxDto[];
  total_count: number;
}

function App() {
  interface BalanceDto {
    address: string;
    asset: string;
    balance: string;
  }
  interface AccountState {
    balances: Record<string, number>;
    nonce: number;
  }

  const [balance, setBalance] = useState<BalanceDto | null>(null);
  const [data, setData] = useState<ListResponse | null>(null);
  const [accounts, setAccounts] = useState<Record<string, AccountState> | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [view, setView] = useState<"transactions" | "accounts">("transactions");

  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        if (view === "transactions") {
          // Fetch Transactions
          const url = query
            ? `http://localhost:3001/api/transactions?limit=20&query=${encodeURIComponent(
                query,
              )}`
            : "http://localhost:3001/api/transactions?limit=20";

          const res = await fetch(url);
          const json = await res.json();
          setData(json);

          // Fetch Balance (only if query looks like an address)
          if (
            query &&
            (query.startsWith("nbex") || query.startsWith("passivo:"))
          ) {
            const balUrl = `http://localhost:3001/api/balance?query=${encodeURIComponent(query)}`;
            const balRes = await fetch(balUrl);
            const balJson = await balRes.json();
            setBalance(balJson);
          } else {
            setBalance(null);
          }
        } else {
          // Fetch Accounts
          const res = await fetch("http://localhost:3001/api/accounts");
          const json = await res.json();
          setAccounts(json);
          setBalance(null);
        }
      } catch (e) {
        console.error("Failed to fetch", e);
      } finally {
        setLoading(false);
      }
    };

    // Debounce retrieval or poll if no query
    const t = setTimeout(fetchData, 500);

    let interval: number | undefined;
    if (!query && view === "transactions") {
      interval = setInterval(fetchData, 5000);
    }

    // Poll accounts less frequency
    if (view === "accounts") {
      interval = setInterval(fetchData, 10000);
    }

    return () => {
      clearTimeout(t);
      clearInterval(interval);
    };
  }, [query, view]);

  return (
    <div className="container">
      <header>
        <h1>1961 Explorer</h1>
        <div className="tabs">
          <button
            className={view === "transactions" ? "active" : ""}
            onClick={() => setView("transactions")}
          >
            Transactions
          </button>
          <button
            className={view === "accounts" ? "active" : ""}
            onClick={() => setView("accounts")}
          >
            Accounts (Rich List)
          </button>
        </div>
      </header>

      {view === "transactions" && (
        <div className="search-container">
          <input
            type="text"
            placeholder="Search by Wallet Address or Hash..."
            className="search-input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
      )}

      {balance && view === "transactions" && (
        <div className="card balance-card">
          <h2>Wallet Balance</h2>
          <div className="balance-info">
            <span className="balance-label">Address:</span>{" "}
            <span className="hash">{balance.address}</span>
          </div>
          <div className="balance-info">
            <span className="balance-label">Balance:</span>{" "}
            <span className="amount large">{balance.balance}</span>{" "}
            <span className="badge">{balance.asset}</span>
          </div>
        </div>
      )}

      <div className="card">
        {loading && !data && !accounts ? (
          <p>Loading...</p>
        ) : (
          <>
            {view === "transactions" && (
              <table>
                <thead>
                  <tr>
                    <th>Hash</th>
                    <th>From</th>
                    <th>To</th>
                    <th>Amount</th>
                    <th>Asset</th>
                    <th>Payer</th>
                    <th>Time</th>
                  </tr>
                </thead>
                <tbody>
                  {data?.transactions.map((tx, index) => (
                    <tr key={`${tx.tx_hash}-${index}`}>
                      <td>
                        <span className="hash" title={tx.tx_hash}>
                          {tx.tx_hash.substring(0, 16)}...
                        </span>
                      </td>
                      <td>
                        <span className="hash" title={tx.from}>
                          {tx.from
                            .replace("passivo:wallet:", "")
                            .substring(0, 12)}
                          ...
                        </span>
                      </td>
                      <td>
                        <span className="hash" title={tx.to}>
                          {tx.to
                            .replace("passivo:wallet:", "")
                            .substring(0, 12)}
                          ...
                        </span>
                      </td>
                      <td className="amount">{tx.amount}</td>
                      <td>
                        <span className="badge">{tx.asset}</span>
                      </td>
                      <td>
                        {tx.fee_payer && (
                          <span
                            className="hash"
                            title={`Paid by ${tx.fee_payer}`}
                          >
                            ⚡{" "}
                            {tx.fee_payer
                              .replace("passivo:wallet:", "")
                              .slice(0, 8)}
                            ...
                          </span>
                        )}
                      </td>
                      <td>{new Date(tx.timestamp * 1000).toLocaleString()}</td>
                    </tr>
                  ))}
                  {data?.transactions.length === 0 && (
                    <tr>
                      <td colSpan={7} style={{ textAlign: "center" }}>
                        No transactions found.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            )}

            {view === "accounts" && (
              <table>
                <thead>
                  <tr>
                    <th>Address</th>
                    <th>Balances</th>
                    <th>Nonce</th>
                  </tr>
                </thead>
                <tbody>
                  {accounts &&
                    Object.entries(accounts).map(([addr, state]) => (
                      <tr key={addr}>
                        <td>
                          <span className="hash">
                            {addr.replace("passivo:wallet:", "")}
                          </span>
                        </td>
                        <td>
                          <div className="flex gap-2">
                            {Object.entries(state.balances).map(
                              ([asset, amount]) => (
                                <span key={asset} className="badge">
                                  {amount} {asset}
                                </span>
                              ),
                            )}
                          </div>
                        </td>
                        <td>{state.nonce}</td>
                      </tr>
                    ))}
                  {(!accounts || Object.keys(accounts).length === 0) && (
                    <tr>
                      <td colSpan={3} style={{ textAlign: "center" }}>
                        No active accounts found.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default App;
