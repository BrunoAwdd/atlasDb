import { useState } from "react";
import QRCode from "react-qr-code";
import { Button } from "@/components/ui/button";
import { Copy, Check, QrCode } from "lucide-react";
import { getAssetSymbol } from "@/lib/assets";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  rectSortingStrategy,
} from "@dnd-kit/sortable";
import { SortableAssetItem } from "./wallet/SortableAssetItem";

interface AddressSectionProps {
  type: string;
  data: {
    address: string;

    balances: Record<string, string>;
    nonce?: string | number;
  };
}

import { useFavorites } from "@/hooks/useFavorites";

export function AddressSection({ type, data }: AddressSectionProps) {
  const [showQr, setShowQr] = useState(false);
  const [copied, setCopied] = useState(false);
  const { favorites: FAVORITE_ORDER, reorderFavorites } = useFavorites();

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;

    if (active.id !== over?.id) {
      const oldIndex = FAVORITE_ORDER.indexOf(String(active.id));
      const newIndex = FAVORITE_ORDER.indexOf(String(over!.id));

      const newOrder = arrayMove(FAVORITE_ORDER, oldIndex, newIndex);
      reorderFavorites(newOrder);
    }
  };

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

  // 1. Extract Native Token (ATLAS)
  const atlasEntry = balanceEntries.find(
    ([id]) => id === "passivo:wallet:mint/ATLAS",
  );
  const atlasAmount = atlasEntry ? Number(atlasEntry[1]) : 0;

  // 2. Filter remaining assets for the grid
  const gridEntries = balanceEntries.filter(
    ([id]) => id !== "passivo:wallet:mint/ATLAS",
  );

  // Split into Pinned (Highlights)
  const highlights: [string, string][] = [];

  // Sort by defined favorite order, then alphabetical
  const sortedEntries = [...gridEntries].sort((a, b) => {
    const symbolA = getAssetSymbol(a[0]);
    const symbolB = getAssetSymbol(b[0]);
    return symbolA.localeCompare(symbolB);
  });

  sortedEntries.forEach((entry) => {
    const symbol = getAssetSymbol(entry[0]);
    if (FAVORITE_ORDER.includes(symbol)) {
      highlights.push(entry);
    }
  });

  // Sort highlights specifically by the FAVORITE_ORDER index
  highlights.sort((a, b) => {
    const symbolA = getAssetSymbol(a[0]);
    const symbolB = getAssetSymbol(b[0]);
    return FAVORITE_ORDER.indexOf(symbolA) - FAVORITE_ORDER.indexOf(symbolB);
  });

  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      <div className="flex flex-col space-y-4">
        {/* Header with Native Token */}
        <div className="flex items-center justify-between px-2">
          <div className="flex flex-col">
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-widest">
              {type === "exposed" ? "Total Balance" : "Hidden Balance"}
            </span>
            <div className="flex items-baseline gap-1 mt-1">
              <span className="text-2xl font-bold tracking-tight text-foreground">
                {new Intl.NumberFormat("en-US", {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 6,
                }).format(atlasAmount)}
              </span>
              <span className="text-xs font-semibold text-primary/80">
                ATLAS
              </span>
            </div>
          </div>

          <span className="text-[10px] font-medium text-muted-foreground/50 bg-secondary/30 px-2 py-0.5 rounded-full self-start">
            {highlights.length} Pinned
          </span>
        </div>

        {/* Highlights Grid */}
        <div className="grid grid-cols-2 gap-3 w-full">
          {highlights.length === 0 ? (
            <div className="col-span-2 text-center py-8 bg-secondary/10 rounded-2xl border border-dashed border-border/30">
              <span className="text-muted-foreground text-sm">
                No pinned assets
              </span>
            </div>
          ) : (
            // Drag and Drop Grid for ALL Items (Single or Multiple)
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={highlights.map((h) => getAssetSymbol(h[0]))}
                strategy={rectSortingStrategy}
              >
                {highlights.map(([asset, amount], index) => {
                  const symbol = getAssetSymbol(asset);
                  // Dynamic spanning: If total is odd, first item is Hero
                  const isOddTotal = highlights.length % 2 !== 0;
                  const isFirst = index === 0;
                  const isHero = isOddTotal && isFirst;

                  const spanClass = isHero ? "col-span-2" : "col-span-1";

                  return (
                    <SortableAssetItem
                      key={symbol}
                      id={symbol}
                      balance={[asset, amount]}
                      className={spanClass}
                      variant={isHero ? "hero" : "default"}
                    />
                  );
                })}
              </SortableContext>
            </DndContext>
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
            Nonce (Chain)
          </span>
          <span className="text-xs font-mono font-bold text-foreground">
            {data?.nonce || "0"}
          </span>
        </div>
      </div>
    </div>
  );
}
