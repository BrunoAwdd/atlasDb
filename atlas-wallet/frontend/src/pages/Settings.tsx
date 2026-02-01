import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { getAllVaults, loadVault, saveVault } from "@/utils/storage";
import {
  Copy,
  Download,
  Upload,
  CheckCircle2,
  AlertCircle,
  Eye,
  EyeOff,
  ArrowLeft,
} from "lucide-react";
import init, { load_vault } from "../pkg/atlas_wallet";
import wasmUrl from "../pkg/atlas_wallet_bg.wasm?url";

export default function Settings() {
  const navigate = useNavigate();
  const location = useLocation();
  const defaultTab = location.state?.defaultTab || "export";
  const [vaults, setVaults] = useState<string[]>([]);
  const [selectedVault, setSelectedVault] = useState("");
  const [exportData, setExportData] = useState("");
  const [importName, setImportName] = useState("");
  const [importData, setImportData] = useState("");
  const [status, setStatus] = useState<{
    message: string;
    type: "success" | "error" | "";
  }>({ message: "", type: "" });

  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);

  useEffect(() => {
    const loadVaults = async () => {
      const all = await getAllVaults();
      const names = Object.keys(all);
      setVaults(names);
      if (names.length > 0 && !selectedVault) {
        setSelectedVault(names[0]);
        setExportData(""); // Reset export data on init
      }
    };
    loadVaults();
  }, []);

  const refreshVaults = async () => {
    const all = await getAllVaults();
    const names = Object.keys(all);
    setVaults(names);
  };

  const handleExport = async () => {
    if (!selectedVault) return;
    if (!password) {
      setStatus({
        message: "Digite sua senha para desbloquear.",
        type: "error",
      });
      return;
    }

    try {
      const data = await loadVault(selectedVault);
      if (!data) {
        setStatus({ message: "Erro ao ler dados da carteira.", type: "error" });
        return;
      }

      // Verify Password with WASM
      try {
        await init(wasmUrl);
        const vaultBytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
        load_vault(password, vaultBytes); // Will throw if invalid

        // Success
        setExportData(data);
        setStatus({
          message: "Carteira desbloqueada! Cópia liberada.",
          type: "success",
        });
      } catch (e) {
        console.error(e);
        setStatus({ message: "Senha incorreta.", type: "error" });
        setExportData("");
      }
    } catch (e) {
      console.error(e);
      setStatus({ message: "Erro inesperado.", type: "error" });
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(exportData);
    setStatus({
      message: "Copiado para a área de transferência!",
      type: "success",
    });
  };

  const handleImport = async () => {
    if (!importName || !importData) {
      setStatus({
        message: "Preencha o nome e os dados da carteira.",
        type: "error",
      });
      return;
    }

    try {
      // Basic validation of base64
      try {
        atob(importData);
      } catch {
        setStatus({
          message: "Formato de dados inválido (não é Base64).",
          type: "error",
        });
        return;
      }

      await saveVault(importName, importData);
      setStatus({
        message: "Carteira importada com sucesso!",
        type: "success",
      });
      setImportName("");
      setImportData("");
      refreshVaults();
    } catch (e) {
      console.error(e);
      setStatus({ message: "Falha ao salvar carteira.", type: "error" });
    }
  };

  return (
    <div className="container mx-auto p-6 max-w-2xl animate-in fade-in duration-500">
      <div className="flex items-center gap-4 mb-6">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(-1)}
          className="text-muted-foreground hover:text-foreground"
          title="Voltar"
        >
          <ArrowLeft className="h-6 w-6" />
        </Button>
        <h1 className="text-3xl font-bold">Configurações</h1>
      </div>

      <Tabs defaultValue={defaultTab} className="space-y-6">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="export">Exportar Carteira</TabsTrigger>
          <TabsTrigger value="import">Importar Carteira</TabsTrigger>
        </TabsList>

        {/* --- EXPORT TAB --- */}
        <TabsContent value="export">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Download className="w-5 h-5 text-blue-500" />
                Backup & Exportação
              </CardTitle>
              <CardDescription>
                Selecione uma carteira local para visualizar seus dados
                criptografados. Salve este código em local seguro.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Selecionar Carteira</Label>
                <Select
                  value={selectedVault}
                  onValueChange={(v) => {
                    setSelectedVault(v);
                    setExportData("");
                    setPassword("");
                    setStatus({ message: "", type: "" });
                  }}
                >
                  <SelectTrigger>
                    <SelectValue placeholder="Escolha uma carteira" />
                  </SelectTrigger>
                  <SelectContent>
                    {vaults.map((v) => (
                      <SelectItem key={v} value={v}>
                        {v}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>Senha da Carteira</Label>
                <div className="relative">
                  <Input
                    type={showPassword ? "text" : "password"}
                    placeholder="••••••••"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="pr-10"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="absolute right-0 top-0 h-9 w-9 text-muted-foreground hover:text-foreground"
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

              <Button
                onClick={handleExport}
                className="w-full"
                disabled={!selectedVault}
              >
                Gerar Backup
              </Button>

              {exportData && (
                <div className="space-y-2 pt-4 border-t border-border">
                  <Label>Dados da Carteira (Base64)</Label>
                  <div className="relative">
                    <Textarea
                      readOnly
                      value={exportData}
                      className="min-h-[120px] font-mono text-xs bg-secondary/50 pr-12 resize-none"
                    />
                    <Button
                      size="icon"
                      variant="ghost"
                      className="absolute top-2 right-2 text-muted-foreground hover:text-foreground"
                      onClick={handleCopy}
                      title="Copiar"
                    >
                      <Copy className="w-4 h-4" />
                    </Button>
                  </div>
                  <p className="text-[10px] text-muted-foreground">
                    * Este código contém sua chave privada criptografada com sua
                    senha.
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* --- IMPORT TAB --- */}
        <TabsContent value="import">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Upload className="w-5 h-5 text-orange-500" />
                Restaurar Carteira
              </CardTitle>
              <CardDescription>
                Cole o código de backup (Base64) de uma carteira existente para
                restaurá-la neste dispositivo.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label>Nome da Nova Carteira</Label>
                <Input
                  placeholder="MinhaCarteiraRestaurada"
                  value={importName}
                  onChange={(e) => setImportName(e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label>Dados do Backup (Base64)</Label>
                <Textarea
                  placeholder="Cole aqui a string base64..."
                  value={importData}
                  onChange={(e) => setImportData(e.target.value)}
                  className="min-h-[100px] font-mono text-xs"
                />
              </div>

              <Button
                onClick={handleImport}
                className="w-full"
                variant="secondary"
              >
                Importar Carteira
              </Button>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* STATUS FEEDBACK */}
      {status.message && (
        <div
          className={`mt-6 p-4 rounded-lg flex items-center gap-3 ${
            status.type === "success"
              ? "bg-green-500/10 text-green-500 border border-green-500/20"
              : "bg-red-500/10 text-red-500 border border-red-500/20"
          }`}
        >
          {status.type === "success" ? (
            <CheckCircle2 className="w-5 h-5" />
          ) : (
            <AlertCircle className="w-5 h-5" />
          )}
          <span className="text-sm font-medium">{status.message}</span>
        </div>
      )}
    </div>
  );
}
