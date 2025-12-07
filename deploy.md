# Guia de Deployment do AtlasDB

Este guia explica como compilar e preparar o binário do AtlasDB para distribuição em **Linux (Manjaro), Windows e macOS**.

## 1. Estratégias de Build

Como você possui diferentes Sistemas Operacionais e usa `rocksdb` (que requer compilação nativa C++), **Cross-Compilation (compilação cruzada) é difícil**.

**Estratégia Recomendada:** Compile o binário **uma vez por família de Sistema Operacional**.

1.  Compile no Linux -> Use para o Manjaro.
2.  Compile em uma máquina Windows -> Copie o `.exe` para as outras 3 máquinas Windows.
3.  Compile no Mac -> Use para o Mac.

---

### A. Linux (Manjaro)

Compile na sua máquina de desenvolvimento atual.

```bash
# Compilar
cargo build --release -p atlas-node
```

- **Local do Binário**: `target/release/atlas-node`
- **Copiar para**: Máquina Manjaro.

---

### B. Windows (x4)

É altamente recomendado compilar **em uma das máquinas Windows** para evitar erros complexos de cross-compilation com o `rocksdb`.

1.  **Instale o Rust no Windows**: Baixe o `rustup-init.exe` em [rust-lang.org](https://www.rust-lang.org/tools/install).
2.  **Instale LLVM/Clang** (Necessário para o RocksDB):
    - Baixe e instale o LLVM em [releases.llvm.org](https://releases.llvm.org/).
    - Defina a variável de ambiente `LIBCLANG_PATH` se necessário.
3.  **Clone e Compile**:
    ```powershell
    git clone <seu-repo-url>
    cd atlasDb
    cargo build --release -p atlas-node
    ```
4.  **Distribua**:
    - **Local do Binário**: `target\release\atlas-node.exe`
    - **Copiar para**: Todas as 4 máquinas Windows.

**Alternativa: Cross-Compile do Linux (Avançado)**
_Requer `mingw-w64` e configuração cuidadosa do ambiente._

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu -p atlas-node
```

_Nota: Isso frequentemente falha devido à linkagem C++ do RocksDB. Se falhar, use o método "Compilar no Windows"._

---

### C. macOS (Opcional)

Se decidir manter o Mac, compile diretamente nele.

1.  **Instale o Rust**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2.  **Instale Xcode Command Line Tools**: `xcode-select --install`
3.  **Compile**:
    ```bash
    cargo build --release -p atlas-node
    ```

- **Local do Binário**: `target/release/atlas-node`

---

## 2. Preparar para Distribuição

Para cada nó, crie uma pasta (ex: `atlas_node/`) contendo:

1.  **Binário**: `atlas-node` (Linux/Mac) ou `atlas-node.exe` (Windows)
2.  **Configuração**: `config.json` (Único para cada nó)
3.  **Chaves**: `keys/keypair` (Único para cada nó)

### Dicas de Configuração para OS Misto

- **Caminhos**: No `config.json`, use barras normais `/` mesmo no Windows, ou barras duplas invertidas `\\`. O Rust lida com `/` corretamente no Windows.
  - Exemplo: `"data_dir": "data/db"` funciona em todos.
- **Endereços IP**: Garanta que todas as máquinas possam se "pingar". Você pode precisar liberar a porta (padrão `4001` para P2P, `50051` para API) no **Firewall do Windows**.

## 3. Executando o Nó

### Linux / Mac

```bash
chmod +x atlas-node
./atlas-node --config config.json
```

### Windows (PowerShell / CMD)

```powershell
.\atlas-node.exe --config config.json
```
