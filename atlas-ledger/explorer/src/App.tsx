import { BrowserRouter as Router, Routes, Route } from "react-router-dom";
import Navbar from "./components/Navbar";
import Home from "./pages/Home";
import AddressDetail from "./pages/AddressDetail";
import TransactionDetail from "./pages/TransactionDetail";
import Assets from "./pages/Assets";
import Transactions from "./pages/Transactions";
import Accounts from "./pages/Accounts";

function App() {
  return (
    <Router>
      <div className="min-h-screen bg-background text-foreground flex font-sans">
        <Navbar />
        <main className="flex-1 p-8 overflow-y-auto h-screen">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/address/:address" element={<AddressDetail />} />
            <Route path="/tx/:hash" element={<TransactionDetail />} />
            <Route path="/assets" element={<Assets />} />
            <Route path="/transactions" element={<Transactions />} />
            <Route path="/accounts" element={<Accounts />} />
            {/* Fallback for undefined routes */}
            <Route path="*" element={<Home />} />
          </Routes>
        </main>
      </div>
    </Router>
  );
}

export default App;
