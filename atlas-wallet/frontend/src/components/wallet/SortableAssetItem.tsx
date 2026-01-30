import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { getAssetSymbol } from "@/lib/assets";

interface SortableAssetItemProps {
  id: string; // The Asset Symbol
  balance: [string, string]; // [Symbol, Amount]
  className?: string;
  variant?: "default" | "hero";
}

export function SortableAssetItem({
  id,
  balance,
  className,
  variant = "default",
}: SortableAssetItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    zIndex: isDragging ? 50 : "auto",
    opacity: isDragging ? 0.5 : 1,
  };

  const [assetId, amount] = balance;
  const symbol = getAssetSymbol(assetId);

  if (variant === "hero") {
    return (
      <div
        ref={setNodeRef}
        style={style}
        {...attributes}
        {...listeners}
        className={`col-span-2 flex flex-col items-center justify-center p-8 bg-gradient-to-br from-background via-secondary/10 to-primary/5 rounded-3xl border border-border transition-all duration-300 hover:border-primary/20 hover:shadow-lg group cursor-grab active:cursor-grabbing select-none ${className}`}
      >
        <span className="text-sm font-bold text-muted-foreground uppercase tracking-[0.2em] mb-3 group-hover:text-primary transition-colors">
          {symbol}
        </span>

        <h2 className="text-6xl font-black tracking-tighter text-foreground">
          {new Intl.NumberFormat("en-US", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
          }).format(Number(amount))}
        </h2>

        <div className="mt-4 h-1 w-12 bg-primary/20 rounded-full group-hover:w-24 group-hover:bg-primary transition-all duration-500" />
      </div>
    );
  }

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      className={`flex flex-col p-4 bg-gradient-to-br from-background to-secondary/30 rounded-2xl border border-border/40 shadow-sm hover:shadow-md transition-all duration-300 group cursor-grab active:cursor-grabbing select-none ${className}`}
    >
      <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider mb-1 group-hover:text-primary transition-colors">
        {symbol}
      </span>
      <h2 className="text-2xl font-black tracking-tighter text-foreground">
        {new Intl.NumberFormat("en-US", {
          notation: "compact",
          compactDisplay: "short",
          maximumFractionDigits: 1,
        }).format(Number(amount))}
      </h2>
    </div>
  );
}
