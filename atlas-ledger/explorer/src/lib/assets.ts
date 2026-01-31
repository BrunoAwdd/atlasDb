export const ASSET_MAP: Record<string, string> = {
  USD: "wallet:mint/USD",
  EUR: "wallet:mint/EUR",
  GBP: "wallet:mint/GBP",
  BRL: "wallet:mint/BRL",
  ATLAS: "wallet:mint/ATLAS",
  XAU: "wallet:mint/XAU",
};

export const REVERSE_ASSET_MAP: Record<string, string> = Object.entries(
  ASSET_MAP,
).reduce(
  (acc, [k, v]) => {
    acc[v] = k;
    return acc;
  },
  {} as Record<string, string>,
);

export function getFullAssetId(symbol: string): string {
  const upper = symbol.toUpperCase();
  return ASSET_MAP[upper] || symbol; // Return as-is if no map found (allow custom/full IDs)
}

export function getAssetSymbol(fullId: string): string {
  if (!fullId) return "UNK";
  // Check exact map match
  if (REVERSE_ASSET_MAP[fullId]) return REVERSE_ASSET_MAP[fullId];

  // Fallback: split by '/'
  const parts = fullId.split("/");
  if (parts.length > 1) return parts[parts.length - 1];

  return fullId;
}
