const STORAGE_KEY = "nimble_storage";

function getStorage(): any {
  if (typeof chrome !== "undefined" && chrome.storage?.local) {
    // OBS: Para usar com chrome.storage, isso precisaria ser assíncrono.
    // Aqui vamos manter localStorage como padrão.
    return new Promise((resolve) => {
      chrome.storage.local.get([STORAGE_KEY], (result) => {
        resolve(
          result[STORAGE_KEY]
            ? JSON.parse(result[STORAGE_KEY] as string)
            : { vaults: {} }
        );
      });
    });
  } else {
    const data = localStorage.getItem(STORAGE_KEY);
    return Promise.resolve(data ? JSON.parse(data) : { vaults: {} });
  }
}

function saveStorage(data: any): Promise<void> {
  if (typeof chrome !== "undefined" && chrome.storage?.local) {
    return new Promise((resolve) => {
      chrome.storage.local.set({ [STORAGE_KEY]: JSON.stringify(data) }, () =>
        resolve()
      );
    });
  } else {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    return Promise.resolve();
  }
}

export async function saveVault(name: string, base64: string): Promise<string> {
  try {
    const storage = await getStorage();
    storage.vaults[name] = base64;
    await saveStorage(storage);
    return "Carteira criada e salva com sucesso!";
  } catch (error: any) {
    return `Erro ao salvar a carteira: ${error.message}`;
  }
}
export async function loadVault(name: string): Promise<string | null> {
  const storage = await getStorage();
  return storage.vaults[name] || null;
}

export async function getAllVaults(): Promise<Record<string, string>> {
  const storage = await getStorage();
  return storage.vaults;
}

// ⚠️ CUIDADO: Apenas mantenha isso se realmente for necessário
export async function deleteVault(name: string): Promise<void> {
  const storage = await getStorage();
  delete storage.vaults[name];
  await saveStorage(storage);
}
