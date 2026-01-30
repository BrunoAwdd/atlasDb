import { useState, useEffect, useCallback } from "react";
import { getAssetSymbol } from "@/lib/assets";

const STORAGE_KEY = "atlas_wallet_favorites";
const DEFAULT_FAVORITES = ["USD", "EUR", "BRL", "ATLAS"];

export function useFavorites() {
  // Initialize state from Storage
  const [favorites, setFavorites] = useState<string[]>(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      return stored ? JSON.parse(stored) : DEFAULT_FAVORITES;
    } catch (e) {
      console.warn("Failed to load favorites", e);
      return DEFAULT_FAVORITES;
    }
  });

  // Listener for Cross-Component Sync
  useEffect(() => {
    const handleStorageChange = () => {
      try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored) {
          const parsed = JSON.parse(stored);
          // Only update if different to avoid excess renders
          setFavorites((prev) => {
            if (JSON.stringify(prev) !== JSON.stringify(parsed)) {
              return parsed;
            }
            return prev;
          });
        }
      } catch (e) {
        console.error(e);
      }
    };

    window.addEventListener("favorites-updated", handleStorageChange);
    window.addEventListener("storage", handleStorageChange);

    return () => {
      window.removeEventListener("favorites-updated", handleStorageChange);
      window.removeEventListener("storage", handleStorageChange);
    };
  }, []);

  const reorderFavorites = useCallback((newOrder: string[]) => {
    setFavorites(() => {
      // Persist and Dispatch
      localStorage.setItem(STORAGE_KEY, JSON.stringify(newOrder));
      window.dispatchEvent(new Event("favorites-updated"));
      return newOrder;
    });
  }, []);

  const toggleFavorite = useCallback((assetId: string) => {
    const symbol = getAssetSymbol(assetId);

    setFavorites((prev) => {
      let newFavs;
      if (prev.includes(symbol)) {
        // Unpin: Remove from list
        newFavs = prev.filter((f) => f !== symbol);
      } else {
        // Pin: Add to START of list (Promote to top)
        newFavs = [symbol, ...prev];
      }

      // 1. Persist to Storage
      localStorage.setItem(STORAGE_KEY, JSON.stringify(newFavs));

      // 2. Dispatch Event for other components
      window.dispatchEvent(new Event("favorites-updated"));

      return newFavs;
    });
  }, []);

  const isFavorite = useCallback(
    (assetId: string) => {
      const symbol = getAssetSymbol(assetId);
      return favorites.includes(symbol);
    },
    [favorites],
  );

  return { favorites, toggleFavorite, isFavorite, reorderFavorites };
}
