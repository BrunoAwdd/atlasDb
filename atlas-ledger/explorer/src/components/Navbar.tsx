import { Link, useLocation } from "react-router-dom";
import { LayoutDashboard, Activity, Ticket, Users } from "lucide-react";

export default function Navbar() {
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  const navItems = [
    { name: "Dashboard", path: "/", icon: LayoutDashboard },
    { name: "Transactions", path: "/transactions", icon: Activity },
    { name: "Assets", path: "/assets", icon: Ticket },
    // { name: "Blocks", path: "/blocks", icon: Box },
    { name: "Accounts", path: "/accounts", icon: Users },
    // { name: "Settings", path: "/settings", icon: Settings },
  ];

  return (
    <nav className="w-64 border-r border-border/50 bg-secondary/10 flex flex-col p-4 space-y-2 h-screen sticky top-0">
      <div className="px-4 py-6 mb-4">
        <h1 className="text-2xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-400 purple-600 to-white">
          1961 Scan
        </h1>
        <div className="text-xs text-muted-foreground mt-1">
          TESTNET Explorer
        </div>
      </div>

      {navItems.map((item) => (
        <Link
          key={item.path}
          to={item.path}
          className={`flex items-center space-x-3 px-4 py-3 rounded-xl transition-all ${
            isActive(item.path)
              ? "bg-blue-600/20 text-blue-400"
              : "text-muted-foreground hover:bg-secondary/20 hover:text-white"
          }`}
        >
          <item.icon size={18} />
          <span className="font-medium">{item.name}</span>
        </Link>
      ))}

      <div className="mt-auto px-4 py-4">
        <div className="p-4 bg-secondary/20 rounded-xl border border-secondary/50">
          <p className="text-xs text-muted-foreground uppercase font-bold mb-2">
            Network Status
          </p>
          <div className="flex items-center space-x-2 text-green-400">
            <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
            <span className="text-sm font-mono">1.2k TPS</span>
          </div>
        </div>
      </div>
    </nav>
  );
}
