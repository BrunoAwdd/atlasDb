import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface SendTransactionFormProps {
  toAddress: string;
  onAddressChange: (val: string) => void;
  amount: string;
  onAmountChange: (val: string) => void;
  asset: string;
  onAssetChange: (val: string) => void;
  onSend: () => void;
  status: string;
}

export function SendTransactionForm({
  toAddress,
  onAddressChange,
  amount,
  onAmountChange,
  asset,
  onAssetChange,
  onSend,
  status,
}: SendTransactionFormProps) {
  return (
    <>
      <div className="space-y-4">
        <div className="space-y-2">
          <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Destinatário
          </label>
          <Input
            placeholder="Endereço da carteira"
            className="bg-secondary/50 border-0 h-11 focus-visible:ring-primary font-mono text-xs"
            value={toAddress}
            onChange={(e) => onAddressChange(e.target.value)}
          />
        </div>

        <div className="grid grid-cols-3 gap-4">
          <div className="col-span-2 space-y-2">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Valor
            </label>
            <Input
              placeholder="0.00"
              type="number"
              className="bg-secondary/50 border-0 h-11 focus-visible:ring-primary"
              value={amount}
              onChange={(e) => onAmountChange(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Ativo
            </label>
            <Input
              placeholder="MOX"
              className="bg-secondary/50 border-0 h-11 focus-visible:ring-primary font-mono uppercase"
              value={asset}
              onChange={(e) => onAssetChange(e.target.value.toUpperCase())}
            />
          </div>
        </div>

        <Button
          onClick={onSend}
          className="w-full h-11 text-sm font-medium shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all mt-2"
        >
          Enviar Transação
        </Button>
      </div>
      {status && (
        <div className="p-3 rounded-lg bg-secondary/50 border border-border/50 mt-4">
          <p className="text-xs text-center font-medium opacity-90">{status}</p>
        </div>
      )}
    </>
  );
}
