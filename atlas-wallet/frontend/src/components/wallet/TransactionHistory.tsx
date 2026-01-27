import { ArrowDownLeft, ArrowUpRight } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";

interface TransactionHistoryProps {
  history: any[];
  activeProfile: "exposed" | "hidden";
  wallet: any;
}

export function TransactionHistory({
  history,
  activeProfile,
  wallet,
}: TransactionHistoryProps) {
  const navigate = useNavigate();

  return (
    <div className="space-y-4 pt-4">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Recent Transactions
        </h3>
        <Button
          variant="link"
          size="sm"
          className="h-auto p-0 text-[10px] text-primary"
          onClick={() => navigate("/history")}
        >
          See all
        </Button>
      </div>

      <div className="space-y-3">
        <div className="space-y-3">
          {history.length > 0 ? (
            history.map((tx: any) => {
              const currentAddress =
                activeProfile === "exposed"
                  ? wallet.exposed.address
                  : wallet.hidden.address;
              // Determine direction based on current active profile
              // Note: If tx is internal, it might be relevant to both.
              const isSender =
                tx.from.includes(currentAddress) || tx.from === currentAddress;

              return (
                <div
                  key={tx.txHash}
                  className="flex items-center justify-between p-3 bg-secondary/30 rounded-xl border border-border/40 hover:bg-secondary/50 transition-colors cursor-default"
                >
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-8 h-8 rounded-full flex items-center justify-center ${
                        isSender
                          ? "bg-red-500/10 text-red-500"
                          : "bg-green-500/10 text-green-500"
                      }`}
                    >
                      {!isSender ? (
                        <ArrowDownLeft className="h-4 w-4" />
                      ) : (
                        <ArrowUpRight className="h-4 w-4" />
                      )}
                    </div>
                    <div className="overflow-hidden">
                      <p
                        className="text-xs font-medium truncate w-32"
                        title={tx.txHash}
                      >
                        {isSender ? "Enviado" : "Recebido"}
                      </p>
                      <p
                        className="text-[10px] text-muted-foreground truncate"
                        title={tx.txHash}
                      >
                        {tx.txHash.substring(0, 10)}...
                      </p>
                      <p className="text-[10px] text-muted-foreground">
                        {new Date(Number(tx.timestamp) * 1000).toLocaleString()}
                      </p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p
                      className={`text-xs font-bold ${
                        !isSender ? "text-green-500" : "text-foreground"
                      }`}
                    >
                      {!isSender ? "+" : "-"}
                      {tx.amount} {tx.asset}
                    </p>
                    <p className="text-[10px] text-muted-foreground">
                      Confirmed
                    </p>
                  </div>
                </div>
              );
            })
          ) : (
            <div className="text-center py-8 opacity-50 text-xs">
              No recent transactions
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
