import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { PanelRight, Eye, EyeOff } from "lucide-react";
import { Input } from "@/components/ui/input";

import { useNavigate } from "react-router-dom";
import {
  saveVault,
  loadVault as loadStoredVault,
  getAllVaults,
} from "@/utils/storage";

import "../App.css";
import init, { create_vault, load_vault, get_data } from "../pkg/atlas_wallet";
import wasmUrl from "../pkg/atlas_wallet_bg.wasm?url";

export default function Home() {
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [vaultSession, setVaultSession] = useState(null);
  const [status, setStatus] = useState("");

  const [vaults, setVaults] = useState<string[]>([]);
  const [selectedVault, setSelectedVault] = useState<string>("");

  const navigate = useNavigate();

  useEffect(() => {
    const fetchVaults = async () => {
      const allVaults = await getAllVaults();
      const keys = Object.keys(allVaults);
      setVaults(keys);
      if (keys.length > 0) setSelectedVault(keys[0]);
    };
    fetchVaults();
  }, []);

  const handleCreateVault = async () => {
    try {
      if (!password) {
        setStatus("Digite a senha para desbloquear.");
        return;
      }
      setStatus("Carregando carteira...");
      await init(wasmUrl);

      const result = await create_vault(password);

      const vaultName = result.get("exposed_address") || "default_vault";
      const vaultData = result.get("encrypted");

      const vaultBytes =
        vaultData instanceof Uint8Array
          ? result
          : new Uint8Array(Object.values(vaultData));

      const base64 = btoa(String.fromCharCode(...vaultBytes));
      const saveMessage = await saveVault(vaultName, base64);

      setStatus(saveMessage);
    } catch (err) {
      console.error("Erro ao criar carteira:", err);
      setStatus("Erro ao criar carteira.");
    }
  };

  const handleLoadVault = async () => {
    try {
      setStatus("Carregando carteira...");
      await init(wasmUrl);

      const base64 = await loadStoredVault(selectedVault);

      if (!base64) {
        setStatus("Carteira não encontrada.");
        return;
      }

      const vaultBytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));

      try {
        load_vault(password, vaultBytes);
        const session = await get_data();
        setVaultSession(session);
        setStatus("Carteira carregada com sucesso!");
        navigate("/wallet");
      } catch (error) {
        console.error("Erro ao carregar carteira:", error);
        setStatus("Senha incorreta ou carteira inválida.");
      }
    } catch (err) {
      console.error("Erro geral ao carregar carteira:", err);
      setStatus("Erro inesperado ao carregar a carteira.");
    }
  };

  return (
    <div className="flex flex-col h-full bg-background text-foreground animate-in fade-in duration-500">
      {/* Header Section */}
      <header className="relative flex items-center justify-center p-6 pb-2">
        <Button
          variant="ghost"
          size="icon"
          className="absolute right-4 top-4 text-muted-foreground hover:text-foreground transition-colors"
          title="Abrir no painel lateral"
          onClick={async () => {
            // ... keep existing logic
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
        <div className="text-center">
          <h1 className="text-2xl font-bold tracking-tight">1961</h1>
          <p className="text-xs text-muted-foreground tracking-widest uppercase mt-1">
            Crypto Wallet
          </p>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex flex-col px-6 py-4 space-y-6 overflow-y-auto">
        {/* Helper visual or spacing */}
        <div className="flex-1 flex flex-col justify-center space-y-6">
          {vaults.length > 0 ? (
            <div className="space-y-2">
              <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Account
              </label>
              <div className="relative">
                <select
                  className="w-full appearance-none bg-secondary/50 border-0 rounded-lg px-4 py-3 text-sm font-medium focus:ring-2 focus:ring-primary transition-all outline-none"
                  value={selectedVault}
                  onChange={(e) => setSelectedVault(e.target.value)}
                >
                  {vaults.map((vaultName) => (
                    <option key={vaultName} value={vaultName}>
                      {vaultName}
                    </option>
                  ))}
                </select>
                <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none opacity-50">
                  <svg
                    width="10"
                    height="6"
                    viewBox="0 0 10 6"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M1 1L5 5L9 1" />
                  </svg>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-center py-8 opacity-50">
              <p className="text-sm">No wallets found</p>
            </div>
          )}

          <div className="space-y-2">
            <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Password
            </label>
            <div className="relative">
              <Input
                type={showPassword ? "text" : "password"}
                placeholder="••••••••"
                className="bg-secondary/50 border-0 h-11 focus-visible:ring-primary pr-10"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="absolute right-0 top-0 h-11 w-11 text-muted-foreground hover:text-foreground"
                onClick={() => setShowPassword(!showPassword)}
                tabIndex={-1}
              >
                {showPassword ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>

          <div className="pt-2">
            <Button
              onClick={handleLoadVault}
              className="w-full h-11 text-sm font-medium shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all"
            >
              Access Wallet
            </Button>

            <div className="mt-4 text-center">
              <Button
                variant="link"
                onClick={handleCreateVault}
                className="text-xs text-muted-foreground hover:text-primary"
              >
                Create New Wallet
              </Button>
            </div>
          </div>
        </div>

        {/* Status Messages */}
        {status && (
          <div className="p-3 rounded-lg bg-secondary/50 border border-border/50">
            <p className="text-xs text-center font-medium opacity-90">
              {status}
            </p>
          </div>
        )}

        {/* Debug Info (Optional, kept minimal) */}
        {vaultSession && (
          <div className="bg-secondary p-3 rounded-lg text-[10px] font-mono opacity-50 overflow-hidden">
            <p className="truncate">{JSON.stringify(vaultSession)}</p>
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="p-4 text-center border-t border-border/40">
        <p className="text-[10px] text-muted-foreground/60 font-medium">
          1961 BLOCKCHAIN v1.0 • SECURE ENCLAVE
        </p>
      </footer>
    </div>
  );
}
