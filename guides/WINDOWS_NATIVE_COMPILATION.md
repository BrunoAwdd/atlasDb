# Compilação Nativa no Windows (Plano C)

Se a versão cross-compiled (gerada no Linux) não funcionar, o caminho mais seguro é compilar o projeto diretamente na máquina Windows onde ele vai rodar. Isso garante compatibilidade total com o processador e o sistema operacional.

## Pré-requisitos

Você precisará instalar as seguintes ferramentas no Windows:

### 1. Rust Lang

Baixe e instale o `rustup-init.exe` do site oficial:

- [https://rustup.rs/](https://rustup.rs/)
- Durante a instalação, se ele perguntar sobre o "Microsoft C++ Build Tools", aceite e instale (é necessário para o Linker).

### 2. Git

Baixe e instale o Git for Windows:

- [https://git-scm.com/download/win](https://git-scm.com/download/win)

### 3. LLVM (Crucial para RocksDB)

O `rocksdb` precisa do LLVM/Clang para compilar o código C++ no Windows.

- Baixe o instalador: [https://github.com/llvm/llvm-project/releases](https://github.com/llvm/llvm-project/releases) (Procure por `LLVM-xx.x.x-win64.exe`)
- **IMPORTANTE:** Durante a instalação, marque a opção **"Add LLVM to the system PATH"** for all users.

## Passo a Passo

1.  **Clone o Projeto:**
    Abra o PowerShell ou Git Bash e baixe o código:

    ```powershell
    git clone https://github.com/SeuUsuario/atlasDb.git
    cd atlasDb
    ```

2.  **Configure o LLVM (Se precisar):**
    As vezes o Rust não acha o LLVM sozinho. Defina a variável:

    ```powershell
    $env:LIBCLANG_PATH="C:\Program Files\LLVM\bin"
    ```

3.  **Compile:**

    ```powershell
    cargo build --release -p atlas-node
    ```

    _Isso vai demorar uns 10-20 minutos na primeira vez._

4.  **Execute:**
    O executável será criado em `target/release/atlas-node.exe`.
    Você pode copiar o `start_windows_node.bat` para essa pasta `target/release` e rodar ele dali.

## Erros Comuns

- **"Is your clang valid?"**: Significa que o LLVM não está instalado ou o PATH não foi configurado. Reinicie o terminal ou instale o LLVM novamente marcando a opção de PATH.
- **"linker `link.exe` not found"**: Falta o "C++ Build Tools" do Visual Studio. Rode o instalador do Rustup de novo ou baixe o "Build Tools for Visual Studio" na Microsoft.
