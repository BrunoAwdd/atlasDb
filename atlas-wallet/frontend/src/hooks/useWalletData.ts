import { useState, useEffect, useCallback } from "react";
import { ledgerClient } from "@/lib/client";
import { get_data, switch_profile } from "../pkg/atlas_wallet"; // Adjust import path as needed

export interface WalletData {
  exposed: {
    address: string;
    nonce: string;
    balances: Record<string, string>;
  };
  hidden: {
    address: string;
    nonce: string;
    balances: Record<string, string>;
  };
}

export function useWalletData() {
  const [wallet, setWallet] = useState<WalletData | null>(null);
  const [status, setStatus] = useState("");
  const [activeProfile, setActiveProfile] = useState<"exposed" | "hidden">(
    "exposed",
  );
  const [history, setHistory] = useState<any[]>([]);

  const fetchData = useCallback(async () => {
    // --- gRPC Integration ---
    let session;
    try {
      console.log("Fetching data from WASM...");
      session = await get_data();
      console.log("Identity Loaded from WASM:", session);
    } catch (error) {
      console.error(
        "Error loading identity from WASM (not initialized?):",
        error,
      );
      setStatus("Sessão não encontrada.");
      return;
    }

    if (session) {
      let exposedAddr, hiddenAddr;

      if (session instanceof Map) {
        exposedAddr = session.get("exposed").get("address");
        hiddenAddr = session.get("hidden").get("address");
      } else {
        // Standard Object from serde_wasm_bindgen
        exposedAddr = session.exposed?.address;
        hiddenAddr = session.hidden?.address;
      }

      if (!exposedAddr || !hiddenAddr) {
        console.error("Endereços não encontrados no session:", session);
        setStatus("Dados da carteira incompletos.");
        return;
      }

      // Determine which address to track for history based on activeProfile
      const currentTrackingAddr =
        activeProfile === "exposed" ? exposedAddr : hiddenAddr;

      // Fetch History for *Active Profile*
      try {
        ledgerClient
          .GetStatement({ address: currentTrackingAddr, limit: 20 })
          .then((res) => {
            if (res && res.transactions) {
              setHistory(res.transactions);
            }
          });
      } catch (e) {
        console.error(e);
      }

      // Define supported assets
      const ASSETS = ["USD", "BRL", "GBP", "EUR"];

      // Fetch Real Balances via gRPC
      const fetchBalanceAndNonce = async (addr: string, asset: string) => {
        try {
          const res = await ledgerClient.GetBalance({
            address: addr,
            asset: asset,
          });
          return { asset, balance: res.balance, nonce: Number(res.nonce) || 0 };
        } catch (e) {
          console.warn(`Failed to fetch ${asset} balance for ${addr}:`, e);
          return { asset, balance: "0", nonce: 0 };
        }
      };

      // Fetch all assets for Exposed and Hidden
      const exposedPromises = ASSETS.map((a) =>
        fetchBalanceAndNonce(exposedAddr, a),
      );
      const hiddenPromises = ASSETS.map((a) =>
        fetchBalanceAndNonce(hiddenAddr, a),
      );

      const [exposedResults, hiddenResults] = await Promise.all([
        Promise.all(exposedPromises),
        Promise.all(hiddenPromises),
      ]);

      // Construct Balances Map
      const exposedBalances: Record<string, string> = {};
      let maxExposedNonce = 0;

      exposedResults.forEach((r) => {
        if (Number(r.balance) > 0 || r.asset === "USD") {
          exposedBalances[r.asset] = r.balance;
        }
        if (r.nonce > maxExposedNonce) maxExposedNonce = r.nonce;
      });

      const hiddenBalances: Record<string, string> = {};
      let maxHiddenNonce = 0;

      hiddenResults.forEach((r) => {
        if (Number(r.balance) > 0 || r.asset === "USD") {
          hiddenBalances[r.asset] = r.balance;
        }
        if (r.nonce > maxHiddenNonce) maxHiddenNonce = r.nonce;
      });

      // Determine nonce (Ledger is source of truth)
      const exposedNonce = maxExposedNonce.toString();
      const hiddenNonce = maxHiddenNonce.toString();

      // Debug: Show fetched balances in status
      setStatus(`Sync OK.`);

      setWallet((prev) => {
        // Security: Ensure Nonce never decreases (Handling Race Conditions)
        let safeExposedNonce = exposedNonce;
        let safeHiddenNonce = hiddenNonce;

        if (prev) {
          if (BigInt(prev.exposed.nonce) > BigInt(exposedNonce)) {
            safeExposedNonce = prev.exposed.nonce;
          }
          if (BigInt(prev.hidden.nonce) > BigInt(hiddenNonce)) {
            safeHiddenNonce = prev.hidden.nonce;
          }
        }

        return {
          exposed: {
            address: exposedAddr,
            nonce: safeExposedNonce,
            balances: exposedBalances,
          },
          hidden: {
            address: hiddenAddr,
            nonce: safeHiddenNonce,
            balances: hiddenBalances,
          },
        };
      });
    } else {
      setStatus("Carteira vazia / Erro no formato.");
    }
  }, [activeProfile]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const switchProfile = async (val: string) => {
    if (val === activeProfile) return;
    const newProfile = val as "exposed" | "hidden";

    // Optimistic Update: Update UI immediately
    setActiveProfile(newProfile);
    setStatus("Trocando perfil...");

    try {
      await switch_profile();
      setStatus(`Perfil trocado para: ${newProfile} com sucesso!`);
    } catch (error) {
      console.error("Failed to switch profile:", error);
      setStatus("Erro ao trocar perfil. Tente novamente.");
      // Revert if critical? For now, we assume backend toggle eventually works or we are out of sync anyway.
      // If we revert: setActiveProfile(activeProfile);
    }
  };

  const incrementNonce = useCallback((profile: "exposed" | "hidden") => {
    setWallet((prev) => {
      if (!prev) return null;
      const currentNonce = Number(prev[profile].nonce);
      const newNonce = (currentNonce + 1).toString();

      return {
        ...prev,
        [profile]: {
          ...prev[profile],
          nonce: newNonce,
        },
      };
    });
  }, []);

  return {
    wallet,
    history,
    status,
    setStatus,
    activeProfile,
    switchProfile,
    refresh: fetchData,
    incrementNonce,
  };
}
