import { useState } from "react";
import { ledgerClient } from "@/lib/client";
import { sing_transfer } from "../pkg/atlas_wallet";
import { getFullAssetId } from "@/lib/assets";
import type { WalletData } from "./useWalletData";

interface TransactionData {
  from: string;
  nonce: bigint | number;
  timestamp: bigint | number;
  [key: string]: any;
}

interface TransferData {
  transaction: TransactionData | Map<string, any>;
  signature: string | Uint8Array;
  public_key: string | Uint8Array;
  [key: string]: any;
}

interface WasmResult {
  transfer?: TransferData;
  get?: (key: string) => any;
  [key: string]: any;
}

export function useTransactionSender({
  wallet,
  activeProfile,
  refresh,
  incrementNonce,
}: {
  wallet: WalletData | null;
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
      const fullAssetId = getFullAssetId(asset);

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
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const result: WasmResult | Map<string, any> = await sing_transfer(
        toAddress,
        BigInt(amount),
        fullAssetId,
        txMemo,
        nextNonce,
      );

      console.log("Assinatura gerada (RAW):", result);

      // Helper function for bytes to hex
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const toHex = (bytes: any) => {
        if (!bytes) return "";
        // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call
        const arr = Array.from(bytes) as number[];
        return arr.map((b) => b.toString(16).padStart(2, "0")).join("");
      };

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      let sigRaw: any, pkRaw: any;
      let sigIsHex = false;
      let pkIsHex = false;

      // Handle Map (wasm-bindgen default) or Object
      if (result instanceof Map) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        const transfer = result.get("transfer");
        if (transfer instanceof Map) {
          sigRaw = transfer.get("signature");
          pkRaw = transfer.get("public_key");
        } else {
          // WASM might return struct with direct fields
          // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
          sigRaw = transfer.signature;
          // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
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
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        signatureHex = sigRaw;
      } else {
        signatureHex = toHex(sigRaw);
      }

      let publicKeyHex = "";
      if (pkIsHex) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        publicKeyHex = pkRaw;
      } else {
        publicKeyHex = toHex(pkRaw);
      }

      setStatus("Enviando (gRPC)...");

      // 2. Submit to Ledger
      // CRITICAL: Use the 'from' address that was ACTUALLY signed by the WASM module.
      let currentAddr = "";
      let nonceStr = "0";
      let timestampStr = "0";

      if (result instanceof Map) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        const transfer = result.get("transfer");
        // Check if transaction is a Map or Object
        const transaction =
          transfer instanceof Map
            ? transfer.get("transaction")
            : // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
              transfer.transaction;

        if (transaction instanceof Map) {
          currentAddr = transaction.get("from");
          nonceStr = transaction.get("nonce").toString();
          timestampStr = transaction.get("timestamp").toString();
        } else {
          // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
          currentAddr = transaction.from;
          // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call
          nonceStr = transaction.nonce.toString();
          // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call
          timestampStr = transaction.timestamp.toString();
        }
      } else {
        // Object structure
        const tx = result.transfer!.transaction;

        if (tx instanceof Map) {
          currentAddr = tx.get("from");
          nonceStr = tx.get("nonce").toString();
          timestampStr = tx.get("timestamp").toString();
        } else {
          currentAddr = (tx as TransactionData).from;
          nonceStr = (tx as TransactionData).nonce.toString();
          timestampStr = (tx as TransactionData).timestamp.toString();
        }
      }

      if (!currentAddr) {
        // Fallback to state if something went wrong parsing (should not happen)
        console.warn(
          "Could not extract 'from' from signed result, falling back to activeProfile state.",
        );
        currentAddr =
          activeProfile === "exposed"
            ? // eslint-disable-next-line @typescript-eslint/no-non-null-asserted-optional-chain
              wallet?.exposed.address!
            : // eslint-disable-next-line @typescript-eslint/no-non-null-asserted-optional-chain
              wallet?.hidden.address!;
      }

      console.log("Submitting Transaction FROM:", currentAddr);

      const txResponse = await ledgerClient.SubmitTransaction({
        from: currentAddr,
        to: toAddress,
        amount: amount.toString(),
        asset: fullAssetId, // Use selected asset
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
