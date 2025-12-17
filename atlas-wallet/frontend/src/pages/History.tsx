import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { ArrowLeft, ArrowUpRight, ArrowDownLeft, Search } from "lucide-react";
import { Input } from "@/components/ui/input";

export default function History() {
  const navigate = useNavigate();

  // Mock Data Grouped
  const sections = [
    {
      title: "Hoje",
      transactions: [
        {
          id: 1,
          type: "receive",
          amount: "500.00",
          date: "14:30",
          address: "0x123...abc",
          status: "completed",
        },
        {
          id: 4,
          type: "send",
          amount: "50.00",
          date: "10:15",
          address: "0xStore...XYZ",
          status: "completed",
        },
      ],
    },
    {
      title: "Ontem",
      transactions: [
        {
          id: 2,
          type: "send",
          amount: "120.50",
          date: "09:15",
          address: "0x456...def",
          status: "completed",
        },
      ],
    },
    {
      title: "Dezembro 2024",
      transactions: [
        {
          id: 3,
          type: "receive",
          amount: "1000.00",
          date: "12 Dez",
          address: "0x789...ghi",
          status: "completed",
        },
        {
          id: 5,
          type: "send",
          amount: "200.00",
          date: "10 Dez",
          address: "0xService...ABC",
          status: "completed",
        },
        {
          id: 6,
          type: "receive",
          amount: "45.00",
          date: "05 Dez",
          address: "0xFriend...123",
          status: "completed",
        },
      ],
    },
  ];

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
        {/* Search Bar (Optional Polish) */}
        <div className="p-4 sticky top-0 bg-background z-0">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Buscar transação..."
              className="pl-9 bg-secondary/50 border-0 h-10 text-xs"
            />
          </div>
        </div>

        <div className="px-4 pb-6 space-y-6">
          {sections.map((section) => (
            <div key={section.title} className="space-y-3">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground sticky top-14 bg-background/95 py-2 backdrop-blur-sm -mx-2 px-2">
                {section.title}
              </h3>
              <div className="space-y-2">
                {section.transactions.map((tx) => (
                  <div
                    key={tx.id}
                    className="flex items-center justify-between p-3 bg-secondary/30 rounded-xl border border-border/40 hover:bg-secondary/50 transition-colors cursor-pointer group"
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className={`w-10 h-10 rounded-full flex items-center justify-center transition-transform group-hover:scale-110 ${
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
                      <div>
                        <p className="text-sm font-medium">
                          {tx.type === "receive" ? "Recebido" : "Enviado"}
                        </p>
                        <p className="text-[10px] text-muted-foreground font-mono">
                          {tx.address}
                        </p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p
                        className={`text-sm font-bold ${
                          tx.type === "receive"
                            ? "text-green-500"
                            : "text-foreground"
                        }`}
                      >
                        {tx.type === "receive" ? "+" : "-"}
                        {tx.amount}
                      </p>
                      <p className="text-[10px] text-muted-foreground">
                        {tx.date}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </main>
    </div>
  );
}
