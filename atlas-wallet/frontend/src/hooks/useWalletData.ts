import { useState, useEffect, useCallback } from "react";
import { ledgerClient } from "@/lib/client";
import { get_data, switch_profile } from "../pkg/atlas_wallet";
import { ASSET_MAP } from "@/lib/assets";

interface WalletBalance {
  asset: string;
  balance: string;
  nonce: number;
}

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

interface WasmSession {
  exposed?: { address: string };
  hidden?: { address: string };
  get?: (key: string) => any;
}

export function useWalletData() {
  const [wallet, setWallet] = useState<WalletData | null>(null);
  const [status, setStatus] = useState("");
  const [activeProfile, setActiveProfile] = useState<"exposed" | "hidden">(
    "exposed",
  );
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [history, setHistory] = useState<any[]>([]);

  const fetchData = useCallback(async () => {
    // --- gRPC Integration ---
    let session: WasmSession | Map<string, any>;
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
      let exposedAddr: string | undefined;
      let hiddenAddr: string | undefined;

      if (session instanceof Map) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-call
        exposedAddr = session.get("exposed")?.get("address");
        // eslint-disable-next-line @typescript-eslint/no-unsafe-call
        hiddenAddr = session.get("hidden")?.get("address");
      } else {
        // Standard Object from serde_wasm_bindgen
        exposedAddr = (session as WasmSession).exposed?.address;
        hiddenAddr = (session as WasmSession).hidden?.address;
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
      const ASSETS = Object.values(ASSET_MAP);

      // Fetch Real Balances via gRPC
      const fetchBalanceAndNonce = async (
        addr: string,
        asset: string,
      ): Promise<WalletBalance> => {
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
        fetchBalanceAndNonce(exposedAddr!, a),
      );
      const hiddenPromises = ASSETS.map((a) =>
        fetchBalanceAndNonce(hiddenAddr!, a),
      );

      const [exposedResults, hiddenResults] = await Promise.all([
        Promise.all(exposedPromises),
        Promise.all(hiddenPromises),
      ]);

      // Construct Balances Map
      const exposedBalances: Record<string, string> = {};
      let maxExposedNonce = 0;

      exposedResults.forEach((r) => {
        if (Number(r.balance) > 0 || r.asset === ASSET_MAP["USD"]) {
          exposedBalances[r.asset] = r.balance;
        }
        if (r.nonce > maxExposedNonce) maxExposedNonce = r.nonce;
      });

      const hiddenBalances: Record<string, string> = {};
      let maxHiddenNonce = 0;

      hiddenResults.forEach((r) => {
        if (Number(r.balance) > 0 || r.asset === ASSET_MAP["USD"]) {
          hiddenBalances[r.asset] = r.balance;
        }
        if (r.nonce > maxHiddenNonce) maxHiddenNonce = r.nonce;
      });

      // Determine nonce (Ledger is source of truth)
      const exposedNonce = maxExposedNonce.toString();
      const hiddenNonce = maxHiddenNonce.toString();

      // Debug: Show fetched balances in status
      setStatus(`Sync OK.`);

      setWallet((_prev) => {
        // Source of Truth: The Chain (ledgerClient results)
        // We do NOT preserve local nonce if chain is lower (e.g. after a reset/wipe).
        const safeExposedNonce = exposedNonce;
        const safeHiddenNonce = hiddenNonce;

        return {
          exposed: {
            address: exposedAddr!,
            nonce: safeExposedNonce,
            balances: exposedBalances,
          },
          hidden: {
            address: hiddenAddr!,
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
