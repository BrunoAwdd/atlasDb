const express = require("express");
const cors = require("cors");
const bodyParser = require("body-parser");
const { spawn, exec } = require("child_process");
const path = require("path");

const app = express();
app.use(cors());
app.use(bodyParser.json());

const PORT = 3001;
const PROJECT_ROOT = path.resolve(__dirname, "../../");

// Node Configuration
const nodes = [
  {
    id: 1,
    name: "Node 1",
    command: "./target/debug/atlas-node",
    args: [
      "--listen",
      "/ip4/127.0.0.1/tcp/4001",
      "--grpc-port",
      "50081",
      "--config",
      "example/node1/config.json",
      "--keypair",
      "example/node1/keypair",
    ],
    process: null,
    status: "stopped",
  },
  {
    id: 2,
    name: "Node 2",
    command: "./target/debug/atlas-node",
    args: [
      "--listen",
      "/ip4/127.0.0.1/tcp/4002",
      "--grpc-port",
      "50082",
      "--config",
      "example/node2/config.json",
      "--keypair",
      "example/node2/keypair",
    ],
    process: null,
    status: "stopped",
  },
  {
    id: 3,
    name: "Node 3",
    command: "./target/debug/atlas-node",
    args: [
      "--listen",
      "/ip4/127.0.0.1/tcp/4003",
      "--grpc-port",
      "50083",
      "--config",
      "example/node3/config.json",
      "--keypair",
      "example/node3/keypair",
    ],
    process: null,
    status: "stopped",
  },
  {
    id: 4,
    name: "Node 4",
    command: "./target/debug/atlas-node",
    args: [
      "--listen",
      "/ip4/127.0.0.1/tcp/4004",
      "--grpc-port",
      "50084",
      "--config",
      "example/node4/config.json",
      "--keypair",
      "example/node4/keypair",
    ],
    process: null,
    status: "stopped",
  },
];

// Helper to update status based on actual PID check
const checkStatus = () => {
  nodes.forEach((node) => {
    if (node.process) {
      if (node.process.exitCode !== null) {
        node.status = "stopped";
        node.process = null;
      } else {
        node.status = "running";
      }
    }
  });
};

// API: Get Nodes
app.get("/api/nodes", (req, res) => {
  checkStatus();
  const nodeList = nodes.map((n) => ({
    id: n.id,
    name: n.name,
    status: n.status,
    pid: n.process ? n.process.pid : null,
  }));
  res.json(nodeList);
});

// API: Start Node
app.post("/api/nodes/:id/start", (req, res) => {
  const id = parseInt(req.params.id);
  const node = nodes.find((n) => n.id === id);

  if (!node) return res.status(404).json({ error: "Node not found" });
  if (node.process && node.process.exitCode === null) {
    return res.status(400).json({ error: "Node already running" });
  }

  console.log(`Starting ${node.name}...`);

  const fs = require("fs");
  const logDir = path.join(PROJECT_ROOT, "tools/dashboard/logs");
  if (!fs.existsSync(logDir)) {
    fs.mkdirSync(logDir, { recursive: true });
  }

  const out = fs.openSync(path.join(logDir, `node_${id}.log`), "a");
  const err = fs.openSync(path.join(logDir, `node_${id}.err`), "a");

  // Spawn process
  const child = spawn(node.command, node.args, {
    cwd: PROJECT_ROOT,
    stdio: ["ignore", out, err], // Redirect to files
  });

  node.process = child;
  node.status = "running";

  child.on("close", (code) => {
    console.log(`${node.name} exited with code ${code}`);
    node.status = "stopped";
    node.process = null;
  });

  res.json({ message: `${node.name} started`, pid: child.pid });
});

// API: Stop Node
app.post("/api/nodes/:id/stop", (req, res) => {
  const id = parseInt(req.params.id);
  const node = nodes.find((n) => n.id === id);

  if (!node) return res.status(404).json({ error: "Node not found" });
  if (!node.process) return res.status(400).json({ error: "Node not running" });

  console.log(`Stopping ${node.name}...`);
  node.process.kill(); // SIGTERM
  node.status = "stopped";
  // node.process = null; // Will be cleared in 'close' event

  res.json({ message: `${node.name} stopping...` });
});

// API: Submit Proposal
app.post("/api/proposals", (req, res) => {
  const { content } = req.body;
  if (!content) return res.status(400).json({ error: "Content required" });

  console.log(`Submitting proposal: ${content}`);

  // Use CLI to submit
  const cmd = `cargo run -p atlas-core --bin cli -- http://127.0.0.1:50081 "${content}"`;

  exec(cmd, { cwd: PROJECT_ROOT }, (error, stdout, stderr) => {
    if (error) {
      console.error(`CLI Error: ${error.message}`);
      return res.status(500).json({ error: error.message, stderr });
    }
    res.json({ message: "Proposal submitted", output: stdout });
  });
});

// API: Get Node Status
app.get("/api/nodes/:id/status", (req, res) => {
  const id = parseInt(req.params.id);
  const node = nodes.find((n) => n.id === id);

  if (!node) return res.status(404).json({ error: "Node not found" });

  // Extract port from args (e.g., --grpc-port 50081)
  const portIndex = node.args.indexOf("--grpc-port");
  if (portIndex === -1)
    return res.status(500).json({ error: "GRPC port not configured" });
  const port = node.args[portIndex + 1];
  const url = `http://127.0.0.1:${port}`;

  const cmd = `cargo run -p atlas-core --bin cli -- status ${url}`;

  exec(cmd, { cwd: PROJECT_ROOT }, (error, stdout, stderr) => {
    if (error) {
      // console.error(`CLI Error: ${error.message}`);
      return res.status(500).json({ error: error.message, stderr });
    }

    // Parse output
    // Node ID: 1
    // Leader ID: 1
    // Height: 10
    // View: 0
    const lines = stdout.split("\n");
    const status = {};
    lines.forEach((line) => {
      if (line.startsWith("Node ID:"))
        status.nodeId = line.split(":")[1].trim();
      if (line.startsWith("Leader ID:"))
        status.leaderId = line.split(":")[1].trim();
      if (line.startsWith("Height:")) status.height = line.split(":")[1].trim();
      if (line.startsWith("View:")) status.view = line.split(":")[1].trim();
    });

    res.json(status);
  });
});

app.listen(PORT, () => {
  console.log(`Dashboard Backend running on http://localhost:${PORT}`);
});
