import { useEffect, useState } from 'react';
import { Sidebar } from '@components/layout/Sidebar';
import { ChatPanel } from '@components/layout/ChatPanel';
import { ToolPanel } from '@components/layout/ToolPanel';
import { ApprovalLayer } from '@components/approval/ApprovalLayer';
import { CommandPalette } from '@components/palette/CommandPalette';
import { SettingsModal } from '@components/settings/SettingsModal';
import { useAgentEvents } from '@hooks/useAgentEvents';
import { useApprovalEvents } from '@hooks/useApprovalEvents';
import { usePalette } from '@hooks/usePalette';
import { useConfigStore } from '@store/configStore';

export default function App() {
  useAgentEvents();
  useApprovalEvents();
  const palette = usePalette();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { load } = useConfigStore();
  useEffect(() => { load(); }, [load]);
  useEffect(() => {
    const h = () => setSettingsOpen(true);
    window.addEventListener('nexus:open-settings', h);
    return () => window.removeEventListener('nexus:open-settings', h);
  }, []);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-space-950 text-space-100">
      <Sidebar /><ChatPanel /><ToolPanel /><ApprovalLayer />
      <CommandPalette open={palette.isOpen} onClose={palette.close} />
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <div className="fixed bottom-5 right-5 z-40 flex flex-col gap-2">
        <button onClick={() => setSettingsOpen(true)} className="flex h-11 w-11 items-center justify-center rounded-xl border border-space-600/50 bg-space-800/60 text-space-400 shadow-lg backdrop-blur transition-all hover:scale-105 hover:text-aurora-400" title="Settings">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        </button>
        <button onClick={palette.open} className="flex h-11 w-11 items-center justify-center rounded-xl border border-space-600/50 bg-space-800/60 text-space-400 shadow-lg backdrop-blur transition-all hover:scale-105 hover:text-cyan-400" title="Command Palette (Ctrl+K)">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M21 21l-4.35-4.35"/><circle cx="11" cy="11" r="8"/></svg>
        </button>
      </div>
    </div>
  );
}
