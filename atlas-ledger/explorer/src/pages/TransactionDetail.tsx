import { useParams } from "react-router-dom";
import { useEffect, useState } from "react";
import { FileText, CheckCircle, Clock } from "lucide-react";
import { getAssetSymbol } from "../lib/assets";

interface TxDto {
  tx_hash: string;
  from: string;
  to: string;
  amount: string;
  asset: string;
  timestamp: number;
  memo: string;
  fee_payer?: string;
  block_height?: number; // assuming backend provides this eventually
}

export default function TransactionDetail() {
  const { hash } = useParams();
  const [tx, setTx] = useState<TxDto | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Fetch Tx. Currently we might have to filter from the list API if there isn't a dedicated one?
    // Or assuming /api/transactions?query=HASH works (it currently does for search!)
    if (!hash) return;

    const fetchTx = async () => {
      try {
        // Query by hash
        const res = await fetch(
          `${import.meta.env.VITE_NODE_URL}/api/transactions?query=${encodeURIComponent(hash)}`,
        );
        const json = await res.json();
        if (json.transactions && json.transactions.length > 0) {
          setTx(json.transactions[0]);
        }
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    };
    fetchTx();
  }, [hash]);

  if (loading)
    return (
      <div className="p-8 text-center animate-pulse">
        Loading Transaction...
      </div>
    );
  if (!tx)
    return (
      <div className="p-8 text-center text-red-500">Transaction Not Found</div>
    );

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <h2 className="text-2xl font-bold flex items-center space-x-2">
        <FileText className="text-blue-500" />
        <span>Transaction Details</span>
      </h2>

      <div className="card p-0 overflow-hidden divide-y divide-border/50">
        <div className="p-4 flex flex-col md:flex-row gap-4">
          <span className="w-48 font-semibold text-muted-foreground flex items-center gap-2">
            <CheckCircle size={16} className="text-green-500" /> Status
          </span>
          <span className="badge bg-green-500/20 text-green-400">Success</span>
        </div>

        <div className="p-4 flex flex-col md:flex-row gap-4">
          <span className="w-48 font-semibold text-muted-foreground">
            Transaction Hash
          </span>
          <span className="font-mono text-sm break-all">{tx.tx_hash}</span>
        </div>

        <div className="p-4 flex flex-col md:flex-row gap-4">
          <span className="w-48 font-semibold text-muted-foreground flex items-center gap-2">
            <Clock size={16} /> Timestamp
          </span>
          <span>
            {new Date(tx.timestamp * 1000).toLocaleString()} (
            {new Date(tx.timestamp * 1000).toUTCString()})
          </span>
        </div>

        <div className="p-4 flex flex-col md:flex-row gap-4">
          <span className="w-48 font-semibold text-muted-foreground">From</span>
          <span className="font-mono text-blue-400 break-all">{tx.from}</span>
        </div>

        <div className="p-4 flex flex-col md:flex-row gap-4">
          <span className="w-48 font-semibold text-muted-foreground">To</span>
          <span className="font-mono text-blue-400 break-all">{tx.to}</span>
        </div>

        <div className="p-4 flex flex-col md:flex-row gap-4 bg-secondary/5">
          <span className="w-48 font-semibold text-muted-foreground">
            Value
          </span>
          <div className="flex items-center gap-2">
            <span className="text-xl font-bold">{tx.amount}</span>
            <span className="badge">{getAssetSymbol(tx.asset)}</span>
          </div>
        </div>

        {tx.fee_payer && (
          <div className="p-4 flex flex-col md:flex-row gap-4 bg-yellow-500/5">
            <span className="w-48 font-semibold text-yellow-500 flex items-center gap-2">
              ⚡ Fee Payer
            </span>
            <span className="font-mono text-muted-foreground">
              {tx.fee_payer}
            </span>
          </div>
        )}

        {tx.memo && (
          <div className="p-4 flex flex-col md:flex-row gap-4">
            <span className="w-48 font-semibold text-muted-foreground">
              Memo
            </span>
            <span className="font-mono text-xs bg-black/20 p-2 rounded w-full break-all whitespace-pre-wrap">
              {tx.memo}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
