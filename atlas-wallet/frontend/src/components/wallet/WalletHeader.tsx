import { Button } from "@/components/ui/button";
import { LogOut, PanelRight } from "lucide-react";

interface WalletHeaderProps {
  onLogout: () => void;
  onOpenSidePanel: () => void;
}

export function WalletHeader({ onLogout, onOpenSidePanel }: WalletHeaderProps) {
  return (
    <header className="relative flex items-center justify-center p-6 border-b border-border/40">
      <Button
        variant="ghost"
        size="icon"
        className="absolute left-4 top-4 text-muted-foreground hover:text-foreground transition-colors"
        onClick={onLogout}
        title="Sair / Logout"
      >
        <LogOut className="h-5 w-5" />
      </Button>

      <h1 className="text-xl font-bold tracking-tight">Carteira</h1>

      <Button
        variant="ghost"
        size="icon"
        className="absolute right-4 top-4 text-muted-foreground hover:text-foreground transition-colors"
        title="Abrir no painel lateral"
        onClick={onOpenSidePanel}
      >
        <PanelRight className="h-5 w-5" />
      </Button>
    </header>
  );
}
