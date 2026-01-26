import { useState, useEffect } from "react";
import axios from "axios";

const API_URL = "http://localhost:3001/api";

function App() {
  const [nodes, setNodes] = useState([]);
  const [proposal, setProposal] = useState("");
  const [logs, setLogs] = useState([]);

  const fetchNodes = async () => {
    try {
      const res = await axios.get(`${API_URL}/nodes`);
      setNodes((prev) => {
        // Merge new data with existing state to preserve 'details'
        return res.data.map((newNode) => {
          const existing = prev.find((p) => p.id === newNode.id);
          return existing ? { ...newNode, details: existing.details } : newNode;
        });
      });
    } catch (err) {
      console.error(err);
    }
  };

  const fetchStatus = async (nodeId) => {
    try {
      const res = await axios.get(`${API_URL}/nodes/${nodeId}/status`);
      setNodes((prev) =>
        prev.map((n) => (n.id === nodeId ? { ...n, details: res.data } : n))
      );
    } catch (err) {
      console.error(`Failed to fetch status for node ${nodeId}`, err);
    }
  };

  useEffect(() => {
    fetchNodes();
    const interval = setInterval(() => {
      fetchNodes();
      // Poll status for running nodes
      nodes.forEach((node) => {
        if (node.status === "running") {
          fetchStatus(node.id);
        }
      });
    }, 2000); // Poll every 2s
    return () => clearInterval(interval);
  }, [nodes.length]); // Re-bind if nodes list changes length (unlikely but safe)

  const handleStart = async (id) => {
    try {
      await axios.post(`${API_URL}/nodes/${id}/start`);
      fetchNodes();
      addLog(`Node ${id} starting...`);
    } catch (err) {
      addLog(
        `Error starting Node ${id}: ${err.response?.data?.error || err.message}`
      );
    }
  };

  const handleStop = async (id) => {
    try {
      await axios.post(`${API_URL}/nodes/${id}/stop`);
      fetchNodes();
      addLog(`Node ${id} stopping...`);
    } catch (err) {
      addLog(
        `Error stopping Node ${id}: ${err.response?.data?.error || err.message}`
      );
    }
  };

  const handleSubmitProposal = async (e) => {
    e.preventDefault();
    if (!proposal) return;
    try {
      addLog(`Submitting proposal: "${proposal}"...`);
      const res = await axios.post(`${API_URL}/proposals`, {
        content: proposal,
      });
      addLog(`Proposal result: ${res.data.output}`);
      setProposal("");
    } catch (err) {
      addLog(
        `Error submitting proposal: ${err.response?.data?.error || err.message}`
      );
    }
  };

  const addLog = (msg) => {
    setLogs((prev) => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev]);
  };

  return (
    <div className="min-h-screen bg-gray-900 text-white p-8 font-sans">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-3xl font-bold mb-8 text-blue-400">
          AtlasDB Cluster Dashboard
        </h1>

        {/* Nodes Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          {nodes.map((node) => (
            <div
              key={node.id}
              className={`p-6 rounded-xl border ${
                node.status === "running"
                  ? "border-green-500 bg-green-900/20"
                  : "border-red-500 bg-red-900/20"
              } transition-all`}
            >
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-semibold">{node.name}</h2>
                <div className="flex items-center gap-2">
                  {node.details &&
                    node.details.leaderId &&
                    node.details.nodeId === node.details.leaderId && (
                      <span className="bg-yellow-500 text-black text-xs font-bold px-2 py-1 rounded flex items-center gap-1">
                        <span>👑</span> LEADER
                      </span>
                    )}
                  <span
                    className={`w-3 h-3 rounded-full ${
                      node.status === "running"
                        ? "bg-green-500 animate-pulse"
                        : "bg-red-500"
                    }`}
                  ></span>
                </div>
              </div>
              <div className="text-sm text-gray-400 mb-4">
                <div className="mb-2">
                  Status:{" "}
                  <span
                    className={
                      node.status === "running"
                        ? "text-green-400"
                        : "text-red-400"
                    }
                  >
                    {node.status.toUpperCase()}
                  </span>
                </div>
                {node.pid && (
                  <div className="text-xs mb-2">PID: {node.pid}</div>
                )}
                {node.details && node.status === "running" && (
                  <div className="mt-2 pt-2 border-t border-gray-700">
                    <div className="text-xs text-yellow-400">
                      Leader ID: {node.details.leaderId}
                    </div>
                    <div className="text-xs text-blue-300">
                      Height: {node.details.height}
                    </div>
                    <div className="text-xs text-purple-300">
                      View: {node.details.view}
                    </div>
                  </div>
                )}
              </div>
              <div className="flex gap-2">
                {node.status !== "running" ? (
                  <button
                    onClick={() => handleStart(node.id)}
                    className="flex-1 bg-green-600 hover:bg-green-700 text-white py-2 rounded transition-colors"
                  >
                    Start
                  </button>
                ) : (
                  <button
                    onClick={() => handleStop(node.id)}
                    className="flex-1 bg-red-600 hover:bg-red-700 text-white py-2 rounded transition-colors"
                  >
                    Stop
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>

        {/* Proposal Section */}
        <div className="bg-gray-800 p-6 rounded-xl border border-gray-700 mb-8">
          <h2 className="text-xl font-semibold mb-4 text-purple-400">
            Submit Proposal
          </h2>
          <form onSubmit={handleSubmitProposal} className="flex gap-4">
            <input
              type="text"
              value={proposal}
              onChange={(e) => setProposal(e.target.value)}
              placeholder="Enter proposal content..."
              className="flex-1 bg-gray-900 border border-gray-600 rounded px-4 py-2 text-white focus:outline-none focus:border-blue-500"
            />
            <button
              type="submit"
              className="bg-purple-600 hover:bg-purple-700 text-white px-6 py-2 rounded font-medium transition-colors"
            >
              Submit
            </button>
          </form>
        </div>

        {/* Logs Section */}
        <div className="bg-gray-800 p-6 rounded-xl border border-gray-700">
          <h2 className="text-xl font-semibold mb-4 text-gray-400">
            Dashboard Logs
          </h2>
          <div className="bg-black rounded p-4 h-64 overflow-y-auto font-mono text-sm text-gray-300">
            {logs.length === 0 ? (
              <div className="text-gray-600 italic">No logs yet...</div>
            ) : (
              logs.map((log, i) => (
                <div
                  key={i}
                  className="mb-1 border-b border-gray-800 pb-1 last:border-0"
                >
                  {log}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
