import { HashRouter, Routes, Route } from "react-router-dom";
import Home from "@/pages/Home";
import Wallet from "@/pages/Wallet";
import History from "@/pages/History";
import Settings from "@/pages/Settings";

function App() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/wallet" element={<Wallet />} />
        <Route path="/history" element={<History />} />
        <Route path="/settings" element={<Settings />} />
      </Routes>
    </HashRouter>
  );
}

export default App;
