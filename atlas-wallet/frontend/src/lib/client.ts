import { LedgerServiceClientImpl } from "../proto/ledger";
import { GrpcWebRpc } from "./rpc";

// In production this should be env var
const LEDGER_URL = import.meta.env.VITE_LEDGER_URL || "http://localhost:50051";

export const rpc = new GrpcWebRpc(LEDGER_URL);
export const ledgerClient = new LedgerServiceClientImpl(rpc);
