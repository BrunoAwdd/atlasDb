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
}

interface ListResponse {
  transactions: TxDto[];
  total_count: number;
}

function App() {
  const [data, setData] = useState<ListResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");

  useEffect(() => {
    const fetchData = async () => {
      try {
        const url = query
          ? `http://localhost:3001/api/transactions?limit=20&query=${encodeURIComponent(
              query
            )}`
          : "http://localhost:3001/api/transactions?limit=20";

        const res = await fetch(url);
        const json = await res.json();
        setData(json);
      } catch (e) {
        console.error("Failed to fetch", e);
      } finally {
        setLoading(false);
      }
    };

    // Debounce retrieval or poll if no query
    const t = setTimeout(fetchData, 500);

    // Only set interval if there's no query (live mode), otherwise just search once per change
    let interval: number | undefined;
    if (!query) {
      interval = setInterval(fetchData, 5000);
    }

    return () => {
      clearTimeout(t);
      clearInterval(interval);
    };
  }, [query]);

  return (
    <div className="container">
      <header>
        <h1>1961 Explorer</h1>
        <p>Live Transaction Feed</p>
      </header>

      <div className="search-container">
        <input
          type="text"
          placeholder="Search by Wallet Address or Hash..."
          className="search-input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      <div className="card">
        {loading && !data ? (
          <p>Loading transactions...</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Hash</th>
                <th>From</th>
                <th>To</th>
                <th>Amount</th>
                <th>Asset</th>
                <th>Time</th>
              </tr>
            </thead>
            <tbody>
              {data?.transactions.map((tx, index) => (
                <tr key={`${tx.tx_hash}-${index}`}>
                  <td>
                    <span className="hash" title={tx.tx_hash}>
                      {tx.tx_hash}
                    </span>
                  </td>
                  <td>
                    <span className="hash" title={tx.from}>
                      {tx.from.replace("passivo:wallet:", "")}
                    </span>
                  </td>
                  <td>
                    <span className="hash" title={tx.to}>
                      {tx.to.replace("passivo:wallet:", "")}
                    </span>
                  </td>
                  <td className="amount">{tx.amount}</td>
                  <td>
                    <span className="badge">{tx.asset}</span>
                  </td>
                  <td>{new Date(tx.timestamp).toLocaleString()}</td>
                </tr>
              ))}
              {data?.transactions.length === 0 && (
                <tr>
                  <td colSpan={6} style={{ textAlign: "center" }}>
                    No transactions found.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

export default App;
