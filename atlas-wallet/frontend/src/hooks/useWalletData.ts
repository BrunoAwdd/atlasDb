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
          .GetStatement({ address: currentTrackingAddr, limit: "20" })
          .then((res) => {
            if (res && res.transactions) {
              setHistory(res.transactions);
            }
          });
      } catch (e) {
        console.error(e);
      }

      // Fetch Real Balances via gRPC
      let exposedBalBRL = "0";
      let hiddenBalBRL = "0";
      let exposedBalMOX = "0";
      let hiddenBalMOX = "0";

      try {
        // Parallel fetch for BRL and MOX
        const [expBRLRes, hidBRLRes, expMOXRes, hidMOXRes] = await Promise.all([
          ledgerClient.GetBalance({ address: exposedAddr, asset: "BRL" }),
          ledgerClient.GetBalance({ address: hiddenAddr, asset: "BRL" }),
          ledgerClient.GetBalance({ address: exposedAddr, asset: "MOX" }),
          ledgerClient.GetBalance({ address: hiddenAddr, asset: "MOX" }),
        ]);

        exposedBalBRL = expBRLRes.balance;
        hiddenBalBRL = hidBRLRes.balance;
        exposedBalMOX = expMOXRes.balance;
        hiddenBalMOX = hidMOXRes.balance;
      } catch (err) {
        console.error("gRPC Balance Fetch Error:", err);
        setStatus("Offline: Não foi possível buscar saldo.");
      }

      // Attempt to extract nonces if available in session
      let exposedNonce = "0";
      let hiddenNonce = "0";

      if (session instanceof Map) {
        // Try getting nonce from map
        const expMap = session.get("exposed");
        const hidMap = session.get("hidden");
        if (expMap && expMap.get("nonce") !== undefined)
          exposedNonce = expMap.get("nonce").toString();
        if (hidMap && hidMap.get("nonce") !== undefined)
          hiddenNonce = hidMap.get("nonce").toString();
      } else {
        // Try getting nonce from object (Standard Object from serde_wasm_bindgen)
        const sessAny = session as any;
        if (sessAny.exposed?.nonce !== undefined)
          exposedNonce = sessAny.exposed.nonce.toString();
        if (sessAny.hidden?.nonce !== undefined)
          hiddenNonce = sessAny.hidden.nonce.toString();
      }

      // Set wallet with structured balances
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

    setStatus("Trocando perfil...");
    await switch_profile();
    setStatus(`Perfil trocado para: ${newProfile} com sucesso!`);
    setActiveProfile(newProfile);
    // setShowQr(false); // Only if we want to handle this - but QR is not in use in the snippet provided for Wallet.tsx
  };

  return {
    wallet,
    history,
    status,
    setStatus,
    activeProfile,
    switchProfile,
    refresh: fetchData,
  };
}
