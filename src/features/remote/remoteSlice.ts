import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { servers } from "./remoteSamples";

interface Connection {
  id: string;
  serverId: string;
  connected: boolean;
  alias: string;
  host: string;
  port: number;
  user: string;
  authType: string;
  lastConnected: string;
}

const connections: Connection[] = [
  { id: "ssh-staging", serverId: "staging", connected: true, alias: "Staging", host: "staging.example.invalid",
    port: 22, user: "demo", authType: "SSH Agent", lastConnected: "2026-11-13 09:40 UTC" },
  { id: "ssh-build", serverId: "build", connected: false, alias: "Build runner", host: "build.example.invalid",
    port: 2222, user: "runner-demo", authType: "密钥", lastConnected: "2026-11-12 11:30 UTC" },
];

interface Transfer { id: number; hostId: string; file: string; completed: boolean }
interface Session { id: number; hostId: string; lines: { command: string; reply: string }[] }
const sessions: Session[] = servers.map((host, index) => ({ id: index + 1, hostId: host.id, lines: [] }));
const transfers: Transfer[] = [];

const slice = createSlice({
  name: "remote",
  initialState: { servers, connections, sessions, transfers, nextSession: sessions.length + 1, nextTransfer: 1 },
  reducers: {
    setConnection(state, { payload }: PayloadAction<{ id: string; connected: boolean; at: string }>) {
      const connection = state.connections.find(item => item.id === payload.id);
      if (!connection) return;
      connection.connected = payload.connected;
      if (connection.connected) connection.lastConnected = payload.at;
    },
    enqueueTransfer(state, { payload }: PayloadAction<{ hostId: string; file: string }>) {
      if (state.connections.some(item => item.serverId === payload.hostId && item.connected)) {
        state.transfers.push({ ...payload, id: state.nextTransfer++, completed: false });
      }
    },
    completeTransfer(state, { payload }: PayloadAction<number>) {
      const transfer = state.transfers.find(item => item.id === payload);
      if (transfer && state.connections.some(item => item.serverId === transfer.hostId && item.connected)) transfer.completed = true;
    },
    newSession(state, { payload }: PayloadAction<string>) {
      if (state.connections.some(item => item.serverId === payload && item.connected)) {
        state.sessions.push({ id: state.nextSession++, hostId: payload, lines: [] });
      }
    },
    sendCommand(state, { payload }: PayloadAction<{ sessionId: number; command: string }>) {
      const session = state.sessions.find(item => item.id === payload.sessionId);
      const host = state.servers.find(item => item.id === session?.hostId);
      const command = payload.command.trim();
      if (!session || !host || !command || !state.connections.some(item => item.serverId === host.id && item.connected)) return;
      let reply: string;
      switch (command) {
        case "help": reply = "可用示例：help、pwd、hostname、ls"; break;
        case "pwd": reply = host.cwd; break;
        case "hostname": reply = host.name; break;
        case "ls": reply = host.files.map(file => file.name + (file.children ? "/" : "")).join("  "); break;
        default: reply = "没有对应的预设回显。输入 help 查看示例。";
      }
      session.lines.push({ command, reply });
    },
  },
});

export const { setConnection, enqueueTransfer, completeTransfer, newSession, sendCommand } = slice.actions;
export const remoteReducer = slice.reducer;
