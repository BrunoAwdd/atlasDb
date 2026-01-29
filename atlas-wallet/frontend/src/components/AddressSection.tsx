import { useState } from "react";
import QRCode from "react-qr-code";
import { Button } from "@/components/ui/button";
import { Copy, Check, QrCode } from "lucide-react";

interface AddressSectionProps {
  type: string;
  data: {
    address: string;

    balances: Record<string, string>;
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

  const balanceEntries = data?.balances ? Object.entries(data.balances) : [];
  // Prioritize USD if exists to be main, or just take first
  const mainBalance = balanceEntries[0] || ["USD", "0"];
  const otherBalances = balanceEntries.slice(1);

  return (
    <div className="space-y-4 animate-in fade-in duration-300">
      <div className="flex flex-col items-center justify-center py-6 bg-secondary/20 rounded-2xl border border-border/50 space-y-2">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-widest mb-1">
          Balance {type === "exposed" ? "Total" : "Hidden"}
        </span>

        <div className="flex flex-col items-center w-full px-4">
          {balanceEntries.length === 0 ? (
            <div className="text-center py-2">
              <h2 className="text-4xl font-black tracking-tighter bg-gradient-to-br from-foreground to-muted-foreground bg-clip-text text-transparent">
                0.00
              </h2>
              <span className="text-xs font-bold text-muted-foreground/60 tracking-widest uppercase mt-1 block">
                USD
              </span>
            </div>
          ) : (
            <>
              {/* Primary Balance (First One) */}
              <div className="text-center py-2 mb-4">
                <h2 className="text-4xl font-black tracking-tighter text-foreground drop-shadow-sm">
                  {new Intl.NumberFormat("en-US", {
                    minimumFractionDigits: 2,
                  }).format(Number(balanceEntries[0][1]))}
                </h2>
                <span className="text-xs font-bold text-primary/80 tracking-widest uppercase mt-1 block">
                  {balanceEntries[0][0]}
                </span>
              </div>

              {/* Other Assets Grid */}
              {balanceEntries.length > 1 && (
                <div className="grid grid-cols-3 gap-2 w-full max-w-xs animate-in slide-in-from-bottom-2 duration-500">
                  {balanceEntries.slice(1).map(([asset, amount]) => (
                    <div
                      key={asset}
                      className="flex flex-col items-center justify-center p-2 rounded-lg bg-background/40 border border-border/40 hover:bg-background/60 transition-colors"
                    >
                      <span className="text-sm font-bold text-foreground/90">
                        {new Intl.NumberFormat("en-US", {
                          notation: "compact",
                          compactDisplay: "short",
                          maximumFractionDigits: 1,
                        }).format(Number(amount))}
                      </span>
                      <span className="text-[10px] font-semibold text-muted-foreground uppercase">
                        {asset}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
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
