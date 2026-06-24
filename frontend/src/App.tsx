import { useEffect } from 'react';
import { Sidebar } from '@components/layout/Sidebar';
import { ChatPanel } from '@components/layout/ChatPanel';
import { ToolPanel } from '@components/layout/ToolPanel';
import { ApprovalLayer } from '@components/approval/ApprovalLayer';
import { CommandPalette } from '@components/palette/CommandPalette';
import { useAgentEvents } from '@hooks/useAgentEvents';
import { useApprovalEvents } from '@hooks/useApprovalEvents';
import { usePalette } from '@hooks/usePalette';
import { useConfigStore } from '@store/configStore';

export default function App() {
  // Wire up IPC event listeners
  useAgentEvents();
  useApprovalEvents();

  // Command Palette (Cmd+K / Ctrl+K)
  const palette = usePalette();

  const { load } = useConfigStore();
  useEffect(() => {
    load();
  }, [load]);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-nexus-950 text-nexus-100">
      <Sidebar />
      <ChatPanel />
      <ToolPanel />
      <ApprovalLayer />

      {/* Command Palette — Cmd+K / Ctrl+K */}
      <CommandPalette open={palette.isOpen} onClose={palette.close} />

      {/* Hint badge ở góc dưới phải */}
      {!palette.isOpen && (
        <button
          onClick={palette.open}
          className="fixed bottom-4 right-4 z-40 flex items-center gap-2 rounded-full border border-nexus-700 bg-nexus-900/80 px-3 py-1.5 text-2xs text-nexus-400 shadow-lg backdrop-blur hover:bg-nexus-800 hover:text-nexus-200 transition-colors"
          title="Open command palette"
        >
          <span>⌘K</span>
          <span>Quick actions</span>
        </button>
      )}
    </div>
  );
}
