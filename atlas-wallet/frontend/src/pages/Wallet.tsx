import { useState, useEffect } from "react";
import QRCode from "react-qr-code";
import { ledgerClient } from "@/lib/client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";

import { useNavigate } from "react-router-dom";
import {
  PanelRight,
  Copy,
  Check,
  QrCode,
  ArrowUpRight,
  ArrowDownLeft,
  LogOut,
} from "lucide-react";
import { get_data, sing_transfer, switch_profile } from "../pkg/atlas_wallet";

function WalletView() {
  const [wallet, setWallet] = useState<any>(null); // você pode definir o tipo melhor depois

  const [toAddress, setToAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [status, setStatus] = useState("");
  const [activeProfile, setActiveProfile] = useState<"exposed" | "hidden">(
    "exposed"
  );

  const [copied, setCopied] = useState(false);
  const [showQr, setShowQr] = useState(false);
  const [history, setHistory] = useState<any[]>([]);

  const navigate = useNavigate();

  useEffect(() => {
    fetchData();
  }, [activeProfile]);

  async function fetchData() {
    // --- gRPC Integration ---
    let session;
    try {
      // Assume WASM initialized by Home.tsx.
      // If we reload page here, state is lost anyway, so we must rely on Home to init and load.
      // Calling init() again here RESETS the WASM memory (wiping the session).
      // console.log("Initializing WASM in Wallet...");
      // await init(wasmUrl);

      console.log("Fetching data from WASM...");
      session = await get_data();
      console.log("Identity Loaded from WASM:", session);
    } catch (error) {
      console.error(
        "Error loading identity from WASM (not initialized?):",
        error
      );
      // If error (e.g. wasm not init), we assume no session.
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

      // ... rest of the logic

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

      // Set wallet with structured balances
      setWallet({
        exposed: {
          address: exposedAddr,
          balances: { BRL: exposedBalBRL, MOX: exposedBalMOX },
        },
        hidden: {
          address: hiddenAddr,
          balances: { BRL: hiddenBalBRL, MOX: hiddenBalMOX },
        },
      });
    } else {
      setStatus("Carteira vazia / Erro no formato.");
    }
  }

  const handleSend = async () => {
    try {
      setStatus("Assinando (WASM)...");

      // await init(wasmUrl); // Don't reset state!
      const password = prompt("Digite sua senha para assinar a transação");

      if (!password) {
        setStatus("Envio cancelado.");
        return;
      }

      // Define Memo constant to ensure Signature matches Verification
      const txMemo = "Web Wallet Transfer";

      // 1. Sign with WASM
      // Result is a Map: { id: "...", transfer: { transaction: {...}, signature: [...], public_key: [...] } }
      const result = await sing_transfer(
        toAddress,
        BigInt(amount),
        password,
        txMemo
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
        // The new WASM returns signature as Hex String (from sig2)
        if (typeof sigRaw === "string") sigIsHex = true;
      } else {
        // Fallback or Object structure
        if (result.transfer) {
          sigRaw = result.transfer.signature;
          pkRaw = result.transfer.public_key;
          if (typeof sigRaw === "string") sigIsHex = true;
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

      // Convert Public Key to Hex (it comes as byte array/vector from WASM)
      const publicKeyHex = toHex(pkRaw);

      // console.log("Sending Signature (Hex):", signatureHex);
      // console.log("Sending Public Key (Hex):", publicKeyHex);

      setStatus("Enviando (gRPC)...");

      // 2. Submit to Ledger
      const currentAddr =
        activeProfile === "exposed"
          ? wallet.exposed.address
          : wallet.hidden.address;

      const txResponse = await ledgerClient.SubmitTransaction({
        from: currentAddr,
        to: toAddress,
        amount: amount.toString(),
        asset: "BRL",
        memo: txMemo, // Use the SAME memo signed by WASM
        signature: signatureHex,
        public_key: publicKeyHex,
      });

      console.log("Ledger Response:", txResponse);

      if (txResponse.success) {
        setStatus(`Sucesso! TxHash: ${txResponse.txHash}`);
        setAmount("");
        setToAddress("");
        // Refresh balance immediately
        setTimeout(() => fetchData(), 1000); // Wait a bit for ledger processing?
      } else {
        setStatus(`Falha: ${txResponse.errorMessage}`);
      }
    } catch (error) {
      console.error("Erro na transação:", error);
      setStatus("Erro ao processar transação.");
    }
  };

  const handleSwitchProfile = async (val: string) => {
    if (val === activeProfile) return;

    // Na logica original isso togglava, mas com Tabs é melhor setar direto
    // Mas o WASM toggle pode ser stateful, entao cuidado.
    // Assumindo que switch_profile apenas alterna o estado interno do WASM
    // Idealmente deveriamos syncar com o valor da Tab.

    // Como a implementacao original era um toggle cego, vamos manter porem
    // tentar garantir consistencia ou apenas chamar se for diferente.
    // Se o WASM tem estado interno de "quem é o ativo", isso pode desincronizar.
    // Vou assumir que o handleSwitchProfile original era a unica via.

    const newProfile = val as "exposed" | "hidden";

    setStatus("Trocando perfil...");
    // await init(wasmUrl);
    await switch_profile();
    setStatus(`Perfil trocado para: ${newProfile} com sucesso!`);
    setActiveProfile(newProfile);
    setShowQr(false); // Reset QR on switch
  };

  const copyToClipboard = async (text: string) => {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
      setStatus("Erro ao copiar.");
    }
  };

  const AddressSection = ({ type, data }: { type: string; data: any }) => (
    <div className="space-y-4 animate-in fade-in duration-300">
      <div className="flex flex-col items-center justify-center py-6 bg-secondary/20 rounded-2xl border border-border/50 space-y-2">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-widest mb-1">
          Saldo {type === "exposed" ? "Total" : "Oculto"}
        </span>
        <div className="flex flex-col items-center">
          <h2 className="text-3xl font-bold tracking-tight">
            {data?.balances?.BRL || "0"}{" "}
            <span className="text-sm font-medium text-muted-foreground">
              BRL
            </span>
          </h2>
          <h2 className="text-xl font-bold tracking-tight text-muted-foreground/80">
            {data?.balances?.MOX || "0"}{" "}
            <span className="text-xs font-medium">MOX</span>
          </h2>
        </div>
      </div>

      <div className="bg-secondary/40 p-4 rounded-xl border border-border/50 space-y-3">
        <div className="flex justify-between items-center">
          <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">
            Endereço {type === "exposed" ? "Público" : "Privado"}
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
      </div>
    </div>
  );

  return (
    <div className="flex flex-col h-full bg-background text-foreground animate-in fade-in duration-500">
      {/* Header */}
      <header className="relative flex items-center justify-center p-6 border-b border-border/40">
        <Button
          variant="ghost"
          size="icon"
          className="absolute left-4 top-4 text-muted-foreground hover:text-foreground transition-colors"
          onClick={() => {
            // Force reload to clear WASM memory completely
            window.location.href =
              window.location.origin + window.location.pathname;
          }}
          title="Sair / Logout"
        >
          <LogOut className="h-5 w-5" />
        </Button>

        <h1 className="text-xl font-bold tracking-tight">Carteira</h1>

        <Button
          variant="ghost"
          size="icon"
          className="absolute right-4 top-4 text-muted-foreground hover:text-foreground transition-colors"
          title="Abrir no painel lateral"
          onClick={async () => {
            if (
              typeof chrome !== "undefined" &&
              chrome.sidePanel &&
              chrome.windows
            ) {
              const currentWindow = await chrome.windows.getCurrent();
              if (currentWindow.id) {
                await chrome.sidePanel.open({ windowId: currentWindow.id });
                window.close();
              }
            } else {
              setStatus("Disponível apenas na extensão.");
            }
          }}
        >
          <PanelRight className="h-5 w-5" />
        </Button>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex flex-col p-6 overflow-y-auto space-y-6">
        {wallet ? (
          <>
            {/* Profile Tabs */}
            <Tabs
              defaultValue="exposed"
              className="w-full"
              onValueChange={handleSwitchProfile}
            >
              <TabsList className="grid w-full grid-cols-2 mb-6 bg-secondary/60 p-1 h-11">
                <TabsTrigger
                  value="exposed"
                  className="text-xs font-semibold uppercase tracking-wider data-[state=active]:bg-background data-[state=active]:shadow-sm"
                >
                  Exposed
                </TabsTrigger>
                <TabsTrigger
                  value="hidden"
                  className="text-xs font-semibold uppercase tracking-wider data-[state=active]:bg-background data-[state=active]:shadow-sm"
                >
                  Hidden
                </TabsTrigger>
              </TabsList>

              {/* Account Details Card */}
              <div className="space-y-6">
                <TabsContent value="exposed" className="mt-0">
                  <AddressSection type="exposed" data={wallet.exposed} />
                </TabsContent>

                <TabsContent value="hidden" className="mt-0">
                  <AddressSection type="hidden" data={wallet.hidden} />
                </TabsContent>
              </div>
            </Tabs>

            <div className="relative py-2">
              <div className="absolute inset-0 flex items-center">
                <span className="w-full border-t border-border/60" />
              </div>
              <div className="relative flex justify-center text-xs uppercase">
                <span className="bg-background px-2 text-muted-foreground font-medium">
                  Transferir
                </span>
              </div>
            </div>

            {/* Send Form */}
            <div className="space-y-4">
              <div className="space-y-2">
                <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  Destinatário
                </label>
                <Input
                  placeholder="Endereço da carteira"
                  className="bg-secondary/50 border-0 h-11 focus-visible:ring-primary font-mono text-xs"
                  value={toAddress}
                  onChange={(e) => setToAddress(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  Valor
                </label>
                <div className="relative">
                  <Input
                    placeholder="0.00"
                    type="number"
                    className="bg-secondary/50 border-0 h-11 focus-visible:ring-primary pr-12"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                  />
                  <div className="absolute right-4 top-1/2 -translate-y-1/2 text-xs font-bold text-muted-foreground">
                    MOX
                  </div>
                </div>
              </div>

              <Button
                onClick={handleSend}
                className="w-full h-11 text-sm font-medium shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all mt-2"
              >
                Enviar Transação
              </Button>
            </div>

            {/* Transactions History Section */}
            <div className="space-y-4 pt-4">
              <div className="flex items-center justify-between">
                <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  Histórico Recente
                </h3>
                <Button
                  variant="link"
                  size="sm"
                  className="h-auto p-0 text-[10px] text-primary"
                  onClick={() => navigate("/history")}
                >
                  Ver tudo
                </Button>
              </div>

              <div className="space-y-3">
                <div className="space-y-3">
                  {history.length > 0 ? (
                    history.map((tx: any) => {
                      const currentAddress =
                        activeProfile === "exposed"
                          ? wallet.exposed.address
                          : wallet.hidden.address;
                      // Determine direction based on current active profile
                      // Note: If tx is internal, it might be relevant to both.
                      const isSender =
                        tx.from.includes(currentAddress) ||
                        tx.from === currentAddress;

                      return (
                        <div
                          key={tx.txHash}
                          className="flex items-center justify-between p-3 bg-secondary/30 rounded-xl border border-border/40 hover:bg-secondary/50 transition-colors cursor-default"
                        >
                          <div className="flex items-center gap-3">
                            <div
                              className={`w-8 h-8 rounded-full flex items-center justify-center ${
                                isSender
                                  ? "bg-red-500/10 text-red-500"
                                  : "bg-green-500/10 text-green-500"
                              }`}
                            >
                              {!isSender ? (
                                <ArrowDownLeft className="h-4 w-4" />
                              ) : (
                                <ArrowUpRight className="h-4 w-4" />
                              )}
                            </div>
                            <div className="overflow-hidden">
                              <p
                                className="text-xs font-medium truncate w-32"
                                title={tx.txHash}
                              >
                                {isSender ? "Enviado" : "Recebido"}
                              </p>
                              <p
                                className="text-[10px] text-muted-foreground truncate"
                                title={tx.txHash}
                              >
                                {tx.txHash.substring(0, 10)}...
                              </p>
                              <p className="text-[10px] text-muted-foreground">
                                {new Date(
                                  Number(tx.timestamp)
                                ).toLocaleString()}
                              </p>
                            </div>
                          </div>
                          <div className="text-right">
                            <p
                              className={`text-xs font-bold ${
                                !isSender ? "text-green-500" : "text-foreground"
                              }`}
                            >
                              {!isSender ? "+" : "-"}
                              {tx.amount} {tx.asset}
                            </p>
                            <p className="text-[10px] text-muted-foreground">
                              Confirmado
                            </p>
                          </div>
                        </div>
                      );
                    })
                  ) : (
                    <div className="text-center py-8 opacity-50 text-xs">
                      Sem transações recentes
                    </div>
                  )}
                </div>

                {/* Empty State Mock (Hidden for now, just example) */}
                {/* <div className="text-center py-8 opacity-50 text-xs">Sem transações recentes</div> */}
              </div>
            </div>

            {status && (
              <div className="p-3 rounded-lg bg-secondary/50 border border-border/50 mt-4">
                <p className="text-xs text-center font-medium opacity-90">
                  {status}
                </p>
              </div>
            )}
          </>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center p-8 text-center space-y-4 opacity-50">
            <div className="w-16 h-16 rounded-full bg-secondary flex items-center justify-center mb-2">
              <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="text-muted-foreground"
              >
                <circle cx="12" cy="12" r="10" />
                <path d="M12 8v4" />
                <path d="M12 16h.01" />
              </svg>
            </div>
            <p className="text-sm font-medium">Nenhuma carteira carregada</p>
            <Button variant="outline" size="sm" onClick={() => navigate("/")}>
              Voltar ao Início
            </Button>
          </div>
        )}
      </main>
    </div>
  );
}

export default WalletView;
