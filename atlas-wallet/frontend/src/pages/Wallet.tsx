import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { AddressSection } from "@/components/AddressSection";

import { useWalletData } from "@/hooks/useWalletData";
import { useTransactionSender } from "@/hooks/useTransactionSender";
import { WalletHeader } from "@/components/wallet/WalletHeader";
import { SendTransactionForm } from "@/components/wallet/SendTransactionForm";
import { TransactionHistory } from "@/components/wallet/TransactionHistory";

function WalletView() {
  const navigate = useNavigate();
  const {
    wallet,
    history,
    status,
    setStatus,
    activeProfile,
    switchProfile,
    refresh,
    incrementNonce,
  } = useWalletData();
  const {
    toAddress,
    setToAddress,
    amount,
    setAmount,
    asset,
    setAsset,
    handleSend,
    status: txStatus,
  } = useTransactionSender({
    wallet,
    activeProfile,
    refresh,
    incrementNonce,
  });

  return (
    <div className="flex flex-col h-full bg-background text-foreground animate-in fade-in duration-500">
      <div className="bg-primary/10 text-[10px] text-center py-1 font-mono text-muted-foreground">
        {status}
      </div>
      <WalletHeader
        onLogout={() => {
          // Force reload to clear WASM memory completely
          window.location.href =
            window.location.origin + window.location.pathname;
        }}
        onOpenSidePanel={async () => {
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
      />

      <main className="flex-1 flex flex-col p-6 overflow-y-auto space-y-6">
        {wallet ? (
          <>
            <Tabs
              defaultValue="exposed"
              className="w-full"
              onValueChange={switchProfile}
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
                  Transfer
                </span>
              </div>
            </div>

            <SendTransactionForm
              toAddress={toAddress}
              onAddressChange={setToAddress}
              amount={amount}
              onAmountChange={setAmount}
              asset={asset}
              onAssetChange={setAsset}
              onSend={handleSend}
              status={txStatus}
            />

            <TransactionHistory
              history={history}
              activeProfile={activeProfile}
              wallet={wallet}
            />
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
            <p className="text-sm font-medium">No wallets found</p>
            <Button variant="outline" size="sm" onClick={() => navigate("/")}>
              Back to Home
            </Button>
          </div>
        )}
      </main>
    </div>
  );
}

export default WalletView;
