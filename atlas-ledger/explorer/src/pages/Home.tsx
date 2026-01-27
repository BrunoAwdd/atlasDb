import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Search, Activity, Users, Ticket } from "lucide-react"; // Fake icons

export default function Home() {
  const [query, setQuery] = useState("");
  const navigate = useNavigate();

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!query) return;

    // Basic heuristic detection
    if (query.startsWith("nbex") || query.startsWith("passivo:")) {
      navigate(`/address/${query}`);
    } else if (query.length >= 64 || query.length === 44) {
      // Hex or Base64 (approx)
      navigate(`/tx/${query}`);
    } else {
      // Default to address if unsure, or show error?
      // Let's assume address for short queries or internal names?
      navigate(`/address/${query}`);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center min-h-[80vh] space-y-8">
      <div className="text-center space-y-4">
        <h1 className="text-5xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-400 to-purple-600">
          Atlas Explorer
        </h1>
        <p className="text-muted-foreground text-lg">
          The Atlas Chain Block Explorer
        </p>
      </div>

      <div className="w-full max-w-2xl">
        <form onSubmit={handleSearch} className="relative group">
          <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
            <Search className="h-5 w-5 text-gray-400 group-focus-within:text-blue-500 transition-colors" />
          </div>
          <input
            type="text"
            className="w-full pl-12 pr-4 py-4 bg-secondary/30 border border-border/50 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none transition-all text-lg shadow-lg backdrop-blur-sm"
            placeholder="Search by Address, Transaction Hash, Block..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </form>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 w-full max-w-4xl mt-12">
        <Link
          to="/transactions"
          className="p-6 rounded-xl bg-secondary/20 border border-border/50 hover:bg-secondary/40 transition-all group"
        >
          <div className="flex items-center space-x-4">
            <div className="p-3 bg-blue-500/10 rounded-lg text-blue-500 group-hover:bg-blue-500/20">
              <Activity size={24} />
            </div>
            <div>
              <h3 className="font-semibold text-lg">Latest Transactions</h3>
              <p className="text-sm text-muted-foreground">
                View real-time chain activity
              </p>
            </div>
          </div>
        </Link>

        <Link
          to="/accounts"
          className="p-6 rounded-xl bg-secondary/20 border border-border/50 hover:bg-secondary/40 transition-all group"
        >
          <div className="flex items-center space-x-4">
            <div className="p-3 bg-purple-500/10 rounded-lg text-purple-500 group-hover:bg-purple-500/20">
              <Users size={24} />
            </div>
            <div>
              <h3 className="font-semibold text-lg">Rich List</h3>
              <p className="text-sm text-muted-foreground">Top token holders</p>
            </div>
          </div>
        </Link>

        <Link
          to="/assets"
          className="p-6 rounded-xl bg-secondary/20 border border-border/50 hover:bg-secondary/40 transition-all group"
        >
          <div className="flex items-center space-x-4">
            <div className="p-3 bg-green-500/10 rounded-lg text-green-500 group-hover:bg-green-500/20">
              <Ticket size={24} />
            </div>
            <div>
              <h3 className="font-semibold text-lg">Token Registry</h3>
              <p className="text-sm text-muted-foreground">
                Verified assets list
              </p>
            </div>
          </div>
        </Link>
      </div>
    </div>
  );
}
