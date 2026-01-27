import { useState } from "react";
import QRCode from "react-qr-code";
import { Button } from "@/components/ui/button";
import { Copy, Check, QrCode } from "lucide-react";

interface AddressSectionProps {
  type: string;
  data: {
    address: string;
    balances: {
      BRL: string;
      MOX: string;
    };
    nonce?: string | number;
  };
}

export function AddressSection({ type, data }: AddressSectionProps) {
  const [showQr, setShowQr] = useState(false);
  const [copied, setCopied] = useState(false);

  const copyToClipboard = async (text: string) => {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  };

  return (
    <div className="space-y-4 animate-in fade-in duration-300">
      <div className="flex flex-col items-center justify-center py-6 bg-secondary/20 rounded-2xl border border-border/50 space-y-2">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-widest mb-1">
          Balance {type === "exposed" ? "Total" : "Hidden"}
        </span>
        <div className="flex flex-col items-center">
          <h2 className="text-3xl font-bold tracking-tight">
            {data?.balances?.BRL || "0"}{" "}
            <span className="text-sm font-medium text-muted-foreground">
              BRL
            </span>
          </h2>
          <h2 className="text-xl font-bold tracking-tight text-muted-foreground/80">
            {data?.balances?.MOX || "0"}{" "}
            <span className="text-xs font-medium">MOX</span>
          </h2>
        </div>
      </div>

      <div className="bg-secondary/40 p-4 rounded-xl border border-border/50 space-y-3">
        <div className="flex justify-between items-center">
          <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">
            {type === "exposed" ? "Public" : "Private"} Address
          </label>
          <div className="flex gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={() => setShowQr(!showQr)}
              title="Mostrar QR Code"
            >
              <QrCode className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={() => copyToClipboard(data?.address)}
              title="Copiar endereço"
            >
              {copied ? (
                <Check className="h-3.5 w-3.5 text-green-500" />
              ) : (
                <Copy className="h-3.5 w-3.5" />
              )}
            </Button>
          </div>
        </div>

        <p className="text-xs font-mono break-all text-muted-foreground bg-background/50 p-2 rounded border border-border/20">
          {data?.address || "N/A"}
        </p>

        {showQr && data?.address && (
          <div className="flex justify-center pt-2 pb-1 animate-in zoom-in-50 duration-300">
            <div className="p-3 bg-white rounded-xl shadow-sm">
              <QRCode value={data.address} size={150} />
            </div>
          </div>
        )}

        <div className="flex justify-between items-center pt-2 border-t border-border/20 mt-2">
          <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-widest">
            Nonce Local
          </span>
          <span className="text-xs font-mono font-bold text-foreground">
            {data?.nonce || "0"}
          </span>
        </div>
      </div>
    </div>
  );
}
