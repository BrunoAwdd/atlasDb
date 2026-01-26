import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { ArrowLeft, ArrowUpRight, ArrowDownLeft, Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { useWalletData } from "@/hooks/useWalletData";
import { useMemo } from "react";

export default function History() {
  const navigate = useNavigate();
  const { history, wallet, activeProfile } = useWalletData();

  // Get current address to determine direction (Send/Receive)
  const currentAddress = useMemo(() => {
    if (!wallet) return "";
    return activeProfile === "exposed"
      ? wallet.exposed.address
      : wallet.hidden.address;
  }, [wallet, activeProfile]);

  // Process and Group Transactions
  const groupedTransactions = useMemo(() => {
    if (!history || history.length === 0) return [];

    const groups: { title: string; transactions: any[] }[] = [];
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);

    const isSameDay = (d1: Date, d2: Date) => {
      return (
        d1.getDate() === d2.getDate() &&
        d1.getMonth() === d2.getMonth() &&
        d1.getFullYear() === d2.getFullYear()
      );
    };

    // Sort by timestamp desc (just to be safe, though API returns sorted)
    const sorted = [...history].sort(
      (a, b) => Number(b.timestamp) - Number(a.timestamp),
    );

    sorted.forEach((tx) => {
      // Check if timestamp is seconds (small) or ms (large)
      let date = new Date(Number(tx.timestamp));
      if (Number(tx.timestamp) < 10000000000) {
        date = new Date(Number(tx.timestamp) * 1000);
      }

      let title = "";
      if (isSameDay(date, today)) {
        title = "Hoje";
      } else if (isSameDay(date, yesterday)) {
        title = "Ontem";
      } else {
        title = date.toLocaleDateString("pt-BR", {
          day: "2-digit",
          month: "short",
          year: "numeric",
        });
      }

      // Add to group
      const existingGroup = groups.find((g) => g.title === title);

      // Determine Type using simple string check
      const isReceive =
        tx.to === currentAddress || tx.to.includes(currentAddress);

      const parsedTx = {
        id: tx.txHash,
        type: isReceive ? "receive" : "send",
        amount: tx.amount, // TODO: Format decimals if needed (it's string u128)
        asset: tx.asset,
        date: date.toLocaleTimeString("pt-BR", {
          hour: "2-digit",
          minute: "2-digit",
        }),
        address: isReceive ? tx.from : tx.to,
        status: "completed", // Assumed finalized for now
        memo: tx.memo,
      };

      if (existingGroup) {
        existingGroup.transactions.push(parsedTx);
      } else {
        groups.push({ title, transactions: [parsedTx] });
      }
    });

    return groups;
  }, [history, currentAddress]);

  return (
    <div className="flex flex-col h-full bg-background text-foreground animate-in slide-in-from-right duration-300">
      {/* Header */}
      <header className="relative flex items-center justify-center p-6 border-b border-border/40 bg-background/80 backdrop-blur-md sticky top-0 z-10">
        <Button
          variant="ghost"
          size="icon"
          className="absolute left-4 top-4 text-muted-foreground hover:text-foreground transition-colors"
          onClick={() => navigate("/wallet")}
        >
          <ArrowLeft className="h-5 w-5" />
        </Button>

        <h1 className="text-xl font-bold tracking-tight">Histórico</h1>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto">
        {/* Search Bar */}
        <div className="p-4 sticky top-0 bg-background z-0">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Buscar transação..."
              className="pl-9 bg-secondary/50 border-0 h-10 text-xs focus-visible:ring-1"
            />
          </div>
        </div>

        <div className="px-4 pb-6 space-y-6">
          {groupedTransactions.length === 0 ? (
            <div className="text-center py-10 opacity-50">
              <p className="text-sm">Nenhuma transação encontrada.</p>
            </div>
          ) : (
            groupedTransactions.map((section) => (
              <div key={section.title} className="space-y-3">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground sticky top-14 bg-background/95 py-2 backdrop-blur-sm -mx-2 px-2 z-10">
                  {section.title}
                </h3>
                <div className="space-y-2">
                  {section.transactions.map((tx) => (
                    <div
                      key={tx.id}
                      className="flex items-center justify-between p-3 bg-secondary/30 rounded-xl border border-border/40 hover:bg-secondary/50 transition-colors cursor-pointer group"
                    >
                      <div className="flex items-center gap-3 overflow-hidden">
                        <div
                          className={`w-10 h-10 rounded-full flex items-center justify-center transition-transform group-hover:scale-110 flex-shrink-0 ${
                            tx.type === "receive"
                              ? "bg-green-500/10 text-green-500"
                              : "bg-red-500/10 text-red-500"
                          }`}
                        >
                          {tx.type === "receive" ? (
                            <ArrowDownLeft className="h-5 w-5" />
                          ) : (
                            <ArrowUpRight className="h-5 w-5" />
                          )}
                        </div>
                        <div className="min-w-0">
                          <p className="text-sm font-medium truncate">
                            {tx.type === "receive" ? "Recebido" : "Enviado"}
                          </p>
                          <p
                            className="text-[10px] text-muted-foreground font-mono truncate max-w-[120px]"
                            title={tx.address}
                          >
                            {tx.address}
                          </p>
                        </div>
                      </div>
                      <div className="text-right flex-shrink-0">
                        <p
                          className={`text-sm font-bold ${
                            tx.type === "receive"
                              ? "text-green-500"
                              : "text-foreground"
                          }`}
                        >
                          {tx.type === "receive" ? "+" : "-"}
                          {tx.amount}{" "}
                          <span className="text-[10px]">{tx.asset}</span>
                        </p>
                        <p className="text-[10px] text-muted-foreground">
                          {tx.date}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))
          )}
        </div>
      </main>
    </div>
  );
}
