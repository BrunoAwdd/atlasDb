import { useState } from "react";
import { ledgerClient } from "@/lib/client";
import { sing_transfer } from "../pkg/atlas_wallet";

export function useTransactionSender({
  wallet,
  activeProfile,
  refresh,
  incrementNonce,
}: {
  wallet: any;
  activeProfile: "exposed" | "hidden";
  refresh: () => void;
  incrementNonce: (p: "exposed" | "hidden") => void;
}) {
  const [status, setStatus] = useState("");
  const [toAddress, setToAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [asset, setAsset] = useState("USD");

  const handleSend = async () => {
    try {
      setStatus("Assinando (WASM)...");

      // await init(wasmUrl); // Don't reset state!

      // Define Memo constant to ensure Signature matches Verification
      const txMemo = "Web Wallet Transfer";

      // 0. Get Current Nonce from Wallet State (Verified against Ledger)
      const currentNonce = wallet?.[activeProfile]?.nonce
        ? BigInt(wallet[activeProfile].nonce)
        : 0n;
      const nextNonce = currentNonce + 1n;

      console.log(
        `Using Nonce: ${nextNonce} (Current Account Nonce: ${currentNonce})`,
      );

      // 1. Sign with WASM
      // Result is a Map: { id: "...", transfer: { transaction: {...}, signature: [...], public_key: [...] } }
      const result = await sing_transfer(
        toAddress,
        BigInt(amount),
        asset,
        txMemo,
        nextNonce,
      );

      console.log("Assinatura gerada (RAW):", result);
      console.log("Type of result:", result?.constructor?.name);
      if (result instanceof Map) {
        console.log("Result keys:", Array.from(result.keys()));
        const transfer = result.get("transfer");
        console.log("Transfer object:", transfer);
        if (transfer instanceof Map)
          console.log("Transfer keys:", Array.from(transfer.keys()));
      } else {
        console.log("Result keys (Object):", Object.keys(result));
        console.log("Transfer object:", result?.transfer);
      }

      // Helper function for bytes to hex
      const toHex = (bytes: any) => {
        if (!bytes) return "";
        const arr = Array.from(bytes) as number[];
        return arr.map((b) => b.toString(16).padStart(2, "0")).join("");
      };

      let sigRaw: any, pkRaw: any; // Keep any to handle various WASM return shapes safely
      let sigIsHex = false;
      let pkIsHex = false;

      // Handle Map (wasm-bindgen default) or Object
      if (result instanceof Map) {
        const transfer = result.get("transfer");
        if (transfer instanceof Map) {
          sigRaw = transfer.get("signature");
          pkRaw = transfer.get("public_key");
        } else {
          // WASM might return struct with direct fields
          sigRaw = transfer.signature;
          pkRaw = transfer.public_key;
        }
        // The new WASM returns signature/pk as Hex String
        if (typeof sigRaw === "string") sigIsHex = true;
        if (typeof pkRaw === "string") pkIsHex = true;
      } else {
        // Fallback or Object structure
        if (result.transfer) {
          sigRaw = result.transfer.signature;
          pkRaw = result.transfer.public_key;
          if (typeof sigRaw === "string") sigIsHex = true;
          if (typeof pkRaw === "string") pkIsHex = true;
        }
      }

      // Ensure we have data
      if (!sigRaw || !pkRaw) {
        throw new Error("Falha ao extrair assinatura/chave pública do WASM");
      }

      let signatureHex = "";
      if (sigIsHex) {
        signatureHex = sigRaw;
      } else {
        signatureHex = toHex(sigRaw);
      }

      let publicKeyHex = "";
      if (pkIsHex) {
        publicKeyHex = pkRaw;
      } else {
        publicKeyHex = toHex(pkRaw);
      }

      // console.log("Sending Signature (Hex):", signatureHex);
      // console.log("Sending Public Key (Hex):", publicKeyHex);

      setStatus("Enviando (gRPC)...");

      // 2. Submit to Ledger
      // CRITICAL: Use the 'from' address that was ACTUALLY signed by the WASM module.
      // Relying on 'wallet.exposed.address' or 'activeProfile' here is risky because
      // the WASM internal state might be different (e.g. if switch_profile failed or desynced).
      let currentAddr = "";
      let nonceStr = "0";
      let timestampStr = "0";

      if (result instanceof Map) {
        const transfer = result.get("transfer");
        // Check if transaction is a Map or Object
        const transaction =
          transfer instanceof Map
            ? transfer.get("transaction")
            : transfer.transaction;

        if (transaction instanceof Map) {
          currentAddr = transaction.get("from");
          nonceStr = transaction.get("nonce").toString();
          timestampStr = transaction.get("timestamp").toString();
        } else {
          currentAddr = transaction.from;
          nonceStr = transaction.nonce.toString();
          timestampStr = transaction.timestamp.toString();
        }
      } else {
        // Object structure
        currentAddr = result.transfer.transaction.from;
        nonceStr = result.transfer.transaction.nonce.toString();
        timestampStr = result.transfer.transaction.timestamp.toString();
      }

      if (!currentAddr) {
        // Fallback to state if something went wrong parsing (should not happen)
        console.warn(
          "Could not extract 'from' from signed result, falling back to activeProfile state.",
        );
        currentAddr =
          activeProfile === "exposed"
            ? wallet.exposed.address
            : wallet.hidden.address;
      }

      console.log("Submitting Transaction FROM:", currentAddr);

      const txResponse = await ledgerClient.SubmitTransaction({
        from: currentAddr,
        to: toAddress,
        amount: amount.toString(),
        asset: asset, // Use selected asset
        memo: txMemo, // Use the SAME memo signed by WASM
        signature: signatureHex,
        publicKey: publicKeyHex, // Correct field name (camelCase)
        nonce: Number(nonceStr),
        timestamp: Number(timestampStr),
      });

      console.log("Ledger Response:", txResponse);

      if (txResponse.success) {
        setStatus(`Sucesso! TxHash: ${txResponse.txHash}`);
        setAmount("");
        setToAddress("");

        // Optimistic update: Increment nonce locally to prevent race conditions
        incrementNonce(activeProfile);

        // Refresh balance immediately (but data might be stale for a second)
        setTimeout(() => refresh(), 1000);
      } else {
        setStatus(`Falha: ${txResponse.errorMessage}`);
      }
    } catch (error) {
      console.error("Erro na transação:", error);
      setStatus("Erro ao processar transação.");
    }
  };

  return {
    toAddress,
    setToAddress,
    amount,
    setAmount,
    asset,
    setAsset,
    handleSend,
    status,
  };
}
