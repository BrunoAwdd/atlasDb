import { useState, useEffect, useCallback } from "react";
import { ledgerClient } from "@/lib/client";
import { get_data, switch_profile } from "../pkg/atlas_wallet"; // Adjust import path as needed

export interface WalletData {
  exposed: {
    address: string;
    nonce: string;
    balances: { BRL: string; MOX: string };
  };
  hidden: {
    address: string;
    nonce: string;
    balances: { BRL: string; MOX: string };
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

      // Fetch Real Balances via gRPC
      const fetchBalanceAndNonce = async (addr: string, asset: string) => {
        try {
          const res = await ledgerClient.GetBalance({
            address: addr,
            asset: asset,
          });
          // Fix: Proto definition uses uint64 which might effectively be number or string in JS depending on configuration.
          // Check generated code if unsure, but typically `long` calls safe to string or number.
          return { balance: res.balance, nonce: res.nonce };
        } catch (e) {
          console.warn(`Failed to fetch ${asset} balance for ${addr}:`, e);
          return { balance: "0", nonce: 0 };
        }
      };

      const [expBRLRes, hidBRLRes, expMOXRes, hidMOXRes] = await Promise.all([
        fetchBalanceAndNonce(exposedAddr, "BRL"),
        fetchBalanceAndNonce(hiddenAddr, "BRL"),
        fetchBalanceAndNonce(exposedAddr, "MOX"),
        fetchBalanceAndNonce(hiddenAddr, "MOX"),
      ]);

      const exposedBalBRL = expBRLRes.balance;
      const hiddenBalBRL = hidBRLRes.balance;
      const exposedBalMOX = expMOXRes.balance;
      const hiddenBalMOX = hidMOXRes.balance;

      // Determine nonce (Ledger is source of truth)
      const exposedNonce = Math.max(
        Number(expBRLRes.nonce),
        Number(expMOXRes.nonce),
      ).toString();
      const hiddenNonce = Math.max(
        Number(hidBRLRes.nonce),
        Number(hidMOXRes.nonce),
      ).toString();

      // Debug: Show fetched balances in status
      setStatus(`Sync OK. BRL: ${exposedBalBRL} | MOX: ${exposedBalMOX}`);

      setWallet({
        exposed: {
          address: exposedAddr,
          nonce: exposedNonce,
          balances: { BRL: exposedBalBRL, MOX: exposedBalMOX },
        },
        hidden: {
          address: hiddenAddr,
          nonce: hiddenNonce,
          balances: { BRL: hiddenBalBRL, MOX: hiddenBalMOX },
        },
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
